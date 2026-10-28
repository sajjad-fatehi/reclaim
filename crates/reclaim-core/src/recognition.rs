//! Read-only recognition over one scanner generation. Findings never grant removal authority.

use crate::scan::{display, Kind, Record, Root};
use rustix::fs::{self, Mode, OFlags};
use serde::Serialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::fs::File;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

mod families;
mod git;

const MAX_METADATA_BYTES: usize = 65_536;
const MAX_TOTAL_METADATA: usize = 8 * 1024 * 1024;
const MAX_METADATA_FILES: usize = 4096;
const MAX_DIAGNOSTICS: usize = 256;
const MAX_PATH_INDEX_BYTES: usize = 16 * 1024 * 1024;
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

struct Recognizer<'records> {
    records: &'records [Record],
    paths: HashMap<PathBuf, usize>,
    children: HashMap<(usize, std::ffi::OsString), usize>,
    children_by_parent: Vec<Vec<usize>>,
    contents: HashMap<usize, Option<String>>,
    bytes_read: usize,
    reads: usize,
    cancel: &'records AtomicBool,
    started: Instant,
    result: Recognition,
    artifacts: HashMap<usize, usize>,
    projects: HashSet<usize>,
}

/// Only native scanner records may be supplied. No display path is resolved as authority.
pub fn recognize(_generation: u32, records: &[Record], cancel: &AtomicBool) -> Recognition {
    let mut recognizer = Recognizer {
        records,
        paths: HashMap::new(),
        children: HashMap::new(),
        children_by_parent: vec![Vec::new(); records.len()],
        contents: HashMap::new(),
        bytes_read: 0,
        reads: 0,
        cancel,
        started: Instant::now(),
        result: Recognition::default(),
        artifacts: HashMap::new(),
        projects: HashSet::new(),
    };
    recognizer.run();
    recognizer.result
}

impl Recognizer<'_> {
    fn run(&mut self) {
        let mut path_bytes = 0;
        for (index, record) in self.records.iter().enumerate() {
            if self.stopped() {
                return;
            }
            if let Some(parent) = record.parent {
                self.children.insert((parent, record.name.clone()), index);
                self.children_by_parent[parent].push(index);
            }
            if let Some(path) = self.path(index) {
                path_bytes += path.as_os_str().len();
                if path_bytes > MAX_PATH_INDEX_BYTES {
                    self.result.limited = true;
                    self.diagnostic(index, "path_index_budget_exhausted");
                    return;
                }
                self.paths.insert(path, index);
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

    fn path(&self, index: usize) -> Option<PathBuf> {
        let mut components = Vec::new();
        let mut current = Some(index);
        while let Some(index) = current {
            let record = self.records.get(index)?;
            components.push(record.name.as_os_str());
            current = record.parent;
        }
        Some(components.into_iter().rev().collect())
    }

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

    fn diagnostic(&mut self, index: usize, reason: &str) {
        if self.result.diagnostics.len() >= MAX_DIAGNOSTICS {
            self.result.diagnostics_truncated = true;
            return;
        }
        self.result.diagnostics.push(Diagnostic {
            finding_id: self.records[index].finding.id.clone(),
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

    fn open_checked(&self, index: usize) -> Result<File, &'static str> {
        let mut chain = Vec::new();
        let mut current = Some(index);
        while let Some(index) = current {
            chain.push(index);
            current = self.records[index].parent;
        }
        chain.reverse();
        let root_index = *chain.first().ok_or("invalid_record")?;
        let root = Root::open(Path::new(&self.records[root_index].name))
            .map_err(|_| "metadata_unavailable_or_changed")?;
        let mut file = root.directory;
        self.check_identity(root_index, &file)?;
        for index in chain.into_iter().skip(1) {
            let record = &self.records[index];
            let flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
            let flags = if record.finding.kind == Kind::Directory {
                flags | OFlags::DIRECTORY
            } else {
                flags
            };
            file = File::from(
                fs::openat(&file, &record.name, flags, Mode::empty())
                    .map_err(|_| "metadata_unavailable_or_changed")?,
            );
            self.check_identity(index, &file)?;
        }
        Ok(file)
    }

    fn check_identity(&self, index: usize, file: &File) -> Result<(), &'static str> {
        let observed = &self.records[index].finding;
        let metadata = fs::fstat(file).map_err(|_| "metadata_unavailable_or_changed")?;
        let identity = format!("{}:{}", metadata.st_dev, metadata.st_ino);
        if observed.identity.as_ref() != Some(&identity) || observed.issue.is_some() {
            return Err("metadata_unavailable_or_changed");
        }
        Ok(())
    }

    fn read_checked(&mut self, index: usize) -> Result<String, &'static str> {
        let record = &self.records[index];
        if record.finding.kind != Kind::File || !record.finding.complete {
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
            if matches!(
                self.records[index].name.to_str(),
                Some(
                    "node_modules"
                        | ".venv"
                        | "venv"
                        | "target"
                        | ".git"
                        | ".next"
                        | ".nuxt"
                        | ".svelte-kit"
                        | ".turbo"
                )
            ) {
                return true;
            }
            parent = self.records[index].parent;
        }
        false
    }

    fn nearest_project(&self, index: usize) -> Option<usize> {
        let mut current = Some(index);
        while let Some(index) = current {
            if self.projects.contains(&index) {
                return Some(index);
            }
            current = self.records[index].parent;
        }
        None
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
        self.artifacts.insert(index, self.result.findings.len());
        self.result.findings.push(Artifact {
            finding_id: self.records[index].finding.id.clone(),
            family,
            nature,
            project_id,
            confidence: Confidence::Verified,
            evidence: vec![Evidence {
                finding_id: self.records[evidence].finding.id.clone(),
                method: method.into(),
            }],
            consequence: consequence.into(),
            cleanup_support: CleanupSupport::InspectionOnly,
            regenerable,
            complete: self.records[index].finding.complete,
            exclusive_logical_bytes: "0".into(),
            exclusive_allocated_bytes: "0".into(),
            owned_logical_bytes: None,
            owned_allocated_bytes: None,
            worktree: None,
        });
    }

    fn account(&mut self) {
        let mut exclusive = vec![(0_u64, 0_u64); self.result.findings.len()];
        let mut owned = HashMap::<usize, (u64, u64)>::new();
        for index in 0..self.records.len() {
            if index % 128 == 0 && self.stopped() {
                break;
            }
            let record = &self.records[index];
            let (logical, allocated) = match record.finding.kind {
                Kind::File if record.finding.alias_of.is_none() => {
                    (record.finding.logical_bytes, record.finding.allocated_bytes)
                }
                Kind::Directory => (
                    record.finding.folded_logical_bytes,
                    record.finding.folded_allocated_bytes,
                ),
                _ => continue,
            };
            if logical == 0 && allocated == 0 {
                continue;
            }
            let mut current = Some(index);
            while let Some(index) = current {
                if let Some(artifact) = self.artifacts.get(&index) {
                    exclusive[*artifact].0 = exclusive[*artifact].0.saturating_add(logical);
                    exclusive[*artifact].1 = exclusive[*artifact].1.saturating_add(allocated);
                    break;
                }
                current = self.records[index].parent;
            }
            if let Some(project) = self.nearest_project(index) {
                let totals = owned.entry(project).or_default();
                totals.0 = totals.0.saturating_add(logical);
                totals.1 = totals.1.saturating_add(allocated);
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
