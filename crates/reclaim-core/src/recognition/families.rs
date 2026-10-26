use super::*;

impl Recognizer<'_> {
    pub(super) fn inventory(&mut self, index: usize) {
        let record = &self.records[index];
        if record.finding.alias_of.is_some() || record.finding.issue.is_some() {
            return;
        }
        if record.finding.kind == Kind::File {
            self.configured_store(index);
        }
        if record.finding.kind == Kind::Directory {
            self.tool_store(index);
            self.editor(index);
        }
    }

    pub(super) fn classify(&mut self, index: usize) {
        let record = &self.records[index];
        if record.finding.alias_of.is_some() || record.finding.issue.is_some() {
            return;
        }
        let name = record.name.to_string_lossy();
        if matches!(
            name.as_ref(),
            ".env"
                | ".env.local"
                | ".env.production"
                | ".ssh"
                | ".aws"
                | "credentials"
                | "auth.json"
                | "History"
                | "Backups"
                | "workspaceStorage"
                | "globalStorage"
                | "User"
                | "sessions"
                | "projects"
        ) || name.ends_with(".pem")
            || name.ends_with(".key")
        {
            self.add(
                index,
                Family::Protected,
                Nature::Protected,
                index,
                "sensitive_or_persistent_state",
                false,
            );
            return;
        }
        if self.records[index].finding.kind != Kind::Directory {
            return;
        }
        self.project_artifact(index);
        if record.parent.is_none() {
            let path = Path::new(&record.name);
            let selected_temporary = matches!(path.to_str(), Some("/private/tmp" | "/tmp"))
                || std::env::var_os("TMPDIR")
                    .is_some_and(|configured| Path::new(&configured) == path);
            if selected_temporary {
                self.add(
                    index,
                    Family::Temporary,
                    Nature::Temporary,
                    index,
                    "explicitly_scanned_temporary_root",
                    false,
                );
            }
        }
    }

    fn project_artifact(&mut self, index: usize) {
        let Some(parent) = self.records[index].parent else {
            return;
        };
        let Some(project) = self.nearest_project(parent) else {
            return;
        };
        if project != parent {
            return;
        }
        let name = self.records[index].name.to_string_lossy().into_owned();
        if name == "node_modules" {
            if let Some(lockfile) = self.child(index, ".package-lock.json") {
                if self.json(lockfile).is_some_and(|value| {
                    value
                        .get("lockfileVersion")
                        .and_then(Value::as_u64)
                        .is_some()
                        && value.get("packages").is_some_and(Value::is_object)
                }) {
                    self.add(
                        index,
                        Family::JavaScript,
                        Nature::Dependencies,
                        lockfile,
                        "npm_install_metadata",
                        true,
                    );
                }
            }
            if let Some(metadata) = self.child(index, ".modules.yaml") {
                if self.text(metadata).is_some_and(|text| {
                    text.lines()
                        .any(|line| line.starts_with("virtualStoreDir:"))
                        && text.lines().any(|line| line.starts_with("layoutVersion:"))
                }) {
                    self.add(
                        index,
                        Family::JavaScript,
                        Nature::Dependencies,
                        metadata,
                        "pnpm_install_metadata",
                        true,
                    );
                }
            }
            if let Some(metadata) = self.child(index, ".yarn-integrity") {
                if self
                    .json(metadata)
                    .is_some_and(|value| value.get("topLevelPatterns").is_some_and(Value::is_array))
                {
                    self.add(
                        index,
                        Family::JavaScript,
                        Nature::Dependencies,
                        metadata,
                        "yarn_install_metadata",
                        true,
                    );
                }
            }
            // Bun and older installers have no common lock inside node_modules.
            if let Some(manifest) = self.child(project, "package.json") {
                if let Some(value) = self.json(manifest) {
                    for section in ["dependencies", "devDependencies"] {
                        let Some(dependencies) = value.get(section).and_then(Value::as_object)
                        else {
                            continue;
                        };
                        for dependency in dependencies.keys().take(256) {
                            let Some(installed) =
                                self.child(index, &format!("{dependency}/package.json"))
                            else {
                                continue;
                            };
                            if self.json(installed).is_some_and(|value| {
                                value.get("name").and_then(Value::as_str) == Some(dependency)
                            }) {
                                self.add(
                                    index,
                                    Family::JavaScript,
                                    Nature::Dependencies,
                                    installed,
                                    "declared_and_installed_dependency",
                                    true,
                                );
                                return;
                            }
                        }
                    }
                }
            }
        }
        if matches!(name.as_str(), ".venv" | "venv") {
            if let Some(metadata) = self.child(index, "pyvenv.cfg") {
                if self.text(metadata).is_some_and(|text| {
                    text.lines().any(|line| {
                        line.split_once('=').is_some_and(|(key, value)| {
                            key.trim() == "home" && !value.trim().is_empty()
                        })
                    })
                }) {
                    self.add(
                        index,
                        Family::Python,
                        Nature::Dependencies,
                        metadata,
                        "python_environment_metadata",
                        true,
                    );
                }
            }
        }
        if name == "target" {
            if let Some(metadata) = self.child(index, ".rustc_info.json") {
                if self
                    .json(metadata)
                    .is_some_and(|value| value.get("rustc_fingerprint").is_some())
                    && self.child(project, "Cargo.toml").is_some()
                {
                    self.add(
                        index,
                        Family::Rust,
                        Nature::Build,
                        metadata,
                        "cargo_rustc_fingerprint",
                        true,
                    );
                }
            }
        }
        let Some(manifest) = self.child(project, "package.json") else {
            return;
        };
        let Some(package) = self.json(manifest) else {
            return;
        };
        let dependencies = |dependency: &str| {
            ["dependencies", "devDependencies"].iter().any(|section| {
                package
                    .get(section)
                    .and_then(|value| value.get(dependency))
                    .is_some_and(Value::is_string)
            })
        };
        for (directory, dependency, marker) in [
            (".next", "next", "BUILD_ID"),
            (".turbo", "turbo", "cache"),
            (".nuxt", "nuxt", "nuxt.json"),
            (".svelte-kit", "@sveltejs/kit", "tsconfig.json"),
        ] {
            if name != directory || !dependencies(dependency) {
                continue;
            }
            if let Some(metadata) = self.child(index, marker) {
                let valid = if marker.ends_with(".json") {
                    self.json(metadata).is_some()
                } else if marker == "BUILD_ID" {
                    self.text(metadata)
                        .is_some_and(|text| !text.trim().is_empty())
                } else {
                    self.records[metadata].finding.kind == Kind::Directory
                };
                if valid {
                    self.add(
                        index,
                        Family::JavaScript,
                        Nature::Build,
                        metadata,
                        "framework_dependency_and_output_metadata",
                        true,
                    );
                }
            }
        }
        if matches!(name.as_str(), "logs" | ".logs") {
            let explicit_log = package
                .get("scripts")
                .and_then(Value::as_object)
                .is_some_and(|scripts| {
                    scripts.values().filter_map(Value::as_str).any(|script| {
                        script.split_once(">").is_some_and(|(_, output)| {
                            let output = output.trim_start_matches('>').trim();
                            output.starts_with(&format!("{name}/"))
                                || output.starts_with(&format!("./{name}/"))
                        })
                    })
                });
            if explicit_log
                && self.children_by_parent[index].iter().any(|child| {
                    self.records[*child].finding.kind == Kind::File
                        && self.records[*child]
                            .name
                            .to_string_lossy()
                            .ends_with(".log")
                })
            {
                self.add(
                    index,
                    Family::ProjectLogs,
                    Nature::Logs,
                    manifest,
                    "project_script_redirects_to_log_directory",
                    false,
                );
            }
        }
    }

    fn has_cache_tag(&mut self, index: usize) -> Option<usize> {
        let tag = self.child(index, "CACHEDIR.TAG")?;
        self.text(tag)
            .filter(|text| text.starts_with("Signature: 8a477f597d28d172789f06886806bc55"))?;
        Some(tag)
    }

    fn tool_store(&mut self, index: usize) {
        let Some(path) = self.path(index) else { return };
        let name = Path::new(&self.records[index].name)
            .file_name()
            .unwrap_or_else(|| OsStr::new(""))
            .to_string_lossy();
        let tagged = self.has_cache_tag(index);
        let defaults: &[(Family, &[&str], &[&str])] = &[
            (
                Family::Npm,
                &[".npm"],
                &["_cacache/index-v5", "_cacache/content-v2"],
            ),
            (Family::Pnpm, &["pnpm/store", ".pnpm-store"], &["v3/files"]),
            (Family::Yarn, &["Yarn", "yarn/v6", ".yarn/cache"], &[]),
            (Family::Bun, &[".bun/install/cache"], &[]),
            (Family::Pip, &["Caches/pip", ".cache/pip"], &["http-v2"]),
            (Family::Uv, &["Caches/uv", ".cache/uv"], &["wheels-v5"]),
            (Family::Go, &["go/pkg/mod"], &["cache/download"]),
            (
                Family::Cargo,
                &[".cargo/registry"],
                &["cache", "src", "index"],
            ),
            (Family::Homebrew, &["Caches/Homebrew"], &["downloads"]),
        ];
        for (family, suffixes, markers) in defaults {
            let location_matches = suffixes.iter().any(|suffix| path.ends_with(suffix));
            if !location_matches {
                continue;
            }
            if markers
                .iter()
                .all(|marker| self.child(index, marker).is_some())
            {
                let evidence = if let Some(marker) = markers.first() {
                    self.child(index, marker)
                } else {
                    self.package_cache_entry(index, *family)
                };
                if let Some(evidence) = evidence {
                    self.add(
                        index,
                        *family,
                        Nature::SharedStore,
                        evidence,
                        "conventional_location_and_tool_layout",
                        true,
                    );
                }
            }
            if let Some(tag) = tagged {
                self.add(
                    index,
                    *family,
                    Nature::SharedStore,
                    tag,
                    "conventional_location_and_cache_signature",
                    true,
                );
            }
        }
        for (variable, family) in [
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
        ] {
            if std::env::var_os(variable).is_some_and(|configured| Path::new(&configured) == path) {
                self.add(
                    index,
                    family,
                    Nature::SharedStore,
                    index,
                    "process_environment_location_inventory",
                    false,
                );
                let artifact = self.artifacts[&index];
                self.result.findings[artifact].confidence = Confidence::Contextual;
            }
        }
        if (path.ends_with(".cache/go-build") || name == "go-build")
            && self.child(index, "README").is_some()
        {
            let metadata = self.child(index, "README").unwrap();
            if self.text(metadata).is_some_and(|text| {
                text.contains("Go build cache")
                    || text.contains("cached build artifacts from the Go build system")
            }) {
                self.add(
                    index,
                    Family::Go,
                    Nature::SharedStore,
                    metadata,
                    "go_build_cache_readme",
                    true,
                );
            }
        }
        if self.child(index, "bin/node").is_some() {
            if let Some(header) = self.child(index, "include/node/node_version.h") {
                if self
                    .text(header)
                    .is_some_and(|text| text.contains("#define NODE_MAJOR_VERSION"))
                {
                    let conventional = path
                        .parent()
                        .is_some_and(|path| path.ends_with("versions/node"));
                    if conventional {
                        self.add(
                            index,
                            Family::Nvm,
                            Nature::Runtime,
                            header,
                            "node_distribution_in_nvm_versions",
                            false,
                        );
                    }
                }
            }
        }
    }

    fn package_cache_entry(&mut self, index: usize, family: Family) -> Option<usize> {
        for child in self.children_by_parent[index].clone() {
            let name = self.records[child].name.to_string_lossy();
            if family == Family::Yarn && (name.starts_with("npm-") || name.ends_with(".zip")) {
                return Some(child);
            }
            if family == Family::Bun {
                if let Some(manifest) = self.child(child, "package.json") {
                    if self.json(manifest).is_some_and(|value| {
                        value.get("name").is_some_and(Value::is_string)
                            && value.get("version").is_some_and(Value::is_string)
                    }) {
                        return Some(manifest);
                    }
                }
            }
        }
        None
    }

    fn configured_store(&mut self, index: usize) {
        let name = self.records[index].name.to_string_lossy();
        let configuration = match name.as_ref() {
            ".npmrc" => Some((Family::Npm, "cache", false)),
            ".yarnrc.yml" => Some((Family::Yarn, "cacheFolder", false)),
            "pip.conf" => Some((Family::Pip, "cache-dir", false)),
            "uv.toml" => Some((Family::Uv, "cache-dir", true)),
            "bunfig.toml" => Some((Family::Bun, "install.cache.dir", true)),
            _ => None,
        };
        let Some((family, key, is_toml)) = configuration else {
            return;
        };
        let Some(content) = self.text(index) else {
            return;
        };
        let configured = if is_toml {
            match content.parse::<toml::Table>() {
                Ok(table) => {
                    let mut current = None;
                    for (position, key) in key.split('.').enumerate() {
                        current = if position == 0 {
                            table.get(key)
                        } else {
                            current.and_then(|value| value.get(key))
                        };
                    }
                    current.and_then(toml::Value::as_str).map(str::to_owned)
                }
                Err(_) => {
                    self.diagnostic(index, "malformed_tool_configuration");
                    None
                }
            }
        } else {
            content
                .lines()
                .filter_map(|line| line.split_once(['=', ':']))
                .find(|(candidate, _)| candidate.trim() == key)
                .map(|(_, value)| value.trim().trim_matches(['\'', '"']).to_owned())
        };
        let Some(configured) = configured else { return };
        let Some(parent) = self
            .path(index)
            .and_then(|path| path.parent().map(Path::to_owned))
        else {
            return;
        };
        let path = if Path::new(&configured).is_absolute() {
            PathBuf::from(configured)
        } else {
            parent.join(configured)
        };
        let Some(path) = normalize(&path) else {
            self.diagnostic(index, "unsupported_configuration_path");
            return;
        };
        let Some(target) = self.paths.get(&path).copied() else {
            self.diagnostic(index, "configured_location_outside_scan_or_unavailable");
            return;
        };
        if self.records[target].finding.kind != Kind::Directory {
            self.diagnostic(index, "configured_location_not_directory");
            return;
        }
        self.add(
            target,
            family,
            Nature::SharedStore,
            index,
            "scoped_tool_configuration_location_inventory",
            false,
        );
        if let Some(artifact) = self.artifacts.get(&target) {
            self.result.findings[*artifact].confidence = Confidence::Contextual;
        }
    }

    fn editor(&mut self, index: usize) {
        let Some(path) = self.path(index) else { return };
        for (suffix, family) in [
            ("Application Support/Code", Family::VsCode),
            ("Application Support/Cursor", Family::Cursor),
        ] {
            if !path.ends_with(suffix) {
                continue;
            }
            let Some(settings) = self.child(index, "User/settings.json") else {
                continue;
            };
            if self.json(settings).is_none() {
                continue;
            }
            if self.child(index, "User").is_some()
                && (self.child(index, "logs").is_some()
                    || self.child(index, "CachedData").is_some())
            {
                self.add(
                    index,
                    family,
                    Nature::EditorState,
                    settings,
                    "editor_user_profile_and_runtime_layout",
                    false,
                );
                self.editor_children(index, family);
            }
        }
        let name = self.records[index].name.to_string_lossy();
        if path
            .components()
            .any(|component| component.as_os_str() == "JetBrains")
            && (name.starts_with("IntelliJIdea")
                || name.starts_with("PyCharm")
                || name.starts_with("WebStorm")
                || name.starts_with("RustRover"))
        {
            let marker = self
                .child(index, "options/ide.general.xml")
                .or_else(|| self.child(index, ".home"));
            if let Some(metadata) = marker {
                if self.text(metadata).is_some_and(|text| {
                    text.contains("<application>") || Path::new(text.trim()).is_absolute()
                }) {
                    self.add(
                        index,
                        Family::JetBrains,
                        Nature::EditorState,
                        metadata,
                        "jetbrains_product_and_profile_metadata",
                        false,
                    );
                    self.editor_children(index, Family::JetBrains);
                }
            }
        }
        for (suffix, family, marker) in [
            (".claude", Family::Claude, "settings.json"),
            (".codex", Family::Codex, "config.toml"),
            (".Codex", Family::Codex, "config.toml"),
        ] {
            if !path.ends_with(suffix) {
                continue;
            }
            let Some(metadata) = self.child(index, marker) else {
                continue;
            };
            let valid = if marker.ends_with("json") {
                self.json(metadata).is_some()
            } else {
                self.text(metadata)
                    .is_some_and(|text| text.parse::<toml::Table>().is_ok())
            };
            if valid {
                self.add(
                    index,
                    family,
                    Nature::EditorState,
                    metadata,
                    "coding_tool_configuration_and_location",
                    false,
                );
                self.editor_children(index, family);
            }
        }
    }

    fn editor_children(&mut self, index: usize, family: Family) {
        for (name, nature) in [
            ("logs", Nature::Logs),
            ("CachedData", Nature::Build),
            ("Cache", Nature::Build),
            ("caches", Nature::Build),
        ] {
            if let Some(child) = self.child(index, name) {
                self.add(
                    child,
                    family,
                    nature,
                    index,
                    "verified_editor_owned_child",
                    false,
                );
            }
        }
    }
}

pub(super) fn normalize(path: &Path) -> Option<PathBuf> {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::RootDir => result.push("/"),
            Component::Normal(name) => result.push(name),
            Component::CurDir => {}
            Component::ParentDir => {
                if !result.pop() {
                    return None;
                }
            }
            _ => return None,
        }
    }
    result.is_absolute().then_some(result)
}
