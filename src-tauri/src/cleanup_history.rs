//! Private, descriptor-relative atomic history; persisted data never authorizes execution.
use crate::scan::{BoundaryResult, ErrorCode};
use reclaim_core::cleanup::History;
use reclaim_core::scan::Root;
use rustix::fs::{self, AtFlags, Mode, OFlags};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

const MAX_BYTES: u64 = 4 * 1024 * 1024;
const FILE_NAME: &str = "cleanup-history.json";

fn open_parent(path: &Path) -> BoundaryResult<File> {
    std::fs::create_dir_all(path)?;
    let parent = Root::open(path)?;
    if !parent.owned_by_current_user()? {
        return Err(ErrorCode::AccessDenied.into());
    }
    Ok(parent.directory)
}

pub(crate) fn read(path: &Path) -> BoundaryResult<History> {
    let parent = open_parent(path)?;
    let descriptor = match fs::openat(
        &parent,
        FILE_NAME,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
    ) {
        Ok(descriptor) => descriptor,
        Err(rustix::io::Errno::NOENT) => return Ok(History::default()),
        Err(error) => return Err(std::io::Error::from(error).into()),
    };
    let metadata = fs::fstat(&descriptor).map_err(std::io::Error::from)?;
    if fs::FileType::from_raw_mode(metadata.st_mode) != fs::FileType::RegularFile
        || metadata.st_uid != rustix::process::getuid().as_raw()
        || metadata.st_mode & 0o077 != 0
        || metadata.st_nlink != 1
    {
        return Err(ErrorCode::AccessDenied.into());
    }
    let mut bytes = Vec::new();
    File::from(descriptor)
        .take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let mut history: History =
        serde_json::from_slice(&bytes).map_err(|_| ErrorCode::InvalidRequest)?;
    if !history.validate() {
        return Err(ErrorCode::InvalidRequest.into());
    }
    history.mark_interrupted();
    Ok(history)
}

pub(crate) fn write(path: &Path, history: &History) -> BoundaryResult<()> {
    if !history.validate() {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let bytes = serde_json::to_vec(history).map_err(|_| ErrorCode::Internal)?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let parent = open_parent(path)?;
    let temporary_name = format!(".cleanup-{}", uuid::Uuid::new_v4());
    let descriptor = fs::openat(
        &parent,
        &temporary_name,
        OFlags::CREATE | OFlags::EXCL | OFlags::WRONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::RUSR | Mode::WUSR,
    )
    .map_err(std::io::Error::from)?;
    let result = (|| {
        let mut temporary = File::from(descriptor);
        temporary.write_all(&bytes)?;
        temporary.sync_all()?;
        fs::renameat(&parent, &temporary_name, &parent, FILE_NAME).map_err(std::io::Error::from)?;
        parent.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::unlinkat(&parent, &temporary_name, AtFlags::empty());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use reclaim_core::cleanup::{ItemOutcome, Method, Operation, Outcome};
    use std::os::unix::fs::{symlink, PermissionsExt};

    #[test]
    fn atomic_private_roundtrip_restart_corruption_and_clear() {
        let temporary = tempfile::tempdir().unwrap();
        let path = temporary.path().canonicalize().unwrap();
        assert!(read(&path).unwrap().operations.is_empty());
        let mut history = History::default();
        history
            .record(Operation {
                id: "fixture".into(),
                started_at_ms: 1,
                finished_at_ms: None,
                items: vec![ItemOutcome {
                    label: "SYNTHETIC fixture".into(),
                    method: Method::DockerImage,
                    outcome: Outcome::Pending,
                    explanation: "Started".into(),
                }],
            })
            .unwrap();
        write(&path, &history).unwrap();
        let file = path.join(FILE_NAME);
        assert_eq!(
            std::fs::metadata(&file).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            read(&path).unwrap().operations[0].items[0].outcome,
            Outcome::Unknown
        );
        // An orphaned atomic-write file is not a completed history update.
        std::fs::write(path.join(".cleanup-interrupted"), b"partial").unwrap();
        assert_eq!(read(&path).unwrap().operations.len(), 1);
        std::fs::write(&file, b"{broken").unwrap();
        assert!(read(&path).is_err());
        assert_eq!(std::fs::read(&file).unwrap(), b"{broken");
        write(&path, &History::default()).unwrap();
        assert!(read(&path).unwrap().operations.is_empty());
    }

    #[test]
    fn rejects_symlink_hardlink_nonregular_and_oversized_history() {
        let temporary = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let path = temporary.path().canonicalize().unwrap();
        let sentinel = outside.path().join("sentinel");
        std::fs::write(&sentinel, b"protected").unwrap();
        let file = path.join(FILE_NAME);
        symlink(&sentinel, &file).unwrap();
        assert!(read(&path).is_err());
        write(&path, &History::default()).unwrap();
        assert_eq!(std::fs::read(&sentinel).unwrap(), b"protected");
        std::fs::remove_file(&file).unwrap();
        std::fs::hard_link(&sentinel, &file).unwrap();
        assert!(read(&path).is_err());
        std::fs::remove_file(&file).unwrap();
        std::fs::create_dir(&file).unwrap();
        assert!(read(&path).is_err());
        std::fs::remove_dir(&file).unwrap();
        write(&path, &History::default()).unwrap();
        File::options()
            .write(true)
            .open(&file)
            .unwrap()
            .set_len(MAX_BYTES + 1)
            .unwrap();
        assert!(read(&path).is_err());
    }
}
