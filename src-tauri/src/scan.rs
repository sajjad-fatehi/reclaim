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
pub struct RecognitionPage {
    pub generation: u32,
    pub finished: bool,
    pub cancelled: bool,
    pub limited: bool,
    pub offset: usize,
    pub next_offset: Option<usize>,
    pub findings: Vec<reclaim_core::recognition::Artifact>,
    pub diagnostics: Vec<reclaim_core::recognition::Diagnostic>,
    pub diagnostics_truncated: bool,
}

struct Session {
    generation: u32,
    scope: Scope,
    data: Arc<Mutex<ScanData>>,
    cancel: Arc<AtomicBool>,
    /// Browse reads share one clock; status polling has its own so the poll
    /// that observes the terminal state never starves the summary read after it.
    last_read: Option<Instant>,
    last_status_read: Option<Instant>,
    index: Option<browse::ChildIndex>,
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
    retention: scan::RetentionPolicy,
}

impl ScanService {
    /// Retains every entry up to a large budget; for fixtures and tests that inspect small trees.
    pub fn exhaustive() -> Self {
        Self {
            authority: Mutex::default(),
            retention: scan::RetentionPolicy::exhaustive(scan::RETENTION_NODE_BUDGET),
        }
    }
}

/// Digits only, no sign or leading zeros: the exact text this service would have issued.
pub(crate) fn canonical_number(text: &str) -> Option<u64> {
    text.parse::<u64>()
        .ok()
        .filter(|number| number.to_string() == text)
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
            last_status_read: None,
            index: None,
        });
        let failure_data = data.clone();
        let retention = self.retention;
        let spawned = std::thread::Builder::new()
            .name("reclaim-scan".into())
            .spawn(move || {
                if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    scan::scan_with_policy(roots, exclusions, retention, data, cancel)
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
        throttle(&mut session.last_status_read)?;
        let progress = lock(&session.data).progress.clone();
        Ok(progress)
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

fn throttle(last_read: &mut Option<Instant>) -> BoundaryResult<()> {
    let now = Instant::now();
    if last_read.is_some_and(|last| now.duration_since(last) < READ_INTERVAL) {
        return Err(ErrorCode::RateLimited.into());
    }
    *last_read = Some(now);
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
            throttle(&mut session.last_status_read)?;
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
pub async fn scan_cancel<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, Arc<ScanService>>,
    generation: u32,
) -> BoundaryResult<Progress> {
    authorize(&window)?;
    state.cancel(generation)
}

#[tauri::command]
pub async fn recognition_page<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, Arc<ScanService>>,
    generation: u32,
    offset: usize,
    limit: usize,
) -> BoundaryResult<RecognitionPage> {
    authorize(&window)?;
    state.recognition_page(generation, offset, limit)
}

impl ScanService {
    pub fn recognition_page(
        &self,
        generation: u32,
        offset: usize,
        limit: usize,
    ) -> BoundaryResult<RecognitionPage> {
        if limit == 0 || limit > scan::MAX_PAGE {
            return Err(ErrorCode::InvalidRequest.into());
        }
        let mut authority = lock(&self.authority);
        let session = authority.session(generation)?;
        throttle(&mut session.last_read)?;
        let data = lock(&session.data);
        if data.progress.status == Status::Running {
            return Err(ErrorCode::Busy.into());
        }
        let report = &data.recognition;
        if offset > report.findings.len() {
            return Err(ErrorCode::InvalidRequest.into());
        }
        let end = offset.saturating_add(limit).min(report.findings.len());
        Ok(RecognitionPage {
            generation,
            finished: report.finished,
            cancelled: report.cancelled,
            limited: report.limited,
            offset,
            next_offset: (end < report.findings.len()).then_some(end),
            findings: report.findings[offset..end].to_vec(),
            diagnostics: if offset == 0 {
                report.diagnostics.clone()
            } else {
                Vec::new()
            },
            diagnostics_truncated: report.diagnostics_truncated,
        })
    }
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
        let service = Arc::new(ScanService::exhaustive());
        let location = service.register_native_location(temporary.path()).unwrap();
        let scope = lock(&service.authority).register(&[location.id]).unwrap();
        (temporary, service, scope)
    }

    pub(super) fn terminal(service: &ScanService) -> Progress {
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
                .children(progress.generation, None, None, 201, browse::Sort::Size)
                .err()
                .unwrap()
                .code,
            ErrorCode::InvalidRequest
        );
        let roots = service
            .children(progress.generation, None, None, 1, browse::Sort::Size)
            .unwrap();
        assert_eq!(roots.findings.len(), 1);
        assert_eq!(roots.next_cursor, None);
        assert_eq!(
            service
                .children(progress.generation, None, None, 1, browse::Sort::Size)
                .err()
                .unwrap()
                .code,
            ErrorCode::RateLimited
        );
        lock(&service.authority).session.as_mut().unwrap().last_read = None;
        let files = service
            .children(
                progress.generation,
                Some(&roots.findings[0].id),
                None,
                200,
                browse::Sort::Size,
            )
            .unwrap();
        assert_eq!(files.findings.len(), 1);
        assert_eq!(files.next_cursor, None);
        service.start(&scope.id).unwrap();
        assert_eq!(
            service.cancel(progress.generation).unwrap_err().code,
            ErrorCode::StaleGeneration
        );
        assert_eq!(
            service.summary(progress.generation).err().unwrap().code,
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
            last_status_read: None,
            index: None,
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
        let service = ScanService::exhaustive();
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
        let service = ScanService::exhaustive();
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

    #[test]
    fn exclusions_preserve_recognition_and_shared_session_authority() {
        let (temporary, service, mut scope) = fixture();
        fs::write(
            temporary.path().join("package.json"),
            r#"{"name":"synthetic-project"}"#,
        )
        .unwrap();
        fs::create_dir(temporary.path().join("node_modules")).unwrap();
        fs::write(
            temporary.path().join("node_modules/.package-lock.json"),
            r#"{"lockfileVersion":3,"packages":{}}"#,
        )
        .unwrap();
        fs::create_dir(temporary.path().join("cache")).unwrap();
        fs::write(
            temporary.path().join("cache/unvisited"),
            b"excluded fixture",
        )
        .unwrap();
        scope.exclusions = vec![Exclusion {
            relative_path: "cache".into(),
            subtree: true,
        }];
        lock(&service.authority).scope = Some(scope.clone());
        let started = service.start(&scope.id).unwrap();
        let progress = terminal(&service);
        assert_eq!(progress.status, Status::Partial);
        assert_eq!(
            progress.current_location.as_deref(),
            Some(scope.roots[0].display_path.as_str())
        );
        service
            .with_terminal_scan(started.generation, |data| {
                assert!(data.recognition.finished);
                assert!(data.records.iter().any(|record| record.name == "cache"
                    && record.finding.issue == Some(Issue::Excluded)));
                assert!(!data.records.iter().any(|record| record.name == "unvisited"));
            })
            .unwrap();
        let recognized = service
            .recognition_page(started.generation, 0, scan::MAX_PAGE)
            .unwrap();
        assert!(recognized.finished);
        assert!(!recognized.findings.is_empty());
        assert!(recognized.findings.iter().all(|artifact| artifact
            .finding_id
            .starts_with(&format!("{}:", started.generation))));
        assert_eq!(
            fs::read(temporary.path().join("cache/unvisited")).unwrap(),
            b"excluded fixture"
        );
    }

    #[test]
    fn recognition_pages_share_scan_authority_and_reject_invalid_requests() {
        let (temporary, service, scope) = fixture();
        fs::write(
            temporary.path().join("package.json"),
            r#"{"name":"synthetic-project"}"#,
        )
        .unwrap();
        let started = service.start(&scope.id).unwrap();
        terminal(&service);
        assert_eq!(
            service
                .recognition_page(started.generation, 0, 0)
                .err()
                .unwrap()
                .code,
            ErrorCode::InvalidRequest
        );
        let page = service.recognition_page(started.generation, 0, 1).unwrap();
        assert!(page.finished);
        assert!(!page.cancelled);
        assert_eq!(page.findings.len(), 1);
        assert_eq!(
            page.findings[0].cleanup_support,
            reclaim_core::recognition::CleanupSupport::InspectionOnly
        );
        assert_eq!(
            page.findings[0].owned_logical_bytes.as_ref().unwrap(),
            &terminal(&service).logical_bytes.to_string()
        );
        assert_eq!(
            service
                .children(started.generation, None, None, 1, browse::Sort::Size)
                .err()
                .unwrap()
                .code,
            ErrorCode::RateLimited
        );
        lock(&service.authority).session.as_mut().unwrap().last_read = None;
        assert_eq!(
            service
                .recognition_page(started.generation, 100, 1)
                .err()
                .unwrap()
                .code,
            ErrorCode::InvalidRequest
        );
        service.start(&scope.id).unwrap();
        assert_eq!(
            service
                .recognition_page(started.generation, 0, 1)
                .err()
                .unwrap()
                .code,
            ErrorCode::StaleGeneration
        );
        terminal(&service);
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
        let mut context = crate::test_context();
        context.config_mut().app.windows.clear();
        let app = mock_builder()
            .manage(service.clone())
            .invoke_handler(tauri::generate_handler![
                scan_start,
                scan_current,
                scan_status,
                browse::scan_summary,
                browse::scan_children,
                browse::scan_search,
                browse::scan_findings,
                scan_cancel,
                scan_register_scope,
                recognition_page,
                explore::scan_inspect,
                explore::scan_reveal
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
        let summary = ipc(&window, "scan_summary", json!({"generation": generation})).unwrap();
        assert_eq!(summary["progress"]["logicalBytes"], "5");
        assert_eq!(summary["roots"].as_array().unwrap().len(), 1);
        let root_id = summary["roots"][0]["id"].as_str().unwrap().to_owned();
        let read = |service: &ScanService| {
            lock(&service.authority).session.as_mut().unwrap().last_read = None
        };
        read(&service);
        let children = ipc(
            &window,
            "scan_children",
            json!({"generation": generation, "parentId": root_id, "cursor": null, "limit": 200, "sort": "size"}),
        )
        .unwrap();
        assert_eq!(children["findings"].as_array().unwrap().len(), 1);
        assert_eq!(children["total"], 1);
        for forged in [
            json!({"generation": generation, "parentId": "/outside", "cursor": null, "limit": 10, "sort": "size"}),
            json!({"generation": generation, "parentId": 7, "cursor": null, "limit": 10, "sort": "size"}),
            json!({"generation": generation, "parentId": root_id, "cursor": "forged", "limit": 10, "sort": "size"}),
            json!({"generation": generation, "parentId": root_id, "cursor": null, "limit": 201, "sort": "size"}),
            json!({"generation": generation, "parentId": root_id, "cursor": null, "limit": 10, "sort": "path"}),
            json!({"generation": generation + 1, "parentId": null, "cursor": null, "limit": 10, "sort": "size"}),
        ] {
            read(&service);
            assert!(ipc(&window, "scan_children", forged).is_err());
        }
        read(&service);
        let stale = ipc(
            &window,
            "scan_summary",
            json!({"generation": generation + 1}),
        )
        .unwrap_err();
        assert_eq!(stale["code"], "stale_generation");
        read(&service);
        let found = ipc(
            &window,
            "scan_search",
            json!({"generation": generation, "query": "fixture", "limit": 10}),
        )
        .unwrap();
        assert_eq!(found["total"], 1);
        read(&service);
        assert!(ipc(
            &window,
            "scan_findings",
            json!({"generation": generation, "ids": ["/outside"]})
        )
        .is_err());
        read(&service);
        let resolved = ipc(
            &window,
            "scan_findings",
            json!({"generation": generation, "ids": [root_id]}),
        )
        .unwrap();
        assert_eq!(resolved.as_array().unwrap().len(), 1);
        read(&service);
        assert_eq!(
            ipc(&window, "scan_summary", json!({"generation": generation})).unwrap()["progress"]
                ["retention"]["budgetReached"],
            false
        );
        let other = tauri::WebviewWindowBuilder::new(&app, "other", Default::default())
            .build()
            .unwrap();
        read(&service);
        assert!(ipc(&other, "scan_summary", json!({"generation": generation})).is_err());
        assert!(ipc(
            &other,
            "scan_search",
            json!({"generation": generation, "query": "a", "limit": 1})
        )
        .is_err());
        assert!(ipc(&other, "scan_cancel", json!({"generation": generation})).is_err());
        assert!(ipc(
            &other,
            "recognition_page",
            json!({"generation": generation, "offset": 0, "limit": 200})
        )
        .is_err());
        lock(&service.authority).session.as_mut().unwrap().last_read = None;
        let recognized = ipc(
            &window,
            "recognition_page",
            json!({"generation": generation, "offset": 0, "limit": 200}),
        )
        .unwrap();
        assert_eq!(recognized["finished"], true);
        assert_eq!(recognized["findings"], json!([]));

        assert!(ipc(
            &other,
            "scan_inspect",
            json!({"generation": generation, "findingId": format!("{generation}:0")})
        )
        .is_err());
        assert!(ipc(
            &other,
            "scan_reveal",
            json!({"generation": generation, "findingId": format!("{generation}:0")})
        )
        .is_err());
        assert!(ipc(
            &window,
            "scan_reveal",
            json!({"generation": generation, "findingId": "/outside/forged"})
        )
        .is_err());
        assert!(ipc(
            &window,
            "scan_reveal",
            json!({"generation": generation, "findingId": 42})
        )
        .is_err());
        let inspected = ipc(
            &window,
            "scan_inspect",
            json!({"generation": generation, "findingId": format!("{generation}:0")}),
        )
        .unwrap();
        assert!(inspected["displayPath"].as_str().unwrap().starts_with('/'));
        assert!(app.state::<Arc<ScanService>>().cancel(u32::MAX).is_err());
    }
}

pub mod browse;
pub(crate) mod cleanup;
pub mod explore;
mod fingerprint;

#[cfg(test)]
pub(crate) mod cleanup_review_tests {
    use super::*;
    use crate::cleanup::{CleanupService, Selection};
    use crate::docker::DockerService;
    use std::fs;
    use std::os::unix::fs::symlink;

    pub(crate) fn rescan(scanner: &ScanService) -> u32 {
        let scope = lock(&scanner.authority).scope.as_ref().unwrap().id.clone();
        let generation = scanner.start(&scope).unwrap().generation;
        super::tests::terminal(scanner);
        generation
    }

    pub(crate) fn fixture() -> (tempfile::TempDir, Arc<ScanService>, u32) {
        let temporary = tempfile::tempdir().unwrap();
        fs::create_dir(temporary.path().join("node_modules")).unwrap();
        fs::write(
            temporary.path().join("package.json"),
            br#"{"name":"SYNTHETIC"}"#,
        )
        .unwrap();
        fs::write(
            temporary.path().join("node_modules/.package-lock.json"),
            br#"{"lockfileVersion":3,"packages":{}}"#,
        )
        .unwrap();
        fs::write(temporary.path().join("source.rs"), b"source sentinel").unwrap();
        fs::write(temporary.path().join(".env"), b"secret sentinel").unwrap();
        fs::create_dir(temporary.path().join("target")).unwrap();
        fs::write(
            temporary.path().join("Cargo.toml"),
            "[package]\nname = \"synthetic\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        fs::write(
            temporary.path().join("target/.rustc_info.json"),
            br#"{"rustc_fingerprint":123}"#,
        )
        .unwrap();
        let service = Arc::new(ScanService::exhaustive());
        let location = service.register_native_location(temporary.path()).unwrap();
        let scope = lock(&service.authority).register(&[location.id]).unwrap();
        let generation = service.start(&scope.id).unwrap().generation;
        super::tests::terminal(&service);
        (temporary, service, generation)
    }

    pub(crate) fn select(scanner: &ScanService, generation: u32, name: &str) -> Selection {
        let finding_id = scanner
            .with_terminal_scan(generation, |data| {
                data.records
                    .iter()
                    .find(|record| record.name == name)
                    .unwrap()
                    .finding
                    .id
                    .clone()
            })
            .unwrap();
        Selection::Filesystem {
            generation,
            finding_id,
        }
    }

    const DEEP: &str = "node_modules/a/b/c/d/e";

    /// Default retention folds `DEEP`: no record below depth 2 survives, only totals do.
    fn folded_fixture(extra_files: usize) -> (tempfile::TempDir, Arc<ScanService>, u32, String) {
        let temporary = tempfile::tempdir().unwrap();
        let modules = temporary.path().join("node_modules");
        let deep = temporary.path().join(DEEP);
        fs::create_dir_all(&deep).unwrap();
        fs::write(
            temporary.path().join("package.json"),
            br#"{"name":"SYNTHETIC"}"#,
        )
        .unwrap();
        fs::write(
            modules.join(".package-lock.json"),
            br#"{"lockfileVersion":3,"packages":{}}"#,
        )
        .unwrap();
        fs::write(deep.join("one.js"), b"one").unwrap();
        fs::write(deep.join("two.js"), b"second").unwrap();
        fs::create_dir(deep.join("leaf")).unwrap();
        fs::write(deep.join("leaf/three.js"), b"3").unwrap();
        for chunk in 0..extra_files.div_ceil(1_000) {
            let directory = modules.join(format!("bulk{chunk}"));
            fs::create_dir(&directory).unwrap();
            for file in 0..1_000.min(extra_files - chunk * 1_000) {
                fs::write(directory.join(file.to_string()), b"").unwrap();
            }
        }
        let service = Arc::new(ScanService::default());
        let location = service.register_native_location(temporary.path()).unwrap();
        let scope = lock(&service.authority).register(&[location.id]).unwrap();
        let generation = service.start(&scope.id).unwrap().generation;
        super::tests::terminal(&service);
        let finding_id = service
            .with_terminal_scan(generation, |data| {
                assert!(data.records.iter().all(|record| record.name != "one.js"));
                let record = data
                    .records
                    .iter()
                    .find(|record| record.name == "node_modules")
                    .unwrap();
                assert!(data
                    .records
                    .iter()
                    .any(|record| record.finding.detail == scan::Detail::Summarized));
                record.finding.id.clone()
            })
            .unwrap();
        (temporary, service, generation, finding_id)
    }

    #[test]
    fn summarized_target_is_eligible_and_its_plan_revalidates_until_anything_inside_changes() {
        use std::os::unix::fs::MetadataExt;
        type Mutation = Box<dyn Fn(&std::path::Path)>;
        let mutations: Vec<(&str, Mutation)> = vec![
            (
                "add file",
                Box::new(|deep| fs::write(deep.join("new.js"), b"n").unwrap()),
            ),
            (
                "add directory",
                Box::new(|deep| fs::create_dir(deep.join("new")).unwrap()),
            ),
            (
                "remove file",
                Box::new(|deep| fs::remove_file(deep.join("two.js")).unwrap()),
            ),
            (
                "remove directory",
                Box::new(|deep| fs::remove_dir_all(deep.join("leaf")).unwrap()),
            ),
            (
                "rename file",
                Box::new(|deep| fs::rename(deep.join("one.js"), deep.join("uno.js")).unwrap()),
            ),
            (
                "rename directory",
                Box::new(|deep| fs::rename(deep.join("leaf"), deep.join("leaf2")).unwrap()),
            ),
            (
                "replace inode, same name size and mtime",
                Box::new(|deep| {
                    let path = deep.join("one.js");
                    let modified = fs::metadata(&path).unwrap().modified().unwrap();
                    let inode = fs::metadata(&path).unwrap().ino();
                    fs::remove_file(&path).unwrap();
                    let mut tries = 0;
                    loop {
                        fs::write(&path, b"one").unwrap();
                        fs::File::options()
                            .write(true)
                            .open(&path)
                            .unwrap()
                            .set_modified(modified)
                            .unwrap();
                        if fs::metadata(&path).unwrap().ino() != inode || tries > 50 {
                            break;
                        }
                        fs::remove_file(&path).unwrap();
                        tries += 1;
                    }
                }),
            ),
            (
                "change mtime only",
                Box::new(|deep| {
                    let file = fs::File::options()
                        .write(true)
                        .open(deep.join("one.js"))
                        .unwrap();
                    file.set_modified(
                        std::time::SystemTime::now() + std::time::Duration::from_secs(3600),
                    )
                    .unwrap();
                }),
            ),
            (
                "change size",
                Box::new(|deep| fs::write(deep.join("two.js"), b"second!!").unwrap()),
            ),
            (
                "change deep nested file",
                Box::new(|deep| fs::write(deep.join("leaf/three.js"), b"33").unwrap()),
            ),
            (
                "replace file with symlink",
                Box::new(|deep| {
                    fs::remove_file(deep.join("one.js")).unwrap();
                    symlink("two.js", deep.join("one.js")).unwrap();
                }),
            ),
        ];
        for (label, mutate) in mutations {
            let (temporary, service, generation, finding_id) = folded_fixture(0);
            let planned = service.cleanup_target(generation, &finding_id).unwrap();
            assert!(
                service.cleanup_target(generation, &finding_id).unwrap() == planned,
                "unchanged tree must revalidate: {label}"
            );
            mutate(&temporary.path().join(DEEP));
            let current = service.cleanup_target(generation, &finding_id);
            assert!(
                current.is_err() || current.is_ok_and(|current| current != planned),
                "plan survived: {label}"
            );
        }
    }

    #[test]
    fn target_with_more_than_250k_entries_is_eligible_and_revalidates() {
        let (_temporary, service, generation, finding_id) = folded_fixture(250_000);
        let planned = service.cleanup_target(generation, &finding_id).unwrap();
        assert!(planned.fingerprint.count > 250_000);
        assert!(service.cleanup_target(generation, &finding_id).unwrap() == planned);
    }

    #[test]
    fn walk_issues_or_git_inside_a_trash_target_make_it_ineligible() {
        use std::os::unix::fs::PermissionsExt;
        let (temporary, service, generation, finding_id) = folded_fixture(0);
        let deep = temporary.path().join(DEEP);
        fs::create_dir(deep.join(".git")).unwrap();
        assert!(service.cleanup_target(generation, &finding_id).is_err());
        fs::remove_dir(deep.join(".git")).unwrap();
        fs::set_permissions(deep.join("leaf"), fs::Permissions::from_mode(0o000)).unwrap();
        let denied = service.cleanup_target(generation, &finding_id);
        fs::set_permissions(deep.join("leaf"), fs::Permissions::from_mode(0o755)).unwrap();
        assert!(denied.is_err());
        let (temporary, service, generation, finding_id) = folded_fixture(0);
        let deep = temporary.path().join(DEEP);
        fs::hard_link(deep.join("one.js"), deep.join("leaf/linked.js")).unwrap();
        assert!(service.cleanup_target(generation, &finding_id).is_err());
    }

    #[test]
    fn cleanup_review_fails_closed_for_recognized_source_secret_and_overlap() {
        let (temporary, scanner, generation) = fixture();
        let cleanup = CleanupService::default();
        let docker = DockerService::default();
        let dependency = select(&scanner, generation, "node_modules");
        let report = cleanup
            .review(
                &scanner,
                &docker,
                vec![
                    dependency.clone(),
                    dependency,
                    select(&scanner, generation, "source.rs"),
                    select(&scanner, generation, ".env"),
                    select(&scanner, generation, ".package-lock.json"),
                ],
            )
            .unwrap();
        assert_eq!(report.items.len(), 4);
        assert_eq!(
            report.items[0].method,
            Some(reclaim_core::cleanup::Method::Trash)
        );
        assert!(report.plan_id.is_some());
        assert!(report
            .items
            .iter()
            .all(|item| !item.unavailable_reason.is_empty()));
        assert!(
            report.items[3].artifact.is_none(),
            "descendants must not inherit eligibility"
        );
        assert_eq!(
            fs::read(temporary.path().join("source.rs")).unwrap(),
            b"source sentinel"
        );
        assert_eq!(
            fs::read(temporary.path().join(".env")).unwrap(),
            b"secret sentinel"
        );
        assert!(temporary
            .path()
            .join("node_modules/.package-lock.json")
            .exists());
        let mut artifact = report.items[0].artifact.clone().unwrap();
        use reclaim_core::recognition::{ChangeState, Worktree};
        for changes in [
            ChangeState::Dirty,
            ChangeState::Untracked,
            ChangeState::Ignored,
            ChangeState::Unknown,
            ChangeState::Clean,
        ] {
            for (main, locked) in [(true, false), (false, true), (false, false)] {
                artifact.worktree = Some(Worktree {
                    repository_id: None,
                    metadata_id: None,
                    branch: None,
                    main,
                    detached: false,
                    locked,
                    changes,
                });
                let reason = reclaim_core::cleanup::filesystem_unavailability(Some(&artifact));
                assert!(!reason.is_empty());
                if locked && !main {
                    assert!(reason.contains("locked"));
                }
                if !main && !locked && changes != ChangeState::Clean {
                    assert!(reason.contains("changes"));
                }
            }
        }
    }

    #[test]
    fn cleanup_review_rejects_forged_stale_unobserved_and_oversized_requests() {
        let (_temporary, scanner, generation) = fixture();
        let cleanup = CleanupService::default();
        let docker = DockerService::default();
        for finding_id in ["/outside/sentinel", "../source", "1:999999"] {
            assert!(cleanup
                .review(
                    &scanner,
                    &docker,
                    vec![Selection::Filesystem {
                        generation,
                        finding_id: finding_id.into()
                    }]
                )
                .is_err());
        }
        assert!(cleanup
            .review(
                &scanner,
                &docker,
                vec![Selection::Docker {
                    finding_id: format!("docker:Image:sha256:{}", "a".repeat(64))
                }]
            )
            .is_err());
        assert!(cleanup
            .review(
                &scanner,
                &docker,
                vec![Selection::Filesystem {
                    generation: generation + 1,
                    finding_id: format!("{generation}:0")
                }]
            )
            .is_err());
        assert!(cleanup.review(&scanner, &docker, Vec::new()).is_err());
        assert!(cleanup
            .review(
                &scanner,
                &docker,
                vec![select(&scanner, generation, "node_modules"); 101]
            )
            .is_err());
        assert!(serde_json::from_value::<Selection>(serde_json::json!({"source":"filesystem","generation":generation,"findingId":"1:0","path":"/outside","force":true})).is_err());
    }

    #[test]
    fn cleanup_review_detects_target_and_ancestor_substitution_without_touching_sentinels() {
        let (temporary, scanner, generation) = fixture();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("sentinel"), b"outside").unwrap();
        let cleanup = CleanupService::default();
        let docker = DockerService::default();
        let target = select(&scanner, generation, "node_modules");
        let child = select(&scanner, generation, ".package-lock.json");
        fs::rename(
            temporary.path().join("node_modules"),
            temporary.path().join("original"),
        )
        .unwrap();
        symlink(outside.path(), temporary.path().join("node_modules")).unwrap();
        for selection in [target, child] {
            let report = cleanup.review(&scanner, &docker, vec![selection]).unwrap();
            assert!(report.items[0].changed);
        }
        assert_eq!(
            fs::read(outside.path().join("sentinel")).unwrap(),
            b"outside"
        );
        fs::remove_file(temporary.path().join("node_modules")).unwrap();
        fs::create_dir(temporary.path().join("node_modules")).unwrap();
        let report = cleanup
            .review(
                &scanner,
                &docker,
                vec![select(&scanner, generation, "node_modules")],
            )
            .unwrap();
        assert!(report.items[0].changed);
        assert!(temporary
            .path()
            .join("original/.package-lock.json")
            .exists());
    }
}
