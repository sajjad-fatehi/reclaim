/* global window, document, setTimeout, location */
(async () => {
  const invoke = window.__TAURI_INTERNALS__.invoke;
  const pause = () => new Promise((resolve) => setTimeout(resolve, 120));
  const check = (condition, message) => {
    if (!condition) throw new Error(message);
  };
  document.body.textContent = 'SYNTHETIC FIXTURE — native scan IPC running…';
  try {
    const scope = await invoke('scan_register_scope', {
      locationIds: ['SMALL_LOCATION_ID'],
    });
    const started = await invoke('scan_start', { scopeId: scope.id });
    let complete;
    do {
      await pause();
      complete = await invoke('scan_status', {
        generation: started.generation,
      });
    } while (complete.status === 'running');
    check(
      complete.status === 'complete' && complete.logicalBytes === '5',
      'small scan accounting',
    );
    await pause();
    const page = await invoke('scan_page', {
      generation: started.generation,
      offset: 0,
      limit: 200,
    });
    check(page.findings.length === 2, 'small page hierarchy');
    const largeScope = await invoke('scan_register_scope', {
      locationIds: ['LARGE_LOCATION_ID'],
    });
    const running = await invoke('scan_start', { scopeId: largeScope.id });
    const progress = await invoke('scan_status', {
      generation: running.generation,
    });
    check(
      progress.status === 'running',
      'observe live progress before cancellation',
    );
    await invoke('scan_cancel', { generation: running.generation });
    let cancelled;
    do {
      await pause();
      cancelled = await invoke('scan_status', {
        generation: running.generation,
      });
    } while (cancelled.status === 'running');
    check(cancelled.status === 'cancelled', 'cooperative cancellation');
    await pause();
    const partial = await invoke('scan_page', {
      generation: running.generation,
      offset: 0,
      limit: 200,
    });
    check(
      partial.findings.length > 0 && partial.findings.length <= 200,
      'bounded retained findings',
    );
    let staleRejected = false;
    try {
      await invoke('scan_cancel', { generation: started.generation });
    } catch (error) {
      staleRejected = error.code === 'stale_generation';
    }
    check(staleRejected, 'stale generation rejected');
    const summary = `SYNTHETIC FIXTURE — PASS\n\nNative macOS WebView → Rust IPC\n\nSmall scan: 5 logical bytes, 2 findings, complete\nLarge fixture: 20,000 files\nLive progress: ${progress.entries} entries\nCancellation: ${cancelled.status}, ${cancelled.entries} retained entries\nPage: ${partial.findings.length} findings (maximum 200)\nStale generation: rejected\n\nRead-only metadata. No user files scanned.`;
    document.body.textContent = summary;
    document.body.style.whiteSpace = 'pre-wrap';
    document.body.style.padding = '32px';
    location.href = `/scan-smoke-pass?smallBytes=5&liveEntries=${progress.entries}&cancelledEntries=${cancelled.entries}&page=${partial.findings.length}&stale=rejected`;
  } catch (error) {
    document.body.textContent = `SYNTHETIC FIXTURE — FAIL\n${JSON.stringify(error)}`;
    location.href = `/scan-smoke-fail?error=${encodeURIComponent(error.message ?? JSON.stringify(error))}`;
  }
})();
