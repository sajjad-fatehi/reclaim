//! Finding identities, never display paths, authorize explicit Finder handoff.
use super::*;
use reclaim_core::recognition::Artifact;
use reclaim_core::scan::{Kind, Record};
use rustix::fs::{self, AtFlags, Mode, OFlags};
use std::fs::File;
use std::path::PathBuf;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Inspection {
    pub finding_id: String,
    pub display_path: String,
    pub copy_path: Option<String>,
    pub artifact: Option<Artifact>,
    pub recognized_at_id: Option<String>,
}

pub(super) fn resolve(data: &ScanData, finding_id: &str) -> BoundaryResult<usize> {
    if finding_id.len() > 64 {
        return Err(ErrorCode::InvalidRequest.into());
    }
    data.records
        .iter()
        .position(|record| record.finding.id == finding_id)
        .ok_or_else(|| ErrorCode::InvalidRequest.into())
}

fn changed() -> BoundaryError {
    BoundaryError {
        code: ErrorCode::Unavailable,
        issue: Some(Issue::Changed),
    }
}

fn check_identity(record: &Record, metadata: &rustix::fs::Stat) -> BoundaryResult<()> {
    if record.finding.identity.as_deref()
        != Some(format!("{}:{}", metadata.st_dev, metadata.st_ino).as_str())
    {
        return Err(changed());
    }
    Ok(())
}

/// Check every ancestor by descriptor; never follow a replaced ancestor or final symlink.
pub(super) fn validate_target(
    data: &ScanData,
    index: usize,
    authorized_roots: &[PathBuf],
) -> BoundaryResult<PathBuf> {
    let path = data.path(index).ok_or(ErrorCode::InvalidRequest)?;
    let mut chain = Vec::new();
    let mut current = Some(index);
    while let Some(index) = current {
        chain.push(index);
        current = data.records[index].parent;
    }
    chain.reverse();
    let root_index = *chain.first().ok_or(ErrorCode::InvalidRequest)?;
    let root_record = &data.records[root_index];
    let root_path = Path::new(&root_record.name);
    if !authorized_roots.iter().any(|root| root == root_path) || !path.starts_with(root_path) {
        return Err(ErrorCode::InvalidScope.into());
    }
    let mut directory = Root::open(root_path)?.directory;
    check_identity(
        root_record,
        &fs::fstat(&directory).map_err(std::io::Error::from)?,
    )?;
    for (position, index) in chain.iter().enumerate().skip(1) {
        let record = &data.records[*index];
        let metadata = fs::statat(&directory, &record.name, AtFlags::SYMLINK_NOFOLLOW)
            .map_err(std::io::Error::from)?;
        check_identity(record, &metadata)?;
        if position + 1 < chain.len() {
            directory = File::from(
                fs::openat(
                    &directory,
                    &record.name,
                    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                    Mode::empty(),
                )
                .map_err(std::io::Error::from)?,
            );
            check_identity(
                record,
                &fs::fstat(&directory).map_err(std::io::Error::from)?,
            )?;
        } else if record.finding.kind == Kind::File {
            let modified = u64::try_from(metadata.st_mtime)
                .ok()
                .and_then(|seconds| seconds.checked_mul(1000));
            if record.finding.modified_at_ms != modified
                || (record.finding.alias_of.is_none()
                    && record.finding.logical_bytes != u64::try_from(metadata.st_size).unwrap_or(0))
            {
                return Err(changed());
            }
        }
    }
    // Finder may resolve aliases. Do not offer symlink handoff at all.
    if data.records[index].finding.kind == Kind::Symlink {
        return Err(changed());
    }
    Ok(path)
}

impl ScanService {
    pub fn inspect(&self, generation: u32, finding_id: &str) -> BoundaryResult<Inspection> {
        let mut authority = lock(&self.authority);
        let session = authority.session(generation)?;
        let data = lock(&session.data);
        let index = resolve(&data, finding_id)?;
        let path = data.path(index).ok_or(ErrorCode::InvalidRequest)?;
        let copy_path = path.to_str().map(str::to_owned);
        // Debug escaping is complete and reversible for invalid bytes/control characters.
        let display_path = match path.to_str() {
            Some(text)
                if !text.chars().any(|character| {
                    character.is_control()
                        || matches!(character, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
                }) =>
            {
                text.to_owned()
            }
            _ => format!("{:?}", path.as_os_str()),
        };
        let mut ancestor = Some(index);
        let mut artifact = None;
        while let Some(index) = ancestor {
            artifact = data
                .recognition
                .findings
                .iter()
                .find(|artifact| artifact.finding_id == data.records[index].finding.id)
                .cloned();
            if artifact.is_some() {
                break;
            }
            ancestor = data.records[index].parent;
        }
        Ok(Inspection {
            finding_id: finding_id.to_owned(),
            display_path,
            copy_path,
            recognized_at_id: artifact
                .as_ref()
                .map(|artifact| artifact.finding_id.clone()),
            artifact,
        })
    }

    pub fn reveal_with(
        &self,
        generation: u32,
        finding_id: &str,
        handoff: impl FnOnce(&Path) -> BoundaryResult<()>,
    ) -> BoundaryResult<()> {
        let mut authority = lock(&self.authority);
        let scope = authority.session(generation)?.scope.clone();
        let authorized_roots = scope
            .roots
            .iter()
            .map(|location| {
                authority
                    .candidates
                    .iter()
                    .find(|candidate| candidate.location.id == location.id)
                    .map(|candidate| candidate.root.path.clone())
                    .ok_or_else(|| BoundaryError::from(ErrorCode::InvalidScope))
            })
            .collect::<BoundaryResult<Vec<_>>>()?;
        let session = authority.session(generation)?;
        let data = lock(&session.data);
        let index = resolve(&data, finding_id)?;
        let path = validate_target(&data, index, &authorized_roots)?;
        handoff(&path)
    }
}

#[tauri::command]
pub async fn scan_inspect<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, Arc<ScanService>>,
    generation: u32,
    finding_id: String,
) -> BoundaryResult<Inspection> {
    authorize(&window)?;
    state.inspect(generation, &finding_id)
}

#[tauri::command]
pub async fn scan_reveal<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, Arc<ScanService>>,
    generation: u32,
    finding_id: String,
) -> BoundaryResult<()> {
    authorize(&window)?;
    state.reveal_with(generation, &finding_id, |path| {
        let url = objc2_foundation::NSURL::from_file_path(path).ok_or(ErrorCode::Unavailable)?;
        objc2_app_kit::NSWorkspace::sharedWorkspace().activateFileViewerSelectingURLs(
            &objc2_foundation::NSArray::from_slice(&[url.as_ref()]),
        );
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::os::unix::{ffi::OsStringExt, fs::symlink};

    fn fixture() -> (tempfile::TempDir, ScanService, u32) {
        let temporary = tempfile::tempdir().unwrap();
        std::fs::create_dir(temporary.path().join("folder")).unwrap();
        std::fs::write(temporary.path().join("folder/file"), b"synthetic").unwrap();
        let service = ScanService::exhaustive();
        let location = service.register_native_location(temporary.path()).unwrap();
        let scope = lock(&service.authority).register(&[location.id]).unwrap();
        let generation = service.start(&scope.id).unwrap().generation;
        super::super::tests::terminal(&service);
        (temporary, service, generation)
    }
    fn finding_id(service: &ScanService, name: &str) -> String {
        let authority = lock(&service.authority);
        let data = lock(&authority.session.as_ref().unwrap().data);
        data.records
            .iter()
            .find(|record| record.name == name)
            .unwrap()
            .finding
            .id
            .clone()
    }
    #[test]
    fn reveal_rejects_forged_stale_missing_changed_and_out_of_scope_targets() {
        let (temporary, service, generation) = fixture();
        let finding_id = finding_id(&service, "file");
        let expected = temporary.path().canonicalize().unwrap().join("folder/file");
        service
            .reveal_with(generation, &finding_id, |path| {
                assert_eq!(path, expected);
                Ok(())
            })
            .unwrap();
        assert_eq!(
            service
                .reveal_with(generation, "/tmp/forged", |_| panic!("must not handoff"))
                .unwrap_err()
                .code,
            ErrorCode::InvalidRequest
        );
        assert_eq!(
            service
                .reveal_with(generation + 1, &finding_id, |_| panic!("must not handoff"))
                .unwrap_err()
                .code,
            ErrorCode::StaleGeneration
        );
        std::fs::write(&expected, b"changed size").unwrap();
        assert_eq!(
            service
                .reveal_with(generation, &finding_id, |_| panic!("must not handoff"))
                .unwrap_err()
                .issue,
            Some(Issue::Changed)
        );
        std::fs::remove_file(&expected).unwrap();
        assert_eq!(
            service
                .reveal_with(generation, &finding_id, |_| panic!("must not handoff"))
                .unwrap_err()
                .issue,
            Some(Issue::Vanished)
        );
        assert!(service
            .inspect(generation, &finding_id)
            .unwrap()
            .display_path
            .ends_with("folder/file"));
        let mut authority = lock(&service.authority);
        let session = authority.session.as_mut().unwrap();
        let data = lock(&session.data);
        let index = resolve(&data, &finding_id).unwrap();
        assert_eq!(
            validate_target(&data, index, &[]).unwrap_err().code,
            ErrorCode::InvalidScope
        );
    }
    #[test]
    fn changed_ancestor_and_symlink_cannot_redirect_finder() {
        let (temporary, service, generation) = fixture();
        let file_id = finding_id(&service, "file");
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("file"), b"sentinel").unwrap();
        std::fs::rename(
            temporary.path().join("folder"),
            temporary.path().join("old"),
        )
        .unwrap();
        symlink(outside.path(), temporary.path().join("folder")).unwrap();
        assert!(service
            .reveal_with(generation, &file_id, |_| panic!("must not handoff"))
            .is_err());
        assert_eq!(
            std::fs::read(outside.path().join("file")).unwrap(),
            b"sentinel"
        );
    }
    #[test]
    fn invalid_bytes_keep_native_identity_and_full_escaped_path() {
        let (_temporary, service, generation) = fixture();
        let name = OsString::from_vec(vec![b'.', 0xff, b'x']);
        let file_id = finding_id(&service, "file");
        // APFS rejects invalid UTF-8 names, so inject a native record to test lossless transport.
        {
            let authority = lock(&service.authority);
            let mut data = lock(&authority.session.as_ref().unwrap().data);
            let index = resolve(&data, &file_id).unwrap();
            data.records[index].name = name.clone();
        }
        let details = service.inspect(generation, &file_id).unwrap();
        assert!(details.copy_path.is_none());
        assert!(details.display_path.contains("\\xFF"));
        let authority = lock(&service.authority);
        let data = lock(&authority.session.as_ref().unwrap().data);
        let path = data.path(resolve(&data, &file_id).unwrap()).unwrap();
        assert_eq!(path.file_name(), Some(name.as_os_str()));
        assert!(objc2_foundation::NSURL::from_file_path(path).is_some());
    }
    #[test]
    fn inspection_inherits_nearest_authoritative_recognition_and_retains_full_path() {
        let (temporary, service, _) = fixture();
        std::fs::write(
            temporary.path().join("package.json"),
            r#"{"name":"SYNTHETIC-project"}"#,
        )
        .unwrap();
        let scope_id = lock(&service.authority)
            .session
            .as_ref()
            .unwrap()
            .scope
            .id
            .clone();
        let generation = service.start(&scope_id).unwrap().generation;
        super::super::tests::terminal(&service);
        let file_id = finding_id(&service, "file");
        let inspection = service.inspect(generation, &file_id).unwrap();
        let artifact = inspection.artifact.unwrap();
        assert_eq!(
            artifact.family,
            reclaim_core::recognition::Family::JavaScript
        );
        assert_eq!(artifact.nature, reclaim_core::recognition::Nature::Project);
        assert!(!artifact.evidence.is_empty());
        assert_eq!(
            inspection.recognized_at_id.as_ref(),
            Some(&artifact.finding_id)
        );
        assert_ne!(artifact.finding_id, file_id);
        assert_eq!(
            inspection.copy_path.as_deref(),
            temporary
                .path()
                .canonicalize()
                .unwrap()
                .join("folder/file")
                .to_str()
        );
        {
            let authority = lock(&service.authority);
            let mut data = lock(&authority.session.as_ref().unwrap().data);
            let index = resolve(&data, &file_id).unwrap();
            data.records[index].name = format!("{}終", "long".repeat(400)).into();
        }
        assert!(
            service
                .inspect(generation, &file_id)
                .unwrap()
                .display_path
                .len()
                > scan::MAX_DISPLAY
        );
    }
}
