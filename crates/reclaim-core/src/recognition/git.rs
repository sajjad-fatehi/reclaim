use super::families::normalize;
use super::*;
use std::process::{Command, Stdio};

impl Recognizer<'_> {
    pub(super) fn git_worktree(&mut self, index: usize) {
        let Some(marker) = self.child(index, ".git") else {
            return;
        };
        let main = self.kind_of(marker) == Kind::Directory;
        self.add(
            index,
            Family::Git,
            Nature::Worktree,
            marker,
            "git_marker_requires_verification",
            false,
        );
        let artifact = self.artifacts[&index];
        if self.result.findings[artifact].nature == Nature::Worktree {
            self.result.findings[artifact].confidence = Confidence::Contextual;
        }
        self.result.findings[artifact].worktree = Some(Worktree {
            repository_id: None,
            metadata_id: None,
            branch: None,
            main,
            detached: false,
            locked: false,
            changes: ChangeState::Unknown,
        });
        let metadata = if main {
            Some(marker)
        } else {
            self.text(marker)
                .and_then(|text| {
                    text.strip_prefix("gitdir: ")
                        .map(str::trim)
                        .map(str::to_owned)
                })
                .and_then(|target| self.resolve(index, &target))
        };
        let Some(metadata) = metadata else {
            self.diagnostic(marker, "worktree_metadata_outside_scan_or_invalid");
            return;
        };
        let common = if main {
            Some(metadata)
        } else {
            self.probe(metadata, "commondir")
                .and_then(|record| self.text(record))
                .and_then(|target| self.resolve(metadata, target.trim()))
        };
        let Some(common) = common else {
            self.diagnostic(marker, "worktree_common_metadata_outside_scan_or_invalid");
            return;
        };
        if self.probe(common, "objects").is_none() || self.probe(common, "config").is_none() {
            self.diagnostic(common, "worktree_repository_metadata_missing");
            return;
        }
        let Some(head_index) = self.probe(metadata, "HEAD") else {
            self.diagnostic(metadata, "worktree_head_missing");
            return;
        };
        let Some(head) = self.text(head_index) else {
            return;
        };
        let head = head.trim();
        let branch = head
            .strip_prefix("ref: refs/heads/")
            .filter(|branch| !branch.is_empty() && !branch.chars().any(char::is_control))
            .map(|branch| display(OsStr::new(branch)));
        let detached =
            head.len() == 40 && head.bytes().all(|character| character.is_ascii_hexdigit());
        if branch.is_none() && !detached {
            self.diagnostic(head_index, "malformed_worktree_head");
            return;
        }
        if !main {
            let backlink = self
                .probe(metadata, "gitdir")
                .and_then(|record| self.text(record))
                .and_then(|target| self.resolve(metadata, target.trim()));
            if backlink != Some(marker)
                || self
                    .parent_of(metadata)
                    .and_then(|parent| self.parent_of(parent))
                    != Some(common)
            {
                self.diagnostic(marker, "worktree_registry_backlink_mismatch");
                return;
            }
        }
        let locked = self.probe(metadata, "locked").is_some()
            || self.probe(metadata, "index.lock").is_some()
            || self.probe(common, "index.lock").is_some();
        let repository = self.parent_of(common);
        let family = self
            .path(index)
            .map(|path| {
                if path
                    .components()
                    .any(|component| component.as_os_str() == ".claude")
                {
                    Family::Claude
                } else if path.components().any(|component| {
                    component.as_os_str() == ".codex" || component.as_os_str() == ".Codex"
                }) {
                    Family::Codex
                } else {
                    Family::Git
                }
            })
            .unwrap_or(Family::Git);
        self.projects.insert(index);
        let artifact = self.artifacts[&index];
        self.result.findings[artifact].project_id = self.retained_id(index);
        self.result.findings[artifact].confidence = Confidence::Verified;
        if self.result.findings[artifact].nature == Nature::Worktree {
            self.result.findings[artifact].family = family;
        }
        let marker_id = self.id_of(marker).to_owned();
        self.result.findings[artifact].evidence.push(Evidence {
            finding_id: marker_id,
            method: "git_worktree_registry".into(),
        });
        let changes = if locked {
            ChangeState::Unknown
        } else {
            self.git_status(index, metadata, common)
        };
        self.result.findings[artifact].worktree = Some(Worktree {
            repository_id: repository.and_then(|repository| self.retained_id(repository)),
            metadata_id: self.retained_id(metadata),
            branch,
            main,
            detached,
            locked,
            changes,
        });
    }

    fn resolve(&mut self, base: usize, target: &str) -> Option<usize> {
        let target = Path::new(target);
        let path = if target.is_absolute() {
            target.to_owned()
        } else {
            self.path(base)?.join(target)
        };
        let path = normalize(&path)?;
        self.locate(&path)
    }

    fn git_status(&mut self, index: usize, metadata: usize, common: usize) -> ChangeState {
        if self.stopped() {
            return ChangeState::Unknown;
        }
        if !self.complete_of(index) || !self.complete_of(common) || !self.complete_of(metadata) {
            self.diagnostic(index, "worktree_incomplete_scope");
            return ChangeState::Unknown;
        }
        let Some(configuration) = self.probe(common, "config") else {
            self.diagnostic(common, "git_configuration_unavailable");
            return ChangeState::Unknown;
        };
        let Some(content) = self.text(configuration) else {
            return ChangeState::Unknown;
        };
        if !safe_configuration(&content) || self.probe(metadata, "config.worktree").is_some() {
            self.diagnostic(configuration, "unsupported_git_configuration");
            return ChangeState::Unknown;
        }
        let Some(worktree_path) = self.path(index) else {
            return ChangeState::Unknown;
        };
        let Some(metadata_path) = self.path(metadata) else {
            return ChangeState::Unknown;
        };
        let Some(common_path) = self.path(common) else {
            return ChangeState::Unknown;
        };
        if self.open_checked(index).is_err()
            || self.open_checked(metadata).is_err()
            || self.open_checked(common).is_err()
        {
            self.diagnostic(index, "worktree_changed");
            return ChangeState::Unknown;
        }
        let roots = self
            .records
            .iter()
            .filter(|record| record.parent.is_none())
            .map(|record| PathBuf::from(&record.name))
            .collect::<Vec<_>>();
        match status(
            &worktree_path,
            &metadata_path,
            &common_path,
            &roots,
            self.cancel,
        ) {
            Ok(changes) => changes,
            Err(reason) => {
                self.diagnostic(index, reason);
                ChangeState::Unknown
            }
        }
    }
}

/// Refuse includes, filters, alternate worktrees and extensions before running Git.
fn safe_configuration(content: &str) -> bool {
    let mut section = String::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(['#', ';']) {
            continue;
        }
        if line.ends_with('\\') {
            return false;
        }
        if line.starts_with('[') {
            if !line.ends_with(']') {
                return false;
            }
            section = line[1..line.len() - 1]
                .split_whitespace()
                .next()
                .unwrap_or_default()
                .to_ascii_lowercase();
            if !matches!(
                section.as_str(),
                "core" | "remote" | "branch" | "user" | "init"
            ) {
                return false;
            }
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return false;
        };
        let key = key.trim().to_ascii_lowercase();
        if section == "core"
            && !matches!(
                key.as_str(),
                "repositoryformatversion"
                    | "filemode"
                    | "bare"
                    | "logallrefupdates"
                    | "ignorecase"
                    | "precomposeunicode"
                    | "symlinks"
            )
        {
            return false;
        }
        if section == "core" && key == "bare" && value.trim() != "false" {
            return false;
        }
        if section == "core" && key == "repositoryformatversion" && value.trim() != "0" {
            return false;
        }
    }
    !section.is_empty()
}

#[cfg(target_os = "macos")]
fn status(
    worktree: &Path,
    metadata: &Path,
    common: &Path,
    roots: &[PathBuf],
    cancel: &AtomicBool,
) -> Result<ChangeState, &'static str> {
    use rustix::fs::OFlags;
    use std::os::fd::AsFd;
    let executable = Path::new("/Library/Developer/CommandLineTools/usr/bin/git");
    require_tools(executable, Path::new("/usr/bin/sandbox-exec"))?;
    // Seatbelt confines Git's own opens, including raced symlinks and config/attribute reads.
    let mut profile = String::from("(version 1)(allow default)(deny network*)(deny file-write*)(allow file-write* (literal \"/dev/null\"))(deny process-exec)(allow process-exec (literal \"/Library/Developer/CommandLineTools/usr/bin/git\"))(deny file-read*)(allow file-read* (literal \"/\") (subpath \"/usr\") (subpath \"/System\") (subpath \"/Library/Developer/CommandLineTools\") (subpath \"/private/var/db/dyld\") (literal \"/dev/null\") (literal \"/dev/urandom\")");
    profile.push_str(")(allow file-read-metadata");
    for root in roots {
        for ancestor in root.ancestors().skip(1) {
            let ancestor = ancestor.to_str().ok_or("unsupported_native_path_for_git")?;
            profile.push_str(" (literal ");
            profile.push_str(
                &serde_json::to_string(ancestor).map_err(|_| "unsupported_native_path_for_git")?,
            );
            profile.push(')');
        }
    }
    profile.push(')');
    profile.push_str("(allow file-read*");
    for root in roots {
        let root = root.to_str().ok_or("unsupported_native_path_for_git")?;
        profile.push_str(" (subpath ");
        profile
            .push_str(&serde_json::to_string(root).map_err(|_| "unsupported_native_path_for_git")?);
        profile.push(')');
    }
    profile.push(')');
    let mut child = Command::new("/usr/bin/sandbox-exec")
        .args(["-p", &profile])
        .arg(executable)
        .args([
            "--no-optional-locks",
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.untrackedCache=false",
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "core.attributesFile=/dev/null",
            "-c",
            "diff.renames=false",
            "-c",
            "status.renames=false",
            "-c",
            "safe.directory=*",
        ])
        .args([
            "status",
            "--porcelain=v1",
            "-z",
            "--ignored=matching",
            "--untracked-files=normal",
            "--ignore-submodules=all",
            "--no-ahead-behind",
        ])
        .current_dir(worktree)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", "/nonexistent")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_DIR", metadata)
        .env("GIT_COMMON_DIR", common)
        .env("GIT_WORK_TREE", worktree)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|_| "git_unavailable")?;
    let outcome = (|| {
        let mut output = child.stdout.take().ok_or("git_unavailable")?;
        let flags = rustix::fs::fcntl_getfl(output.as_fd()).map_err(|_| "git_unavailable")?;
        rustix::fs::fcntl_setfl(output.as_fd(), flags | OFlags::NONBLOCK)
            .map_err(|_| "git_unavailable")?;
        let started = Instant::now();
        let mut bytes = Vec::new();
        let mut buffer = [0_u8; 4096];
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err("git_cancelled");
            }
            if started.elapsed() > Duration::from_secs(2) {
                return Err("git_time_limit");
            }
            match output.read(&mut buffer) {
                Ok(0) => {
                    if let Some(status) = child.try_wait().map_err(|_| "git_unavailable")? {
                        if !status.success() {
                            return Err("git_status_unavailable");
                        }
                        break;
                    }
                }
                Ok(count) => {
                    bytes.extend_from_slice(&buffer[..count]);
                    if bytes.len() > MAX_METADATA_BYTES {
                        return Err("git_output_limit");
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(_) => return Err("git_status_unavailable"),
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        let mut changes = ChangeState::Clean;
        for record in bytes
            .split(|byte| *byte == 0)
            .filter(|record| !record.is_empty())
        {
            if record.starts_with(b"?? ") {
                if changes != ChangeState::Dirty {
                    changes = ChangeState::Untracked;
                }
            } else if record.starts_with(b"!! ") {
                if changes == ChangeState::Clean {
                    changes = ChangeState::Ignored;
                }
            } else {
                changes = ChangeState::Dirty;
            }
        }
        Ok(changes)
    })();
    if outcome.is_err() {
        let _ = child.kill();
    }
    let _ = child.wait();
    outcome
}

#[cfg(target_os = "macos")]
fn require_tools(executable: &Path, confinement: &Path) -> Result<(), &'static str> {
    if !executable.is_file() || !confinement.is_file() {
        return Err("git_or_read_confinement_unavailable");
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn status(
    _worktree: &Path,
    _metadata: &Path,
    _common: &Path,
    _roots: &[PathBuf],
    _cancel: &AtomicBool,
) -> Result<ChangeState, &'static str> {
    Err("git_read_confinement_unsupported")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "macos")]
    fn missing_tools_and_outside_scope_fail_closed() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().canonicalize().unwrap();
        let executable = Path::new("/Library/Developer/CommandLineTools/usr/bin/git");
        assert_eq!(
            require_tools(
                &root.join("missing-git"),
                Path::new("/usr/bin/sandbox-exec")
            ),
            Err("git_or_read_confinement_unavailable")
        );
        assert_eq!(
            require_tools(executable, &root.join("missing-confinement")),
            Err("git_or_read_confinement_unavailable")
        );
        let repository = root.join("quoted \" scope");
        let outside = root.join("unselected");
        std::fs::create_dir(&repository).unwrap();
        std::fs::create_dir(&outside).unwrap();
        assert!(Command::new(executable)
            .args(["init", "-b", "main"])
            .arg(&repository)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .success());
        let metadata = repository.join(".git");
        let cancel = AtomicBool::new(false);
        assert_eq!(
            status(
                &repository,
                &metadata,
                &metadata,
                std::slice::from_ref(&repository),
                &cancel
            ),
            Ok(ChangeState::Clean)
        );
        assert!(status(&repository, &metadata, &metadata, &[outside], &cancel).is_err());
        assert_eq!(
            status(
                &repository,
                &metadata,
                &metadata,
                std::slice::from_ref(&repository),
                &AtomicBool::new(true)
            ),
            Err("git_cancelled")
        );
    }
    #[test]
    fn rejects_external_git_configuration() {
        assert!(safe_configuration(
            "[core]\nrepositoryformatversion = 0\nbare = false\n"
        ));
        for content in [
            "[include]\npath = /outside",
            "[filter \"secret\"]\nclean = command",
            "[core]\nworktree = /outside",
            "[extensions]\nworktreeConfig = true",
            "[core]\nfsmonitor = command",
        ] {
            assert!(!safe_configuration(content));
        }
    }
}
