//! Descriptor-anchored, metadata-only traversal. No file content is opened.

use rustix::fs::{self, AtFlags, Dir, Mode, OFlags, Stat};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

pub const MAX_ROOTS: usize = 16;
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
    Excluded,
    PermissionDenied,
    Vanished,
    Io,
    Changed,
    MountBoundary,
    DepthLimit,
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
    /// Entries (files and directories) below this node that were measured but not retained.
    pub folded_items: u64,
    #[serde(with = "bytes")]
    pub folded_logical_bytes: u64,
    #[serde(with = "bytes")]
    pub folded_allocated_bytes: u64,
    /// Every entry below this node, retained or folded.
    pub descendant_items: u64,
    pub detail: Detail,
}

/// `Summarized` directories hold folded descendants; their totals stay exact.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Detail {
    Full,
    Summarized,
}

/// Native names remain lossless, never round-tripped through display strings.
#[derive(Clone)]
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
    pub current_location: Option<String>,
    pub finished_at_ms: Option<u64>,
    pub entries: usize,
    pub issue_count: usize,
    pub terminal_issue: Option<Issue>,
    pub volumes: Vec<VolumeAccounting>,
    #[serde(with = "bytes")]
    pub logical_bytes: u64,
    #[serde(with = "bytes")]
    pub allocated_bytes: u64,
    pub retention: Retention,
    /// Set when the hard-link identity cap was exceeded; later shared files count as unique.
    pub sharing_untracked: bool,
}

/// Retention is information, never a coverage gap.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Retention {
    pub nodes: usize,
    #[serde(with = "bytes")]
    pub threshold_bytes: u64,
    pub budget_reached: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum Category {
    Documents,
    Media,
    Archives,
    #[serde(rename = "Other files")]
    Other,
}

pub const CATEGORIES: [Category; 4] = [
    Category::Documents,
    Category::Media,
    Category::Archives,
    Category::Other,
];

impl Category {
    fn position(self) -> usize {
        CATEGORIES
            .iter()
            .position(|category| *category == self)
            .unwrap_or(CATEGORIES.len() - 1)
    }
}

/// Extension table shared with the Overview; unknown or missing extensions are `Other`.
pub fn category_for(name: &OsStr) -> Category {
    const DOCUMENTS: [&[u8]; 11] = [
        b"pdf", b"txt", b"md", b"doc", b"docx", b"xls", b"xlsx", b"csv", b"ppt", b"pptx", b"rtf",
    ];
    const MEDIA: [&[u8]; 12] = [
        b"png", b"jpg", b"jpeg", b"webp", b"gif", b"heic", b"mov", b"mp4", b"mp3", b"wav", b"m4a",
        b"mkv",
    ];
    const ARCHIVES: [&[u8]; 7] = [b"zip", b"gz", b"tar", b"xz", b"7z", b"dmg", b"iso"];
    let extension = name
        .as_bytes()
        .rsplit(|byte| *byte == b'.')
        .next()
        .unwrap_or(&[]);
    let matches = |list: &[&[u8]]| {
        list.iter()
            .any(|known| known.eq_ignore_ascii_case(extension))
    };
    if matches(&DOCUMENTS) {
        Category::Documents
    } else if matches(&MEDIA) {
        Category::Media
    } else if matches(&ARCHIVES) {
        Category::Archives
    } else {
        Category::Other
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryTotal {
    pub name: Category,
    #[serde(with = "bytes")]
    pub logical_bytes: u64,
    #[serde(with = "bytes")]
    pub allocated_bytes: u64,
    pub count: u64,
}

/// A largest-in-category file. Its native path is kept even when the node is not retained.
#[derive(Clone, Debug)]
pub struct TopFile {
    pub category: Category,
    pub path: PathBuf,
    pub logical_bytes: u64,
    pub allocated_bytes: u64,
    pub modified_at_ms: Option<u64>,
    pub identity: String,
}

pub struct ScanData {
    pub progress: Progress,
    pub records: Vec<Record>,
    pub top_files: Vec<TopFile>,
    pub recognition: crate::recognition::Recognition,
    pub pins_exhausted: bool,
}

impl ScanData {
    pub fn new(generation: u32) -> Self {
        Self {
            progress: Progress {
                generation,
                status: Status::Running,
                started_at_ms: now_ms(),
                current_location: None,
                finished_at_ms: None,
                entries: 0,
                issue_count: 0,
                terminal_issue: None,
                volumes: Vec::new(),
                logical_bytes: 0,
                allocated_bytes: 0,
                retention: Retention {
                    nodes: 0,
                    threshold_bytes: 0,
                    budget_reached: false,
                },
                sharing_untracked: false,
            },
            records: Vec::new(),
            top_files: Vec::new(),
            recognition: crate::recognition::Recognition::default(),
            pins_exhausted: false,
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
    pub categories: Vec<CategoryTotal>,
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

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Exclusion {
    pub relative_path: String,
    pub subtree: bool,
}

impl Exclusion {
    pub fn validate(&self) -> bool {
        let path = Path::new(&self.relative_path);
        !self.relative_path.is_empty()
            && self.relative_path.len() <= 1024
            && !self.relative_path.chars().any(char::is_control)
            && !path.is_absolute()
            && self
                .relative_path
                .split('/')
                .all(|part| !part.is_empty() && part != "." && part != "..")
            && path
                .components()
                .all(|component| matches!(component, Component::Normal(_)))
    }
}

pub const RETENTION_DEPTH: usize = 2;
pub const RETENTION_THRESHOLD: u64 = 1 << 20;
pub const RETENTION_TOP_K: usize = 5;
pub const RETENTION_NODE_BUDGET: usize = 250_000;
const PIN_BUDGET: usize = 50_000;
const FILE_IDENTITY_CAP: usize = 2_000_000;
const CATEGORY_TOP_FILES: usize = 50;
const PUBLISH_INTERVAL: u64 = 4_096;

/// Decides which measured nodes stay retained; totals never depend on it.
#[derive(Clone, Copy, Debug)]
pub struct RetentionPolicy {
    /// Directories at most this deep below their root are retained.
    pub depth: usize,
    /// Directories and files with at least this many allocated bytes are retained.
    pub threshold: u64,
    /// Largest files kept per directory (subject to `threshold`).
    pub top_k: usize,
    pub node_budget: usize,
    /// Receives the containing directory path, the entry name and its kind.
    pub pin_predicate: fn(&Path, &OsStr, Kind) -> bool,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            depth: RETENTION_DEPTH,
            threshold: RETENTION_THRESHOLD,
            top_k: RETENTION_TOP_K,
            node_budget: RETENTION_NODE_BUDGET,
            pin_predicate: crate::recognition::is_pinned,
        }
    }
}

impl RetentionPolicy {
    /// Retain every entry, as long as the tree fits `node_budget`.
    pub fn exhaustive(node_budget: usize) -> Self {
        Self {
            depth: usize::MAX,
            threshold: 0,
            top_k: usize::MAX,
            node_budget,
            pin_predicate: crate::recognition::is_pinned,
        }
    }
}

pub fn scan(roots: Vec<Root>, data: Arc<Mutex<ScanData>>, cancel: Arc<AtomicBool>) {
    scan_excluding(roots, Vec::new(), data, cancel);
}

pub fn scan_excluding(
    roots: Vec<Root>,
    exclusions: Vec<(PathBuf, bool)>,
    data: Arc<Mutex<ScanData>>,
    cancel: Arc<AtomicBool>,
) {
    scan_with_policy(roots, exclusions, RetentionPolicy::default(), data, cancel);
}

pub fn scan_with_policy(
    roots: Vec<Root>,
    exclusions: Vec<(PathBuf, bool)>,
    policy: RetentionPolicy,
    data: Arc<Mutex<ScanData>>,
    cancel: Arc<AtomicBool>,
) {
    let mut scanner = Scanner::new(exclusions, policy, data, cancel);
    scanner.run(normalize_roots(roots));
}

#[derive(Clone, Copy, Default)]
struct Fold {
    items: u64,
    logical: u64,
    allocated: u64,
}

impl Fold {
    fn absorb(&mut self, items: u64, logical: u64, allocated: u64) {
        self.items = self.items.saturating_add(items);
        self.logical = self.logical.saturating_add(logical);
        self.allocated = self.allocated.saturating_add(allocated);
    }
}

/// A retained entry; directory totals are subtree totals and children are retained only.
struct Node {
    name: OsString,
    kind: Kind,
    identity: Option<(rustix::fs::Dev, u64)>,
    modified_at_ms: Option<u64>,
    measured_at_ms: u64,
    logical: u64,
    allocated: u64,
    alias: bool,
    issue: Option<Issue>,
    complete: bool,
    pinned: bool,
    descendants: u64,
    fold: Fold,
    children: Vec<Node>,
}

#[derive(Default)]
struct Frame {
    logical: u64,
    allocated: u64,
    descendants: u64,
    issues: u64,
    issue: Option<Issue>,
    children: Vec<Node>,
    files: Vec<Node>,
    fold: Fold,
}

struct DirectoryResult {
    logical: u64,
    allocated: u64,
    descendants: u64,
    issues: u64,
    node: Option<Node>,
}

struct Leaf<'name> {
    name: &'name OsStr,
    kind: Kind,
    identity: Option<(rustix::fs::Dev, u64)>,
    modified_at_ms: Option<u64>,
    logical: u64,
    allocated: u64,
    alias: bool,
    issue: Option<Issue>,
}

#[derive(Clone, Copy, Default)]
struct VolumeTally {
    logical: u64,
    allocated: u64,
    categories: [(u64, u64, u64); 4],
}

struct Scanner {
    exclusions: Vec<(PathBuf, bool)>,
    data: Arc<Mutex<ScanData>>,
    cancel: Arc<AtomicBool>,
    policy: RetentionPolicy,
    generation: u32,
    depth_limit: usize,
    file_identity_cap: usize,
    threshold: u64,
    prune_at: usize,
    nodes: usize,
    pins: usize,
    pins_exhausted: bool,
    budget_reached: bool,
    directories: HashSet<(rustix::fs::Dev, u64)>,
    files: HashSet<(rustix::fs::Dev, u64)>,
    sharing_untracked: bool,
    frames: Vec<Frame>,
    ancestors: Vec<(rustix::fs::Dev, u64)>,
    path: PathBuf,
    records: Vec<Record>,
    entries: usize,
    issues: usize,
    logical: u64,
    allocated: u64,
    unpublished: u64,
    tallies: Vec<VolumeTally>,
    volume: usize,
    top: [Vec<TopFile>; 4],
}

fn top_frame(frames: &mut [Frame]) -> &mut Frame {
    frames
        .last_mut()
        .expect("traversal frames exist while entries are visited")
}

fn modified_ms(metadata: &Stat) -> Option<u64> {
    u64::try_from(metadata.st_mtime)
        .ok()
        .and_then(|seconds| seconds.checked_mul(1000))
}

fn prune_nodes(children: &mut Vec<Node>, fold: &mut Fold, threshold: u64, removed: &mut usize) {
    children.retain_mut(|child| {
        prune_nodes(&mut child.children, &mut child.fold, threshold, removed);
        let removable = !child.pinned && child.children.is_empty() && child.allocated < threshold;
        if removable {
            fold.absorb(1 + child.descendants, child.logical, child.allocated);
            *removed += 1;
        }
        !removable
    });
}

fn flatten(node: Node, parent: Option<usize>, generation: u32, records: &mut Vec<Record>) {
    let index = records.len();
    let Node {
        name,
        kind,
        identity,
        modified_at_ms,
        measured_at_ms,
        logical,
        allocated,
        alias,
        issue,
        complete,
        descendants,
        fold,
        children,
        ..
    } = node;
    let identity = identity.map(|(device, inode)| format!("{device}:{inode}"));
    let parent_id = parent.map(|parent| records[parent].finding.id.clone());
    records.push(Record {
        finding: Finding {
            id: format!("{generation}:{index}"),
            parent_id,
            display_name: display(&name),
            kind,
            logical_bytes: logical,
            allocated_bytes: allocated,
            measured_at_ms,
            modified_at_ms,
            alias_of: alias.then(|| identity.clone()).flatten(),
            identity,
            complete,
            issue,
            folded_items: fold.items,
            folded_logical_bytes: fold.logical,
            folded_allocated_bytes: fold.allocated,
            descendant_items: descendants,
            detail: if fold.items == 0 {
                Detail::Full
            } else {
                Detail::Summarized
            },
        },
        name,
        parent,
    });
    for child in children {
        flatten(child, Some(index), generation, records);
    }
}

impl Scanner {
    fn new(
        exclusions: Vec<(PathBuf, bool)>,
        policy: RetentionPolicy,
        data: Arc<Mutex<ScanData>>,
        cancel: Arc<AtomicBool>,
    ) -> Self {
        let generation = lock(&data).progress.generation;
        Self {
            exclusions,
            data,
            cancel,
            policy,
            generation,
            depth_limit: MAX_DEPTH,
            file_identity_cap: FILE_IDENTITY_CAP,
            threshold: policy.threshold,
            prune_at: policy.node_budget,
            nodes: 0,
            pins: 0,
            pins_exhausted: false,
            budget_reached: false,
            directories: HashSet::new(),
            files: HashSet::new(),
            sharing_untracked: false,
            frames: Vec::new(),
            ancestors: Vec::new(),
            path: PathBuf::new(),
            records: Vec::new(),
            entries: 0,
            issues: 0,
            logical: 0,
            allocated: 0,
            unpublished: 0,
            tallies: Vec::new(),
            volume: 0,
            top: Default::default(),
        }
    }

    fn run(&mut self, roots: Vec<Root>) {
        for root in roots {
            let metrics = volume(&root);
            self.register_volume(metrics, &root);
            if self.stopped() {
                break;
            }
            let name = root.path.into_os_string();
            self.path = PathBuf::from(&name);
            self.entries += 1;
            self.nodes += 1;
            let result = match fs::fstat(&root.directory) {
                Ok(metadata) => self.walk_directory(root.directory, &name, metadata, 0),
                Err(error) => self.issue_leaf_root(&name, io_issue(&error.into())),
            };
            if let Some(node) = result.node {
                flatten(node, None, self.generation, &mut self.records);
            }
            self.publish();
        }
        self.publish();
        let recognition =
            crate::recognition::recognize(&self.records, &self.cancel, self.pins_exhausted);
        let top_files: Vec<TopFile> = self.top.iter().flatten().cloned().collect();
        let mut data = lock(&self.data);
        data.records = std::mem::take(&mut self.records);
        data.top_files = top_files;
        data.recognition = recognition;
        data.pins_exhausted = self.pins_exhausted;
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

    fn register_volume(&mut self, metrics: Volume, root: &Root) {
        let mut data = lock(&self.data);
        data.progress.current_location = Some(display(root.path.as_os_str()));
        let position = data
            .progress
            .volumes
            .iter()
            .position(|accounting| accounting.volume.id == metrics.id);
        self.volume = match position {
            Some(position) => position,
            None => {
                data.progress.volumes.push(VolumeAccounting {
                    unscanned_or_unexplained_bytes: metrics.used_bytes.clone(),
                    volume: metrics,
                    scope_logical_bytes: 0,
                    scope_allocated_bytes: 0,
                    categories: category_totals(&VolumeTally::default()),
                });
                data.progress.volumes.len() - 1
            }
        };
        if self.tallies.len() <= self.volume {
            self.tallies.resize(self.volume + 1, VolumeTally::default());
        }
    }

    fn stopped(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    /// Publishes counters in batches so the walk never takes the lock per entry.
    fn publish(&mut self) {
        self.unpublished = 0;
        let mut data = lock(&self.data);
        let progress = &mut data.progress;
        progress.entries = self.entries;
        progress.issue_count = self.issues;
        progress.logical_bytes = self.logical;
        progress.allocated_bytes = self.allocated;
        progress.sharing_untracked = self.sharing_untracked;
        progress.retention = Retention {
            nodes: self.nodes,
            threshold_bytes: self.threshold,
            budget_reached: self.budget_reached,
        };
        for (accounting, tally) in progress.volumes.iter_mut().zip(&self.tallies) {
            accounting.scope_logical_bytes = tally.logical;
            accounting.scope_allocated_bytes = tally.allocated;
            accounting.categories = category_totals(tally);
            accounting.unscanned_or_unexplained_bytes = accounting
                .volume
                .used_bytes
                .as_ref()
                .and_then(|bytes| bytes.parse::<u64>().ok())
                .and_then(|used| used.checked_sub(tally.allocated))
                .map(|bytes| bytes.to_string());
        }
    }

    fn count_entry(&mut self) {
        self.entries += 1;
        self.unpublished += 1;
        if self.unpublished >= PUBLISH_INTERVAL {
            self.publish();
        }
    }

    fn note_issue(&mut self, issue: Issue) {
        self.issues += 1;
        let frame = top_frame(&mut self.frames);
        frame.issues += 1;
        frame.issue = Some(issue);
    }

    fn grant_pin(&mut self, wanted: bool) -> bool {
        let granted = wanted && self.pins < PIN_BUDGET;
        self.pins += usize::from(granted);
        self.pins_exhausted |= wanted && !granted;
        granted
    }

    fn walk_directory(
        &mut self,
        directory: File,
        name: &OsStr,
        before: Stat,
        depth: usize,
    ) -> DirectoryResult {
        let identity = (before.st_dev, before.st_ino);
        if self.ancestors.contains(&identity) || !self.directories.insert(identity) {
            return self.finish_directory(name, &before, depth, Frame::default(), true);
        }
        if self.frames.len() >= self.depth_limit {
            return self.issue_directory(name, &before, depth, Issue::DepthLimit);
        }
        let mut entries = match Dir::read_from(&directory) {
            Ok(entries) => entries,
            Err(error) => {
                return self.issue_directory(name, &before, depth, io_issue(&error.into()));
            }
        };
        self.frames.push(Frame::default());
        self.ancestors.push(identity);
        if depth > 0 {
            self.path.push(name);
        }
        while !self.stopped() {
            let entry = match entries.next() {
                None => break,
                Some(Ok(entry)) => entry,
                Some(Err(error)) => {
                    self.note_issue(io_issue(&error.into()));
                    break;
                }
            };
            let entry_name = OsStr::from_bytes(entry.file_name().to_bytes());
            if entry_name == "." || entry_name == ".." {
                continue;
            }
            self.visit_entry(&directory, entry_name, before.st_dev, depth + 1);
        }
        match fs::fstat(&directory) {
            Ok(after) if unchanged(&before, &after) => {}
            Ok(_) => self.note_issue(Issue::Changed),
            Err(error) => self.note_issue(io_issue(&error.into())),
        }
        if depth > 0 {
            self.path.pop();
        }
        self.ancestors.pop();
        let frame = self.frames.pop().unwrap_or_default();
        self.finish_directory(name, &before, depth, frame, false)
    }

    fn issue_directory(
        &mut self,
        name: &OsStr,
        before: &Stat,
        depth: usize,
        issue: Issue,
    ) -> DirectoryResult {
        self.issues += 1;
        let frame = Frame {
            issues: 1,
            issue: Some(issue),
            ..Frame::default()
        };
        self.finish_directory(name, before, depth, frame, false)
    }

    fn issue_leaf_root(&mut self, name: &OsStr, issue: Issue) -> DirectoryResult {
        self.issues += 1;
        DirectoryResult {
            logical: 0,
            allocated: 0,
            descendants: 0,
            issues: 1,
            node: Some(Node {
                name: name.to_owned(),
                kind: Kind::Unknown,
                identity: None,
                modified_at_ms: None,
                measured_at_ms: now_ms(),
                logical: 0,
                allocated: 0,
                alias: false,
                issue: Some(issue),
                complete: false,
                pinned: true,
                descendants: 0,
                fold: Fold::default(),
                children: Vec::new(),
            }),
        }
    }

    /// Post-order decision: the directory totals are final, so retention is exact here.
    fn finish_directory(
        &mut self,
        name: &OsStr,
        before: &Stat,
        depth: usize,
        frame: Frame,
        alias: bool,
    ) -> DirectoryResult {
        let Frame {
            logical,
            allocated,
            descendants,
            issues,
            issue,
            mut children,
            files,
            mut fold,
        } = frame;
        for file in files {
            if file.allocated >= self.threshold {
                children.push(file);
            } else {
                fold.absorb(1, file.logical, file.allocated);
                self.nodes -= 1;
            }
        }
        let pin_wanted =
            issue.is_some() || (self.policy.pin_predicate)(&self.path, name, Kind::Directory);
        let pinned = self.grant_pin(pin_wanted) || depth == 0;
        let retain = pinned
            || depth <= self.policy.depth
            || allocated >= self.threshold
            || !children.is_empty();
        if !retain {
            return DirectoryResult {
                logical,
                allocated,
                descendants,
                issues,
                node: None,
            };
        }
        self.nodes += usize::from(depth > 0);
        let node = Node {
            name: name.to_owned(),
            kind: Kind::Directory,
            identity: Some((before.st_dev, before.st_ino)),
            modified_at_ms: modified_ms(before),
            measured_at_ms: now_ms(),
            logical,
            allocated,
            alias,
            issue,
            complete: alias || (issues == 0 && !self.stopped()),
            pinned,
            descendants,
            fold,
            children,
        };
        DirectoryResult {
            logical,
            allocated,
            descendants,
            issues,
            node: Some(node),
        }
    }

    fn attach(&mut self, result: DirectoryResult) {
        let frame = top_frame(&mut self.frames);
        frame.logical = frame.logical.saturating_add(result.logical);
        frame.allocated = frame.allocated.saturating_add(result.allocated);
        frame.descendants += 1 + result.descendants;
        frame.issues += result.issues;
        match result.node {
            Some(node) => frame.children.push(node),
            None => frame
                .fold
                .absorb(1 + result.descendants, result.logical, result.allocated),
        }
        self.enforce_budget();
    }

    /// Raises the threshold x4 and folds unpinned small leaves until the set fits again.
    fn enforce_budget(&mut self) {
        if self.nodes <= self.prune_at {
            return;
        }
        self.budget_reached = true;
        let budget = self.policy.node_budget;
        let target = budget - budget / 4;
        while self.nodes > target && self.threshold < u64::MAX {
            self.threshold = self.threshold.saturating_mul(4).max(1);
            self.prune();
        }
        self.prune_at = budget.max(self.nodes.saturating_add(budget / 4));
    }

    fn prune(&mut self) {
        let threshold = self.threshold;
        let mut removed = 0;
        for frame in &mut self.frames {
            prune_nodes(
                &mut frame.children,
                &mut frame.fold,
                threshold,
                &mut removed,
            );
            frame.files.retain(|file| {
                let keep = file.allocated >= threshold;
                if !keep {
                    frame.fold.absorb(1, file.logical, file.allocated);
                    removed += 1;
                }
                keep
            });
        }
        self.nodes -= removed;
    }

    fn excluded(&mut self, name: &OsStr) -> (bool, bool) {
        if self.exclusions.is_empty() {
            return (false, false);
        }
        self.path.push(name);
        let subtree = self
            .exclusions
            .iter()
            .any(|(excluded, subtree)| *subtree && self.path.starts_with(excluded));
        let exact = self
            .exclusions
            .iter()
            .any(|(excluded, _)| self.path == *excluded);
        self.path.pop();
        (subtree, exact)
    }

    fn visit_entry(&mut self, parent: &File, name: &OsStr, device: rustix::fs::Dev, depth: usize) {
        self.count_entry();
        let (excluded_subtree, excluded_exact) = self.excluded(name);
        if excluded_subtree {
            self.record_leaf(Leaf::issue(name, Kind::Unknown, None, Issue::Excluded));
            return;
        }
        let before = match fs::statat(parent, name, AtFlags::SYMLINK_NOFOLLOW) {
            Ok(metadata) => metadata,
            Err(error) => {
                self.record_leaf(Leaf::issue(
                    name,
                    Kind::Unknown,
                    None,
                    io_issue(&error.into()),
                ));
                return;
            }
        };
        let entry_kind = kind(&before);
        if excluded_exact && entry_kind != Kind::Directory {
            self.record_leaf(Leaf::issue(name, Kind::Unknown, None, Issue::Excluded));
            return;
        }
        if before.st_dev != device {
            self.record_leaf(Leaf::issue(
                name,
                entry_kind,
                Some(&before),
                Issue::MountBoundary,
            ));
            return;
        }
        if entry_kind == Kind::Directory {
            let result = self.open_and_walk(parent, name, before, depth);
            self.attach(result);
            return;
        }
        let identity = (before.st_dev, before.st_ino);
        let shared = entry_kind == Kind::File && before.st_nlink > 1;
        let alias = shared && !self.claim_file(identity);
        let (logical, allocated) = if entry_kind == Kind::File && !alias {
            (
                before.st_size.max(0) as u64,
                (before.st_blocks.max(0) as u64).saturating_mul(512),
            )
        } else {
            (0, 0)
        };
        let issue = match fs::statat(parent, name, AtFlags::SYMLINK_NOFOLLOW) {
            Ok(after) if unchanged(&before, &after) => None,
            Ok(_) => Some(Issue::Changed),
            Err(error) => Some(io_issue(&error.into())),
        };
        if entry_kind == Kind::File && !alias {
            self.measure_file(name, &before, logical, allocated);
        }
        self.record_leaf(Leaf {
            name,
            kind: entry_kind,
            identity: Some(identity),
            modified_at_ms: modified_ms(&before),
            logical,
            allocated,
            alias,
            issue,
        });
    }

    fn open_and_walk(
        &mut self,
        parent: &File,
        name: &OsStr,
        before: Stat,
        depth: usize,
    ) -> DirectoryResult {
        let directory = match open_directory(parent, name) {
            Ok(directory) => directory,
            Err(error) => return self.issue_directory(name, &before, depth, io_issue(&error)),
        };
        let opened = match fs::fstat(&directory) {
            Ok(opened) => opened,
            Err(error) => {
                return self.issue_directory(name, &before, depth, io_issue(&error.into()));
            }
        };
        if !unchanged(&before, &opened) {
            return self.issue_directory(name, &before, depth, Issue::Changed);
        }
        match same_mount(parent, &directory) {
            Ok(true) => self.walk_directory(directory, name, opened, depth),
            Ok(false) => self.issue_directory(name, &before, depth, Issue::MountBoundary),
            Err(error) => self.issue_directory(name, &before, depth, io_issue(&error)),
        }
    }

    /// True when this file identity is newly charged. Past the cap sharing is unknown.
    fn claim_file(&mut self, identity: (rustix::fs::Dev, u64)) -> bool {
        if self.files.contains(&identity) {
            return false;
        }
        if self.files.len() < self.file_identity_cap {
            self.files.insert(identity);
        } else {
            self.sharing_untracked = true;
        }
        true
    }

    fn measure_file(&mut self, name: &OsStr, metadata: &Stat, logical: u64, allocated: u64) {
        self.logical = self.logical.saturating_add(logical);
        self.allocated = self.allocated.saturating_add(allocated);
        let category = category_for(name);
        let slot = category.position();
        if let Some(tally) = self.tallies.get_mut(self.volume) {
            tally.logical = tally.logical.saturating_add(logical);
            tally.allocated = tally.allocated.saturating_add(allocated);
            let totals = &mut tally.categories[slot];
            totals.0 = totals.0.saturating_add(logical);
            totals.1 = totals.1.saturating_add(allocated);
            totals.2 += 1;
        }
        let ranking = &mut self.top[slot];
        let smallest = ranking.last().map_or(0, |file| file.allocated_bytes);
        if allocated == 0 || (ranking.len() >= CATEGORY_TOP_FILES && allocated <= smallest) {
            return;
        }
        let position = ranking.partition_point(|file| file.allocated_bytes >= allocated);
        ranking.insert(
            position,
            TopFile {
                category,
                path: self.path.join(name),
                logical_bytes: logical,
                allocated_bytes: allocated,
                modified_at_ms: modified_ms(metadata),
                identity: format!("{}:{}", metadata.st_dev, metadata.st_ino),
            },
        );
        ranking.truncate(CATEGORY_TOP_FILES);
    }

    fn record_leaf(&mut self, leaf: Leaf<'_>) {
        let flagged = leaf.issue.is_some();
        if flagged {
            self.issues += 1;
        }
        let pin_wanted = flagged || (self.policy.pin_predicate)(&self.path, leaf.name, leaf.kind);
        let pinned = self.grant_pin(pin_wanted);
        let keep_sized = !pinned && leaf.allocated >= self.threshold && self.policy.top_k > 0;
        let frame = top_frame(&mut self.frames);
        frame.logical = frame.logical.saturating_add(leaf.logical);
        frame.allocated = frame.allocated.saturating_add(leaf.allocated);
        frame.descendants += 1;
        frame.issues += u64::from(flagged);
        if !pinned && !keep_sized {
            frame.fold.absorb(1, leaf.logical, leaf.allocated);
            return;
        }
        let node = Node {
            name: leaf.name.to_owned(),
            kind: leaf.kind,
            identity: leaf.identity,
            modified_at_ms: leaf.modified_at_ms,
            measured_at_ms: now_ms(),
            logical: leaf.logical,
            allocated: leaf.allocated,
            alias: leaf.alias,
            issue: leaf.issue,
            complete: leaf.issue.is_none(),
            pinned,
            descendants: 0,
            fold: Fold::default(),
            children: Vec::new(),
        };
        if pinned {
            frame.children.push(node);
            self.nodes += 1;
            return;
        }
        if frame.files.len() < self.policy.top_k {
            frame.files.push(node);
            self.nodes += 1;
            return;
        }
        let Some((smallest, existing)) = frame
            .files
            .iter()
            .enumerate()
            .map(|(position, file)| (position, file.allocated))
            .min_by_key(|(_, allocated)| *allocated)
        else {
            return;
        };
        let displaced = if node.allocated > existing {
            std::mem::replace(&mut frame.files[smallest], node)
        } else {
            node
        };
        frame.fold.absorb(1, displaced.logical, displaced.allocated);
    }
}

impl Leaf<'_> {
    fn issue<'name>(
        name: &'name OsStr,
        kind: Kind,
        metadata: Option<&Stat>,
        issue: Issue,
    ) -> Leaf<'name> {
        Leaf {
            name,
            kind,
            identity: metadata.map(|metadata| (metadata.st_dev, metadata.st_ino)),
            modified_at_ms: metadata.and_then(modified_ms),
            logical: 0,
            allocated: 0,
            alias: false,
            issue: Some(issue),
        }
    }
}

fn category_totals(tally: &VolumeTally) -> Vec<CategoryTotal> {
    CATEGORIES
        .iter()
        .zip(tally.categories)
        .map(|(name, (logical, allocated, count))| CategoryTotal {
            name: *name,
            logical_bytes: logical,
            allocated_bytes: allocated,
            count,
        })
        .collect()
}

fn same_mount(parent: &File, directory: &File) -> io::Result<bool> {
    let parent_mount = fs::fstatfs(parent)?;
    let child_mount = fs::fstatfs(directory)?;
    #[cfg(target_os = "macos")]
    return Ok(parent_mount.f_mntonname == child_mount.f_mntonname);
    #[cfg(not(target_os = "macos"))]
    Ok(parent_mount.f_fsid == child_mount.f_fsid)
}

pub(crate) fn kind(metadata: &Stat) -> Kind {
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
    use std::os::unix::fs::{symlink, MetadataExt, PermissionsExt};
    use tempfile::TempDir;

    fn fixture() -> (TempDir, PathBuf) {
        let temporary = tempfile::tempdir().unwrap();
        let path = temporary.path().canonicalize().unwrap();
        (temporary, path)
    }

    fn measure_with(paths: &[PathBuf], policy: RetentionPolicy) -> Arc<Mutex<ScanData>> {
        let data = Arc::new(Mutex::new(ScanData::new(1)));
        scan_with_policy(
            paths.iter().map(|path| Root::open(path).unwrap()).collect(),
            Vec::new(),
            policy,
            data.clone(),
            Arc::new(AtomicBool::new(false)),
        );
        data
    }

    fn measure(paths: &[PathBuf]) -> Arc<Mutex<ScanData>> {
        measure_with(paths, RetentionPolicy::exhaustive(usize::MAX))
    }

    fn no_pins(_: &Path, _: &OsStr, _: Kind) -> bool {
        false
    }

    fn scanner(policy: RetentionPolicy) -> Scanner {
        Scanner::new(
            Vec::new(),
            policy,
            Arc::new(Mutex::new(ScanData::new(2))),
            Arc::new(AtomicBool::new(false)),
        )
    }

    fn scanner_with_frame() -> Scanner {
        let mut scanner = scanner(RetentionPolicy::exhaustive(usize::MAX));
        scanner.frames.push(Frame::default());
        scanner
    }

    #[derive(Default, Debug, PartialEq, Eq)]
    struct Brute {
        logical: u64,
        allocated: u64,
        entries: u64,
    }

    /// Independent reference walk: std only, no retention, identity de-duplication by nlink.
    fn brute_force(directory: &Path, seen: &mut HashSet<(u64, u64)>, total: &mut Brute) {
        let Ok(listing) = fs::read_dir(directory) else {
            return;
        };
        for entry in listing {
            let path = entry.unwrap().path();
            let metadata = fs::symlink_metadata(&path).unwrap();
            total.entries += 1;
            if metadata.is_dir() {
                if seen.insert((metadata.dev(), metadata.ino())) {
                    brute_force(&path, seen, total);
                }
                continue;
            }
            if !metadata.is_file()
                || (metadata.nlink() > 1 && !seen.insert((metadata.dev(), metadata.ino())))
            {
                continue;
            }
            total.logical += metadata.len();
            total.allocated += metadata.blocks() * 512;
        }
    }

    fn reference(root: &Path) -> Brute {
        let mut total = Brute {
            entries: 1,
            ..Brute::default()
        };
        let metadata = fs::symlink_metadata(root).unwrap();
        let mut seen = HashSet::from([(metadata.dev(), metadata.ino())]);
        brute_force(root, &mut seen, &mut total);
        total
    }

    fn pseudo_random(state: &mut u64) -> u64 {
        *state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        *state >> 33
    }

    fn populate(directory: &Path, depth: usize, state: &mut u64, files: &mut Vec<PathBuf>) {
        let extensions = ["pdf", "png", "zip", "txt", "bin", "", "MD", "mp4"];
        for index in 0..(2 + pseudo_random(state) % 6) {
            let extension = extensions[(pseudo_random(state) % 8) as usize];
            let path = directory.join(format!("file-{index}.{extension}"));
            if !files.is_empty() && pseudo_random(state).is_multiple_of(5) {
                let target = files[(pseudo_random(state) as usize) % files.len()].clone();
                fs::hard_link(target, &path).unwrap();
                continue;
            }
            let size = match pseudo_random(state) % 4 {
                0 => 0,
                1 => 100,
                2 => 6_000,
                _ => 70_000,
            };
            fs::write(&path, vec![b'x'; size]).unwrap();
            files.push(path);
        }
        if depth == 0 {
            return;
        }
        for index in 0..(1 + pseudo_random(state) % 3) {
            let name = if index == 0 && depth == 2 {
                "node_modules".to_string()
            } else {
                format!("dir-{index}")
            };
            let child = directory.join(name);
            fs::create_dir(&child).unwrap();
            populate(&child, depth - 1, state, files);
        }
    }

    fn build_tree(root: &Path, seed: u64) {
        let mut state = seed;
        let mut files = Vec::new();
        populate(root, 4, &mut state, &mut files);
        let sparse = File::create(root.join("sparse.bin")).unwrap();
        sparse.set_len(3_000_000).unwrap();
        symlink(".", root.join("cycle")).unwrap();
        symlink("missing", root.join("dangling")).unwrap();
        symlink("/", root.join("escape")).unwrap();
    }

    fn assert_invariants(data: &ScanData) {
        let records = &data.records;
        for (index, record) in records.iter().enumerate() {
            if let Some(parent) = record.parent {
                assert!(parent < index, "ancestors precede descendants");
                assert_eq!(
                    record.finding.parent_id.as_ref(),
                    Some(&records[parent].finding.id)
                );
            }
        }
        for (index, record) in records.iter().enumerate() {
            if record.finding.kind != Kind::Directory {
                continue;
            }
            let (mut logical, mut allocated, mut items) = (0_u64, 0_u64, 0_u64);
            for child in records.iter().filter(|child| child.parent == Some(index)) {
                logical += child.finding.logical_bytes;
                allocated += child.finding.allocated_bytes;
                items += 1 + child.finding.descendant_items;
            }
            let finding = &record.finding;
            assert_eq!(
                finding.logical_bytes,
                logical + finding.folded_logical_bytes
            );
            assert_eq!(
                finding.allocated_bytes,
                allocated + finding.folded_allocated_bytes
            );
            assert_eq!(finding.descendant_items, items + finding.folded_items);
            assert_eq!(finding.detail == Detail::Full, finding.folded_items == 0);
        }
        for accounting in &data.progress.volumes {
            let allocated: u64 = accounting
                .categories
                .iter()
                .map(|category| category.allocated_bytes)
                .sum();
            let logical: u64 = accounting
                .categories
                .iter()
                .map(|category| category.logical_bytes)
                .sum();
            assert_eq!(allocated, accounting.scope_allocated_bytes);
            assert_eq!(logical, accounting.scope_logical_bytes);
        }
    }

    #[test]
    fn exclusions_are_literal_bounded_and_do_not_follow_or_measure_skipped_entries() {
        for invalid in ["", "/tmp", "../secret", "a/../b", "a/./b", "a//b", "a\n"] {
            assert!(!Exclusion {
                relative_path: invalid.into(),
                subtree: true
            }
            .validate());
        }
        let (_temporary, path) = fixture();
        fs::create_dir(path.join("cache")).unwrap();
        fs::write(path.join("cache/hidden"), b"excluded").unwrap();
        fs::create_dir(path.join("exact-directory")).unwrap();
        fs::write(path.join("exact-directory/kept"), b"12345").unwrap();
        fs::write(path.join("skip"), b"excluded").unwrap();
        let data = Arc::new(Mutex::new(ScanData::new(1)));
        scan_with_policy(
            vec![Root::open(&path).unwrap()],
            vec![
                (path.join("cache"), true),
                (path.join("skip"), false),
                (path.join("exact-directory"), false),
            ],
            RetentionPolicy::exhaustive(usize::MAX),
            data.clone(),
            Arc::new(AtomicBool::new(false)),
        );
        let data = lock(&data);
        assert_eq!(data.progress.logical_bytes, 5);
        assert_eq!(data.progress.status, Status::Partial);
        assert_eq!(
            data.records
                .iter()
                .filter(|record| record.finding.issue == Some(Issue::Excluded))
                .count(),
            2
        );
        assert!(!data.records.iter().any(|record| record.name == "hidden"));
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
        assert!(data.records.iter().all(|record| record.finding.complete));
        assert!(volume(&Root::open(&path).unwrap()).total_bytes.is_some());
        assert_invariants(&data);
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
        scan_with_policy(
            vec![root],
            Vec::new(),
            RetentionPolicy::exhaustive(usize::MAX),
            data.clone(),
            Arc::new(AtomicBool::new(false)),
        );
        assert_eq!(lock(&data).progress.logical_bytes, 2);
        assert!(!lock(&data)
            .records
            .iter()
            .any(|record| record.name == "replacement"));
    }

    #[test]
    fn cancellation_and_depth_limit_report_explicit_coverage() {
        let (_temporary, path) = fixture();
        for index in 0..50 {
            fs::write(path.join(format!("file-{index}")), b"x").unwrap();
        }
        let mut cancelled = scanner(RetentionPolicy::default());
        cancelled.cancel.store(true, Ordering::Relaxed);
        cancelled.run(vec![Root::open(&path).unwrap()]);
        assert_eq!(lock(&cancelled.data).progress.status, Status::Cancelled);
        assert_eq!(
            lock(&cancelled.data).progress.terminal_issue,
            Some(Issue::Cancelled)
        );
        assert_eq!(lock(&cancelled.data).progress.logical_bytes, 0);
        let mut nested = path.clone();
        for _ in 0..10 {
            nested.push("nested");
            fs::create_dir(&nested).unwrap();
        }
        let mut limited = scanner(RetentionPolicy::default());
        limited.depth_limit = 3;
        limited.run(vec![Root::open(&path).unwrap()]);
        let data = lock(&limited.data);
        assert_eq!(data.progress.status, Status::Partial);
        assert!(data
            .records
            .iter()
            .any(|record| record.finding.issue == Some(Issue::DepthLimit)));
        assert_eq!(data.progress.logical_bytes, 50);
    }

    #[test]
    fn vanished_permission_mount_and_change_have_explicit_coverage() {
        let (_temporary, path) = fixture();
        let root = Root::open(&path).unwrap();
        let mut scanner = scanner_with_frame();
        let metadata = rustix::fs::fstat(&root.directory).unwrap();
        scanner.visit_entry(&root.directory, OsStr::new("missing"), metadata.st_dev, 1);
        assert_eq!(scanner.frames[0].children[0].issue, Some(Issue::Vanished));
        fs::create_dir(path.join("denied")).unwrap();
        fs::write(path.join("denied/inside"), b"hidden").unwrap();
        fs::set_permissions(path.join("denied"), fs::Permissions::from_mode(0o0)).unwrap();
        let denied = open_directory(&root.directory, OsStr::new("denied"));
        let denied_scan = measure_with(
            std::slice::from_ref(&path),
            RetentionPolicy::exhaustive(usize::MAX),
        );
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
            OsStr::new("mounted"),
            metadata.st_dev.wrapping_add(1),
            1,
        );
        assert_eq!(
            scanner.frames[0].children[1].issue,
            Some(Issue::MountBoundary)
        );
        assert_eq!(scanner.frames[0].logical, 0);
        assert_eq!(scanner.logical, 0);
        let before = rustix::fs::fstat(&root.directory).unwrap();
        fs::write(path.join("changed"), b"x").unwrap();
        let after = rustix::fs::fstat(&root.directory).unwrap();
        assert!(!unchanged(&before, &after));
        let mut changed = self::scanner(RetentionPolicy::exhaustive(usize::MAX));
        let result = changed.walk_directory(
            root.directory.try_clone().unwrap(),
            path.as_os_str(),
            before,
            0,
        );
        let node = result.node.unwrap();
        assert_eq!(node.issue, Some(Issue::Changed));
        assert!(!node.complete);
    }

    #[test]
    fn totals_match_brute_force_and_do_not_depend_on_retention() {
        for seed in [1, 7, 99] {
            let (_temporary, path) = fixture();
            build_tree(&path, seed);
            let expected = reference(&path);
            assert!(expected.logical > 3_000_000);
            let policies = [
                RetentionPolicy::default(),
                RetentionPolicy::exhaustive(usize::MAX),
                RetentionPolicy {
                    depth: 0,
                    threshold: u64::MAX,
                    top_k: 0,
                    node_budget: usize::MAX,
                    pin_predicate: no_pins,
                },
                RetentionPolicy {
                    depth: 1,
                    threshold: 0,
                    top_k: 1,
                    node_budget: 7,
                    pin_predicate: no_pins,
                },
                RetentionPolicy {
                    depth: usize::MAX,
                    threshold: 4_096,
                    top_k: 2,
                    node_budget: 25,
                    pin_predicate: crate::recognition::is_pinned,
                },
                RetentionPolicy {
                    depth: 0,
                    threshold: 0,
                    top_k: 3,
                    node_budget: 12,
                    pin_predicate: no_pins,
                },
            ];
            let mut baseline = None;
            for policy in policies {
                let data = measure_with(std::slice::from_ref(&path), policy);
                let data = lock(&data);
                assert_eq!(data.progress.status, Status::Complete);
                assert_eq!(
                    Brute {
                        logical: data.progress.logical_bytes,
                        allocated: data.progress.allocated_bytes,
                        entries: data.progress.entries as u64,
                    },
                    expected
                );
                assert_eq!(data.records[0].finding.logical_bytes, expected.logical);
                assert_eq!(
                    data.records[0].finding.descendant_items,
                    expected.entries - 1
                );
                assert_invariants(&data);
                if policy.pin_predicate as usize == no_pins as usize {
                    assert!(data.records.len() <= policy.node_budget);
                }
                let categories: Vec<_> = data.progress.volumes[0]
                    .categories
                    .iter()
                    .map(|category| {
                        (
                            category.logical_bytes,
                            category.allocated_bytes,
                            category.count,
                        )
                    })
                    .collect();
                match &baseline {
                    None => baseline = Some(categories),
                    Some(first) => assert_eq!(first, &categories),
                }
            }
        }
    }

    #[test]
    fn retention_budget_is_information_and_pins_keep_marker_closure() {
        let (_temporary, path) = fixture();
        fs::create_dir_all(path.join("a/b/c/d/project")).unwrap();
        fs::write(path.join("a/b/c/d/project/package.json"), b"{}").unwrap();
        fs::write(path.join("a/b/c/d/project/noise"), b"n").unwrap();
        for index in 0..30 {
            fs::create_dir(path.join(format!("bulk-{index}"))).unwrap();
        }
        let policy = RetentionPolicy {
            depth: 1,
            threshold: 1 << 20,
            top_k: 2,
            node_budget: 8,
            pin_predicate: crate::recognition::is_pinned,
        };
        let data = measure_with(std::slice::from_ref(&path), policy);
        let data = lock(&data);
        assert_eq!(data.progress.status, Status::Complete);
        assert!(data.progress.retention.budget_reached);
        assert!(data.progress.retention.threshold_bytes > 1 << 20);
        assert_eq!(data.progress.retention.nodes, data.records.len());
        let marker = data
            .records
            .iter()
            .position(|record| record.name == "package.json")
            .expect("pinned marker file is retained");
        let mut current = data.records[marker].parent;
        let mut names = Vec::new();
        while let Some(index) = current {
            names.push(data.records[index].name.clone());
            current = data.records[index].parent;
        }
        assert_eq!(names.len(), 6, "every ancestor of a pin is retained");
        let project = data.records[marker].parent.unwrap();
        assert_eq!(data.records[project].finding.detail, Detail::Summarized);
        assert_eq!(data.records[project].finding.folded_items, 1);
        assert_invariants(&data);
    }

    #[test]
    fn hard_link_identity_cap_degrades_to_flagged_upper_bound() {
        let (_temporary, path) = fixture();
        for pair in ["a", "b"] {
            fs::write(path.join(format!("{pair}1")), b"0123456789").unwrap();
            fs::hard_link(path.join(format!("{pair}1")), path.join(format!("{pair}2"))).unwrap();
        }
        let mut capped = scanner(RetentionPolicy::default());
        capped.file_identity_cap = 1;
        capped.run(vec![Root::open(&path).unwrap()]);
        let data = lock(&capped.data);
        assert!(data.progress.sharing_untracked);
        assert_eq!(data.progress.logical_bytes, 30);
        drop(data);
        let exact = measure(&[path]);
        assert!(!lock(&exact).progress.sharing_untracked);
        assert_eq!(lock(&exact).progress.logical_bytes, 20);
    }

    #[test]
    fn categories_follow_the_extension_table_and_rank_largest_files() {
        for (name, expected) in [
            ("a.PDF", Category::Documents),
            ("a.tar.gz", Category::Archives),
            ("movie.MP4", Category::Media),
            ("pdf", Category::Documents),
            ("no-extension", Category::Other),
            ("archive.zip.bak", Category::Other),
        ] {
            assert_eq!(category_for(OsStr::new(name)), expected, "{name}");
        }
        let (_temporary, path) = fixture();
        fs::write(path.join("small.pdf"), vec![1; 5_000]).unwrap();
        fs::write(path.join("large.pdf"), vec![1; 90_000]).unwrap();
        fs::write(path.join("photo.png"), vec![1; 20_000]).unwrap();
        let data = measure_with(std::slice::from_ref(&path), RetentionPolicy::default());
        let data = lock(&data);
        let documents = data.progress.volumes[0]
            .categories
            .iter()
            .find(|category| category.name == Category::Documents)
            .unwrap();
        assert_eq!((documents.logical_bytes, documents.count), (95_000, 2));
        let ranked: Vec<_> = data
            .top_files
            .iter()
            .filter(|file| file.category == Category::Documents)
            .map(|file| file.path.clone())
            .collect();
        assert_eq!(ranked, [path.join("large.pdf"), path.join("small.pdf")]);
        assert_invariants(&data);
    }

    #[test]
    fn two_hundred_thousand_files_complete_within_the_node_budget() {
        let (_temporary, path) = fixture();
        let mut expected_logical = 0;
        for group in 0..20 {
            let group_path = path.join(format!("group-{group}"));
            fs::create_dir(&group_path).unwrap();
            for bucket in 0..25 {
                let bucket_path = group_path.join(format!("bucket-{bucket}"));
                fs::create_dir(&bucket_path).unwrap();
                for index in 0..400 {
                    let file = File::create(bucket_path.join(format!("f{index}"))).unwrap();
                    if index % 50 == 0 {
                        file.set_len(1_000).unwrap();
                        expected_logical += 1_000;
                    }
                }
            }
        }
        let policy = RetentionPolicy {
            node_budget: 300,
            ..RetentionPolicy::default()
        };
        let data = measure_with(std::slice::from_ref(&path), policy);
        let data = lock(&data);
        assert_eq!(data.progress.status, Status::Complete);
        assert_eq!(data.progress.issue_count, 0);
        assert_eq!(data.progress.entries, 1 + 20 + 500 + 200_000);
        assert_eq!(data.progress.logical_bytes, expected_logical);
        assert!(data.progress.retention.budget_reached);
        assert!(data.records.len() <= 300, "{}", data.records.len());
        assert_eq!(data.records[0].finding.descendant_items, 200_520);
        assert_invariants(&data);
    }
}

#[cfg(test)]
mod lossless_tests {
    use super::*;
    use std::os::unix::ffi::OsStringExt;
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
                folded_items: 0,
                folded_logical_bytes: 0,
                folded_allocated_bytes: 0,
                descendant_items: 0,
                detail: Detail::Full,
            },
        });
        assert_eq!(data.path(0).unwrap().into_os_string(), name);
        assert!(data.records[0].finding.display_name.contains('�'));
        assert_eq!(display(OsStr::new(&"x".repeat(2000))).len(), MAX_DISPLAY);
        assert_eq!(display(OsStr::new("a\nb\u{202e}")), "a�b�");
    }
}
