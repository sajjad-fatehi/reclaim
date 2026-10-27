/* global window, document, setTimeout, location, Event, HTMLInputElement */
window.addEventListener('DOMContentLoaded', () => {
  const pause = () => new Promise((resolve) => setTimeout(resolve, 120));
  const visible = (element) => element.getClientRects().length > 0;
  const button = (name) => [...document.querySelectorAll('button')].find((element) => visible(element) && (element.getAttribute('aria-label') === name || element.textContent.trim() === name));
  const wait = async (read, message) => {
    const deadline = Date.now() + 20000;
    while (Date.now() < deadline) { const result = read(); if (result) return result; await pause(); }
    throw new Error(message);
  };
  const click = async (name) => { const element = await wait(() => button(name), `Missing ${name}`); if (element.disabled) throw new Error(`Disabled ${name}`); element.click(); };
  (async () => {
    try {
      await click('Start scan');
      await wait(() => button('Refresh scan') && !button('Refresh scan').disabled, 'Scan did not settle');
      await click('Explore');
      const search = await wait(() => [...document.querySelectorAll('input[type="search"]')].find(visible), 'Missing Explore search');
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(search, 'node_modules');
      search.dispatchEvent(new Event('input', { bubbles: true }));
      await click('Inspect node_modules');
      await click('Add to review');
      await click('Review & Clean');
      await click('Check eligibility');
      await click('Review confirmation');
      if (document.activeElement.textContent.trim() !== 'Cancel') throw new Error('Confirmation focus not Cancel');
      await click('Cancel');
      if (await window.__TAURI_INTERNALS__.invoke('cleanup_status') !== null) throw new Error('Cancel dispatched work');
      await click('Review confirmation');
      await click('Confirm selected removal');
      await wait(() => [...document.querySelectorAll('h2')].some((heading) => visible(heading) && heading.textContent === 'Cleanup outcomes'), 'No completed outcomes');
      const history = await window.__TAURI_INTERNALS__.invoke('cleanup_history', { restart: true });
      const operation = history.history?.operations[0];
      if (!operation || operation.items.length !== 1 || operation.items[0].outcome !== 'removed' || operation.items[0].method !== 'trash' || operation.finishedAtMs === null) throw new Error('Restarted history did not retain actual Trash outcome');
      location.href = '/cleanup-smoke-pass';
    } catch (error) {
      location.href = '/cleanup-smoke-fail?reason=' + encodeURIComponent(String(error));
    }
  })();
});
