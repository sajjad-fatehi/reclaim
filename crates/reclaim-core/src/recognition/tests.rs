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
    data.recognition = recognize(&data.records, &AtomicBool::new(false), false);
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
    let cancelled = recognize(&data.records, &AtomicBool::new(true), false);
    assert!(cancelled.cancelled);
    assert!(!cancelled.finished);
    data.records[0].finding.complete = false;
    let partial = recognize(&data.records, &AtomicBool::new(false), false);
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
    let recognized = recognize(&data.records, &AtomicBool::new(false), false);
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
fn tree_size_alone_never_sets_limited() {
    let (_directory, root) = fixture();
    write(&root, "file", "fixture");
    let mut data = measure(&root);
    let template = data.records[1].clone();
    data.records[0].name = format!("/{}", "component/".repeat(400)).into();
    for index in 2..6000 {
        let mut record = template.clone();
        record.name = format!("file-{index}").into();
        record.finding.id = format!("7:{index}");
        record.parent = Some(0);
        data.records.push(record);
    }
    let report = recognize(&data.records, &AtomicBool::new(false), false);
    assert!(!report.limited);
    assert!(report.finished);
}

#[test]
fn exhausted_scanner_pins_set_limited_honestly() {
    let (_directory, root) = fixture();
    write(&root, "app/package.json", r#"{"name":"app"}"#);
    let data = measure(&root);
    let report = recognize(&data.records, &AtomicBool::new(false), true);
    assert!(report.limited);
    assert!(!report.finished);
    assert!(report
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.reason == "pin_budget_exhausted"));
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
    let mut recognizer = Recognizer::new(&records, &cancel, false);
    recognizer.projects.insert(0);
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

fn measure_pruned(root: &Path) -> ScanData {
    let data = Arc::new(Mutex::new(ScanData::new(7)));
    scan_with_policy(
        vec![Root::open(root).unwrap()],
        Vec::new(),
        RetentionPolicy {
            depth: 0,
            threshold: u64::MAX,
            top_k: 0,
            node_budget: usize::MAX,
            pin_predicate: is_pinned,
        },
        data.clone(),
        Arc::new(AtomicBool::new(false)),
    );
    Arc::try_unwrap(data).ok().unwrap().into_inner().unwrap()
}

fn path_of(data: &ScanData, id: &str) -> PathBuf {
    let index: usize = id.rsplit(':').next().unwrap().parse().unwrap();
    data.path(index).unwrap()
}

struct ArtifactView {
    key: String,
    evidence: Vec<(String, PathBuf)>,
}

fn view(data: &ScanData) -> Vec<ArtifactView> {
    let mut views: Vec<ArtifactView> = data
        .recognition
        .findings
        .iter()
        .map(|artifact| {
            let worktree = artifact.worktree.as_ref().map(|worktree| {
                format!(
                    "{:?}/{:?}/{}/{}/{}/{:?}/{:?}/{:?}",
                    worktree.branch,
                    worktree.changes,
                    worktree.main,
                    worktree.detached,
                    worktree.locked,
                    worktree.repository_id.as_ref().map(|id| path_of(data, id)),
                    worktree.metadata_id.as_ref().map(|id| path_of(data, id)),
                    artifact.worktree.is_some(),
                )
            });
            let key = format!(
                "{:?}|{:?}|{:?}|{:?}|{:?}|{}|{}|{}|{}|{}|{}|{}|{:?}|{:?}|{}",
                path_of(data, &artifact.finding_id),
                artifact.family,
                artifact.nature,
                artifact.confidence,
                artifact.project_id.as_ref().map(|id| path_of(data, id)),
                artifact.regenerable,
                artifact.complete,
                artifact.exclusive_logical_bytes,
                artifact.exclusive_allocated_bytes,
                artifact.display_name,
                artifact.consequence,
                artifact.parent_id.is_some(),
                artifact.owned_logical_bytes,
                artifact.owned_allocated_bytes,
                worktree.unwrap_or_default(),
            );
            ArtifactView {
                key,
                evidence: artifact
                    .evidence
                    .iter()
                    .map(|evidence| (evidence.method.clone(), path_of(data, &evidence.finding_id)))
                    .collect(),
            }
        })
        .collect();
    views.sort_by(|left, right| left.key.cmp(&right.key));
    views
}

fn differential_fixture(root: &Path) {
    write(
        root,
        "app/package.json",
        r#"{"name":"app","dependencies":{"left-pad":"1","next":"1"}}"#,
    );
    write(
        root,
        "app/node_modules/left-pad/package.json",
        r#"{"name":"left-pad"}"#,
    );
    write(
        root,
        "app/node_modules/left-pad/index.js",
        "module.exports = 1;",
    );
    write(
        root,
        "app/node_modules/other/index.js",
        "module.exports = 2;",
    );
    write(root, "app/.next/BUILD_ID", "build");
    write(root, "app/.next/server/page.js", "page");
    write(root, "app/.env", "SECRET=1");
    write(root, "app/nested/package.json", r#"{"name":"nested"}"#);
    write(root, "app/nested/src/main.js", "main();");
    write(root, "py/pyproject.toml", "[project]\nname = 'py'");
    write(root, "py/.venv/pyvenv.cfg", "home = /fixture/python");
    write(root, "py/.venv/lib/site.py", "x = 1");
    write(
        root,
        "rs/Cargo.toml",
        "[package]\nname='rs'\nversion='0.1.0'",
    );
    write(
        root,
        "rs/target/.rustc_info.json",
        r#"{"rustc_fingerprint":1}"#,
    );
    write(root, "rs/target/debug/build.o", "object");
    write(root, "home/.nvm/versions/node/v20/bin/node", "binary");
    write(
        root,
        "home/.nvm/versions/node/v20/include/node/node_version.h",
        "#define NODE_MAJOR_VERSION 20",
    );
    write(root, "home/.claude/settings.json", "{}");
    write(root, "home/.claude/logs/a.log", "log");
    write(root, "home/.cache/pip/http-v2/entry", "cached");
    write(root, "home/Yarn/npm-pkg.zip", "zip");
    write(
        root,
        "home/Library/Application Support/Code/User/settings.json",
        "{}",
    );
    write(
        root,
        "home/Library/Application Support/Code/logs/x.log",
        "log",
    );
    write(root, "repo/.git/HEAD", "ref: refs/heads/main\n");
    write(
        root,
        "repo/.git/config",
        "[core]\n\trepositoryformatversion = 0\n",
    );
    write(root, "repo/.git/objects/pack/p", "pack");
    write(root, "repo/README.md", "# repo");
    let metadata = root.join("repo/.git/worktrees/wt");
    write(
        root,
        "repo/.git/worktrees/wt/HEAD",
        "ref: refs/heads/feature\n",
    );
    write(root, "repo/.git/worktrees/wt/commondir", "../..\n");
    write(
        root,
        "repo/.git/worktrees/wt/gitdir",
        &format!("{}\n", root.join("wt/.git").display()),
    );
    write(
        root,
        "wt/.git",
        &format!("gitdir: {}\n", metadata.display()),
    );
    write(root, "wt/file.txt", "work");
}

fn assert_same_recognition(full: &ScanData, pruned: &ScanData) {
    let (full_views, pruned_views) = (view(full), view(pruned));
    assert!(!full_views.is_empty());
    assert_eq!(full_views.len(), pruned_views.len());
    for (left, right) in full_views.iter().zip(&pruned_views) {
        assert_eq!(left.key, right.key);
        assert_eq!(left.evidence.len(), right.evidence.len());
        for ((left_method, left_path), (right_method, right_path)) in
            left.evidence.iter().zip(&right.evidence)
        {
            assert_eq!(left_method, right_method);
            assert!(
                left_path.starts_with(right_path),
                "{left_path:?} is not at or below {right_path:?}"
            );
        }
    }
    assert_eq!(full.recognition.limited, pruned.recognition.limited);
    assert_eq!(full.recognition.finished, pruned.recognition.finished);
}

#[test]
fn pruned_retention_recognizes_the_same_artifacts_and_byte_totals() {
    let (_directory, root) = fixture();
    differential_fixture(&root);
    let full = measure(&root);
    let pruned = measure_pruned(&root);
    assert!(pruned.records.len() < full.records.len());
    assert!(!pruned.pins_exhausted);
    assert_eq!(
        full.progress.allocated_bytes,
        pruned.progress.allocated_bytes
    );
    assert_same_recognition(&full, &pruned);
    let natures: Vec<_> = full
        .recognition
        .findings
        .iter()
        .map(|artifact| (artifact.family, artifact.nature))
        .collect();
    for expected in [
        (Family::JavaScript, Nature::Dependencies),
        (Family::JavaScript, Nature::Build),
        (Family::Python, Nature::Dependencies),
        (Family::Rust, Nature::Build),
        (Family::Nvm, Nature::Runtime),
        (Family::Claude, Nature::EditorState),
        (Family::VsCode, Nature::EditorState),
        (Family::Yarn, Nature::SharedStore),
        (Family::Git, Nature::Worktree),
        (Family::Protected, Nature::Protected),
    ] {
        assert!(natures.contains(&expected), "{expected:?} missing");
    }
}

#[test]
fn probing_never_follows_a_symlinked_dependency() {
    let (_directory, root) = fixture();
    let (_outside_guard, outside) = fixture();
    write(
        &root,
        "app/package.json",
        r#"{"name":"app","dependencies":{"dep":"1"}}"#,
    );
    write(&outside, "dep/package.json", r#"{"name":"dep"}"#);
    fs::create_dir_all(root.join("app/node_modules")).unwrap();
    symlink(outside.join("dep"), root.join("app/node_modules/dep")).unwrap();
    for data in [measure(&root), measure_pruned(&root)] {
        assert!(artifact(&data, "app/node_modules").is_none());
        assert!(artifact(&data, "app").is_some());
    }
}

#[test]
fn unretained_configured_store_is_diagnosed_not_invented() {
    let (_directory, root) = fixture();
    write(&root, "home/.npmrc", "cache=store\n");
    write(&root, "home/store/entry", "x");
    let full = measure(&root);
    assert!(artifact(&full, "home/store").is_some());
    let pruned = measure_pruned(&root);
    assert!(artifact(&pruned, "home/store").is_none());
    assert!(pruned
        .recognition
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.reason == "configured_location_not_retained"));
}

#[test]
fn artifacts_carry_display_fields() {
    let (_directory, root) = fixture();
    write(&root, "app/package.json", r#"{"name":"app"}"#);
    let data = measure_pruned(&root);
    let project = artifact(&data, "app").unwrap();
    assert_eq!(project.display_name, "app");
    assert_eq!(project.kind, Kind::Directory);
    assert!(project.parent_id.is_some());
}

#[test]
fn pins_use_parent_context() {
    let pinned =
        |parent: &str, name: &str, kind: Kind| is_pinned(Path::new(parent), OsStr::new(name), kind);
    assert!(pinned("/home/app", "package.json", Kind::File));
    assert!(!pinned(
        "/home/app/node_modules/dep",
        "package.json",
        Kind::File
    ));
    assert!(pinned("/home/app", "node_modules", Kind::Directory));
    assert!(pinned("/home", ".env", Kind::Symlink));
    assert!(pinned("/home/.claude", "logs", Kind::Directory));
    assert!(!pinned("/home/app", "logs", Kind::Directory));
    assert!(pinned("/h/.nvm/versions/node", "v20", Kind::Directory));
    assert!(pinned("/h/repo/.git/worktrees", "wt", Kind::Directory));
    assert!(!pinned("/h/app", "README", Kind::File));
}

fn call_literals(source: &str, needle: &str) -> Vec<String> {
    let mut literals = Vec::new();
    for (position, _) in source.match_indices(needle) {
        let rest = &source[position + needle.len()..];
        let mut depth = 0_i32;
        let mut comma = None;
        for (offset, character) in rest.char_indices() {
            match character {
                '(' => depth += 1,
                ')' if depth == 0 => break,
                ')' => depth -= 1,
                ',' if depth == 0 => {
                    comma = Some(offset);
                    break;
                }
                _ => {}
            }
        }
        let Some(comma) = comma else { continue };
        let argument = rest[comma + 1..].trim_start();
        if let Some(literal) = argument.strip_prefix('"') {
            literals.push(literal[..literal.find('"').unwrap()].to_owned());
        }
    }
    literals
}

#[test]
fn every_literal_read_by_name_is_pinned_or_listed_for_lazy_probing() {
    let sources = [
        include_str!("families.rs"),
        include_str!("git.rs"),
        include_str!("../recognition.rs"),
    ];
    let pinned = |name: &str| {
        let parent = Path::new("/project");
        [Kind::File, Kind::Directory]
            .into_iter()
            .any(|kind| is_pinned(parent, OsStr::new(name), kind))
    };
    let mut checked = 0;
    for source in sources {
        for literal in call_literals(source, ".child(") {
            assert!(pinned(&literal), "child({literal:?}) is not pinned");
            checked += 1;
        }
        for literal in call_literals(source, ".probe(") {
            assert!(
                pins::is_known_probe(&literal) || (!literal.contains('/') && pinned(&literal)),
                "probe({literal:?}) is neither pinned nor listed for lazy probing"
            );
            checked += 1;
        }
    }
    assert!(checked > 20);
    for manifest in ["pyproject.toml", "Cargo.toml", "package.json"] {
        assert!(pinned(manifest));
    }
    for (_, _, marker) in pins::FRAMEWORK_OUTPUTS {
        assert!(pins::is_known_probe(marker));
    }
}
