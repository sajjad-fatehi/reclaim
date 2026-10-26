//! Local configuration is data, never consent to start scanning.
use crate::scan::{authorize, BoundaryResult, ErrorCode, Location, ScanService};
use reclaim_core::scan::{display, lock, Exclusion};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager, Runtime, State, WebviewWindow};

const MAX_CONFIG_BYTES: u64 = 131_072;

#[derive(Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Appearance {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoredRoot {
    key: String,
    path: PathBuf,
}

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Configuration {
    appearance: Appearance,
    roots: Vec<StoredRoot>,
    exclusions: Vec<Exclusion>,
    configured: bool,
}

impl Configuration {
    fn validate(&self) -> bool {
        self.roots.len() <= 16
            && self.roots.iter().all(|root| {
                root.key.len() == 36
                    && root.path.is_absolute()
                    && root.path.as_os_str().len() <= 4096
                    && root.path.components().all(|component| {
                        matches!(
                            component,
                            std::path::Component::RootDir | std::path::Component::Normal(_)
                        )
                    })
            })
            && self.exclusions.len() <= 64
            && self.exclusions.iter().all(Exclusion::validate)
    }
}

#[derive(Default)]
pub struct PreferencesService {
    configuration: Mutex<Option<Configuration>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RootPreference {
    key: String,
    display_path: String,
    location: Option<Location>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preferences {
    appearance: Appearance,
    roots: Vec<RootPreference>,
    exclusions: Vec<Exclusion>,
    configured: bool,
    corrupt: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RootSelection {
    key: Option<String>,
    location_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreferenceUpdate {
    appearance: Appearance,
    roots: Vec<RootSelection>,
    exclusions: Vec<Exclusion>,
}

fn read_configuration(path: &Path) -> Result<Configuration, ()> {
    if !path.exists() {
        return Ok(Configuration::default());
    }
    if fs::symlink_metadata(path)
        .map_err(|_| ())?
        .file_type()
        .is_symlink()
    {
        return Err(());
    }
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|_| ())?
        .take(MAX_CONFIG_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ())?;
    if bytes.len() as u64 > MAX_CONFIG_BYTES {
        return Err(());
    }
    let configuration: Configuration = serde_json::from_slice(&bytes).map_err(|_| ())?;
    configuration.validate().then_some(configuration).ok_or(())
}

fn write_configuration(path: &Path, configuration: &Configuration) -> BoundaryResult<()> {
    if !configuration.validate() {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let parent = path.parent().ok_or(ErrorCode::Internal)?;
    fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary
        .as_file()
        .set_permissions(fs::Permissions::from_mode(0o600))?;
    serde_json::to_writer(&mut temporary, configuration).map_err(|_| ErrorCode::Internal)?;
    temporary.flush()?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|error| error.error)?;
    File::open(parent)?.sync_all()?;
    Ok(())
}

impl PreferencesService {
    pub fn load(&self, path: &Path, scanner: &ScanService) -> Preferences {
        let loaded = read_configuration(path);
        let corrupt = loaded.is_err();
        let configuration = loaded.unwrap_or_default();
        let roots = configuration
            .roots
            .iter()
            .map(|root| RootPreference {
                key: root.key.clone(),
                display_path: display(root.path.as_os_str()),
                location: scanner.register_native_location(&root.path).ok(),
            })
            .collect();
        let response = Preferences {
            appearance: configuration.appearance,
            roots,
            exclusions: configuration.exclusions.clone(),
            configured: configuration.configured,
            corrupt,
        };
        *lock(&self.configuration) = Some(configuration);
        response
    }

    pub fn save(
        &self,
        path: &Path,
        scanner: &ScanService,
        update: PreferenceUpdate,
    ) -> BoundaryResult<()> {
        if update.roots.len() > 16
            || update.exclusions.len() > 64
            || !update.exclusions.iter().all(Exclusion::validate)
        {
            return Err(ErrorCode::InvalidRequest.into());
        }
        let mut current = lock(&self.configuration);
        let previous = current.as_ref().ok_or(ErrorCode::InvalidRequest)?;
        let mut roots: Vec<StoredRoot> = Vec::new();
        for selection in update.roots {
            let root = if let Some(location_id) = selection.location_id {
                StoredRoot {
                    key: uuid::Uuid::new_v4().to_string(),
                    path: scanner.location_path(&location_id)?,
                }
            } else {
                previous
                    .roots
                    .iter()
                    .find(|root| Some(&root.key) == selection.key.as_ref())
                    .ok_or(ErrorCode::InvalidScope)?
                    .clone()
            };
            if !roots.iter().any(|existing| existing.path == root.path) {
                roots.push(root);
            }
        }
        let configuration = Configuration {
            appearance: update.appearance,
            roots,
            exclusions: update.exclusions,
            configured: true,
        };
        write_configuration(path, &configuration)?;
        *current = Some(configuration);
        Ok(())
    }
}

fn configuration_path<R: Runtime>(app: &AppHandle<R>) -> BoundaryResult<PathBuf> {
    Ok(app
        .path()
        .app_config_dir()
        .map_err(|_| ErrorCode::Unavailable)?
        .join("preferences.json"))
}

#[tauri::command]
pub async fn preferences_load<R: Runtime>(
    window: WebviewWindow<R>,
    app: AppHandle<R>,
    scanner: State<'_, Arc<ScanService>>,
    preferences: State<'_, PreferencesService>,
) -> BoundaryResult<Preferences> {
    authorize(&window)?;
    Ok(preferences.load(&configuration_path(&app)?, &scanner))
}

#[tauri::command]
pub async fn preferences_save<R: Runtime>(
    window: WebviewWindow<R>,
    app: AppHandle<R>,
    scanner: State<'_, Arc<ScanService>>,
    preferences: State<'_, PreferencesService>,
    update: PreferenceUpdate,
) -> BoundaryResult<()> {
    authorize(&window)?;
    preferences.save(&configuration_path(&app)?, &scanner, update)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_configuration_roundtrip_corruption_and_untrusted_requests() {
        let temporary = tempfile::tempdir().unwrap();
        let path = temporary.path().join("preferences.json");
        let scanner = ScanService::default();
        let preferences = PreferencesService::default();
        assert!(!preferences.load(&path, &scanner).configured);
        let folder = temporary.path().join("synthetic");
        fs::create_dir(&folder).unwrap();
        let location = scanner.register_native_location(&folder).unwrap();
        preferences
            .save(
                &path,
                &scanner,
                PreferenceUpdate {
                    appearance: Appearance::Dark,
                    roots: vec![RootSelection {
                        key: None,
                        location_id: Some(location.id),
                    }],
                    exclusions: vec![Exclusion {
                        relative_path: "cache".into(),
                        subtree: true,
                    }],
                },
            )
            .unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let restarted_scanner = ScanService::default();
        let restarted = PreferencesService::default();
        let loaded = restarted.load(&path, &restarted_scanner);
        assert_eq!(loaded.roots.len(), 1);
        assert!(loaded.roots[0].location.is_some());
        assert!(restarted_scanner.with_terminal_scan(1, |_| ()).is_err());
        fs::remove_dir(&folder).unwrap();
        assert!(restarted.load(&path, &restarted_scanner).roots[0]
            .location
            .is_none());
        let original = fs::read(&path).unwrap();
        assert!(restarted
            .save(
                &path,
                &restarted_scanner,
                PreferenceUpdate {
                    appearance: Appearance::Light,
                    roots: vec![],
                    exclusions: vec![Exclusion {
                        relative_path: "../escape".into(),
                        subtree: true
                    }],
                }
            )
            .is_err());
        assert_eq!(fs::read(&path).unwrap(), original);
        fs::write(&path, b"{corrupt").unwrap();
        assert!(restarted.load(&path, &restarted_scanner).corrupt);
        assert_eq!(fs::read(&path).unwrap(), b"{corrupt");
        fs::write(&path, vec![b' '; MAX_CONFIG_BYTES as usize + 1]).unwrap();
        assert!(restarted.load(&path, &restarted_scanner).corrupt);
    }
}
