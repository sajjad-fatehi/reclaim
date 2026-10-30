//! Fresh, unpruned subtree fingerprint for cleanup revalidation. Independent of scan retention:
//! every entry is visited and hashed, nothing is stored per entry.
use super::*;
use reclaim_core::scan::{Kind, MAX_DEPTH};
use rustix::fs::{self, AtFlags, Dir, Mode, OFlags, Stat};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs::File;
use std::os::unix::ffi::OsStrExt;

/// Hard-linked files tracked to detect interior aliases; exceeding it fails closed.
const MAX_SHARED_FILES: usize = 1_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Fingerprint {
    pub count: u64,
    pub logical_bytes: u64,
    pub allocated_bytes: u64,
    pub root_identity: (i64, u64),
    pub root_modified: (i64, i64),
    /// Wrapping sum of per-entry SHA-256 digests: order independent, duplicate-sensitive.
    digest: [u64; 4],
}

impl Fingerprint {
    /// Walks the directory behind `root`; any irregularity returns `Unavailable`.
    pub(crate) fn measure(root: &Root, reject_git: bool) -> BoundaryResult<Self> {
        let metadata = fs::fstat(&root.directory).map_err(std::io::Error::from)?;
        let mut walk = Walk {
            device: metadata.st_dev,
            reject_git,
            shared: HashSet::new(),
            fingerprint: Self {
                count: 0,
                logical_bytes: 0,
                allocated_bytes: 0,
                root_identity: (i64::from(metadata.st_dev), metadata.st_ino),
                root_modified: (metadata.st_mtime, metadata.st_mtime_nsec),
                digest: [0; 4],
            },
        };
        walk.add(b"", &metadata, Kind::Directory);
        walk.directory(&root.directory, &metadata, &mut Vec::new(), 0)?;
        Ok(walk.fingerprint)
    }
}

struct Walk {
    device: rustix::fs::Dev,
    reject_git: bool,
    shared: HashSet<(rustix::fs::Dev, u64)>,
    fingerprint: Fingerprint,
}

fn unavailable<Output>() -> BoundaryResult<Output> {
    Err(ErrorCode::Unavailable.into())
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

fn kind_of(metadata: &Stat) -> Kind {
    match fs::FileType::from_raw_mode(metadata.st_mode) {
        fs::FileType::Directory => Kind::Directory,
        fs::FileType::RegularFile => Kind::File,
        fs::FileType::Symlink => Kind::Symlink,
        _ => Kind::Other,
    }
}

fn same_mount(parent: &File, directory: &File) -> std::io::Result<bool> {
    let parent_mount = fs::fstatfs(parent)?;
    let child_mount = fs::fstatfs(directory)?;
    #[cfg(target_os = "macos")]
    return Ok(parent_mount.f_mntonname == child_mount.f_mntonname);
    #[cfg(not(target_os = "macos"))]
    Ok(parent_mount.f_fsid == child_mount.f_fsid)
}

impl Walk {
    fn add(&mut self, relative: &[u8], metadata: &Stat, kind: Kind) {
        let mut hasher = Sha256::new();
        hasher.update((relative.len() as u64).to_le_bytes());
        hasher.update(relative);
        hasher.update([kind as u8]);
        hasher.update(i64::from(metadata.st_dev).to_le_bytes());
        hasher.update(metadata.st_ino.to_le_bytes());
        hasher.update(metadata.st_size.to_le_bytes());
        hasher.update(metadata.st_mtime.to_le_bytes());
        hasher.update(metadata.st_mtime_nsec.to_le_bytes());
        let digest = hasher.finalize();
        let fingerprint = &mut self.fingerprint;
        for (lane, bytes) in fingerprint.digest.iter_mut().zip(digest.chunks_exact(8)) {
            let word = u64::from_le_bytes(bytes.try_into().unwrap_or_default());
            *lane = lane.wrapping_add(word);
        }
        fingerprint.count += 1;
        if kind == Kind::File {
            fingerprint.logical_bytes = fingerprint
                .logical_bytes
                .saturating_add(metadata.st_size.max(0) as u64);
            fingerprint.allocated_bytes = fingerprint
                .allocated_bytes
                .saturating_add((metadata.st_blocks.max(0) as u64).saturating_mul(512));
        }
    }

    fn directory(
        &mut self,
        directory: &File,
        before: &Stat,
        relative: &mut Vec<u8>,
        depth: usize,
    ) -> BoundaryResult<()> {
        if depth >= MAX_DEPTH {
            return unavailable();
        }
        let entries = Dir::read_from(directory).map_err(std::io::Error::from)?;
        for entry in entries {
            let entry = entry.map_err(std::io::Error::from)?;
            let name = OsStr::from_bytes(entry.file_name().to_bytes());
            if name == "." || name == ".." {
                continue;
            }
            let mark = relative.len();
            if mark > 0 {
                relative.push(b'/');
            }
            relative.extend_from_slice(name.as_bytes());
            self.entry(directory, name, relative, depth)?;
            relative.truncate(mark);
        }
        let after = fs::fstat(directory).map_err(std::io::Error::from)?;
        if !unchanged(before, &after) {
            return unavailable();
        }
        Ok(())
    }

    fn entry(
        &mut self,
        parent: &File,
        name: &OsStr,
        relative: &mut Vec<u8>,
        depth: usize,
    ) -> BoundaryResult<()> {
        let before =
            fs::statat(parent, name, AtFlags::SYMLINK_NOFOLLOW).map_err(std::io::Error::from)?;
        let kind = kind_of(&before);
        if before.st_dev != self.device || (self.reject_git && name == ".git") {
            return unavailable();
        }
        match kind {
            Kind::Directory => {
                let opened = fs::openat(
                    parent,
                    name,
                    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                    Mode::empty(),
                )
                .map(File::from)
                .map_err(std::io::Error::from)?;
                let current = fs::fstat(&opened).map_err(std::io::Error::from)?;
                if !unchanged(&before, &current) || !same_mount(parent, &opened)? {
                    return unavailable();
                }
                self.add(relative, &current, kind);
                self.directory(&opened, &current, relative, depth + 1)
            }
            Kind::File | Kind::Symlink | Kind::Other => {
                let shared = kind == Kind::File && before.st_nlink > 1;
                if shared
                    && (self.shared.len() >= MAX_SHARED_FILES
                        || !self.shared.insert((before.st_dev, before.st_ino)))
                {
                    return unavailable();
                }
                let after = fs::statat(parent, name, AtFlags::SYMLINK_NOFOLLOW)
                    .map_err(std::io::Error::from)?;
                if !unchanged(&before, &after) {
                    return unavailable();
                }
                self.add(relative, &before, kind);
                Ok(())
            }
            Kind::Unknown => unavailable(),
        }
    }
}
