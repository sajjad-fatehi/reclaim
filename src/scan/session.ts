import { errorCode } from './contract';
import { readAppearance } from '../shell/appearance';
import type {
  Finding,
  Location,
  Preferences,
  Progress,
  ScanApi,
  Scope,
  Summary,
} from './contract';

export interface ScanSnapshot {
  scope: Scope;
  progress: Progress;
  /** Bounded summary findings: roots, top level and issue examples. */
  findings: Finding[];
  summary: Summary | null;
  settled: boolean;
}
export interface Investigation {
  selectedId?: string | null;
  generation: number;
  category: string;
  findingIds: string[];
}
export interface ScanState {
  preferences: Preferences;
  candidates: Location[];
  unavailableCount: number;
  loading: boolean;
  busy: boolean;
  saving: boolean;
  cancelling: boolean;
  dirty: boolean;
  stale: boolean;
  error: string | null;
  snapshot: ScanSnapshot | null;
  previous: ScanSnapshot | null;
  investigation: Investigation | null;
}
export const emptyPreferences: Preferences = {
  appearance: 'system',
  roots: [],
  exclusions: [],
  configured: false,
  corrupt: false,
};

export function validExclusion(path: string): boolean {
  return (
    path.length > 0 &&
    path.length <= 1024 &&
    !Array.from(path).some(
      (character) =>
        character.charCodeAt(0) < 32 || character.charCodeAt(0) === 127,
    ) &&
    path
      .split('/')
      .every((part) => part !== '' && part !== '.' && part !== '..')
  );
}

function summaryFindings(summary: Summary): Finding[] {
  const unique = new Map<string, Finding>();
  for (const finding of [
    ...summary.roots,
    ...summary.topLevel,
    ...summary.issues.flatMap((group) => group.examples),
  ])
    unique.set(finding.id, finding);
  return [...unique.values()];
}

export class ScanSession {
  private state: ScanState = {
    preferences: emptyPreferences,
    candidates: [],
    unavailableCount: 0,
    loading: true,
    busy: false,
    saving: false,
    cancelling: false,
    dirty: false,
    stale: false,
    error: null,
    snapshot: null,
    previous: null,
    investigation: null,
  };
  private listeners = new Set<() => void>();
  private timer: ReturnType<typeof setTimeout> | undefined;
  private initialized: Promise<void> | null = null;
  private active = false;
  private pendingWrite: Promise<void> = Promise.resolve();
  private revision = 0;
  private polling = false;

  constructor(private readonly api: ScanApi) {}

  getSnapshot = (): ScanState => this.state;
  subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };
  private update(edits: Partial<ScanState>): void {
    this.state = { ...this.state, ...edits };
    this.listeners.forEach((listener) => listener());
  }
  connect = (): (() => void) => {
    this.active = true;
    this.initialized ??= this.initialize();
    void this.initialized.then(() => this.schedule());
    return () => {
      this.active = false;
      clearTimeout(this.timer);
    };
  };
  private async initialize(): Promise<void> {
    try {
      const preferences = await this.api.loadPreferences();
      const { candidates, unavailableCount } = await this.api.locations();
      const selected = preferences.configured
        ? preferences.roots
        : candidates.map((location) => ({
            key: null,
            displayPath: location.displayPath,
            location,
          }));
      this.update({
        preferences: {
          ...preferences,
          roots: selected,
          appearance: preferences.configured
            ? preferences.appearance
            : readAppearance(),
        },
        candidates,
        unavailableCount,
        loading: false,
      });
      const current = await this.api.current();
      if (current.scope && current.progress) {
        this.update({
          snapshot: {
            scope: current.scope,
            progress: current.progress,
            findings: [],
            summary: null,
            settled: false,
          },
        });
      }
    } catch {
      this.update({
        loading: false,
        error:
          'Settings or desktop scan connection unavailable. Reopen Reclaim to retry. No scan started.',
      });
    }
  }
  editPreferences = (edits: Partial<Preferences>): void => {
    this.revision += 1;
    this.update({
      preferences: { ...this.state.preferences, ...edits },
      dirty: true,
      stale:
        this.state.stale ||
        (this.state.snapshot !== null &&
          (edits.roots !== undefined || edits.exclusions !== undefined)),
      error: null,
    });
  };
  pickFolder = async (): Promise<void> => {
    if (this.state.busy || this.state.loading) return;
    this.update({ busy: true, error: null });
    try {
      const location = await this.api.pickFolder();
      if (location) {
        const roots = this.state.preferences.roots.filter(
          (root) => root.displayPath !== location.displayPath,
        );
        this.editPreferences({
          roots: [
            ...roots,
            { key: null, displayPath: location.displayPath, location },
          ],
        });
        this.update({
          candidates: [
            ...this.state.candidates.filter(
              (candidate) => candidate.displayPath !== location.displayPath,
            ),
            location,
          ],
        });
      }
    } catch {
      this.update({
        error:
          'Folder unavailable. Choose a readable folder again; access is never bypassed.',
      });
    } finally {
      this.update({ busy: false });
    }
  };
  save = async (): Promise<boolean> => {
    const preferences = this.state.preferences;
    const revision = this.revision;
    this.update({ saving: true, error: null });
    const write = this.pendingWrite.then(() =>
      this.api.savePreferences(preferences),
    );
    this.pendingWrite = write.catch(() => undefined);
    try {
      await write;
      if (revision === this.revision)
        this.update({
          preferences: { ...preferences, configured: true, corrupt: false },
          dirty: false,
        });
      return true;
    } catch {
      this.update({
        error:
          'Settings could not be saved. Current choices remain in this session. Check local storage and try Save settings again.',
      });
      return false;
    } finally {
      this.update({ saving: false });
    }
  };
  start = async (): Promise<void> => {
    if (
      this.state.loading ||
      this.state.busy ||
      this.state.snapshot?.progress.status === 'running' ||
      (this.state.snapshot && !this.state.snapshot.settled)
    )
      return;
    const preferences = this.state.preferences;
    if (
      preferences.roots.length === 0 ||
      preferences.roots.length > 16 ||
      preferences.roots.some((root) => !root.location)
    ) {
      this.update({
        error:
          'Choose 1–16 readable locations. Remove unavailable locations or select them again.',
      });
      return;
    }
    this.update({ busy: true, error: null });
    try {
      await this.save();
      const scope = await this.api.register(
        preferences.roots.flatMap((root) =>
          root.location ? [root.location.id] : [],
        ),
        preferences.exclusions,
      );
      const progress = await this.api.start(scope.id);
      this.update({
        previous: this.state.snapshot ?? this.state.previous,
        snapshot: {
          scope,
          progress,
          findings: [],
          summary: null,
          settled: false,
        },
        stale:
          this.state.preferences.roots !== preferences.roots ||
          this.state.preferences.exclusions !== preferences.exclusions,
        cancelling: false,
        investigation: null,
      });
      this.schedule();
    } catch (error) {
      this.update({
        error:
          errorCode(error) === 'busy'
            ? 'A native scan is already running. Reopen Reclaim to reconnect.'
            : 'Scan could not start. Reselect changed or unavailable locations, then try again.',
        stale: this.state.snapshot !== null,
      });
    } finally {
      this.update({ busy: false });
    }
  };
  cancel = async (): Promise<void> => {
    const generation = this.state.snapshot?.progress.generation;
    if (generation === undefined || this.state.cancelling) return;
    this.update({ cancelling: true });
    try {
      await this.api.cancel(generation);
    } catch {
      this.update({
        cancelling: false,
        error:
          'Cancellation request failed. Measurements remain available; try Cancel scan again.',
      });
    }
  };
  resume = (): void => {
    this.update({ error: null });
    this.schedule();
  };
  investigate = (investigation: Investigation): void =>
    this.update({ investigation });
  clearInvestigation = (): void => this.update({ investigation: null });
  private isCurrent(generation: number): boolean {
    return (
      this.active && this.state.snapshot?.progress.generation === generation
    );
  }
  private schedule(): void {
    clearTimeout(this.timer);
    if (!this.active || !this.state.snapshot || this.state.snapshot.settled)
      return;
    this.timer = setTimeout(() => void this.poll(), 160);
  }
  private async poll(): Promise<void> {
    if (this.polling) return;
    const snapshot = this.state.snapshot;
    if (!snapshot) return;
    const generation = snapshot.progress.generation;
    this.polling = true;
    try {
      const progress = await this.api.status(generation);
      if (!this.isCurrent(generation)) return;
      if (progress.generation !== generation)
        throw { code: 'stale_generation' };
      if (progress.status === 'running') {
        this.update({ snapshot: { ...snapshot, progress } });
        this.schedule();
        return;
      }
      const summary = await this.api.summary(generation);
      if (!this.isCurrent(generation)) return;
      if (
        summary.generation !== generation ||
        summary.progress.generation !== generation
      )
        throw { code: 'stale_generation' };
      const findings = summaryFindings(summary);
      if (findings.some((finding) => !finding.id.startsWith(`${generation}:`)))
        throw { code: 'stale_generation' };
      this.update({
        snapshot: {
          scope: snapshot.scope,
          progress: summary.progress,
          findings,
          summary,
          settled: true,
        },
        cancelling: false,
      });
    } catch (error) {
      if (this.state.snapshot?.progress.generation !== generation) return;
      if (errorCode(error) === 'rate_limited') {
        this.schedule();
        return;
      }
      this.update({
        stale: true,
        error:
          errorCode(error) === 'stale_generation'
            ? 'Results belong to an older scan generation. Reopen Reclaim to reconnect; retained measurements are stale.'
            : 'Scan connection interrupted. Retained measurements may be incomplete. Retry connection to continue.',
      });
    } finally {
      this.polling = false;
    }
  }
}
