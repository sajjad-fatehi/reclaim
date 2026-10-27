//! Immediate, bounded observations before normal pathname operations; not an atomic race guarantee.
use super::*;
use reclaim_core::cleanup::{filesystem_method, Method};
use reclaim_core::recognition::{self, Nature};
use rustix::fs::{self, AtFlags};
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Stamp {
    path: PathBuf,
    device: i64,
    inode: u64,
    mode: u32,
    size: u64,
    modified: (i64, i64),
    changed: (i64, i64),
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Target {
    pub path: PathBuf,
    pub method: Method,
    pub repository: Option<PathBuf>,
    pub stamps: Vec<Stamp>,
}

fn observe(path: PathBuf, identity_only: bool) -> BoundaryResult<Stamp> {
    let parent = Root::open(path.parent().ok_or(ErrorCode::InvalidRequest)?)?;
    let metadata = fs::statat(
        &parent.directory,
        path.file_name().ok_or(ErrorCode::InvalidRequest)?,
        AtFlags::SYMLINK_NOFOLLOW,
    )
    .map_err(std::io::Error::from)?;
    Ok(Stamp {
        path,
        device: i64::from(metadata.st_dev),
        inode: metadata.st_ino,
        mode: u32::from(metadata.st_mode),
        size: if identity_only {
            0
        } else {
            u64::try_from(metadata.st_size).unwrap_or(0)
        },
        modified: if identity_only {
            (0, 0)
        } else {
            (metadata.st_mtime, metadata.st_mtime_nsec)
        },
        changed: if identity_only {
            (0, 0)
        } else {
            (metadata.st_ctime, metadata.st_ctime_nsec)
        },
    })
}

impl ScanService {
    pub(crate) fn cleanup_target(
        &self,
        generation: u32,
        finding_id: &str,
    ) -> BoundaryResult<Target> {
        let mut authority = lock(&self.authority);
        let scope = authority.session(generation)?.scope.clone();
        if authority.scope.as_ref().map(|current| &current.id) != Some(&scope.id) {
            return Err(ErrorCode::InvalidScope.into());
        }
        let roots = scope
            .roots
            .iter()
            .map(|location| {
                authority
                    .candidates
                    .iter()
                    .find(|candidate| candidate.location.id == location.id)
                    .map(|candidate| candidate.root.path.clone())
                    .ok_or(ErrorCode::InvalidScope)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let session = authority.session(generation)?;
        let data = lock(&session.data);
        if data.progress.status == Status::Running {
            return Err(ErrorCode::Busy.into());
        }
        let index = super::explore::resolve(&data, finding_id)?;
        let record = &data.records[index];
        if record.parent.is_none()
            || !record.finding.complete
            || record.finding.alias_of.is_some()
            || record.finding.issue.is_some()
        {
            return Err(ErrorCode::InvalidRequest.into());
        }
        let path = super::explore::validate_target(&data, index, &roots)?;
        let fresh_recognition =
            recognition::recognize(generation, &data.records, &AtomicBool::new(false));
        let artifact = fresh_recognition
            .findings
            .iter()
            .find(|artifact| artifact.finding_id == finding_id)
            .ok_or(ErrorCode::InvalidRequest)?;
        let method = filesystem_method(Some(artifact)).map_err(|_| ErrorCode::InvalidRequest)?;
        if method == Method::Trash
            && !matches!(
                path.file_name().and_then(|name| name.to_str()),
                Some("node_modules" | "target")
            )
        {
            return Err(ErrorCode::InvalidRequest.into());
        }
        let mut protected = Vec::new();
        for artifact in &fresh_recognition.findings {
            if matches!(
                artifact.nature,
                Nature::Protected | Nature::EditorState | Nature::Worktree
            ) && artifact.finding_id != finding_id
            {
                let nested = super::explore::resolve(&data, &artifact.finding_id)?;
                if data
                    .path(nested)
                    .is_some_and(|nested| nested.starts_with(&path))
                {
                    protected.push(nested);
                }
            }
        }
        if !protected.is_empty() {
            return Err(ErrorCode::InvalidRequest.into());
        }
        let fresh = Arc::new(Mutex::new(ScanData::new(generation)));
        scan::scan(
            vec![Root::open(&path)?],
            fresh.clone(),
            Arc::new(AtomicBool::new(false)),
        );
        let fresh = lock(&fresh);
        if fresh.progress.status != Status::Complete {
            return Err(ErrorCode::Unavailable.into());
        }
        let mut stamps = Vec::new();
        let mut expected = std::collections::HashMap::new();
        for (position, original) in data.records.iter().enumerate() {
            let original_path = data.path(position).ok_or(ErrorCode::InvalidRequest)?;
            if original_path.starts_with(&path) {
                expected.insert(original_path, original);
            }
        }
        if fresh.records.len() != expected.len() {
            return Err(ErrorCode::Unavailable.into());
        }
        for (position, current) in fresh.records.iter().enumerate() {
            let current_path = fresh.path(position).ok_or(ErrorCode::InvalidRequest)?;
            let original = expected.get(&current_path).ok_or(ErrorCode::Unavailable)?;
            if original.finding.identity != current.finding.identity
                || original.finding.kind != current.finding.kind
                || original.finding.modified_at_ms != current.finding.modified_at_ms
                || original.finding.issue.is_some()
                || original.finding.alias_of.is_some()
                || (current.finding.kind == scan::Kind::File
                    && original.finding.logical_bytes != current.finding.logical_bytes)
                || current.name == ".git" && method == Method::Trash
            {
                return Err(ErrorCode::Unavailable.into());
            }
            stamps.push(observe(current_path, false)?);
        }
        let mut ancestor = record.parent;
        while let Some(position) = ancestor {
            let ancestor_path = data.path(position).ok_or(ErrorCode::InvalidRequest)?;
            let metadata = std::fs::symlink_metadata(&ancestor_path)?;
            if data.records[position].finding.identity.as_ref()
                != Some(&format!("{}:{}", metadata.dev(), metadata.ino()))
            {
                return Err(ErrorCode::Unavailable.into());
            }
            stamps.push(observe(ancestor_path, true)?);
            ancestor = data.records[position].parent;
        }
        let mut evidence = artifact.evidence.clone();
        if let Some(project) = &artifact.project_id {
            if let Some(owner) = fresh_recognition
                .findings
                .iter()
                .find(|owner| &owner.finding_id == project)
            {
                evidence.extend(owner.evidence.clone());
            }
        }
        for evidence in evidence {
            let position = super::explore::resolve(&data, &evidence.finding_id)?;
            stamps.push(observe(
                data.path(position).ok_or(ErrorCode::InvalidRequest)?,
                false,
            )?);
        }
        if let Some(metadata) = artifact
            .worktree
            .as_ref()
            .and_then(|worktree| worktree.metadata_id.as_ref())
        {
            let metadata = data
                .path(super::explore::resolve(&data, metadata)?)
                .ok_or(ErrorCode::InvalidRequest)?;
            for (position, _) in data.records.iter().enumerate() {
                let path = data.path(position).ok_or(ErrorCode::InvalidRequest)?;
                if path.starts_with(&metadata) {
                    stamps.push(observe(path, false)?);
                }
            }
        }
        stamps.sort_by(|left, right| left.path.cmp(&right.path));
        stamps.dedup();
        let repository = artifact
            .worktree
            .as_ref()
            .and_then(|worktree| worktree.repository_id.as_ref())
            .and_then(|identity| super::explore::resolve(&data, identity).ok())
            .and_then(|position| data.path(position));
        Ok(Target {
            path,
            method,
            repository,
            stamps,
        })
    }
}
