//! macOS Full Disk Access detection through narrow, read-only directory probes.
//! Probes only open a directory listing; no entry names or contents are kept or logged.

use serde::Serialize;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FullDiskAccess {
    Granted,
    NotGranted,
    Unknown,
}

/// TCC-protected locations that exist on a standard macOS account, relative to home or absolute.
const HOME_PROBES: [&str; 2] = ["Library/Safari", "Library/Mail"];
const SYSTEM_PROBES: [&str; 1] = ["/Library/Application Support/com.apple.TCC"];

pub fn probe_paths(home: Option<&Path>) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = home
        .map(|home| {
            HOME_PROBES
                .iter()
                .map(|relative| home.join(relative))
                .collect()
        })
        .unwrap_or_default();
    paths.extend(SYSTEM_PROBES.iter().map(PathBuf::from));
    paths
}

/// The first conclusive probe decides: readable means granted, denied means not granted.
/// Missing locations and other errors are inconclusive, never a denial.
pub fn classify(probes: impl IntoIterator<Item = io::Result<()>>) -> FullDiskAccess {
    for probe in probes {
        match probe {
            Ok(()) => return FullDiskAccess::Granted,
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
                return FullDiskAccess::NotGranted
            }
            Err(_) => {}
        }
    }
    FullDiskAccess::Unknown
}

pub fn detect(home: Option<&Path>) -> FullDiskAccess {
    classify(
        probe_paths(home)
            .iter()
            .map(|path| std::fs::read_dir(path).map(drop)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn failure(kind: io::ErrorKind) -> io::Result<()> {
        Err(io::Error::from(kind))
    }

    #[test]
    fn readable_probe_means_granted() {
        assert_eq!(classify([Ok(())]), FullDiskAccess::Granted);
        assert_eq!(
            classify([failure(io::ErrorKind::NotFound), Ok(())]),
            FullDiskAccess::Granted
        );
    }

    #[test]
    fn denied_probe_means_not_granted() {
        assert_eq!(
            classify([failure(io::ErrorKind::PermissionDenied)]),
            FullDiskAccess::NotGranted
        );
        assert_eq!(
            classify([
                failure(io::ErrorKind::NotFound),
                failure(io::ErrorKind::PermissionDenied)
            ]),
            FullDiskAccess::NotGranted
        );
    }

    #[test]
    fn inconclusive_probes_mean_unknown() {
        assert_eq!(classify([]), FullDiskAccess::Unknown);
        assert_eq!(
            classify([
                failure(io::ErrorKind::NotFound),
                failure(io::ErrorKind::Other)
            ]),
            FullDiskAccess::Unknown
        );
    }

    #[test]
    fn detect_reports_granted_for_a_readable_probe_directory() {
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(home.path().join("Library/Safari")).unwrap();
        assert_eq!(detect(Some(home.path())), FullDiskAccess::Granted);
    }

    #[test]
    fn probe_paths_skip_home_when_unknown() {
        assert_eq!(probe_paths(None).len(), SYSTEM_PROBES.len());
        assert_eq!(probe_paths(Some(Path::new("/h"))).len(), 3);
    }
}
