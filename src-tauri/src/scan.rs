//! Process-local authority and bounded transport. Never accepts frontend paths.

use reclaim_core::scan::{
    self, lock, Exclusion, Finding, Issue, Progress, Root, ScanData, Status, Volume,
};
use serde::Serialize;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Runtime, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

const MAX_CANDIDATES: usize = 32;
const READ_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidRequest,
    InvalidScope,
    StaleGeneration,
    Busy,
    RateLimited,
    AccessDenied,
    Unavailable,
    Internal,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoundaryError {
    pub code: ErrorCode,
    pub issue: Option<Issue>,
}

impl From<ErrorCode> for BoundaryError {
    fn from(code: ErrorCode) -> Self {
        Self { code, issue: None }
    }
}

impl From<std::io::Error> for BoundaryError {
    fn from(error: std::io::Error) -> Self {
        Self {
            code: ErrorCode::Unavailable,
            issue: Some(scan::io_issue(&error)),
        }
    }
}

pub type BoundaryResult<T> = Result<T, BoundaryError>;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Location {
    pub id: String,
    pub display_path: String,
    pub volume: Volume,
}

struct Candidate {
    location: Location,
    root: Root,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Scope {
    pub id: String,
    pub roots: Vec<Location>,
    pub exclusions: Vec<Exclusion>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Locations {
    pub candidates: Vec<Location>,
    pub unavailable_count: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub progress: Progress,
    pub offset: usize,
    pub next_offset: Option<usize>,
    pub findings: Vec<Finding>,
}

struct Session {
    generation: u32,
    scope: Scope,
    data: Arc<Mutex<ScanData>>,
    cancel: Arc<AtomicBool>,
    last_read: Option<Instant>,
}

#[derive(Default)]
struct Authority {
    candidates: Vec<Candidate>,
    discovered: bool,
    unavailable_count: usize,
    scope: Option<Scope>,
    session: Option<Session>,
    generation: u32,
    picking: bool,
}

#[derive(Default)]
pub struct ScanService {
    authority: Mutex<Authority>,
}

fn token() -> String {
    uuid::Uuid::new_v4().to_string()
}

impl Authority {
    fn ensure_idle(&self) -> BoundaryResult<()> {
        if self
            .session
            .as_ref()
            .is_some_and(|session| lock(&session.data).progress.status == Status::Running)
        {
            return Err(ErrorCode::Busy.into());
        }
        Ok(())
    }

    fn add_candidate(&mut self, path: &Path) -> BoundaryResult<Location> {
        let path = path.canonicalize()?;
        let root = Root::open(&path)?;
        if let Some(index) = self
            .candidates
            .iter()
            .position(|candidate| candidate.root.path == path)
        {
            let current_identity = root.directory.metadata()?;
            let granted_identity = self.candidates[index].root.directory.metadata()?;
            if current_identity.dev() == granted_identity.dev()
                && current_identity.ino() == granted_identity.ino()
            {
                return Ok(self.candidates[index].location.clone());
            }
            let stale_id = self.candidates.remove(index).location.id;
            if self
                .scope
                .as_ref()
                .is_some_and(|scope| scope.roots.iter().any(|location| location.id == stale_id))
            {
                self.scope = None;
            }
        }
        if self.candidates.len() >= MAX_CANDIDATES {
            return Err(ErrorCode::InvalidRequest.into());
        }
        let location = Location {
            id: token(),
            display_path: scan::display(path.as_os_str()),
            volume: scan::volume(&root),
        };
        self.candidates.push(Candidate {
            location: location.clone(),
            root,
        });
        Ok(location)
    }

    fn register(&mut self, ids: &[String]) -> BoundaryResult<Scope> {
        self.ensure_idle()?;
        if ids.is_empty() || ids.len() > scan::MAX_ROOTS || ids.iter().any(|id| id.len() != 36) {
            return Err(ErrorCode::InvalidRequest.into());
        }
        let mut roots = Vec::new();
        for id in ids {
            let candidate = self
                .candidates
                .iter()
                .find(|candidate| candidate.location.id == *id)
                .ok_or(ErrorCode::InvalidScope)?;
            if !roots.iter().any(|location: &Location| location.id == *id) {
                roots.push(candidate.location.clone());
            }
        }
        let scope = Scope {
            id: token(),
            roots,
            exclusions: Vec::new(),
        };
        self.scope = Some(scope.clone());
        Ok(scope)
    }

    fn session(&mut self, generation: u32) -> BoundaryResult<&mut Session> {
        self.session
            .as_mut()
            .filter(|session| session.generation == generation)
            .ok_or_else(|| ErrorCode::StaleGeneration.into())
    }
}

impl ScanService {
    /// Native callers must obtain user selection before registering a location.
    /// This method is deliberately absent from the IPC command surface.
    pub fn register_native_location(&self, path: &Path) -> BoundaryResult<Location> {
        lock(&self.authority).add_candidate(path)
    }

    fn discover(&self) -> Locations {
        let mut authority = lock(&self.authority);
        if !authority.discovered {
            authority.discovered = true;
            if let Some(home) = std::env::var_os("HOME") {
                let home = std::path::PathBuf::from(home);
                for path in [
                    home.clone(),
                    home.join("Desktop"),
                    home.join("Documents"),
                    home.join("Downloads"),
                    home.join("Library/Caches"),
                ] {
                    let candidate = path.canonicalize().and_then(|path| Root::open(&path));
                    match candidate {
                        Ok(root) if root.owned_by_current_user().unwrap_or(false) => {
                            if authority.add_candidate(&path).is_err() {
                                authority.unavailable_count += 1;
                            }
                        }
                        _ => authority.unavailable_count += 1,
                    }
                }
            } else {
                authority.unavailable_count += 1;
            }
        }
        Locations {
            candidates: authority
                .candidates
                .iter()
                .map(|candidate| candidate.location.clone())
                .collect(),
            unavailable_count: authority.unavailable_count,
        }
    }

    /// Native consumers inspect terminal data without granting frontend path access.
    /// Keep inspection short; copy required records before expensive analysis.
    pub fn with_terminal_scan<Output>(
        &self,
        generation: u32,
        inspect: impl FnOnce(&ScanData) -> Output,
    ) -> BoundaryResult<Output> {
        let mut authority = lock(&self.authority);
        let session = authority.session(generation)?;
        let data = lock(&session.data);
        if data.progress.status == Status::Running {
            return Err(ErrorCode::Busy.into());
        }
        Ok(inspect(&data))
    }

    pub fn location_path(&self, id: &str) -> BoundaryResult<std::path::PathBuf> {
        lock(&self.authority)
            .candidates
            .iter()
            .find(|candidate| candidate.location.id == id)
            .map(|candidate| candidate.root.path.clone())
            .ok_or_else(|| ErrorCode::InvalidScope.into())
    }

    pub fn start(&self, scope_id: &str) -> BoundaryResult<Progress> {
        let mut authority = lock(&self.authority);
        authority.ensure_idle()?;
        if scope_id.len() != 36 {
            return Err(ErrorCode::InvalidScope.into());
        }
        let scope = authority
            .scope
            .as_ref()
            .filter(|scope| scope.id == scope_id)
            .ok_or(ErrorCode::InvalidScope)?
            .clone();
        let mut roots = Vec::new();
        for location in &scope.roots {
            let candidate = authority
                .candidates
                .iter()
                .find(|candidate| candidate.location.id == location.id)
                .ok_or(ErrorCode::InvalidScope)?;
            let current = Root::open(&candidate.root.path)?;
            let current_identity = current.directory.metadata()?;
            let granted_identity = candidate.root.directory.metadata()?;
            if current_identity.dev() != granted_identity.dev()
                || current_identity.ino() != granted_identity.ino()
            {
                return Err(BoundaryError {
                    code: ErrorCode::Unavailable,
                    issue: Some(Issue::Changed),
                });
            }
            roots.push(candidate.root.try_clone()?);
        }
        let exclusions = roots
            .iter()
            .flat_map(|root| {
                scope
                    .exclusions
                    .iter()
                    .map(|exclusion| (root.path.join(&exclusion.relative_path), exclusion.subtree))
            })
            .collect();
        authority.generation = authority
            .generation
            .checked_add(1)
            .ok_or(ErrorCode::Internal)?;
        let generation = authority.generation;
        let data = Arc::new(Mutex::new(ScanData::new(generation)));
        let progress = lock(&data).progress.clone();
        let cancel = Arc::new(AtomicBool::new(false));
        authority.session = Some(Session {
            generation,
            scope,
            data: data.clone(),
            cancel: cancel.clone(),
            last_read: None,
        });
        let failure_data = data.clone();
        let spawned = std::thread::Builder::new()
            .name("reclaim-scan".into())
            .spawn(move || {
                if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    scan::scan_excluding(roots, exclusions, data, cancel)
                }))
                .is_err()
                {
                    fail_worker(&failure_data);
                }
            });
        if spawned.is_err() {
            fail_worker(&authority.session.as_ref().ok_or(ErrorCode::Internal)?.data);
            return Err(ErrorCode::Internal.into());
        }
        Ok(progress)
    }

    pub fn status(&self, generation: u32) -> BoundaryResult<Progress> {
        let mut authority = lock(&self.authority);
        let session = authority.session(generation)?;
        throttle(session)?;
        let progress = lock(&session.data).progress.clone();
        Ok(progress)
    }

    pub fn page(&self, generation: u32, offset: usize, limit: usize) -> BoundaryResult<Page> {
        if limit == 0 || limit > scan::MAX_PAGE || offset > scan::MAX_ENTRIES {
            return Err(ErrorCode::InvalidRequest.into());
        }
        let mut authority = lock(&self.authority);
        let session = authority.session(generation)?;
        throttle(session)?;
        let data = lock(&session.data);
        if offset > data.records.len() {
            return Err(ErrorCode::InvalidRequest.into());
        }
        let end = offset.saturating_add(limit).min(data.records.len());
        Ok(Page {
            progress: data.progress.clone(),
            offset,
            next_offset: (end < data.records.len()).then_some(end),
            findings: data.records[offset..end]
                .iter()
                .map(|record| record.finding.clone())
                .collect(),
        })
    }

    pub fn cancel(&self, generation: u32) -> BoundaryResult<Progress> {
        let mut authority = lock(&self.authority);
        let session = authority.session(generation)?;
        session.cancel.store(true, Ordering::Relaxed);
        let progress = lock(&session.data).progress.clone();
        Ok(progress)
    }
}

fn fail_worker(data: &Mutex<ScanData>) {
    let mut data = lock(data);
    data.progress.status = Status::Failed;
    data.progress.terminal_issue = Some(Issue::WorkerFailed);
    data.progress.finished_at_ms = Some(scan::now_ms());
    for record in &mut data.records {
        if record.finding.kind == scan::Kind::Directory {
            record.finding.complete = false;
        }
    }
}

fn throttle(session: &mut Session) -> BoundaryResult<()> {
    let now = Instant::now();
    if session
        .last_read
        .is_some_and(|last| now.duration_since(last) < READ_INTERVAL)
    {
        return Err(ErrorCode::RateLimited.into());
    }
    session.last_read = Some(now);
    Ok(())
}

pub(crate) fn authorize<R: Runtime>(window: &WebviewWindow<R>) -> BoundaryResult<()> {
    if window.label() != "main" {
        return Err(ErrorCode::AccessDenied.into());
    }
    let url = window.url().map_err(|_| ErrorCode::AccessDenied)?;
    let bundled = (url.scheme() == "tauri" && url.host_str() == Some("localhost"))
        || (url.scheme() == "http" && url.host_str() == Some("tauri.localhost"));
    let development = cfg!(debug_assertions)
        && url.scheme() == "http"
        && url.host_str() == Some("localhost")
        && url.port() == Some(1420);
    if !bundled && !development {
        return Err(ErrorCode::AccessDenied.into());
    }
    Ok(())
}

#[tauri::command]
pub async fn scan_locations<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, Arc<ScanService>>,
) -> BoundaryResult<Locations> {
    authorize(&window)?;
    let service = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || service.discover())
        .await
        .map_err(|_| ErrorCode::Internal.into())
}

#[tauri::command]
pub async fn scan_pick_folder<R: Runtime>(
    window: WebviewWindow<R>,
    app: AppHandle<R>,
    state: State<'_, Arc<ScanService>>,
) -> BoundaryResult<Option<Location>> {
    authorize(&window)?;
    let service = state.inner().clone();
    {
        let mut authority = lock(&service.authority);
        if authority.picking {
            return Err(ErrorCode::Busy.into());
        }
        authority.picking = true;
    }
    let outcome = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Select a folder to scan")
            .blocking_pick_folder()
    })
    .await;
    let mut authority = lock(&service.authority);
    authority.picking = false;
    let selected = outcome.map_err(|_| ErrorCode::Internal)?;
    match selected {
        None => Ok(None),
        Some(path) => {
            let path = path.into_path().map_err(|_| ErrorCode::InvalidRequest)?;
            authority.add_candidate(&path).map(Some)
        }
    }
}

#[tauri::command]
pub async fn scan_register_scope<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, Arc<ScanService>>,
    location_ids: Vec<String>,
    exclusions: Option<Vec<Exclusion>>,
) -> BoundaryResult<Scope> {
    authorize(&window)?;
    let exclusions = exclusions.unwrap_or_default();
    if exclusions.len() > 64 || !exclusions.iter().all(Exclusion::validate) {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let mut authority = lock(&state.authority);
    let mut scope = authority.register(&location_ids)?;
    scope.exclusions = exclusions;
    authority.scope = Some(scope.clone());
    Ok(scope)
}

#[tauri::command]
pub async fn scan_start<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, Arc<ScanService>>,
    scope_id: String,
) -> BoundaryResult<Progress> {
    authorize(&window)?;
    state.start(&scope_id)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Current {
    pub scope: Option<Scope>,
    pub progress: Option<Progress>,
}

#[tauri::command]
pub async fn scan_current<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, Arc<ScanService>>,
) -> BoundaryResult<Current> {
    authorize(&window)?;
    let mut authority = lock(&state.authority);
    let scope = authority.scope.clone();
    match authority.session.as_mut() {
        None => Ok(Current {
            scope,
            progress: None,
        }),
        Some(session) => {
            throttle(session)?;
            Ok(Current {
                scope: Some(session.scope.clone()),
                progress: Some(lock(&session.data).progress.clone()),
            })
        }
    }
}

#[tauri::command]
pub async fn scan_status<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, Arc<ScanService>>,
    generation: u32,
) -> BoundaryResult<Progress> {
    authorize(&window)?;
    state.status(generation)
}

#[tauri::command]
pub async fn scan_page<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, Arc<ScanService>>,
    generation: u32,
    offset: usize,
    limit: usize,
) -> BoundaryResult<Page> {
    authorize(&window)?;
    state.page(generation, offset, limit)
}

#[tauri::command]
pub async fn scan_cancel<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, Arc<ScanService>>,
    generation: u32,
) -> BoundaryResult<Progress> {
    authorize(&window)?;
    state.cancel(generation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    use std::fs;
    use tauri::test::{get_ipc_response, mock_builder, MockRuntime, INVOKE_KEY};
    use tauri::Manager;

    fn fixture() -> (tempfile::TempDir, Arc<ScanService>, Scope) {
        let temporary = tempfile::tempdir().unwrap();
        fs::write(temporary.path().join("fixture"), b"12345").unwrap();
        let service = Arc::new(ScanService::default());
        let location = service.register_native_location(temporary.path()).unwrap();
        let scope = lock(&service.authority).register(&[location.id]).unwrap();
        (temporary, service, scope)
    }

    fn terminal(service: &ScanService) -> Progress {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let authority = lock(&service.authority);
            let progress = lock(&authority.session.as_ref().unwrap().data)
                .progress
                .clone();
            if progress.status != Status::Running {
                return progress;
            }
            assert!(Instant::now() < deadline);
            drop(authority);
            std::thread::yield_now();
        }
    }

    #[test]
    fn authority_lifecycle_bounded_pages_rate_limits_and_stale_generations() {
        let (_temporary, service, scope) = fixture();
        assert_eq!(
            service.start("/arbitrary/path").unwrap_err().code,
            ErrorCode::InvalidScope
        );
        assert_eq!(
            lock(&service.authority).register(&[]).unwrap_err().code,
            ErrorCode::InvalidRequest
        );
        assert_eq!(
            lock(&service.authority)
                .register(&[token()])
                .unwrap_err()
                .code,
            ErrorCode::InvalidScope
        );
        assert_eq!(
            lock(&service.authority)
                .register(&vec![token(); 17])
                .unwrap_err()
                .code,
            ErrorCode::InvalidRequest
        );
        assert_eq!(service.start(&scope.id).unwrap().status, Status::Running);
        let progress = terminal(&service);
        assert_eq!(progress.status, Status::Complete);
        assert_eq!(progress.logical_bytes, 5);
        assert_eq!(
            service
                .with_terminal_scan(progress.generation, |data| data
                    .path(1)
                    .unwrap()
                    .file_name()
                    .unwrap()
                    .to_owned())
                .unwrap(),
            "fixture"
        );
        assert_eq!(
            service
                .page(progress.generation, 0, 201)
                .err()
                .unwrap()
                .code,
            ErrorCode::InvalidRequest
        );
        let first = service.page(progress.generation, 0, 1).unwrap();
        assert_eq!(first.findings.len(), 1);
        assert_eq!(first.next_offset, Some(1));
        assert_eq!(
            service.status(progress.generation).unwrap_err().code,
            ErrorCode::RateLimited
        );
        lock(&service.authority).session.as_mut().unwrap().last_read = None;
        let second = service.page(progress.generation, 1, 200).unwrap();
        assert_eq!(second.findings.len(), 1);
        assert_eq!(second.next_offset, None);
        service.start(&scope.id).unwrap();
        assert_eq!(
            service.cancel(progress.generation).unwrap_err().code,
            ErrorCode::StaleGeneration
        );
        assert_eq!(
            service.page(progress.generation, 0, 1).err().unwrap().code,
            ErrorCode::StaleGeneration
        );
        terminal(&service);
    }

    #[test]
    fn busy_and_cancel_are_predictable_and_completed_findings_survive() {
        let (_temporary, service, scope) = fixture();
        let data = Arc::new(Mutex::new(ScanData::new(7)));
        lock(&service.authority).session = Some(Session {
            generation: 7,
            scope: scope.clone(),
            data: data.clone(),
            cancel: Arc::new(AtomicBool::new(false)),
            last_read: None,
        });
        assert_eq!(service.start(&scope.id).unwrap_err().code, ErrorCode::Busy);
        assert_eq!(
            lock(&service.authority)
                .register(&[scope.roots[0].id.clone()])
                .unwrap_err()
                .code,
            ErrorCode::Busy
        );
        assert_eq!(
            service.with_terminal_scan(7, |_| ()).unwrap_err().code,
            ErrorCode::Busy
        );
        assert_eq!(service.cancel(7).unwrap().status, Status::Running);
        assert!(lock(&service.authority)
            .session
            .as_ref()
            .unwrap()
            .cancel
            .load(Ordering::Relaxed));
        fail_worker(&data);
        assert_eq!(lock(&data).progress.status, Status::Failed);
        assert_eq!(
            lock(&data).progress.terminal_issue,
            Some(Issue::WorkerFailed)
        );
    }

    #[test]
    fn replaced_scope_paths_rejected_and_unavailable_paths_are_typed() {
        let (temporary, service, scope) = fixture();
        let path = temporary.path().canonicalize().unwrap();
        fs::rename(path.join("fixture"), path.join("before")).unwrap();
        let granted = path.join("selected");
        fs::create_dir(&granted).unwrap();
        let location = service.register_native_location(&granted).unwrap();
        let replaced_scope = lock(&service.authority).register(&[location.id]).unwrap();
        fs::rename(&granted, path.join("moved")).unwrap();
        fs::create_dir(&granted).unwrap();
        let error = service.start(&replaced_scope.id).unwrap_err();
        assert_eq!(error.issue, Some(Issue::Changed));
        assert_eq!(
            service
                .register_native_location(&path.join("missing"))
                .unwrap_err()
                .issue,
            Some(Issue::Vanished)
        );
        assert_eq!(
            service.start(&scope.id).unwrap_err().code,
            ErrorCode::InvalidScope
        );
    }

    #[test]
    fn explicit_reselection_replaces_stale_grants_and_requires_new_scope() {
        let temporary = tempfile::tempdir().unwrap();
        let selected = temporary.path().join("selected");
        fs::create_dir(&selected).unwrap();
        fs::write(selected.join("original"), b"old").unwrap();
        let service = ScanService::default();
        let original = service.register_native_location(&selected).unwrap();
        let old_scope = lock(&service.authority)
            .register(std::slice::from_ref(&original.id))
            .unwrap();
        assert_eq!(
            service.register_native_location(&selected).unwrap().id,
            original.id
        );

        fs::rename(&selected, temporary.path().join("moved")).unwrap();
        fs::create_dir(&selected).unwrap();
        fs::write(selected.join("replacement"), b"replacement").unwrap();
        assert_eq!(
            service.start(&old_scope.id).unwrap_err().issue,
            Some(Issue::Changed)
        );
        assert!(lock(&service.authority).session.is_none());

        let replacement = service.register_native_location(&selected).unwrap();
        assert_ne!(replacement.id, original.id);
        assert_eq!(lock(&service.authority).candidates.len(), 1);
        assert!(lock(&service.authority).scope.is_none());
        assert_eq!(
            service.start(&old_scope.id).unwrap_err().code,
            ErrorCode::InvalidScope
        );
        assert_eq!(
            service.start(&replacement.id).unwrap_err().code,
            ErrorCode::InvalidScope
        );
        assert_eq!(
            lock(&service.authority)
                .register(&[original.id])
                .unwrap_err()
                .code,
            ErrorCode::InvalidScope
        );
        let new_scope = lock(&service.authority)
            .register(std::slice::from_ref(&replacement.id))
            .unwrap();
        assert_ne!(new_scope.id, old_scope.id);
        assert_eq!(
            service.start(&old_scope.id).unwrap_err().code,
            ErrorCode::InvalidScope
        );
        let started = service.start(&new_scope.id).unwrap();
        let progress = terminal(&service);
        assert_eq!(progress.status, Status::Complete);
        assert_eq!(progress.logical_bytes, 11);
        assert_eq!(
            service
                .with_terminal_scan(started.generation, |data| data
                    .records
                    .iter()
                    .map(|record| record.name.clone())
                    .collect::<Vec<_>>())
                .unwrap(),
            vec![
                selected.canonicalize().unwrap().into_os_string(),
                "replacement".into()
            ]
        );
        assert_eq!(
            service.register_native_location(&selected).unwrap().id,
            replacement.id
        );
        assert_eq!(
            service.start(&old_scope.id).unwrap_err().code,
            ErrorCode::InvalidScope
        );
        assert_eq!(
            fs::read(temporary.path().join("moved/original")).unwrap(),
            b"old"
        );
    }

    #[test]
    fn candidate_limit_allows_reselection_but_rejects_new_locations() {
        let temporary = tempfile::tempdir().unwrap();
        let service = ScanService::default();
        let mut locations = Vec::new();
        for index in 0..MAX_CANDIDATES {
            let path = temporary.path().join(format!("candidate-{index}"));
            fs::create_dir(&path).unwrap();
            locations.push(service.register_native_location(&path).unwrap());
        }
        let selected = temporary.path().join("candidate-0");
        assert_eq!(
            service.register_native_location(&selected).unwrap().id,
            locations[0].id
        );
        fs::rename(&selected, temporary.path().join("moved")).unwrap();
        fs::create_dir(&selected).unwrap();
        assert_ne!(
            service.register_native_location(&selected).unwrap().id,
            locations[0].id
        );
        assert_eq!(lock(&service.authority).candidates.len(), MAX_CANDIDATES);
        let additional = temporary.path().join("additional");
        fs::create_dir(&additional).unwrap();
        assert_eq!(
            service
                .register_native_location(&additional)
                .unwrap_err()
                .code,
            ErrorCode::InvalidRequest
        );
    }

    fn ipc(
        window: &WebviewWindow<MockRuntime>,
        command: &str,
        body: Value,
    ) -> Result<Value, Value> {
        get_ipc_response(
            window,
            tauri::webview::InvokeRequest {
                cmd: command.into(),
                callback: tauri::ipc::CallbackFn(0),
                error: tauri::ipc::CallbackFn(1),
                url: window.url().unwrap(),
                body: tauri::ipc::InvokeBody::Json(body),
                headers: Default::default(),
                invoke_key: INVOKE_KEY.into(),
            },
        )
        .map(|body| body.deserialize::<Value>().unwrap())
    }

    #[test]
    fn ipc_dispatch_serializes_bytes_rejects_invalid_payload_and_wrong_window() {
        let (_temporary, service, scope) = fixture();
        let mut context = tauri::generate_context!();
        context.config_mut().app.windows.clear();
        let app = mock_builder()
            .manage(service.clone())
            .invoke_handler(tauri::generate_handler![
                scan_start,
                scan_current,
                scan_status,
                scan_page,
                scan_cancel,
                scan_register_scope
            ])
            .build(context)
            .unwrap();
        let window = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();
        assert!(ipc(&window, "scan_start", json!({"scopeId": "/outside"})).is_err());
        assert!(ipc(&window, "scan_start", json!({"scopeId": 42})).is_err());
        let started = ipc(&window, "scan_start", json!({"scopeId": scope.id})).unwrap();
        let generation = started["generation"].as_u64().unwrap();
        assert_eq!(started["logicalBytes"], "0");
        terminal(&service);
        let page = ipc(
            &window,
            "scan_page",
            json!({"generation": generation, "offset": 0, "limit": 200}),
        )
        .unwrap();
        assert_eq!(page["progress"]["logicalBytes"], "5");
        assert_eq!(page["findings"].as_array().unwrap().len(), 2);
        let other = tauri::WebviewWindowBuilder::new(&app, "other", Default::default())
            .build()
            .unwrap();
        assert!(ipc(&other, "scan_cancel", json!({"generation": generation})).is_err());
        assert!(app.state::<Arc<ScanService>>().cancel(u32::MAX).is_err());
    }
}
