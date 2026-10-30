//! Immediate, bounded observations before normal pathname operations; not an atomic race guarantee.
use super::fingerprint::Fingerprint;
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
    pub fingerprint: Fingerprint,
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
            recognition::recognize(&data.records, &AtomicBool::new(false), data.pins_exhausted);
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
        let fingerprint = Fingerprint::measure(&Root::open(&path)?, method == Method::Trash)?;
        let finding = &record.finding;
        // Totals and root metadata tie the fresh walk back to what the scan measured and the
        // user reviewed; the digest then pins the exact tree for the pre-dispatch comparison.
        if finding.identity.as_deref()
            != Some(
                format!(
                    "{}:{}",
                    fingerprint.root_identity.0, fingerprint.root_identity.1
                )
                .as_str(),
            )
            || finding.descendant_items.checked_add(1) != Some(fingerprint.count)
            || finding.logical_bytes != fingerprint.logical_bytes
            || finding.allocated_bytes != fingerprint.allocated_bytes
        {
            return Err(ErrorCode::Unavailable.into());
        }
        let mut stamps = Vec::new();
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
            for (position, record) in data.records.iter().enumerate() {
                let path = data.path(position).ok_or(ErrorCode::InvalidRequest)?;
                if !path.starts_with(&metadata) {
                    continue;
                }
                if record.finding.kind == scan::Kind::Directory
                    && record.finding.detail != scan::Detail::Full
                {
                    return Err(ErrorCode::Unavailable.into());
                }
                stamps.push(observe(path, false)?);
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
            fingerprint,
            stamps,
        })
    }
}
