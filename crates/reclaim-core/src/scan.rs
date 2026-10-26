//! Descriptor-anchored, metadata-only traversal. No file content is opened.

use rustix::fs::{self, AtFlags, Dir, Mode, OFlags, Stat};
use serde::Serialize;
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io;
use std::os::unix::ffi::OsStringExt;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

pub const MAX_ROOTS: usize = 16;
pub const MAX_ENTRIES: usize = 100_000;
pub const MAX_DEPTH: usize = 128;
pub const MAX_PAGE: usize = 200;
pub const MAX_DISPLAY: usize = 1_024;

pub fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

pub fn display(value: &OsStr) -> String {
    value
        .to_string_lossy()
        .chars()
        .take(MAX_DISPLAY)
        .map(|character| {
            if character.is_control()
                || matches!(character, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
            {
                '�'
            } else {
                character
            }
        })
        .collect()
}

/// JSON byte quantities are decimal strings, avoiding JavaScript precision loss.
mod bytes {
    use serde::Serializer;
    pub fn serialize<S: Serializer>(value: &u64, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&value.to_string())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Issue {
    PermissionDenied,
    Vanished,
    Io,
    Changed,
    MountBoundary,
    DepthLimit,
    EntryLimit,
    Cancelled,
    WorkerFailed,
}

pub fn io_issue(error: &io::Error) -> Issue {
    match error.kind() {
        io::ErrorKind::PermissionDenied => Issue::PermissionDenied,
        io::ErrorKind::NotFound => Issue::Vanished,
        _ => Issue::Io,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Directory,
    File,
    Symlink,
    Other,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Running,
    Complete,
    Partial,
    Cancelled,
    Failed,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub id: String,
    pub parent_id: Option<String>,
    pub display_name: String,
    pub kind: Kind,
    #[serde(with = "bytes")]
    pub logical_bytes: u64,
    #[serde(with = "bytes")]
    pub allocated_bytes: u64,
    pub measured_at_ms: u64,
    pub modified_at_ms: Option<u64>,
    pub identity: Option<String>,
    pub alias_of: Option<String>,
    pub complete: bool,
    pub issue: Option<Issue>,
}

/// Native names remain lossless, never round-tripped through display strings.
pub struct Record {
    pub finding: Finding,
    pub name: OsString,
    pub parent: Option<usize>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub generation: u32,
    pub status: Status,
    pub started_at_ms: u64,
    pub finished_at_ms: Option<u64>,
    pub entries: usize,
    pub issue_count: usize,
    pub terminal_issue: Option<Issue>,
    pub volumes: Vec<VolumeAccounting>,
    #[serde(with = "bytes")]
    pub logical_bytes: u64,
    #[serde(with = "bytes")]
    pub allocated_bytes: u64,
}

pub struct ScanData {
    pub progress: Progress,
    pub records: Vec<Record>,
}

impl ScanData {
    pub fn new(generation: u32) -> Self {
        Self {
            progress: Progress {
                generation,
                status: Status::Running,
                started_at_ms: now_ms(),
                finished_at_ms: None,
                entries: 0,
                issue_count: 0,
                terminal_issue: None,
                volumes: Vec::new(),
                logical_bytes: 0,
                allocated_bytes: 0,
            },
            records: Vec::new(),
        }
    }

    pub fn path(&self, index: usize) -> Option<PathBuf> {
        let record = self.records.get(index)?;
        let mut components = vec![record.name.as_os_str()];
        let mut parent = record.parent;
        while let Some(index) = parent {
            let record = self.records.get(index)?;
            components.push(record.name.as_os_str());
            parent = record.parent;
        }
        Some(components.into_iter().rev().collect())
    }
}

pub struct Root {
    pub path: PathBuf,
    pub directory: File,
}

impl Root {
    /// Every component must be a real directory, including ancestors.
    pub fn open(path: &Path) -> io::Result<Self> {
        if !path.is_absolute() || path.as_os_str().len() > 4096 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid root"));
        }
        let mut directory = File::from(fs::open(
            "/",
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
            Mode::empty(),
        )?);
        for component in path.components() {
            match component {
                Component::RootDir => continue,
                Component::Normal(name) => {
                    directory = open_directory(&directory, name)?;
                }
                _ => return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid root")),
            }
        }
        Ok(Self {
            path: path.to_owned(),
            directory,
        })
    }

    pub fn owned_by_current_user(&self) -> io::Result<bool> {
        Ok(fs::fstat(&self.directory)?.st_uid == rustix::process::geteuid().as_raw())
    }

    pub fn try_clone(&self) -> io::Result<Self> {
        Ok(Self {
            path: self.path.clone(),
            directory: self.directory.try_clone()?,
        })
    }
}

fn open_directory(parent: &File, name: &OsStr) -> io::Result<File> {
    Ok(File::from(fs::openat(
        parent,
        name,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )?))
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Volume {
    pub id: String,
    pub measured_at_ms: u64,
    pub total_bytes: Option<String>,
    pub available_bytes: Option<String>,
    pub free_bytes: Option<String>,
    pub used_bytes: Option<String>,
    pub metric_issue: Option<Issue>,
}

pub fn volume(root: &Root) -> Volume {
    let id = fs::fstat(&root.directory)
        .map(|metadata| metadata.st_dev.to_string())
        .unwrap_or_default();
    volume_metrics(id, fs::fstatvfs(&root.directory).map_err(Into::into))
}

fn volume_metrics(id: String, metrics: io::Result<fs::StatVfs>) -> Volume {
    let mut result = Volume {
        id,
        measured_at_ms: now_ms(),
        total_bytes: None,
        available_bytes: None,
        free_bytes: None,
        used_bytes: None,
        metric_issue: None,
    };
    match metrics {
        Ok(metrics) => {
            let unit = metrics.f_frsize;
            let total = metrics.f_blocks.checked_mul(unit);
            let free = metrics.f_bfree.checked_mul(unit);
            result.total_bytes = total.map(|bytes| bytes.to_string());
            result.free_bytes = free.map(|bytes| bytes.to_string());
            result.available_bytes = metrics
                .f_bavail
                .checked_mul(unit)
                .map(|bytes| bytes.to_string());
            result.used_bytes = total
                .zip(free)
                .and_then(|(total, free)| total.checked_sub(free))
                .map(|bytes| bytes.to_string());
            if result.total_bytes.is_none()
                || result.free_bytes.is_none()
                || result.available_bytes.is_none()
                || result.used_bytes.is_none()
            {
                result.metric_issue = Some(Issue::Io);
            }
        }
        Err(error) => result.metric_issue = Some(io_issue(&error)),
    }
    result
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeAccounting {
    pub volume: Volume,
    #[serde(with = "bytes")]
    pub scope_logical_bytes: u64,
    #[serde(with = "bytes")]
    pub scope_allocated_bytes: u64,
    /// Arithmetic remainder only; APFS sharing prevents physical attribution.
    pub unscanned_or_unexplained_bytes: Option<String>,
}

pub fn normalize_roots(mut roots: Vec<Root>) -> Vec<Root> {
    roots.sort_by(|left, right| left.path.cmp(&right.path));
    let mut normalized: Vec<Root> = Vec::new();
    for root in roots {
        if !normalized.iter().any(|existing| {
            root.path.starts_with(&existing.path)
                && same_mount(&existing.directory, &root.directory).unwrap_or(false)
        }) {
            normalized.push(root);
        }
    }
    normalized
}

pub fn scan(roots: Vec<Root>, data: Arc<Mutex<ScanData>>, cancel: Arc<AtomicBool>) {
    let mut scanner = Scanner {
        data,
        cancel,
        identities: HashMap::new(),
        ancestors: Vec::new(),
        entry_limit: MAX_ENTRIES,
        depth_limit: MAX_DEPTH,
    };
    scanner.run(normalize_roots(roots));
}

struct Scanner {
    data: Arc<Mutex<ScanData>>,
    cancel: Arc<AtomicBool>,
    identities: HashMap<(rustix::fs::Dev, u64), usize>,
    ancestors: Vec<usize>,
    entry_limit: usize,
    depth_limit: usize,
}

impl Scanner {
    fn run(&mut self, roots: Vec<Root>) {
        for root in roots {
            let metrics = volume(&root);
            let mut data = lock(&self.data);
            if !data
                .progress
                .volumes
                .iter()
                .any(|accounting| accounting.volume.id == metrics.id)
            {
                data.progress.volumes.push(VolumeAccounting {
                    unscanned_or_unexplained_bytes: metrics.used_bytes.clone(),
                    volume: metrics,
                    scope_logical_bytes: 0,
                    scope_allocated_bytes: 0,
                });
            }
            drop(data);
            if self.stopped() {
                break;
            }
            match fs::fstat(&root.directory) {
                Ok(metadata) => {
                    if let Some(index) =
                        self.insert(root.path.into_os_string(), None, Some(&metadata), None)
                    {
                        self.visit_directory(root.directory, index, metadata);
                    }
                }
                Err(error) => {
                    self.insert(
                        root.path.into_os_string(),
                        None,
                        None,
                        Some(io_issue(&error.into())),
                    );
                }
            }
        }
        let mut data = lock(&self.data);
        let cancelled = self.cancel.load(Ordering::Relaxed);
        data.progress.status = if cancelled {
            Status::Cancelled
        } else if data.progress.issue_count > 0 {
            Status::Partial
        } else {
            Status::Complete
        };
        if cancelled {
            data.progress.terminal_issue = Some(Issue::Cancelled);
        }
        data.progress.finished_at_ms = Some(now_ms());
    }

    fn stopped(&mut self) -> bool {
        if self.cancel.load(Ordering::Relaxed) {
            return true;
        }
        if lock(&self.data).records.len() >= self.entry_limit {
            let mut data = lock(&self.data);
            if data.progress.terminal_issue.is_none() {
                data.progress.terminal_issue = Some(Issue::EntryLimit);
                data.progress.issue_count += 1;
            }
            return true;
        }
        false
    }

    fn mark_issue(&self, index: usize, issue: Issue) {
        let mut data = lock(&self.data);
        data.progress.issue_count += 1;
        data.records[index].finding.issue = Some(issue);
        data.records[index].finding.complete = false;
        for ancestor in &self.ancestors {
            data.records[*ancestor].finding.complete = false;
        }
    }

    fn insert(
        &mut self,
        name: OsString,
        parent: Option<usize>,
        metadata: Option<&Stat>,
        issue: Option<Issue>,
    ) -> Option<usize> {
        if self.stopped() {
            return None;
        }
        let mut data = lock(&self.data);
        let index = data.records.len();
        let kind = metadata.map(kind).unwrap_or(Kind::Unknown);
        let identity = metadata.map(|metadata| (metadata.st_dev, metadata.st_ino));
        let alias = identity.and_then(|identity| self.identities.get(&identity).copied());
        let (logical, allocated) = if kind == Kind::File && alias.is_none() && issue.is_none() {
            metadata
                .map(|metadata| {
                    (
                        metadata.st_size.max(0) as u64,
                        (metadata.st_blocks.max(0) as u64).saturating_mul(512),
                    )
                })
                .unwrap_or_default()
        } else {
            (0, 0)
        };
        let finding = Finding {
            id: format!("{}:{index}", data.progress.generation),
            parent_id: parent.map(|parent| data.records[parent].finding.id.clone()),
            display_name: display(&name),
            kind,
            logical_bytes: logical,
            allocated_bytes: allocated,
            measured_at_ms: now_ms(),
            modified_at_ms: metadata
                .and_then(|metadata| u64::try_from(metadata.st_mtime).ok())
                .and_then(|seconds| seconds.checked_mul(1000)),
            identity: identity.map(|(device, inode)| format!("{device}:{inode}")),
            alias_of: alias.map(|alias| data.records[alias].finding.id.clone()),
            complete: issue.is_none() && kind != Kind::Directory,
            issue,
        };
        data.records.push(Record {
            finding,
            name,
            parent,
        });
        data.progress.entries = data.records.len();
        data.progress.logical_bytes = data.progress.logical_bytes.saturating_add(logical);
        data.progress.allocated_bytes = data.progress.allocated_bytes.saturating_add(allocated);
        if let Some((device, _)) = identity {
            if let Some(accounting) = data
                .progress
                .volumes
                .iter_mut()
                .find(|accounting| accounting.volume.id == device.to_string())
            {
                accounting.scope_logical_bytes =
                    accounting.scope_logical_bytes.saturating_add(logical);
                accounting.scope_allocated_bytes =
                    accounting.scope_allocated_bytes.saturating_add(allocated);
                accounting.unscanned_or_unexplained_bytes = accounting
                    .volume
                    .used_bytes
                    .as_ref()
                    .and_then(|bytes| bytes.parse::<u64>().ok())
                    .and_then(|used| used.checked_sub(accounting.scope_allocated_bytes))
                    .map(|bytes| bytes.to_string());
            }
        }
        for ancestor in &self.ancestors {
            let finding = &mut data.records[*ancestor].finding;
            finding.logical_bytes = finding.logical_bytes.saturating_add(logical);
            finding.allocated_bytes = finding.allocated_bytes.saturating_add(allocated);
        }
        if let Some(identity) = identity {
            self.identities.entry(identity).or_insert(index);
        }
        drop(data);
        if let Some(issue) = issue {
            self.mark_issue(index, issue);
        }
        Some(index)
    }

    fn visit_directory(&mut self, directory: File, index: usize, before: Stat) {
        if lock(&self.data).records[index].finding.alias_of.is_some() {
            lock(&self.data).records[index].finding.complete = true;
            return;
        }
        if self.ancestors.len() >= self.depth_limit {
            self.mark_issue(index, Issue::DepthLimit);
            return;
        }
        let mut entries = match Dir::read_from(&directory) {
            Ok(entries) => entries,
            Err(error) => {
                self.mark_issue(index, io_issue(&error.into()));
                return;
            }
        };
        self.ancestors.push(index);
        let issues_before = lock(&self.data).progress.issue_count;
        while !self.stopped() {
            let entry = match entries.next() {
                None => break,
                Some(Ok(entry)) => entry,
                Some(Err(error)) => {
                    self.mark_issue(index, io_issue(&error.into()));
                    break;
                }
            };
            let name = OsString::from_vec(entry.file_name().to_bytes().to_vec());
            if name == "." || name == ".." {
                continue;
            }
            self.visit_entry(&directory, name, index, before.st_dev);
        }
        match fs::fstat(&directory) {
            Ok(after) if unchanged(&before, &after) => {}
            Ok(_) => self.mark_issue(index, Issue::Changed),
            Err(error) => self.mark_issue(index, io_issue(&error.into())),
        }
        self.ancestors.pop();
        let mut data = lock(&self.data);
        let complete = data.progress.issue_count == issues_before
            && !self.cancel.load(Ordering::Relaxed)
            && data.progress.terminal_issue.is_none();
        data.records[index].finding.complete = complete;
    }

    fn visit_entry(
        &mut self,
        parent: &File,
        name: OsString,
        parent_index: usize,
        device: rustix::fs::Dev,
    ) {
        let before = match fs::statat(parent, &name, AtFlags::SYMLINK_NOFOLLOW) {
            Ok(metadata) => metadata,
            Err(error) => {
                self.insert(
                    name,
                    Some(parent_index),
                    None,
                    Some(io_issue(&error.into())),
                );
                return;
            }
        };
        let boundary = before.st_dev != device;
        let Some(index) = self.insert(
            name.clone(),
            Some(parent_index),
            Some(&before),
            boundary.then_some(Issue::MountBoundary),
        ) else {
            return;
        };
        if boundary {
            return;
        }
        if kind(&before) == Kind::Directory {
            match open_directory(parent, &name) {
                Ok(directory) => match fs::fstat(&directory) {
                    Ok(opened) if unchanged(&before, &opened) => {
                        match same_mount(parent, &directory) {
                            Ok(true) => self.visit_directory(directory, index, opened),
                            Ok(false) => self.mark_issue(index, Issue::MountBoundary),
                            Err(error) => self.mark_issue(index, io_issue(&error)),
                        }
                    }
                    Ok(_) => self.mark_issue(index, Issue::Changed),
                    Err(error) => self.mark_issue(index, io_issue(&error.into())),
                },
                Err(error) => self.mark_issue(index, io_issue(&error)),
            }
            return;
        }
        match fs::statat(parent, &name, AtFlags::SYMLINK_NOFOLLOW) {
            Ok(after) if unchanged(&before, &after) => {}
            Ok(_) => self.mark_issue(index, Issue::Changed),
            Err(error) => self.mark_issue(index, io_issue(&error.into())),
        }
    }
}

fn same_mount(parent: &File, directory: &File) -> io::Result<bool> {
    let parent_mount = fs::fstatfs(parent)?;
    let child_mount = fs::fstatfs(directory)?;
    #[cfg(target_os = "macos")]
    return Ok(parent_mount.f_mntonname == child_mount.f_mntonname);
    #[cfg(not(target_os = "macos"))]
    Ok(parent_mount.f_fsid == child_mount.f_fsid)
}

fn kind(metadata: &Stat) -> Kind {
    let file_type = fs::FileType::from_raw_mode(metadata.st_mode);
    match file_type {
        fs::FileType::Directory => Kind::Directory,
        fs::FileType::RegularFile => Kind::File,
        fs::FileType::Symlink => Kind::Symlink,
        _ => Kind::Other,
    }
}

fn unchanged(before: &Stat, after: &Stat) -> bool {
    before.st_dev == after.st_dev
        && before.st_ino == after.st_ino
        && before.st_mode == after.st_mode
        && before.st_size == after.st_size
        && before.st_mtime == after.st_mtime
        && before.st_mtime_nsec == after.st_mtime_nsec
        && before.st_ctime == after.st_ctime
        && before.st_ctime_nsec == after.st_ctime_nsec
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::{symlink, PermissionsExt};
    use tempfile::TempDir;

    fn fixture() -> (TempDir, PathBuf) {
        let temporary = tempfile::tempdir().unwrap();
        let path = temporary.path().canonicalize().unwrap();
        (temporary, path)
    }

    fn measure(paths: &[PathBuf]) -> Arc<Mutex<ScanData>> {
        let data = Arc::new(Mutex::new(ScanData::new(1)));
        scan(
            paths.iter().map(|path| Root::open(path).unwrap()).collect(),
            data.clone(),
            Arc::new(AtomicBool::new(false)),
        );
        data
    }

    #[test]
    fn unavailable_volume_metrics_remain_unknown() {
        let metrics = volume_metrics(
            "fixture-volume".into(),
            Err(io::Error::from(io::ErrorKind::PermissionDenied)),
        );
        assert_eq!(metrics.metric_issue, Some(Issue::PermissionDenied));
        assert!(metrics.total_bytes.is_none());
        assert!(metrics.used_bytes.is_none());
        assert!(metrics.free_bytes.is_none());
        assert!(metrics.available_bytes.is_none());
    }
    #[test]
    fn aggregates_overlap_hardlinks_hidden_packages_sparse_and_lossless_names() {
        let (_temporary, path) = fixture();
        fs::create_dir(path.join("App.app")).unwrap();
        fs::write(path.join("App.app/.hidden"), b"hello").unwrap();
        fs::hard_link(path.join("App.app/.hidden"), path.join("alias")).unwrap();
        let sparse = File::create(path.join("sparse")).unwrap();
        sparse.set_len(1_000_000).unwrap();
        let name = OsString::from_vec(vec![0xff, b'x']);
        fs::write(path.join("plain"), b"abc").unwrap();
        let data = measure(&[path.clone(), path.join("App.app"), path.clone()]);
        let data = lock(&data);
        assert_eq!(data.progress.status, Status::Complete);
        assert_eq!(data.progress.logical_bytes, 1_000_008);
        assert_eq!(data.records[0].finding.logical_bytes, 1_000_008);
        assert_eq!(
            data.records
                .iter()
                .filter(|record| record.parent.is_none())
                .count(),
            1
        );
        assert_eq!(
            data.records
                .iter()
                .filter(|record| record.finding.alias_of.is_some())
                .count(),
            1
        );
        assert!(data.progress.allocated_bytes < data.progress.logical_bytes);
        let index = data
            .records
            .iter()
            .position(|record| record.name == "plain")
            .unwrap();
        assert_eq!(data.path(index).unwrap(), path.join("plain"));
        drop(name);
        assert!(data.records.iter().all(|record| record.finding.complete));
        let metrics = volume(&Root::open(&path).unwrap());
        assert!(metrics.total_bytes.is_some());
    }

    #[test]
    fn explicitly_selected_nested_mount_is_not_pruned_by_ancestor_scope() {
        let (_temporary, path) = fixture();
        let ancestor = Root::open(&path).unwrap();
        // Read only OS mount metadata; no traversal of /dev or mount operation.
        let mut mounted = Root::open(Path::new("/dev")).unwrap();
        assert!(!same_mount(&ancestor.directory, &mounted.directory).unwrap());
        mounted.path = path.join("synthetic-mount");
        let roots = normalize_roots(vec![ancestor, mounted]);
        assert_eq!(roots.len(), 2);
    }

    #[test]
    fn multiple_disjoint_roots_share_one_volume_and_one_hardlink_charge() {
        let (_temporary, path) = fixture();
        let first = path.join("first");
        let second = path.join("second");
        fs::create_dir(&first).unwrap();
        fs::create_dir(&second).unwrap();
        fs::write(first.join("original"), b"1234567").unwrap();
        fs::hard_link(first.join("original"), second.join("alias")).unwrap();
        let data = measure(&[first, second]);
        let data = lock(&data);
        assert_eq!(data.progress.status, Status::Complete);
        assert_eq!(data.progress.logical_bytes, 7);
        assert_eq!(data.progress.volumes.len(), 1);
        let accounting = &data.progress.volumes[0];
        assert_eq!(accounting.scope_logical_bytes, 7);
        assert_eq!(
            accounting.scope_allocated_bytes,
            data.progress.allocated_bytes
        );
        assert_eq!(
            accounting
                .unscanned_or_unexplained_bytes
                .as_ref()
                .unwrap()
                .parse::<u64>()
                .unwrap(),
            accounting
                .volume
                .used_bytes
                .as_ref()
                .unwrap()
                .parse::<u64>()
                .unwrap()
                - accounting.scope_allocated_bytes
        );
    }

    #[test]
    fn symlinks_never_escape_or_cycle_and_root_ancestors_cannot_be_symlinks() {
        let (_temporary, path) = fixture();
        let (_outside, outside) = fixture();
        fs::write(outside.join("sentinel"), b"private").unwrap();
        symlink(&outside, path.join("escape")).unwrap();
        symlink(&path, path.join("cycle")).unwrap();
        assert!(Root::open(&path.join("escape")).is_err());
        assert!(Root::open(&path.join("escape/child")).is_err());
        let data = measure(&[path]);
        let data = lock(&data);
        assert_eq!(data.progress.logical_bytes, 0);
        assert_eq!(data.records.len(), 3);
        assert_eq!(data.progress.status, Status::Complete);
        assert_eq!(fs::read(outside.join("sentinel")).unwrap(), b"private");
    }

    #[test]
    fn root_handle_survives_path_replacement_without_redirecting_scan() {
        let (_temporary, path) = fixture();
        let original = path.join("original");
        fs::create_dir(&original).unwrap();
        fs::write(original.join("inside"), b"ok").unwrap();
        let root = Root::open(&original).unwrap();
        fs::rename(&original, path.join("moved")).unwrap();
        fs::create_dir(&original).unwrap();
        fs::write(original.join("replacement"), b"outside").unwrap();
        let data = Arc::new(Mutex::new(ScanData::new(1)));
        scan(vec![root], data.clone(), Arc::new(AtomicBool::new(false)));
        assert_eq!(lock(&data).progress.logical_bytes, 2);
        assert!(!lock(&data)
            .records
            .iter()
            .any(|record| record.name == "replacement"));
    }

    fn scanner(entry_limit: usize, depth_limit: usize) -> Scanner {
        Scanner {
            data: Arc::new(Mutex::new(ScanData::new(2))),
            cancel: Arc::new(AtomicBool::new(false)),
            identities: HashMap::new(),
            ancestors: Vec::new(),
            entry_limit,
            depth_limit,
        }
    }

    #[test]
    fn bounded_large_deep_trees_and_cancellation_preserve_results() {
        let (_temporary, path) = fixture();
        for index in 0..500 {
            fs::write(path.join(format!("file-{index}")), b"x").unwrap();
        }
        let mut scanner = scanner(20, MAX_DEPTH);
        scanner.run(vec![Root::open(&path).unwrap()]);
        assert_eq!(lock(&scanner.data).records.len(), 20);
        assert_eq!(lock(&scanner.data).progress.status, Status::Partial);
        assert_eq!(
            lock(&scanner.data).progress.terminal_issue,
            Some(Issue::EntryLimit)
        );
        let measured = lock(&scanner.data).progress.logical_bytes;
        scanner.cancel.store(true, Ordering::Relaxed);
        scanner.run(vec![Root::open(&path).unwrap()]);
        assert_eq!(lock(&scanner.data).progress.status, Status::Cancelled);
        assert_eq!(lock(&scanner.data).progress.logical_bytes, measured);
        let mut nested = path.clone();
        for _ in 0..10 {
            nested.push("nested");
            fs::create_dir(&nested).unwrap();
        }
        let mut scanner = self::scanner(MAX_ENTRIES, 3);
        scanner.run(vec![Root::open(&path).unwrap()]);
        assert!(lock(&scanner.data)
            .records
            .iter()
            .any(|record| record.finding.issue == Some(Issue::DepthLimit)));
    }

    #[test]
    fn vanished_permission_mount_and_change_have_explicit_coverage() {
        let (_temporary, path) = fixture();
        let root = Root::open(&path).unwrap();
        let mut scanner = scanner(MAX_ENTRIES, MAX_DEPTH);
        let metadata = rustix::fs::fstat(&root.directory).unwrap();
        scanner.insert(path.clone().into_os_string(), None, Some(&metadata), None);
        scanner.visit_entry(&root.directory, "missing".into(), 0, metadata.st_dev);
        assert_eq!(
            lock(&scanner.data).records[1].finding.issue,
            Some(Issue::Vanished)
        );
        fs::create_dir(path.join("denied")).unwrap();
        fs::set_permissions(path.join("denied"), fs::Permissions::from_mode(0o0)).unwrap();
        let denied = open_directory(&root.directory, OsStr::new("denied"));
        let denied_scan = measure(std::slice::from_ref(&path));
        assert_eq!(lock(&denied_scan).progress.status, Status::Partial);
        assert!(!lock(&denied_scan).records[0].finding.complete);
        assert!(lock(&denied_scan)
            .records
            .iter()
            .any(|record| record.finding.issue == Some(Issue::PermissionDenied)));
        fs::set_permissions(path.join("denied"), fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(io_issue(&denied.unwrap_err()), Issue::PermissionDenied);
        fs::write(path.join("mounted"), b"not-counted").unwrap();
        scanner.visit_entry(
            &root.directory,
            "mounted".into(),
            0,
            metadata.st_dev.wrapping_add(1),
        );
        assert_eq!(
            lock(&scanner.data).records[2].finding.issue,
            Some(Issue::MountBoundary)
        );
        assert_eq!(lock(&scanner.data).progress.logical_bytes, 0);
        let before = rustix::fs::fstat(&root.directory).unwrap();
        fs::write(path.join("changed"), b"x").unwrap();
        let after = rustix::fs::fstat(&root.directory).unwrap();
        assert!(!unchanged(&before, &after));
        let changed_data = Arc::new(Mutex::new(ScanData::new(9)));
        let mut changed_scanner = self::scanner(MAX_ENTRIES, MAX_DEPTH);
        changed_scanner.data = changed_data.clone();
        let index = changed_scanner
            .insert(path.clone().into_os_string(), None, Some(&before), None)
            .unwrap();
        changed_scanner.visit_directory(root.directory.try_clone().unwrap(), index, before);
        assert_eq!(
            lock(&changed_data).records[0].finding.issue,
            Some(Issue::Changed)
        );
        assert!(!lock(&changed_data).records[0].finding.complete);

        scanner.mark_issue(2, Issue::Changed);
        assert!(!lock(&scanner.data).records[2].finding.complete);
    }
}

#[cfg(test)]
mod lossless_tests {
    use super::*;
    #[test]
    fn native_names_remain_lossless_even_when_display_is_replaced_or_truncated() {
        let name = OsString::from_vec(vec![0xff, b'x']);
        let mut data = ScanData::new(1);
        data.records.push(Record {
            name: name.clone(),
            parent: None,
            finding: Finding {
                id: "1:0".into(),
                parent_id: None,
                display_name: display(&name),
                kind: Kind::Unknown,
                logical_bytes: 0,
                allocated_bytes: 0,
                measured_at_ms: 0,
                modified_at_ms: None,
                identity: None,
                alias_of: None,
                complete: false,
                issue: Some(Issue::Io),
            },
        });
        assert_eq!(data.path(0).unwrap().into_os_string(), name);
        assert!(data.records[0].finding.display_name.contains('�'));
        assert_eq!(display(OsStr::new(&"x".repeat(2000))).len(), MAX_DISPLAY);
        assert_eq!(display(OsStr::new("a\nb\u{202e}")), "a�b�");
    }
}
