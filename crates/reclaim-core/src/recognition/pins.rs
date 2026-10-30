//! Single owner of the names recognition reads, shared by the scanner's pin predicate and the
//! rules, so a rule can never depend on a node the scanner is allowed to fold away.

use super::Family;
use crate::scan::Kind;
use std::ffi::OsStr;
use std::path::Path;

/// Directories whose contents are generated; also the context rule for `inside_generated`.
pub(super) const GENERATED_DIRECTORIES: [&str; 9] = [
    "node_modules",
    ".venv",
    "venv",
    "target",
    ".git",
    ".next",
    ".nuxt",
    ".svelte-kit",
    ".turbo",
];

/// Layout roots that hold worktrees and build caches. Retained so their closure stays intact.
const LAYOUT_DIRECTORIES: [&str; 2] = [".worktrees", "DerivedData"];

/// Sensitive or persistent names, classified by name alone and therefore pinned for every kind.
pub(super) const PROTECTED_NAMES: [&str; 14] = [
    ".env",
    ".env.local",
    ".env.production",
    ".ssh",
    ".aws",
    "credentials",
    "auth.json",
    "History",
    "Backups",
    "workspaceStorage",
    "globalStorage",
    "User",
    "sessions",
    "projects",
];
pub(super) const PROTECTED_SUFFIXES: [&str; 2] = [".pem", ".key"];

pub(super) struct ToolStore {
    pub(super) family: Family,
    pub(super) suffixes: &'static [&'static str],
    pub(super) markers: &'static [&'static str],
}

pub(super) const TOOL_STORES: [ToolStore; 9] = [
    ToolStore {
        family: Family::Npm,
        suffixes: &[".npm"],
        markers: &["_cacache/index-v5", "_cacache/content-v2"],
    },
    ToolStore {
        family: Family::Pnpm,
        suffixes: &["pnpm/store", ".pnpm-store"],
        markers: &["v3/files"],
    },
    ToolStore {
        family: Family::Yarn,
        suffixes: &["Yarn", "yarn/v6", ".yarn/cache"],
        markers: &[],
    },
    ToolStore {
        family: Family::Bun,
        suffixes: &[".bun/install/cache"],
        markers: &[],
    },
    ToolStore {
        family: Family::Pip,
        suffixes: &["Caches/pip", ".cache/pip"],
        markers: &["http-v2"],
    },
    ToolStore {
        family: Family::Uv,
        suffixes: &["Caches/uv", ".cache/uv"],
        markers: &["wheels-v5"],
    },
    ToolStore {
        family: Family::Go,
        suffixes: &["go/pkg/mod"],
        markers: &["cache/download"],
    },
    ToolStore {
        family: Family::Cargo,
        suffixes: &[".cargo/registry"],
        markers: &["cache", "src", "index"],
    },
    ToolStore {
        family: Family::Homebrew,
        suffixes: &["Caches/Homebrew"],
        markers: &["downloads"],
    },
];

/// Environment variables naming a cache location, matched by the recognizer when retained.
pub(super) const STORE_ENVIRONMENT: [(&str, Family); 10] = [
    ("npm_config_cache", Family::Npm),
    ("PNPM_STORE_DIR", Family::Pnpm),
    ("YARN_CACHE_FOLDER", Family::Yarn),
    ("BUN_INSTALL_CACHE_DIR", Family::Bun),
    ("PIP_CACHE_DIR", Family::Pip),
    ("UV_CACHE_DIR", Family::Uv),
    ("GOMODCACHE", Family::Go),
    ("GOCACHE", Family::Go),
    ("CARGO_HOME", Family::Cargo),
    ("HOMEBREW_CACHE", Family::Homebrew),
];

/// Tool configuration file name, family, key and whether it is TOML.
pub(super) const TOOL_CONFIGURATIONS: [(&str, Family, &str, bool); 5] = [
    (".npmrc", Family::Npm, "cache", false),
    (".yarnrc.yml", Family::Yarn, "cacheFolder", false),
    ("pip.conf", Family::Pip, "cache-dir", false),
    ("uv.toml", Family::Uv, "cache-dir", true),
    ("bunfig.toml", Family::Bun, "install.cache.dir", true),
];

pub(super) const EDITOR_APPLICATIONS: [(&str, Family); 2] = [
    ("Application Support/Code", Family::VsCode),
    ("Application Support/Cursor", Family::Cursor),
];
pub(super) const JETBRAINS_PRODUCTS: [&str; 4] =
    ["IntelliJIdea", "PyCharm", "WebStorm", "RustRover"];
/// Coding tool directory suffix, family and the configuration file proving it.
pub(super) const CODING_TOOLS: [(&str, Family, &str); 3] = [
    (".claude", Family::Claude, "settings.json"),
    (".codex", Family::Codex, "config.toml"),
    (".Codex", Family::Codex, "config.toml"),
];
pub(super) const EDITOR_CHILDREN: [(&str, super::Nature); 4] = [
    ("logs", super::Nature::Logs),
    ("CachedData", super::Nature::Build),
    ("Cache", super::Nature::Build),
    ("caches", super::Nature::Build),
];

pub(super) const NODE_VERSIONS: &str = "versions/node";
pub(super) const GO_BUILD_CACHE: &str = "go-build";

/// Files read at depth one below a directory that recognition has already identified.
const MARKER_FILES: [&str; 6] = [
    "CACHEDIR.TAG",
    "pyvenv.cfg",
    ".rustc_info.json",
    ".package-lock.json",
    ".modules.yaml",
    ".yarn-integrity",
];
/// Project manifests; meaningless inside generated directories, where projects are not sought.
const MANIFEST_FILES: [&str; 3] = ["package.json", "pyproject.toml", "Cargo.toml"];

/// Relative paths reached only through `probe()`: deep, named by other metadata, or only
/// consulted behind a location check. They are never retained on their own.
#[cfg(test)]
pub(super) const LAZY_PROBES: [&str; 13] = [
    "bin/node",
    "include/node/node_version.h",
    "User/settings.json",
    "User",
    "logs",
    "CachedData",
    "options/ide.general.xml",
    ".home",
    "settings.json",
    "config.toml",
    "README",
    "package.json",
    "objects",
];

/// Framework output directory, the dependency that produces it and the marker proving it.
pub(super) const FRAMEWORK_OUTPUTS: [(&str, &str, &str); 4] = [
    (".next", "next", "BUILD_ID"),
    (".turbo", "turbo", "cache"),
    (".nuxt", "nuxt", "nuxt.json"),
    (".svelte-kit", "@sveltejs/kit", "tsconfig.json"),
];

/// Git registry files read after a worktree marker is verified.
#[cfg(test)]
pub(super) const GIT_REGISTRY_PROBES: [&str; 7] = [
    "commondir",
    "gitdir",
    "locked",
    "index.lock",
    "config",
    "config.worktree",
    "HEAD",
];

fn text(name: &OsStr) -> Option<&str> {
    name.to_str()
}

fn within_generated(parent: &Path) -> bool {
    parent.components().any(|component| {
        text(component.as_os_str()).is_some_and(|name| GENERATED_DIRECTORIES.contains(&name))
    })
}

pub(super) fn is_editor_root(path: &Path) -> bool {
    if EDITOR_APPLICATIONS
        .iter()
        .any(|(suffix, _)| path.ends_with(suffix))
        || CODING_TOOLS
            .iter()
            .any(|(suffix, _, _)| path.ends_with(suffix))
    {
        return true;
    }
    let product = path
        .file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|name| {
            JETBRAINS_PRODUCTS
                .iter()
                .any(|prefix| name.starts_with(prefix))
        });
    product
        && path
            .components()
            .any(|component| component.as_os_str() == "JetBrains")
}

fn is_tool_store(path: &Path) -> bool {
    TOOL_STORES
        .iter()
        .flat_map(|store| store.suffixes)
        .any(|suffix| path.ends_with(suffix))
}

/// Scanner pin predicate: decides from the containing directory, name and kind alone.
pub fn is_pinned(parent: &Path, name: &OsStr, kind: Kind) -> bool {
    let Some(label) = text(name) else {
        return false;
    };
    if PROTECTED_NAMES.contains(&label)
        || PROTECTED_SUFFIXES
            .iter()
            .any(|suffix| label.ends_with(suffix))
    {
        return true;
    }
    if label == ".git" {
        return true;
    }
    match kind {
        Kind::Directory => {
            GENERATED_DIRECTORIES.contains(&label)
                || LAYOUT_DIRECTORIES.contains(&label)
                || label == GO_BUILD_CACHE
                || is_tool_store(&parent.join(name))
                || parent.ends_with(NODE_VERSIONS)
                || is_editor_root(&parent.join(name))
                || (EDITOR_CHILDREN.iter().any(|(child, _)| *child == label)
                    && is_editor_root(parent))
                || label == "worktrees" && parent.ends_with(".git")
                || parent.ends_with(".git/worktrees")
        }
        Kind::File => {
            MARKER_FILES.contains(&label)
                || TOOL_CONFIGURATIONS
                    .iter()
                    .any(|(configuration, ..)| *configuration == label)
                || (MANIFEST_FILES.contains(&label) && !within_generated(parent))
        }
        _ => false,
    }
}

/// True when a relative path literal is covered by a pin or an explicit lazy-probe entry.
#[cfg(test)]
pub(super) fn is_known_probe(relative: &str) -> bool {
    LAZY_PROBES.contains(&relative)
        || GIT_REGISTRY_PROBES.contains(&relative)
        || TOOL_STORES
            .iter()
            .flat_map(|store| store.markers)
            .any(|marker| *marker == relative)
        || FRAMEWORK_OUTPUTS
            .iter()
            .any(|(_, _, marker)| *marker == relative)
}
