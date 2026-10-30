//! Read-only recognition over one scanner generation. Findings never grant removal authority.

use crate::scan::{display, Detail, Kind, Record, Root};
use rustix::fs::{self, AtFlags, Dir, Mode, OFlags};
use serde::Serialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::Read;
use std::os::unix::ffi::OsStrExt;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

mod families;
mod git;
mod pins;

pub use pins::is_pinned;

const MAX_METADATA_BYTES: usize = 65_536;
const MAX_TOTAL_METADATA: usize = 8 * 1024 * 1024;
const MAX_METADATA_FILES: usize = 4096;
const MAX_DIAGNOSTICS: usize = 256;
const MAX_PROBES: usize = 8192;
const MAX_LISTED_ENTRIES: usize = 1024;
const RECOGNITION_BUDGET: Duration = Duration::from_secs(15);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Family {
    Docker,
    JavaScript,
    Python,
    Rust,
    ProjectLogs,
    Npm,
    Pnpm,
    Yarn,
    Bun,
    Pip,
    Uv,
    Nvm,
    Go,
    Cargo,
    Homebrew,
    VsCode,
    Cursor,
    JetBrains,
    Claude,
    Codex,
    Git,
    Temporary,
    Protected,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Nature {
    Project,
    Dependencies,
    Build,
    Logs,
    SharedStore,
    Runtime,
    EditorState,
    Worktree,
    Temporary,
    Protected,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    Verified,
    Contextual,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanupSupport {
    InspectionOnly,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Evidence {
    pub finding_id: String,
    pub method: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    pub finding_id: String,
    pub family: Family,
    pub nature: Nature,
    /// Scanner ID of the nearest verified project; null for shared/unassigned data.
    pub project_id: Option<String>,
    pub display_name: String,
    pub parent_id: Option<String>,
    pub kind: Kind,
    pub confidence: Confidence,
    pub evidence: Vec<Evidence>,
    pub consequence: String,
    pub cleanup_support: CleanupSupport,
    pub regenerable: bool,
    pub complete: bool,
    /// Exclusive scanner contributions, excluding every separately recognized descendant.
    pub exclusive_logical_bytes: String,
    pub exclusive_allocated_bytes: String,
    /// Only project findings carry these totals; includes owned artifacts, excludes nested projects.
    pub owned_logical_bytes: Option<String>,
    pub owned_allocated_bytes: Option<String>,
    pub worktree: Option<Worktree>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Worktree {
    pub repository_id: Option<String>,
    pub metadata_id: Option<String>,
    pub branch: Option<String>,
    pub main: bool,
    pub detached: bool,
    pub locked: bool,
    pub changes: ChangeState,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeState {
    Clean,
    Dirty,
    Untracked,
    Ignored,
    Unknown,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub finding_id: String,
    pub reason: String,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Recognition {
    pub finished: bool,
    pub cancelled: bool,
    pub limited: bool,
    pub diagnostics_truncated: bool,
    pub findings: Vec<Artifact>,
    pub diagnostics: Vec<Diagnostic>,
}

/// A file or directory reached by `probe()` that the scanner did not retain.
struct Probed {
    parent: usize,
    name: OsString,
    kind: Kind,
    identity: String,
}

/// Handles below `records.len()` are retained scanner records; the rest index `probed`.
struct Recognizer<'records> {
    records: &'records [Record],
    roots: Vec<usize>,
    children: HashMap<(usize, OsString), usize>,
    children_by_parent: Vec<Vec<usize>>,
    probed: Vec<Probed>,
    probe_cache: HashMap<(usize, OsString), Option<usize>>,
    probes: usize,
    store_locations: Vec<(Family, PathBuf)>,
    contents: HashMap<usize, Option<String>>,
    bytes_read: usize,
    reads: usize,
    cancel: &'records AtomicBool,
    started: Instant,
    pins_exhausted: bool,
    result: Recognition,
    artifacts: HashMap<usize, usize>,
    projects: HashSet<usize>,
}

/// Only native scanner records may be supplied. No display path is resolved as authority.
/// `pins_exhausted` reports that the scanner could not retain every marker it was asked to.
pub fn recognize(records: &[Record], cancel: &AtomicBool, pins_exhausted: bool) -> Recognition {
    let mut recognizer = Recognizer::new(records, cancel, pins_exhausted);
    recognizer.run();
    recognizer.result
}

impl<'records> Recognizer<'records> {
    fn new(
        records: &'records [Record],
        cancel: &'records AtomicBool,
        pins_exhausted: bool,
    ) -> Self {
        let store_locations = pins::STORE_ENVIRONMENT
            .iter()
            .filter_map(|(variable, family)| {
                std::env::var_os(variable).map(|path| (*family, PathBuf::from(path)))
            })
            .collect();
        Self {
            records,
            roots: Vec::new(),
            children: HashMap::new(),
            children_by_parent: vec![Vec::new(); records.len()],
            probed: Vec::new(),
            probe_cache: HashMap::new(),
            probes: 0,
            store_locations,
            contents: HashMap::new(),
            bytes_read: 0,
            reads: 0,
            cancel,
            started: Instant::now(),
            pins_exhausted,
            result: Recognition::default(),
            artifacts: HashMap::new(),
            projects: HashSet::new(),
        }
    }

    fn run(&mut self) {
        for (index, record) in self.records.iter().enumerate() {
            match record.parent {
                Some(parent) => {
                    self.children.insert((parent, record.name.clone()), index);
                    self.children_by_parent[parent].push(index);
                }
                None => self.roots.push(index),
            }
        }
        for index in 0..self.records.len() {
            if self.stopped() {
                break;
            }
            self.inventory(index);
        }
        for index in 0..self.records.len() {
            if self.stopped() {
                break;
            }
            if self.records[index].finding.kind != Kind::Directory
                || self.records[index].finding.alias_of.is_some()
            {
                continue;
            }
            self.project(index);
        }
        for index in 0..self.records.len() {
            if self.stopped() {
                break;
            }
            self.classify(index);
        }
        self.account();
        if self.pins_exhausted && !self.records.is_empty() {
            self.result.limited = true;
            self.diagnostic(0, "pin_budget_exhausted");
        }
        self.result.finished = !self.result.cancelled && !self.result.limited;
    }

    fn stopped(&mut self) -> bool {
        if self.cancel.load(Ordering::Relaxed) {
            self.result.cancelled = true;
        }
        if self.started.elapsed() >= RECOGNITION_BUDGET {
            self.result.limited = true;
        }
        self.result.cancelled || self.result.limited
    }

    fn record(&self, handle: usize) -> Option<&Record> {
        self.records.get(handle)
    }

    fn probed(&self, handle: usize) -> Option<&Probed> {
        handle
            .checked_sub(self.records.len())
            .and_then(|position| self.probed.get(position))
    }

    fn kind_of(&self, handle: usize) -> Kind {
        match (self.record(handle), self.probed(handle)) {
            (Some(record), _) => record.finding.kind,
            (None, Some(probed)) => probed.kind,
            _ => Kind::Unknown,
        }
    }

    fn complete_of(&self, handle: usize) -> bool {
        self.record(handle)
            .map_or(self.probed(handle).is_some(), |record| {
                record.finding.complete
            })
    }

    fn parent_of(&self, handle: usize) -> Option<usize> {
        match self.record(handle) {
            Some(record) => record.parent,
            None => self.probed(handle).map(|probed| probed.parent),
        }
    }

    fn name_of(&self, handle: usize) -> &OsStr {
        match (self.record(handle), self.probed(handle)) {
            (Some(record), _) => &record.name,
            (None, Some(probed)) => &probed.name,
            _ => OsStr::new(""),
        }
    }

    fn identity_of(&self, handle: usize) -> Option<&str> {
        match (self.record(handle), self.probed(handle)) {
            (Some(record), _) => record.finding.identity.as_deref(),
            (None, Some(probed)) => Some(&probed.identity),
            _ => None,
        }
    }

    /// Scanner ID of the node itself, or of the nearest retained ancestor for a probed one.
    fn id_of(&self, handle: usize) -> &str {
        let mut current = handle;
        while current >= self.records.len() {
            let Some(probed) = self.probed(current) else {
                return "";
            };
            current = probed.parent;
        }
        &self.records[current].finding.id
    }

    /// Scanner ID only when the node is itself retained.
    fn retained_id(&self, handle: usize) -> Option<String> {
        self.record(handle).map(|record| record.finding.id.clone())
    }

    fn path(&self, handle: usize) -> Option<PathBuf> {
        let mut components = Vec::new();
        let mut current = Some(handle);
        while let Some(handle) = current {
            if handle >= self.records.len() + self.probed.len() {
                return None;
            }
            components.push(self.name_of(handle));
            current = self.parent_of(handle);
        }
        Some(components.into_iter().rev().collect())
    }

    /// Compares trailing path components without building the path.
    fn ends_with(&self, handle: usize, suffix: &str) -> bool {
        let mut current = Some(handle);
        for component in Path::new(suffix).components().rev() {
            let Some(handle) = current else { return false };
            if component.as_os_str() != self.name_of(handle) {
                return false;
            }
            current = self.parent_of(handle);
        }
        true
    }

    /// Retained children only; every name read this way must be pinned.
    fn child(&self, parent: usize, relative: &str) -> Option<usize> {
        let mut current = parent;
        for component in Path::new(relative).components() {
            let Component::Normal(name) = component else {
                return None;
            };
            current = *self.children.get(&(current, name.to_owned()))?;
        }
        Some(current)
    }

    /// Retained child first, then a read-only descriptor-anchored lookup below directories that
    /// folded some of their children. Never follows symlinks or leaves the mount.
    fn probe(&mut self, parent: usize, relative: &str) -> Option<usize> {
        let mut current = parent;
        for component in Path::new(relative).components() {
            let Component::Normal(name) = component else {
                return None;
            };
            current = self.step(current, name)?;
        }
        Some(current)
    }

    fn step(&mut self, parent: usize, name: &OsStr) -> Option<usize> {
        let key = (parent, name.to_owned());
        if let Some(found) = self.children.get(&key) {
            return Some(*found);
        }
        if self.lists_every_child(parent) || self.kind_of(parent) != Kind::Directory {
            return None;
        }
        if let Some(cached) = self.probe_cache.get(&key) {
            return *cached;
        }
        if self.stopped() || !self.spend_probe(parent) {
            return None;
        }
        let found = self
            .open_checked(parent)
            .ok()
            .and_then(|directory| self.stat_in(parent, &directory, name));
        self.probe_cache.insert(key, found);
        found
    }

    fn lists_every_child(&self, handle: usize) -> bool {
        self.record(handle).is_some_and(|record| {
            record.finding.detail == Detail::Full || record.finding.alias_of.is_some()
        })
    }

    fn spend_probe(&mut self, near: usize) -> bool {
        if self.probes >= MAX_PROBES {
            self.result.limited = true;
            self.diagnostic(near, "probe_budget_exhausted");
            return false;
        }
        self.probes += 1;
        true
    }

    fn stat_in(&mut self, parent: usize, directory: &File, name: &OsStr) -> Option<usize> {
        let metadata = fs::statat(directory, name, AtFlags::SYMLINK_NOFOLLOW).ok()?;
        if metadata.st_dev != fs::fstat(directory).ok()?.st_dev {
            return None;
        }
        self.probed.push(Probed {
            parent,
            name: name.to_owned(),
            kind: crate::scan::kind(&metadata),
            identity: format!("{}:{}", metadata.st_dev, metadata.st_ino),
        });
        Some(self.records.len() + self.probed.len() - 1)
    }

    /// Retained children plus, below a directory that folded some, a bounded listing.
    fn list_children(&mut self, parent: usize) -> Vec<usize> {
        let mut handles = self
            .children_by_parent
            .get(parent)
            .cloned()
            .unwrap_or_default();
        if self.lists_every_child(parent) || self.kind_of(parent) != Kind::Directory {
            return handles;
        }
        if self.stopped() || !self.spend_probe(parent) {
            return handles;
        }
        let Ok(directory) = self.open_checked(parent) else {
            return handles;
        };
        let Ok(entries) = Dir::read_from(&directory) else {
            return handles;
        };
        let mut listed = 0;
        for entry in entries.flatten() {
            let name = OsStr::from_bytes(entry.file_name().to_bytes());
            if name == "." || name == ".." || self.children.contains_key(&(parent, name.to_owned()))
            {
                continue;
            }
            listed += 1;
            if listed > MAX_LISTED_ENTRIES {
                break;
            }
            let key = (parent, name.to_owned());
            let found = match self.probe_cache.get(&key) {
                Some(cached) => *cached,
                None => {
                    let found = self.stat_in(parent, &directory, name);
                    self.probe_cache.insert(key, found);
                    found
                }
            };
            handles.extend(found);
        }
        handles
    }

    /// Resolves an absolute native path below a scanned root; anything outside is `None`.
    fn locate(&mut self, path: &Path) -> Option<usize> {
        let root = self
            .roots
            .iter()
            .copied()
            .filter(|root| path.starts_with(&self.records[*root].name))
            .max_by_key(|root| self.records[*root].name.len())?;
        let relative = path.strip_prefix(&self.records[root].name).ok()?;
        let mut current = root;
        for component in relative.components() {
            let Component::Normal(name) = component else {
                return None;
            };
            current = self.step(current, name)?;
        }
        Some(current)
    }

    fn diagnostic(&mut self, handle: usize, reason: &str) {
        if self.result.diagnostics.len() >= MAX_DIAGNOSTICS {
            self.result.diagnostics_truncated = true;
            return;
        }
        let finding_id = self.id_of(handle).to_owned();
        self.result.diagnostics.push(Diagnostic {
            finding_id,
            reason: reason.into(),
        });
    }

    fn text(&mut self, index: usize) -> Option<String> {
        if let Some(content) = self.contents.get(&index) {
            return content.clone();
        }
        if self.stopped() {
            return None;
        }
        if self.reads >= MAX_METADATA_FILES || self.bytes_read >= MAX_TOTAL_METADATA {
            self.result.limited = true;
            self.diagnostic(index, "metadata_budget_exhausted");
            return None;
        }
        self.reads += 1;
        let result = self.read_checked(index);
        let content = match result {
            Ok(content) => Some(content),
            Err(reason) => {
                self.diagnostic(index, reason);
                None
            }
        };
        self.contents.insert(index, content.clone());
        content
    }

    fn open_checked(&self, handle: usize) -> Result<File, &'static str> {
        let mut chain = Vec::new();
        let mut current = Some(handle);
        while let Some(handle) = current {
            chain.push(handle);
            current = self.parent_of(handle);
        }
        chain.reverse();
        let root_index = *chain.first().ok_or("invalid_record")?;
        let root = self.record(root_index).ok_or("invalid_record")?;
        let root =
            Root::open(Path::new(&root.name)).map_err(|_| "metadata_unavailable_or_changed")?;
        let mut file = root.directory;
        self.check_identity(root_index, &file)?;
        for handle in chain.into_iter().skip(1) {
            let flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
            let flags = if self.kind_of(handle) == Kind::Directory {
                flags | OFlags::DIRECTORY
            } else {
                flags
            };
            file = File::from(
                fs::openat(&file, self.name_of(handle), flags, Mode::empty())
                    .map_err(|_| "metadata_unavailable_or_changed")?,
            );
            self.check_identity(handle, &file)?;
        }
        Ok(file)
    }

    fn check_identity(&self, handle: usize, file: &File) -> Result<(), &'static str> {
        let metadata = fs::fstat(file).map_err(|_| "metadata_unavailable_or_changed")?;
        let identity = format!("{}:{}", metadata.st_dev, metadata.st_ino);
        let flagged = self
            .record(handle)
            .is_some_and(|record| record.finding.issue.is_some());
        if self.identity_of(handle) != Some(identity.as_str()) || flagged {
            return Err("metadata_unavailable_or_changed");
        }
        Ok(())
    }

    fn read_checked(&mut self, index: usize) -> Result<String, &'static str> {
        if self.kind_of(index) != Kind::File || !self.complete_of(index) {
            return Err("metadata_not_regular_or_incomplete");
        }
        let mut file = self.open_checked(index)?;
        let before = fs::fstat(&file).map_err(|_| "metadata_unavailable_or_changed")?;
        if before.st_size < 0 || before.st_size as usize > MAX_METADATA_BYTES {
            return Err("metadata_too_large");
        }
        let expected_bytes = before.st_size as usize;
        let remaining_bytes = MAX_TOTAL_METADATA.saturating_sub(self.bytes_read);
        if expected_bytes > remaining_bytes {
            self.result.limited = true;
            return Err("metadata_budget_exhausted");
        }
        self.bytes_read += expected_bytes.saturating_add(1).min(remaining_bytes);
        let mut bytes = Vec::new();
        file.by_ref()
            .take((expected_bytes + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| "metadata_unavailable_or_changed")?;
        let after = fs::fstat(&file).map_err(|_| "metadata_unavailable_or_changed")?;
        if bytes.len() > MAX_METADATA_BYTES
            || before.st_size != after.st_size
            || before.st_mtime != after.st_mtime
            || before.st_mtime_nsec != after.st_mtime_nsec
            || before.st_ctime != after.st_ctime
            || before.st_ctime_nsec != after.st_ctime_nsec
        {
            return Err("metadata_changed");
        }
        String::from_utf8(bytes).map_err(|_| "metadata_not_utf8")
    }

    fn json(&mut self, index: usize) -> Option<Value> {
        let content = self.text(index)?;
        match serde_json::from_str::<Value>(&content) {
            Ok(Value::Object(object)) => Some(Value::Object(object)),
            _ => {
                self.diagnostic(index, "malformed_json_metadata");
                None
            }
        }
    }

    fn project(&mut self, index: usize) {
        if self.inside_generated(index) {
            self.git_worktree(index);
            return;
        }
        let mut evidence = None;
        let mut family = Family::Git;
        if let Some(manifest) = self.child(index, "package.json") {
            if let Some(value) = self.json(manifest) {
                if value.get("name").and_then(Value::as_str).is_some()
                    || value.get("dependencies").is_some_and(Value::is_object)
                    || value.get("devDependencies").is_some_and(Value::is_object)
                    || value.get("workspaces").is_some_and(Value::is_array)
                {
                    family = Family::JavaScript;
                    evidence = Some(manifest);
                }
            }
        }
        for (name, selected, section) in [
            ("pyproject.toml", Family::Python, "project"),
            ("Cargo.toml", Family::Rust, "package"),
        ] {
            let Some(manifest) = self.child(index, name) else {
                continue;
            };
            let Some(content) = self.text(manifest) else {
                continue;
            };
            match content.parse::<toml::Table>() {
                Ok(value)
                    if value.get(section).is_some_and(toml::Value::is_table)
                        || value.get("workspace").is_some_and(toml::Value::is_table)
                        || (selected == Family::Python
                            && value.get("tool").is_some_and(toml::Value::is_table)) =>
                {
                    family = selected;
                    evidence = Some(manifest);
                }
                Ok(_) => {}
                Err(_) => self.diagnostic(manifest, "malformed_toml_metadata"),
            }
        }
        if let Some(evidence) = evidence {
            self.projects.insert(index);
            self.add(
                index,
                family,
                Nature::Project,
                evidence,
                "validated_project_manifest",
                false,
            );
        }
        self.git_worktree(index);
    }

    fn inside_generated(&self, index: usize) -> bool {
        let mut parent = Some(index);
        while let Some(index) = parent {
            if self.artifacts.get(&index).is_some_and(|artifact| {
                matches!(
                    self.result.findings[*artifact].nature,
                    Nature::SharedStore | Nature::Runtime | Nature::EditorState
                )
            }) {
                return true;
            }
            if self.records[index]
                .name
                .to_str()
                .is_some_and(|name| pins::GENERATED_DIRECTORIES.contains(&name))
            {
                return true;
            }
            parent = self.records[index].parent;
        }
        false
    }

    fn nearest(&self, start: Option<usize>, contains: impl Fn(usize) -> bool) -> Option<usize> {
        let mut current = start;
        while let Some(index) = current {
            if contains(index) {
                return Some(index);
            }
            current = self.parent_of(index);
        }
        None
    }

    fn nearest_project(&self, index: usize) -> Option<usize> {
        self.nearest(Some(index), |candidate| self.projects.contains(&candidate))
    }

    fn add(
        &mut self,
        index: usize,
        family: Family,
        nature: Nature,
        evidence: usize,
        method: &str,
        regenerable: bool,
    ) {
        let Some(record) = self.records.get(index) else {
            return;
        };
        if self.artifacts.contains_key(&index) {
            return;
        }
        let project_id = self
            .nearest_project(index)
            .map(|project| self.records[project].finding.id.clone());
        let consequence = match nature {
            Nature::Project => "Contains source and project configuration. Preserve source, secrets and uncommitted work.",
            Nature::Dependencies => "Restoring dependencies may require downloads, package scripts and unavailable package versions. Local edits may exist; inspect before removal.",
            Nature::Build => "Rebuilding costs time and may require toolchains or downloads. Outputs can contain hand-written or persistent data; inspect contents first.",
            Nature::Logs => "Logs can contain private diagnostics and may be needed for investigation. No age-based removal guarantee.",
            Nature::SharedStore => "Shared downloads may serve several projects. Removing them can require network access and break offline builds.",
            Nature::Runtime => "Installed runtime or toolchain can be required by projects. Age does not prove inactivity or removability.",
            Nature::EditorState | Nature::Protected => "May contain settings, credentials, chats, workspace history or unsaved work. Preserve; inspection only.",
            Nature::Worktree => "Contains repository work. Clean status does not prove dispensability; ignored and untracked data must be preserved.",
            Nature::Temporary => "Explicitly selected temporary storage can contain active or persistent data. Neither location nor age proves safe removal.",
        };
        let finding = Artifact {
            finding_id: record.finding.id.clone(),
            family,
            nature,
            project_id,
            display_name: record.finding.display_name.clone(),
            parent_id: record.finding.parent_id.clone(),
            kind: record.finding.kind,
            confidence: Confidence::Verified,
            evidence: vec![Evidence {
                finding_id: self.id_of(evidence).to_owned(),
                method: method.into(),
            }],
            consequence: consequence.into(),
            cleanup_support: CleanupSupport::InspectionOnly,
            regenerable,
            complete: record.finding.complete,
            exclusive_logical_bytes: "0".into(),
            exclusive_allocated_bytes: "0".into(),
            owned_logical_bytes: None,
            owned_allocated_bytes: None,
            worktree: None,
        };
        self.artifacts.insert(index, self.result.findings.len());
        self.result.findings.push(finding);
    }

    /// Scanner totals of one retained node: a directory carries its whole subtree, folded
    /// descendants included, and an alias contributes nothing.
    fn subtree_total(&self, index: usize) -> (u64, u64) {
        let Some(record) = self.records.get(index) else {
            return (0, 0);
        };
        match record.finding.kind {
            Kind::Directory => (record.finding.logical_bytes, record.finding.allocated_bytes),
            Kind::File if record.finding.alias_of.is_none() => {
                (record.finding.logical_bytes, record.finding.allocated_bytes)
            }
            _ => (0, 0),
        }
    }

    /// Exclusive = subtree minus the subtrees of the nearest nested artifacts; owned = subtree
    /// minus the subtrees of the nearest nested projects. Both are exact over folded subtrees.
    fn account(&mut self) {
        let mut exclusive = vec![(0_u64, 0_u64); self.result.findings.len()];
        for (index, position) in &self.artifacts {
            exclusive[*position] = self.subtree_total(*index);
        }
        for index in self.artifacts.keys() {
            let parent = self.parent_of(*index);
            let Some(owner) =
                self.nearest(parent, |candidate| self.artifacts.contains_key(&candidate))
            else {
                continue;
            };
            let (logical, allocated) = self.subtree_total(*index);
            let totals = &mut exclusive[self.artifacts[&owner]];
            totals.0 = totals.0.saturating_sub(logical);
            totals.1 = totals.1.saturating_sub(allocated);
        }
        let mut owned = HashMap::<usize, (u64, u64)>::new();
        for index in &self.projects {
            owned.insert(*index, self.subtree_total(*index));
        }
        for index in &self.projects {
            let parent = self.parent_of(*index);
            let Some(owner) = self.nearest(parent, |candidate| self.projects.contains(&candidate))
            else {
                continue;
            };
            let (logical, allocated) = self.subtree_total(*index);
            if let Some(totals) = owned.get_mut(&owner) {
                totals.0 = totals.0.saturating_sub(logical);
                totals.1 = totals.1.saturating_sub(allocated);
            }
        }
        for (index, artifact) in &self.artifacts {
            let finding = &mut self.result.findings[*artifact];
            finding.exclusive_logical_bytes = exclusive[*artifact].0.to_string();
            finding.exclusive_allocated_bytes = exclusive[*artifact].1.to_string();
            if self.projects.contains(index) {
                let totals = owned.get(index).copied().unwrap_or_default();
                finding.owned_logical_bytes = Some(totals.0.to_string());
                finding.owned_allocated_bytes = Some(totals.1.to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests;
