use super::*;
use crate::scan::{scan_with_policy, RetentionPolicy, ScanData};
use std::fs;
use std::os::unix::fs::symlink;
use std::process::Command;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

fn write(root: &Path, relative: &str, content: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn measure(root: &Path) -> ScanData {
    let data = Arc::new(Mutex::new(ScanData::new(7)));
    scan_with_policy(
        vec![Root::open(root).unwrap()],
        Vec::new(),
        RetentionPolicy::exhaustive(usize::MAX),
        data.clone(),
        Arc::new(AtomicBool::new(false)),
    );
    Arc::try_unwrap(data).ok().unwrap().into_inner().unwrap()
}

fn fixture() -> (TempDir, PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().canonicalize().unwrap();
    (directory, path)
}

fn artifact<'data>(data: &'data ScanData, relative: &str) -> Option<&'data Artifact> {
    let root = data.path(0).unwrap();
    let record = data
        .records
        .iter()
        .enumerate()
        .find(|(index, _)| data.path(*index).unwrap() == root.join(relative))?
        .1;
    data.recognition
        .findings
        .iter()
        .find(|artifact| artifact.finding_id == record.finding.id)
}

#[test]
fn every_non_git_family_has_positive_and_false_name_fixtures() {
    let cases = vec![
        (
            Family::JavaScript,
            "app/node_modules",
            vec![
                ("app/package.json", r#"{"name":"app"}"#),
                (
                    "app/node_modules/.package-lock.json",
                    r#"{"lockfileVersion":3,"packages":{}}"#,
                ),
            ],
        ),
        (
            Family::Python,
            "app/.venv",
            vec![
                ("app/pyproject.toml", "[project]\nname = 'app'"),
                ("app/.venv/pyvenv.cfg", "home = /fixture/python"),
            ],
        ),
        (
            Family::Rust,
            "app/target",
            vec![
                (
                    "app/Cargo.toml",
                    "[package]\nname='fixture'\nversion='0.1.0'",
                ),
                ("app/target/.rustc_info.json", r#"{"rustc_fingerprint":42}"#),
            ],
        ),
        (
            Family::ProjectLogs,
            "app/logs",
            vec![
                (
                    "app/package.json",
                    r#"{"name":"app","scripts":{"start":"node server.js > logs/app.log"}}"#,
                ),
                ("app/logs/app.log", "synthetic diagnostic"),
            ],
        ),
        (
            Family::Npm,
            ".npm",
            vec![
                (".npm/_cacache/index-v5/fixture", ""),
                (".npm/_cacache/content-v2/fixture", ""),
            ],
        ),
        (
            Family::Pnpm,
            "pnpm/store",
            vec![("pnpm/store/v3/files/fixture", "")],
        ),
        (
            Family::Yarn,
            ".yarn/cache",
            vec![(".yarn/cache/fixture-npm-1.0.zip", "")],
        ),
        (
            Family::Bun,
            ".bun/install/cache",
            vec![(
                ".bun/install/cache/fixture@1.0/package.json",
                r#"{"name":"fixture","version":"1.0"}"#,
            )],
        ),
        (
            Family::Pip,
            ".cache/pip",
            vec![(".cache/pip/http-v2/fixture", "")],
        ),
        (
            Family::Uv,
            ".cache/uv",
            vec![(".cache/uv/wheels-v5/fixture", "")],
        ),
        (
            Family::Nvm,
            ".nvm/versions/node/v22.0.0",
            vec![
                (".nvm/versions/node/v22.0.0/bin/node", "synthetic binary"),
                (
                    ".nvm/versions/node/v22.0.0/include/node/node_version.h",
                    "#define NODE_MAJOR_VERSION 22",
                ),
            ],
        ),
        (
            Family::Go,
            "go/pkg/mod",
            vec![("go/pkg/mod/cache/download/fixture", "")],
        ),
        (
            Family::Go,
            ".cache/go-build",
            vec![(
                ".cache/go-build/README",
                "This directory holds cached build artifacts from the Go build system.",
            )],
        ),
        (
            Family::Cargo,
            ".cargo/registry",
            vec![
                (".cargo/registry/cache/fixture", ""),
                (".cargo/registry/src/fixture", ""),
                (".cargo/registry/index/fixture", ""),
            ],
        ),
        (
            Family::Homebrew,
            "Library/Caches/Homebrew",
            vec![("Library/Caches/Homebrew/downloads/fixture", "")],
        ),
        (
            Family::VsCode,
            "Library/Application Support/Code",
            vec![
                ("Library/Application Support/Code/User/settings.json", "{}"),
                ("Library/Application Support/Code/CachedData/fixture", ""),
            ],
        ),
        (
            Family::Cursor,
            "Library/Application Support/Cursor",
            vec![
                (
                    "Library/Application Support/Cursor/User/settings.json",
                    "{}",
                ),
                ("Library/Application Support/Cursor/logs/fixture", ""),
            ],
        ),
        (
            Family::JetBrains,
            "Library/Caches/JetBrains/IntelliJIdea2026",
            vec![(
                "Library/Caches/JetBrains/IntelliJIdea2026/.home",
                "/fixture/IntelliJ.app",
            )],
        ),
        (
            Family::Claude,
            ".claude",
            vec![
                (".claude/settings.json", "{}"),
                (".claude/projects/history.json", "protected chat"),
            ],
        ),
        (
            Family::Codex,
            ".codex",
            vec![
                (".codex/config.toml", "model = 'fixture'"),
                (".codex/auth.json", "protected secret"),
            ],
        ),
        (
            Family::Protected,
            "credentials",
            vec![("credentials/secret", "protected")],
        ),
    ];
    for (family, expected, files) in cases {
        let (_directory, root) = fixture();
        for (relative, content) in files {
            write(&root, relative, content);
        }
        let data = measure(&root);
        let found = artifact(&data, expected).unwrap_or_else(|| {
            panic!(
                "missing {family:?} at {expected}; {:?}",
                data.recognition.diagnostics
            )
        });
        assert_eq!(found.family, family, "{expected}");
        assert_eq!(found.cleanup_support, CleanupSupport::InspectionOnly);
        assert!(data.recognition.finished);
        // Names alone never establish regenerable ownership; protected names stay protected.
        let (_negative_directory, negative) = fixture();
        fs::create_dir_all(negative.join(expected)).unwrap();
        let negative = measure(&negative);
        assert!(
            !negative
                .recognition
                .findings
                .iter()
                .any(|finding| finding.family == family && finding.regenerable),
            "false positive {family:?}"
        );
        if family != Family::Protected {
            assert!(
                artifact(&negative, expected).is_none(),
                "name-only recognition {family:?}"
            );
        }
    }
}

#[test]
fn framework_outputs_and_all_dependency_installers_require_project_evidence() {
    for (name, dependency, marker, content) in [
        (".next", "next", "BUILD_ID", "fixture-build"),
        (".nuxt", "nuxt", "nuxt.json", "{}"),
        (".svelte-kit", "@sveltejs/kit", "tsconfig.json", "{}"),
        (".turbo", "turbo", "cache/fixture", ""),
    ] {
        let (_directory, root) = fixture();
        write(
            &root,
            "app/package.json",
            &format!(r#"{{"name":"app","devDependencies":{{"{dependency}":"1"}}}}"#),
        );
        write(&root, &format!("app/{name}/{marker}"), content);
        let data = measure(&root);
        assert_eq!(
            artifact(&data, &format!("app/{name}")).unwrap().nature,
            Nature::Build
        );
        write(&root, "app/package.json", r#"{"name":"app"}"#);
        assert!(artifact(&measure(&root), &format!("app/{name}")).is_none());
    }
    for (metadata, content) in [
        (".modules.yaml", "layoutVersion: 5\nvirtualStoreDir: .pnpm"),
        (".yarn-integrity", r#"{"topLevelPatterns":[]}"#),
        (
            "fixture/package.json",
            r#"{"name":"fixture","version":"1.0"}"#,
        ),
    ] {
        let (_directory, root) = fixture();
        write(
            &root,
            "app/package.json",
            r#"{"name":"app","dependencies":{"fixture":"1"}}"#,
        );
        write(&root, &format!("app/node_modules/{metadata}"), content);
        let data = measure(&root);
        assert_eq!(
            artifact(&data, "app/node_modules").unwrap().nature,
            Nature::Dependencies
        );
    }
}

#[test]
fn scoped_custom_configuration_does_not_follow_outside_paths_or_expand_shell() {
    for (configuration, content, family) in [
        (".npmrc", "cache=./custom", Family::Npm),
        (".yarnrc.yml", "cacheFolder: ./custom", Family::Yarn),
        ("pip.conf", "[global]\ncache-dir=./custom", Family::Pip),
        ("uv.toml", "cache-dir = './custom'", Family::Uv),
        (
            "bunfig.toml",
            "[install.cache]\ndir = './custom'",
            Family::Bun,
        ),
    ] {
        let (_directory, root) = fixture();
        write(&root, configuration, content);
        fs::create_dir(root.join("custom")).unwrap();
        let data = measure(&root);
        let finding = artifact(&data, "custom").unwrap();
        assert_eq!(finding.family, family);
        assert_eq!(finding.confidence, Confidence::Contextual);
        assert!(!finding.regenerable);
        write(
            &root,
            configuration,
            &content.replace("./custom", "/outside/never-read"),
        );
        let outside = measure(&root);
        assert!(artifact(&outside, "custom").is_none());
        assert!(outside.recognition.diagnostics.iter().any(
            |diagnostic| diagnostic.reason == "configured_location_outside_scan_or_unavailable"
        ));
    }
}

#[test]
fn malformed_metadata_missing_scope_symlink_and_changed_identity_stay_unknown() {
    let (_directory, root) = fixture();
    let (_outside_directory, outside) = fixture();
    write(&root, "package.json", "{corrupt");
    write(
        &root,
        "Library/Application Support/Code/User/settings.json",
        "{corrupt",
    );
    write(
        &root,
        "Library/Application Support/Code/CachedData/fixture",
        "",
    );

    write(&root, "pyproject.toml", "[corrupt");
    write(&root, ".git", "gitdir: /outside/never-read");
    write(&outside, "secret", "sentinel");
    symlink(outside.join("secret"), root.join(".npmrc")).unwrap();
    let mut data = measure(&root);
    assert!(!data.recognition.diagnostics.is_empty());
    assert!(data
        .recognition
        .findings
        .iter()
        .all(|finding| !finding.regenerable));
    assert_eq!(
        fs::read_to_string(outside.join("secret")).unwrap(),
        "sentinel"
    );
    let manifest = data
        .records
        .iter()
        .position(|record| record.name == "package.json")
        .unwrap();
    fs::rename(root.join("package.json"), root.join("previous")).unwrap();
    write(&root, "package.json", r#"{"name":"replacement"}"#);
    data.recognition = recognize(7, &data.records, &AtomicBool::new(false));
    assert!(data
        .recognition
        .diagnostics
        .iter()
        .any(
            |diagnostic| diagnostic.finding_id == data.records[manifest].finding.id
                && diagnostic.reason == "metadata_unavailable_or_changed"
        ));
    write(&root, "package.json", &"x".repeat(MAX_METADATA_BYTES + 1));
    assert!(measure(&root)
        .recognition
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.reason == "metadata_too_large"));
}

#[test]
fn shared_nested_hardlinks_and_protected_state_are_not_double_counted() {
    let (_directory, root) = fixture();
    write(&root, "app/package.json", r#"{"name":"app"}"#);
    write(
        &root,
        "app/node_modules/.package-lock.json",
        r#"{"lockfileVersion":3,"packages":{}}"#,
    );
    write(&root, "app/node_modules/local/.env", "secret");
    write(&root, "app/source.ts", "source");
    write(&root, "app/nested/package.json", r#"{"name":"nested"}"#);
    write(&root, "app/nested/source.py", "source");
    write(&root, ".npm/_cacache/index-v5/fixture", "index");
    write(&root, ".npm/_cacache/content-v2/fixture", "shared");
    fs::hard_link(
        root.join(".npm/_cacache/content-v2/fixture"),
        root.join("app/alias"),
    )
    .unwrap();
    let data = measure(&root);
    let total: u64 = data
        .recognition
        .findings
        .iter()
        .map(|finding| finding.exclusive_logical_bytes.parse::<u64>().unwrap())
        .sum();
    assert_eq!(total, data.progress.logical_bytes);
    let protected = artifact(&data, "app/node_modules/local/.env").unwrap();
    assert_eq!(protected.nature, Nature::Protected);
    assert!(!protected.regenerable);
    assert!(artifact(&data, ".npm").unwrap().project_id.is_none());
    let projects: u64 = data
        .recognition
        .findings
        .iter()
        .filter_map(|finding| finding.owned_logical_bytes.as_ref())
        .map(|bytes| bytes.parse::<u64>().unwrap())
        .sum();
    assert!(projects <= total);
    assert!(artifact(&data, "app/nested")
        .unwrap()
        .owned_logical_bytes
        .is_some());
    assert!(!artifact(&data, "app").unwrap().regenerable);
}

#[test]
fn cancellation_and_partial_measurement_do_not_fabricate_completeness() {
    let (_directory, root) = fixture();
    write(&root, "package.json", r#"{"name":"app"}"#);
    let mut data = measure(&root);
    let cancelled = recognize(7, &data.records, &AtomicBool::new(true));
    assert!(cancelled.cancelled);
    assert!(!cancelled.finished);
    data.records[0].finding.complete = false;
    let partial = recognize(7, &data.records, &AtomicBool::new(false));
    assert!(!partial.findings[0].complete);
}

fn git(root: &Path, arguments: &[&str]) {
    let output = Command::new("/Library/Developer/CommandLineTools/usr/bin/git")
        .args(arguments)
        .current_dir(root)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "Fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
        .env("GIT_COMMITTER_NAME", "Fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{:?}: {}",
        arguments,
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[cfg(target_os = "macos")]
fn real_git_claude_codex_worktrees_cover_all_change_states_read_only() {
    let (_directory, root) = fixture();
    fs::create_dir(root.join("repo")).unwrap();
    let repository = root.join("repo");
    git(&repository, &["init", "-b", "main"]);
    write(&repository, "source", "original");
    write(&repository, ".gitignore", "ignored\n");
    git(&repository, &["add", "."]);
    git(&repository, &["commit", "-m", "Synthetic fixture"]);
    for (relative, branch) in [
        (".claude/worktrees/clean", "claude-clean"),
        (".Codex/worktrees/dirty", "codex-dirty"),
        ("linked/untracked", "untracked"),
        ("linked/ignored", "ignored"),
        ("linked/locked", "locked"),
        ("linked/detached", "detached"),
    ] {
        git(
            &repository,
            &[
                "worktree",
                "add",
                "-b",
                branch,
                root.join(relative).to_str().unwrap(),
            ],
        );
    }
    write(&root, ".Codex/worktrees/dirty/source", "modified");
    write(&root, "linked/untracked/new", "untracked");
    write(&root, "linked/ignored/ignored", "persistent ignored data");
    git(
        &repository,
        &[
            "worktree",
            "lock",
            root.join("linked/locked").to_str().unwrap(),
        ],
    );
    git(&root.join("linked/detached"), &["checkout", "--detach"]);
    write(&root, ".claude/settings.json", "{}");
    write(&root, ".Codex/config.toml", "model='fixture'");
    let before = fs::read(repository.join(".git/index")).unwrap();
    let data = measure(&root);
    for (relative, changes) in [
        ("repo", ChangeState::Clean),
        (".claude/worktrees/clean", ChangeState::Clean),
        (".Codex/worktrees/dirty", ChangeState::Dirty),
        ("linked/untracked", ChangeState::Untracked),
        ("linked/ignored", ChangeState::Ignored),
        ("linked/locked", ChangeState::Unknown),
        ("linked/detached", ChangeState::Clean),
    ] {
        let finding = artifact(&data, relative).unwrap();
        let worktree = finding.worktree.as_ref().unwrap();
        assert_eq!(
            worktree.changes, changes,
            "{relative}: {:?}",
            data.recognition.diagnostics
        );
        assert!(worktree.repository_id.is_some());
        assert_eq!(finding.cleanup_support, CleanupSupport::InspectionOnly);
    }
    assert_eq!(
        artifact(&data, ".claude/worktrees/clean").unwrap().family,
        Family::Claude
    );
    assert_eq!(
        artifact(&data, ".Codex/worktrees/dirty").unwrap().family,
        Family::Codex
    );
    assert!(
        artifact(&data, "linked/locked")
            .unwrap()
            .worktree
            .as_ref()
            .unwrap()
            .locked
    );
    assert!(
        artifact(&data, "linked/detached")
            .unwrap()
            .worktree
            .as_ref()
            .unwrap()
            .detached
    );
    assert_eq!(before, fs::read(repository.join(".git/index")).unwrap());
    assert_eq!(
        fs::read_to_string(root.join("linked/ignored/ignored")).unwrap(),
        "persistent ignored data"
    );
    // A worktree alone does not authorize following its registry outside scope.
    let scoped = measure(&root.join(".claude/worktrees/clean"));
    assert_eq!(
        scoped.recognition.findings[0]
            .worktree
            .as_ref()
            .unwrap()
            .changes,
        ChangeState::Unknown
    );
    // Read confinement prevents a malicious config include and external command.
    write(
        &repository,
        ".git/config",
        "[include]\npath = /outside/secret\n",
    );
    let unsupported = measure(&root);
    assert_eq!(
        artifact(&unsupported, "repo")
            .unwrap()
            .worktree
            .as_ref()
            .unwrap()
            .changes,
        ChangeState::Unknown
    );
}

#[test]
fn temporary_paths_are_inventory_only_and_require_selected_root_context() {
    let (_directory, root) = fixture();
    write(&root, "tmp/source", "persistent");
    let data = measure(&root);
    assert!(artifact(&data, "tmp").is_none());
    // Synthetic native records model the canonical macOS /tmp alias without scanning it.
    let mut data = ScanData::new(7);
    let measured = measure(&root);
    data.records = measured.records.clone();
    data.records[0].name = "/private/tmp".into();
    data.records[0].finding.display_name = "/private/tmp".into();
    let recognized = recognize(7, &data.records, &AtomicBool::new(false));
    let temporary = recognized
        .findings
        .iter()
        .find(|finding| finding.family == Family::Temporary)
        .unwrap();
    assert!(!temporary.regenerable);
    assert_eq!(temporary.cleanup_support, CleanupSupport::InspectionOnly);
}

#[test]
fn missing_native_git_is_reported_without_starting_tools() {
    // The production launcher checks fixed executable/confinement paths before spawning.
    // Missing metadata remains inspectable even without any invocation.
    let (_directory, root) = fixture();
    write(&root, ".git/HEAD", "ref: refs/heads/main\n");
    write(
        &root,
        ".git/config",
        "[core]\nrepositoryformatversion=0\nbare=false\n",
    );
    let data = measure(&root);
    assert_eq!(
        data.recognition.findings[0]
            .worktree
            .as_ref()
            .unwrap()
            .changes,
        ChangeState::Unknown
    );
    assert!(!data.recognition.diagnostics.is_empty());
}

#[test]
fn large_native_path_index_stops_before_retaining_unbounded_paths() {
    let (_directory, root) = fixture();
    write(&root, "file", "fixture");
    let mut data = measure(&root);
    let template = data.records[1].clone();
    data.records[0].name = format!("/{}", "component/".repeat(400)).into();
    for index in 2..6000 {
        let mut record = template.clone();
        record.name = format!("file-{index}").into();
        record.finding.id = format!("7:{index}");
        data.records.push(record);
    }
    let report = recognize(7, &data.records, &AtomicBool::new(false));
    assert!(report.limited);
    assert!(!report.finished);
    assert_eq!(report.diagnostics[0].reason, "path_index_budget_exhausted");
}

#[test]
fn malformed_metadata_consumes_read_budget() {
    let (_directory, root) = fixture();
    fs::write(root.join("seed"), vec![0xff; MAX_METADATA_BYTES]).unwrap();
    for index in 0..130 {
        let project = root.join(format!("project-{index}"));
        fs::create_dir(&project).unwrap();
        fs::hard_link(root.join("seed"), project.join("package.json")).unwrap();
    }
    let data = measure(&root);
    assert!(data.recognition.limited);
    assert!(!data.recognition.finished);
    assert!(data
        .recognition
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.reason == "metadata_budget_exhausted"));
}

#[test]
fn account_caps_exclusive_and_owned_byte_totals() {
    fn create_record(
        index: usize,
        parent: Option<usize>,
        kind: Kind,
        logical: u64,
        allocated: u64,
    ) -> Record {
        Record {
            name: format!("synthetic-{index}").into(),
            parent,
            finding: crate::scan::Finding {
                id: format!("7:{index}"),
                parent_id: parent.map(|parent| format!("7:{parent}")),
                display_name: format!("synthetic-{index}"),
                kind,
                logical_bytes: logical,
                allocated_bytes: allocated,
                measured_at_ms: 0,
                modified_at_ms: None,
                identity: None,
                alias_of: None,
                complete: true,
                issue: None,
                folded_items: 0,
                folded_logical_bytes: 0,
                folded_allocated_bytes: 0,
                descendant_items: 0,
                detail: crate::scan::Detail::Full,
            },
        }
    }
    let records = vec![
        create_record(0, None, Kind::Directory, u64::MAX, u64::MAX),
        create_record(1, Some(0), Kind::Directory, u64::MAX, u64::MAX),
        create_record(2, Some(1), Kind::File, u64::MAX - 3, u64::MAX - 7),
        create_record(3, Some(1), Kind::File, 9, 11),
    ];
    let cancel = AtomicBool::new(false);
    let mut recognizer = Recognizer {
        records: &records,
        paths: HashMap::new(),
        children: HashMap::new(),
        children_by_parent: vec![Vec::new(); records.len()],
        contents: HashMap::new(),
        bytes_read: 0,
        reads: 0,
        cancel: &cancel,
        started: Instant::now(),
        result: Recognition::default(),
        artifacts: HashMap::new(),
        projects: HashSet::from([0]),
    };
    recognizer.add(
        0,
        Family::JavaScript,
        Nature::Project,
        0,
        "synthetic_project",
        false,
    );
    recognizer.add(
        1,
        Family::JavaScript,
        Nature::Dependencies,
        1,
        "synthetic_dependencies",
        true,
    );
    recognizer.account();
    let serialized = serde_json::to_value(&recognizer.result.findings).unwrap();
    let capped = u64::MAX.to_string();
    assert_eq!(serialized[0]["exclusiveLogicalBytes"], "0");
    assert_eq!(serialized[0]["exclusiveAllocatedBytes"], "0");
    assert_eq!(serialized[0]["ownedLogicalBytes"], capped);
    assert_eq!(serialized[0]["ownedAllocatedBytes"], capped);
    assert_eq!(serialized[1]["exclusiveLogicalBytes"], capped);
    assert_eq!(serialized[1]["exclusiveAllocatedBytes"], capped);
}
