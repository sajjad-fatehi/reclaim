//! Summary-first reads over the immutable retained set of a terminal session.
use super::*;
use reclaim_core::scan::{Category, Kind, CATEGORIES, MAX_PAGE, MAX_ROOTS};
use serde::Deserialize;
use std::cmp::Reverse;
use std::collections::HashSet;

const TOP_LEVEL_LIMIT: usize = 64;
const TOP_FILES_PER_CATEGORY: usize = 25;
const ISSUE_EXAMPLES: usize = 5;
const MAX_QUERY_CHARACTERS: usize = 256;
const MAX_CURSOR_BYTES: usize = 64;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Sort {
    Size,
    Name,
    Modified,
}

impl Sort {
    fn code(self) -> &'static str {
        match self {
            Sort::Size => "size",
            Sort::Name => "name",
            Sort::Modified => "modified",
        }
    }
}

/// Retained children by parent in compact form, built once per terminal session.
pub(super) struct ChildIndex {
    roots: Vec<u32>,
    starts: Vec<u32>,
    items: Vec<u32>,
}

impl ChildIndex {
    pub(super) fn build(data: &ScanData) -> Self {
        let count = data.records.len();
        let mut starts = vec![0u32; count + 1];
        let mut roots = Vec::new();
        for (index, record) in data.records.iter().enumerate() {
            match record.parent {
                Some(parent) => starts[parent + 1] += 1,
                None => roots.push(index as u32),
            }
        }
        for position in 0..count {
            starts[position + 1] += starts[position];
        }
        let mut cursor = starts.clone();
        let mut items = vec![0u32; starts[count] as usize];
        for (index, record) in data.records.iter().enumerate() {
            if let Some(parent) = record.parent {
                items[cursor[parent] as usize] = index as u32;
                cursor[parent] += 1;
            }
        }
        Self {
            roots,
            starts,
            items,
        }
    }

    fn children(&self, parent: Option<usize>) -> &[u32] {
        match parent {
            None => &self.roots,
            Some(parent) => {
                &self.items[self.starts[parent] as usize..self.starts[parent + 1] as usize]
            }
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopFileEntry {
    pub category: Category,
    pub display_name: String,
    pub display_path: String,
    pub logical_bytes: String,
    pub allocated_bytes: String,
    pub modified_at_ms: Option<u64>,
    pub identity: String,
    pub finding_id: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueGroup {
    pub issue: Issue,
    pub count: usize,
    pub examples: Vec<Finding>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub generation: u32,
    pub progress: Progress,
    pub roots: Vec<Finding>,
    pub top_level: Vec<Finding>,
    pub top_files: Vec<TopFileEntry>,
    pub issues: Vec<IssueGroup>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FoldedRow {
    pub items: u64,
    pub logical_bytes: String,
    pub allocated_bytes: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Children {
    pub generation: u32,
    pub parent_id: Option<String>,
    pub sort: Sort,
    pub total: usize,
    pub next_cursor: Option<String>,
    pub findings: Vec<Finding>,
    pub folded: Option<FoldedRow>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Search {
    pub generation: u32,
    pub total: usize,
    pub findings: Vec<Finding>,
}

fn invalid() -> BoundaryError {
    ErrorCode::InvalidRequest.into()
}

fn validate_limit(limit: usize) -> BoundaryResult<usize> {
    if limit == 0 || limit > MAX_PAGE {
        return Err(invalid());
    }
    Ok(limit)
}

fn sorted(data: &ScanData, ids: &[u32], sort: Sort) -> Vec<u32> {
    let mut ordered = ids.to_vec();
    let finding = |id: &u32| &data.records[*id as usize].finding;
    match sort {
        Sort::Size => {
            ordered.sort_unstable_by_key(|id| (Reverse(finding(id).allocated_bytes), *id))
        }
        Sort::Modified => {
            ordered.sort_unstable_by_key(|id| (Reverse(finding(id).modified_at_ms), *id))
        }
        Sort::Name => {
            ordered.sort_by_cached_key(|id| (finding(id).display_name.to_lowercase(), *id))
        }
    }
    ordered
}

fn cursor_prefix(generation: u32, parent: Option<usize>, sort: Sort) -> String {
    match parent {
        Some(parent) => format!("{generation}.{parent}.{}.", sort.code()),
        None => format!("{generation}.r.{}.", sort.code()),
    }
}

fn parse_cursor(
    cursor: Option<&str>,
    generation: u32,
    parent: Option<usize>,
    sort: Sort,
    total: usize,
) -> BoundaryResult<usize> {
    let Some(cursor) = cursor else {
        return Ok(0);
    };
    let offset = cursor
        .strip_prefix(cursor_prefix(generation, parent, sort).as_str())
        .filter(|_| cursor.len() <= MAX_CURSOR_BYTES)
        .and_then(canonical_number)
        .ok_or_else(invalid)?;
    let offset = usize::try_from(offset).map_err(|_| invalid())?;
    if offset == 0 || offset >= total {
        return Err(invalid());
    }
    Ok(offset)
}

fn byte_text(value: u64) -> String {
    value.to_string()
}

impl ScanService {
    /// Terminal-only access; the child index is built on first use and reused.
    fn browse<Output>(
        &self,
        generation: u32,
        read: impl FnOnce(&ScanData, &ChildIndex) -> BoundaryResult<Output>,
    ) -> BoundaryResult<Output> {
        let mut authority = lock(&self.authority);
        let session = authority.session(generation)?;
        throttle(&mut session.last_read)?;
        let data = lock(&session.data);
        if data.progress.status == Status::Running {
            return Err(ErrorCode::Busy.into());
        }
        let index = session
            .index
            .get_or_insert_with(|| ChildIndex::build(&data));
        read(&data, index)
    }

    pub fn summary(&self, generation: u32) -> BoundaryResult<Summary> {
        self.browse(generation, |data, index| {
            let roots = index.roots.iter().take(MAX_ROOTS);
            let mut top_level: Vec<u32> = index
                .roots
                .iter()
                .flat_map(|root| index.children(Some(*root as usize)).iter().copied())
                .collect();
            top_level = sorted(data, &top_level, Sort::Size);
            top_level.truncate(TOP_LEVEL_LIMIT);

            let mut taken = vec![0usize; CATEGORIES.len()];
            let selected: Vec<_> = data
                .top_files
                .iter()
                .filter(|file| {
                    let slot = CATEGORIES
                        .iter()
                        .position(|category| *category == file.category)
                        .unwrap_or(CATEGORIES.len() - 1);
                    taken[slot] += 1;
                    taken[slot] <= TOP_FILES_PER_CATEGORY
                })
                .collect();
            let wanted: HashSet<&str> =
                selected.iter().map(|file| file.identity.as_str()).collect();
            let retained: std::collections::HashMap<&str, &str> = data
                .records
                .iter()
                .filter(|record| {
                    record.finding.kind == Kind::File && record.finding.alias_of.is_none()
                })
                .filter_map(|record| {
                    let identity = record.finding.identity.as_deref()?;
                    wanted
                        .contains(identity)
                        .then_some((identity, record.finding.id.as_str()))
                })
                .collect();
            let top_files = selected
                .into_iter()
                .map(|file| TopFileEntry {
                    category: file.category,
                    display_name: scan::display(
                        file.path.file_name().unwrap_or(file.path.as_os_str()),
                    ),
                    display_path: scan::display(file.path.as_os_str()),
                    logical_bytes: byte_text(file.logical_bytes),
                    allocated_bytes: byte_text(file.allocated_bytes),
                    modified_at_ms: file.modified_at_ms,
                    identity: file.identity.clone(),
                    finding_id: retained
                        .get(file.identity.as_str())
                        .map(|id| (*id).to_owned()),
                })
                .collect();

            let mut issues: Vec<IssueGroup> = Vec::new();
            for record in &data.records {
                let Some(issue) = record.finding.issue else {
                    continue;
                };
                let position = issues.iter().position(|group| group.issue == issue);
                let group = match position {
                    Some(position) => &mut issues[position],
                    None => {
                        issues.push(IssueGroup {
                            issue,
                            count: 0,
                            examples: Vec::new(),
                        });
                        issues.last_mut().expect("group was just pushed")
                    }
                };
                group.count += 1;
                if group.examples.len() < ISSUE_EXAMPLES {
                    group.examples.push(record.finding.clone());
                }
            }

            Ok(Summary {
                generation,
                progress: data.progress.clone(),
                roots: roots
                    .map(|id| data.records[*id as usize].finding.clone())
                    .collect(),
                top_level: top_level
                    .into_iter()
                    .map(|id| data.records[id as usize].finding.clone())
                    .collect(),
                top_files,
                issues,
            })
        })
    }

    pub fn children(
        &self,
        generation: u32,
        parent_id: Option<&str>,
        cursor: Option<&str>,
        limit: usize,
        sort: Sort,
    ) -> BoundaryResult<Children> {
        let limit = validate_limit(limit)?;
        self.browse(generation, |data, index| {
            let parent = parent_id.map(|id| explore::resolve(data, id)).transpose()?;
            let ordered = sorted(data, index.children(parent), sort);
            let offset = parse_cursor(cursor, generation, parent, sort, ordered.len())?;
            let end = offset.saturating_add(limit).min(ordered.len());
            let folded = parent.map(|parent| {
                let finding = &data.records[parent].finding;
                FoldedRow {
                    items: finding.folded_items,
                    logical_bytes: byte_text(finding.folded_logical_bytes),
                    allocated_bytes: byte_text(finding.folded_allocated_bytes),
                }
            });
            Ok(Children {
                generation,
                parent_id: parent_id.map(str::to_owned),
                sort,
                total: ordered.len(),
                next_cursor: (end < ordered.len())
                    .then(|| format!("{}{end}", cursor_prefix(generation, parent, sort))),
                findings: ordered[offset..end]
                    .iter()
                    .map(|id| data.records[*id as usize].finding.clone())
                    .collect(),
                folded,
            })
        })
    }

    pub fn search(&self, generation: u32, query: &str, limit: usize) -> BoundaryResult<Search> {
        let limit = validate_limit(limit)?;
        let needle = query.trim().to_lowercase();
        if needle.is_empty() || query.chars().count() > MAX_QUERY_CHARACTERS {
            return Err(invalid());
        }
        self.browse(generation, |data, _| {
            let mut matches: Vec<u32> = data
                .records
                .iter()
                .enumerate()
                .filter(|(_, record)| {
                    record.parent.is_some()
                        && record
                            .name
                            .to_string_lossy()
                            .to_lowercase()
                            .contains(&needle)
                })
                .map(|(index, _)| index as u32)
                .collect();
            let total = matches.len();
            matches = sorted(data, &matches, Sort::Size);
            matches.truncate(limit);
            Ok(Search {
                generation,
                total,
                findings: matches
                    .into_iter()
                    .map(|id| data.records[id as usize].finding.clone())
                    .collect(),
            })
        })
    }

    pub fn findings(&self, generation: u32, ids: &[String]) -> BoundaryResult<Vec<Finding>> {
        validate_limit(ids.len())?;
        self.browse(generation, |data, _| {
            ids.iter()
                .map(|id| {
                    explore::resolve(data, id).map(|index| data.records[index].finding.clone())
                })
                .collect()
        })
    }
}

#[tauri::command]
pub async fn scan_summary<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, Arc<ScanService>>,
    generation: u32,
) -> BoundaryResult<Summary> {
    authorize(&window)?;
    state.summary(generation)
}

#[tauri::command]
pub async fn scan_children<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, Arc<ScanService>>,
    generation: u32,
    parent_id: Option<String>,
    cursor: Option<String>,
    limit: usize,
    sort: Sort,
) -> BoundaryResult<Children> {
    authorize(&window)?;
    state.children(
        generation,
        parent_id.as_deref(),
        cursor.as_deref(),
        limit,
        sort,
    )
}

#[tauri::command]
pub async fn scan_search<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, Arc<ScanService>>,
    generation: u32,
    query: String,
    limit: usize,
) -> BoundaryResult<Search> {
    authorize(&window)?;
    state.search(generation, &query, limit)
}

#[tauri::command]
pub async fn scan_findings<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, Arc<ScanService>>,
    generation: u32,
    ids: Vec<String>,
) -> BoundaryResult<Vec<Finding>> {
    authorize(&window)?;
    state.findings(generation, &ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn terminal_service(service: ScanService, root: &std::path::Path) -> (Arc<ScanService>, u32) {
        let service = Arc::new(service);
        let location = service.register_native_location(root).unwrap();
        let scope = lock(&service.authority).register(&[location.id]).unwrap();
        let generation = service.start(&scope.id).unwrap().generation;
        super::super::tests::terminal(&service);
        (service, generation)
    }

    fn reset_throttle(service: &ScanService) {
        lock(&service.authority).session.as_mut().unwrap().last_read = None;
    }

    fn names(children: &Children) -> Vec<&str> {
        children
            .findings
            .iter()
            .map(|finding| finding.display_name.as_str())
            .collect()
    }

    fn sorted_fixture() -> (tempfile::TempDir, Arc<ScanService>, u32, String) {
        let temporary = tempfile::tempdir().unwrap();
        fs::write(temporary.path().join("Bravo.txt"), vec![0u8; 3000]).unwrap();
        fs::write(temporary.path().join("alpha.txt"), vec![0u8; 100]).unwrap();
        fs::write(temporary.path().join("charlie.txt"), vec![0u8; 9000]).unwrap();
        fs::write(temporary.path().join("delta.txt"), vec![0u8; 9000]).unwrap();
        let (service, generation) = terminal_service(ScanService::exhaustive(), temporary.path());
        reset_throttle(&service);
        let root = service
            .children(generation, None, None, 1, Sort::Size)
            .unwrap()
            .findings[0]
            .id
            .clone();
        (temporary, service, generation, root)
    }

    #[test]
    fn children_paging_is_deterministic_for_each_sort() {
        let (_temporary, service, generation, root) = sorted_fixture();
        for sort in [Sort::Size, Sort::Name, Sort::Modified] {
            let mut collected = Vec::new();
            let mut cursor: Option<String> = None;
            loop {
                reset_throttle(&service);
                let page = service
                    .children(generation, Some(&root), cursor.as_deref(), 3, sort)
                    .unwrap();
                assert_eq!(page.total, 4);
                assert_eq!(page.sort, sort);
                collected.extend(page.findings.iter().map(|finding| finding.id.clone()));
                cursor = page.next_cursor;
                if cursor.is_none() {
                    break;
                }
            }
            reset_throttle(&service);
            let whole = service
                .children(generation, Some(&root), None, 200, sort)
                .unwrap();
            let whole_ids: Vec<_> = whole
                .findings
                .iter()
                .map(|finding| finding.id.clone())
                .collect();
            assert_eq!(
                collected, whole_ids,
                "{sort:?} paging matches a single read"
            );
            reset_throttle(&service);
            let again = service
                .children(generation, Some(&root), None, 200, sort)
                .unwrap();
            assert_eq!(names(&again), names(&whole));
        }
        reset_throttle(&service);
        let by_size = service
            .children(generation, Some(&root), None, 200, Sort::Size)
            .unwrap();
        assert_eq!(names(&by_size)[..2], ["charlie.txt", "delta.txt"]);
        reset_throttle(&service);
        let by_name = service
            .children(generation, Some(&root), None, 200, Sort::Name)
            .unwrap();
        assert_eq!(
            names(&by_name),
            ["alpha.txt", "Bravo.txt", "charlie.txt", "delta.txt"]
        );
    }

    #[test]
    fn forged_ids_cursors_stale_generations_and_limits_are_rejected() {
        let (_temporary, service, generation, root) = sorted_fixture();
        let code = |result: BoundaryResult<Children>| result.err().unwrap().code;
        let mut attempts = Vec::new();
        for limit in [0usize, 201, usize::MAX] {
            reset_throttle(&service);
            attempts.push(code(service.children(
                generation,
                None,
                None,
                limit,
                Sort::Size,
            )));
        }
        for forged in [
            "/etc/passwd",
            "",
            ":",
            "1:",
            "x:1",
            &format!("{generation}:999999"),
            &format!("{generation}:+1"),
            &format!("{generation}:01"),
            &format!("{}:0", generation + 1),
            &format!("{generation}:0:0"),
            &format!("{generation}:{}", u64::MAX),
        ] {
            reset_throttle(&service);
            attempts.push(code(service.children(
                generation,
                Some(forged),
                None,
                10,
                Sort::Size,
            )));
        }
        reset_throttle(&service);
        let first = service
            .children(generation, Some(&root), None, 1, Sort::Size)
            .unwrap();
        let cursor = first.next_cursor.unwrap();
        let other_parent = format!("{generation}.r.size.1");
        for forged in [
            "garbage",
            "",
            &format!("{generation}.1.size.0"),
            &format!("{generation}.1.size.4"),
            &format!("{generation}.1.size.99"),
            &format!("{generation}.1.size.01"),
            &format!("{generation}.1.size.-1"),
            &format!("{generation}.1.name.1"),
            &format!("{}.1.size.1", generation + 1),
            other_parent.as_str(),
            &format!("{cursor}{}", "9".repeat(80)),
        ] {
            reset_throttle(&service);
            attempts.push(code(service.children(
                generation,
                Some(&root),
                Some(forged),
                10,
                Sort::Size,
            )));
        }
        assert!(
            attempts
                .iter()
                .all(|code| *code == ErrorCode::InvalidRequest),
            "{attempts:?}"
        );
        reset_throttle(&service);
        assert_eq!(
            service
                .children(generation, Some(&root), Some(&cursor), 10, Sort::Name)
                .err()
                .unwrap()
                .code,
            ErrorCode::InvalidRequest,
            "cursor is bound to its sort"
        );
        reset_throttle(&service);
        assert!(service
            .children(generation, Some(&root), Some(&cursor), 10, Sort::Size)
            .is_ok());
        reset_throttle(&service);
        assert_eq!(
            service
                .children(generation + 1, None, None, 10, Sort::Size)
                .err()
                .unwrap()
                .code,
            ErrorCode::StaleGeneration
        );
        reset_throttle(&service);
        assert_eq!(
            service.summary(generation + 1).err().unwrap().code,
            ErrorCode::StaleGeneration
        );
        reset_throttle(&service);
        assert_eq!(
            service.search(generation + 1, "a", 5).err().unwrap().code,
            ErrorCode::StaleGeneration
        );
        reset_throttle(&service);
        assert_eq!(
            service
                .findings(generation + 1, std::slice::from_ref(&root))
                .err()
                .unwrap()
                .code,
            ErrorCode::StaleGeneration
        );
    }

    #[test]
    fn search_and_findings_validate_input_and_report_totals() {
        let (_temporary, service, generation, root) = sorted_fixture();
        for (query, limit) in [("", 5), ("   ", 5), ("txt", 0), ("txt", 201)] {
            reset_throttle(&service);
            assert_eq!(
                service.search(generation, query, limit).err().unwrap().code,
                ErrorCode::InvalidRequest
            );
        }
        reset_throttle(&service);
        assert_eq!(
            service
                .search(generation, &"a".repeat(257), 5)
                .err()
                .unwrap()
                .code,
            ErrorCode::InvalidRequest
        );
        reset_throttle(&service);
        let found = service.search(generation, "TXT", 2).unwrap();
        assert_eq!((found.total, found.findings.len()), (4, 2));
        assert_eq!(found.findings[0].display_name, "charlie.txt");
        reset_throttle(&service);
        let root_name = service
            .findings(generation, std::slice::from_ref(&root))
            .unwrap()
            .remove(0)
            .display_name;
        reset_throttle(&service);
        let by_path = service.search(generation, &root_name, 5).unwrap();
        assert_eq!(by_path.total, 0, "root path is not a searchable name");
        reset_throttle(&service);
        assert_eq!(service.search(generation, "zzz", 5).unwrap().total, 0);

        reset_throttle(&service);
        let ids: Vec<String> = found
            .findings
            .iter()
            .map(|finding| finding.id.clone())
            .collect();
        let resolved = service.findings(generation, &ids).unwrap();
        assert_eq!(
            resolved
                .iter()
                .map(|finding| &finding.id)
                .collect::<Vec<_>>(),
            ids.iter().collect::<Vec<_>>()
        );
        for forged in [
            vec![],
            vec!["/outside".to_owned()],
            vec![ids[0].clone(), "999:1".to_owned()],
            vec![ids[0].clone(); 201],
        ] {
            reset_throttle(&service);
            assert_eq!(
                service.findings(generation, &forged).err().unwrap().code,
                ErrorCode::InvalidRequest
            );
        }
    }

    #[test]
    fn reads_wait_for_the_terminal_session_and_share_the_throttle() {
        let (_temporary, service, generation, _root) = sorted_fixture();
        assert_eq!(
            service.summary(generation).err().unwrap().code,
            ErrorCode::RateLimited
        );
        reset_throttle(&service);
        lock(&lock(&service.authority).session.as_ref().unwrap().data)
            .progress
            .status = Status::Running;
        assert_eq!(
            service.summary(generation).err().unwrap().code,
            ErrorCode::Busy
        );
    }

    #[test]
    fn summary_directly_after_status_is_not_rate_limited() {
        let (_temporary, service, generation, _root) = sorted_fixture();
        reset_throttle(&service);
        service.status(generation).unwrap();
        assert!(service.summary(generation).is_ok());
    }

    #[test]
    fn child_index_is_built_once_and_reused_across_reads() {
        let (_temporary, service, generation, _root) = sorted_fixture();
        let pointer = |service: &ScanService| {
            lock(&service.authority)
                .session
                .as_ref()
                .unwrap()
                .index
                .as_ref()
                .map(|index| index.items.as_ptr())
        };
        reset_throttle(&service);
        service.summary(generation).unwrap();
        let built = pointer(&service);
        assert!(built.is_some());
        reset_throttle(&service);
        service.summary(generation).unwrap();
        reset_throttle(&service);
        service
            .children(generation, None, None, 1, Sort::Size)
            .unwrap();
        assert_eq!(pointer(&service), built);
    }

    #[test]
    fn folded_row_and_summary_report_pruned_subtrees_honestly() {
        let temporary = tempfile::tempdir().unwrap();
        let deep = temporary.path().join("a/b/c");
        fs::create_dir_all(&deep).unwrap();
        for index in 0..10 {
            fs::write(deep.join(format!("small-{index}.bin")), b"x").unwrap();
        }
        fs::write(temporary.path().join("a/b/readme.md"), b"x").unwrap();
        let (service, generation) = terminal_service(ScanService::default(), temporary.path());
        reset_throttle(&service);
        let summary = service.summary(generation).unwrap();
        assert_eq!(summary.roots.len(), 1);
        assert_eq!(summary.top_level.len(), 1);
        assert_eq!(summary.top_level[0].display_name, "a");
        reset_throttle(&service);
        let folded = service
            .children(generation, Some(&summary.roots[0].id), None, 10, Sort::Size)
            .unwrap()
            .folded
            .unwrap();
        assert_eq!(folded.items, 0);
        let mut cursor = None;
        let mut total_folded = 0;
        let mut parent = summary.roots[0].id.clone();
        loop {
            reset_throttle(&service);
            let page = service
                .children(
                    generation,
                    Some(&parent),
                    cursor.as_deref(),
                    200,
                    Sort::Size,
                )
                .unwrap();
            total_folded += page.folded.as_ref().map_or(0, |row| row.items);
            cursor = page.next_cursor;
            match page.findings.last() {
                Some(next) if cursor.is_none() && next.kind == Kind::Directory => {
                    parent = next.id.clone();
                }
                _ => break,
            }
        }
        assert!(total_folded > 0, "some directory reports a folded row");
    }

    #[test]
    fn summary_stays_small_on_a_200k_file_fixture() {
        let temporary = tempfile::tempdir().unwrap();
        let extensions = ["txt", "png", "zip", "bin"];
        for directory in 0..200 {
            let path = temporary.path().join(format!("directory-{directory:03}"));
            fs::create_dir(&path).unwrap();
            for file in 0..1000 {
                let name = format!("file-{file:04}.{}", extensions[file % extensions.len()]);
                fs::write(path.join(name), vec![0u8; file % 7]).unwrap();
            }
        }
        let started = std::time::Instant::now();
        let (service, generation) = terminal_service(ScanService::default(), temporary.path());
        reset_throttle(&service);
        let summary = service.summary(generation).unwrap();
        let encoded = serde_json::to_vec(&summary).unwrap();
        assert!(
            encoded.len() < 100_000,
            "summary is {} bytes",
            encoded.len()
        );
        assert!(summary.top_level.len() <= TOP_LEVEL_LIMIT);
        assert_eq!(summary.progress.entries, 1 + 200 + 200_000);
        assert!(started.elapsed() < Duration::from_secs(120));
    }
}
