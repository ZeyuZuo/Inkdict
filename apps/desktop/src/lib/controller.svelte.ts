import { tick } from 'svelte';
import { api, apiError, isTauri } from './api';
import type { Entry, Focus, Library, Link, PackInfo, Settings, Word } from './types';
import { Sequence } from './query';
import { emptyHistory, leave, travel, visit, type History, type View } from './history';
import { applyTheme, browserStorage, loadTheme, type ThemeMode } from './storage';
import { createSamples, record, type Metric } from './perf';
import { moveSelection } from './selection';
import { installTheme } from './theme';

const frame = () => new Promise<void>(resolve => requestAnimationFrame(() => resolve()));
export type EntryPosition = Focus | ({ kind: 'preserve' } & View);
/** Rows per group; the list scrolls. */
const LIMIT = 40;
/** Typing pauses this long before the top row's entry is shown. */
const SHOW_DELAY = 90;

interface OpenOptions {
  /** Take the current history slot (entries shown while typing). */
  replace?: boolean;
  /** Headword of the entry whose link led here. */
  from?: string | null;
  /** Travelling through history: use this one instead of adding a visit. */
  navigation?: History;
  view?: View | null;
}

export class Dictionary {
  text = $state('');
  words = $state<Word[]>([]);
  selection = $state(-1);
  /** Narrow window: the list drops down while typing, closes once a row is opened. */
  listOpen = $state(false);
  searched = $state('');
  loading = $state(false);
  error = $state('');
  pack = $state<PackInfo | null>(null);
  library = $state<Library>({ active: null, packs: [] });
  current = $state<Entry | null>(null);
  position = $state<EntryPosition>({ kind: 'top' });
  /** Headword of the entry whose link led to the current one. */
  from = $state<string | null>(null);
  entryStarted = $state(0);
  history = $state<History>(emptyHistory());
  settings = $state<Settings>({ hide_pronunciation: false, hide_examples: false, font_scale: 1 });
  theme = $state<ThemeMode>('system');
  narrow = $state(false);
  pinned = $state(false);
  showSettings = $state(false);
  showPerf = $state(false);
  perfVersion = $state(0);
  readonly samples = createSamples();
  /** Set by the entry pane: where the reader is, saved with each visit. */
  view: () => View = () => ({ top: 0, tabs: [] });
  private storage = browserStorage();
  private queries = new Sequence();
  private entries = new Sequence();
  private packs = new Sequence();
  /** The pack being switched to, until that switch finishes. */
  private switching: string | null = null;
  private showTimer: ReturnType<typeof setTimeout> | undefined;
  private shownWhileTyping = false;
  /** The latest lookup: Enter waits for it when it is still running. */
  private listing: Promise<Word[] | null> = Promise.resolve([]);
  /** Where Alt+← / Alt+→ is heading while the entry loads, so quick presses add up. */
  private travelling: History | null = null;

  async initialize() {
    this.setTheme(loadTheme(this.storage));
    window.__zdictPerf = this.samples;
    const results = await Promise.allSettled([
      this.loadPack(),
      api.settings().then(settings => { this.settings = settings; }),
      this.refreshLibrary(),
    ]);
    for (const result of results) if (result.status === 'rejected') this.report(result.reason);
    if (this.pack) void this.warmUp();
  }

  /** The open pack and its theme; none is not an error. */
  private async loadPack() {
    try {
      this.pack = await api.packInfo();
      installTheme(await api.themeCss());
    } catch (error) {
      if (apiError(error).code !== 'no_pack') throw error;
      this.pack = null;
    }
  }

  async refreshLibrary() {
    this.library = await api.library();
  }

  /** Render one entry off-screen once: WebKit's first entry layout took
   *  ~300 ms, later ones ~17 ms, so pay that before the first lookup. */
  private async warmUp() {
    try {
      const [first] = await api.words('a', 1);
      if (!first) return;
      const entry = await api.entry(first.entry);
      const box = document.createElement('div');
      box.className = 'entry';
      box.setAttribute('aria-hidden', 'true');
      box.style.cssText = 'position:absolute;left:-10000px;top:0;width:46rem;visibility:hidden;pointer-events:none';
      box.innerHTML = entry.html;
      document.body.append(box);
      void box.offsetHeight;
      await frame();
      box.remove();
    } catch { /* only an optimisation */ }
  }

  report(error: unknown) {
    const failure = apiError(error);
    this.error = failure.code === 'no_pack' ? '还没有打开词典。在设置里添加一部词典。' : failure.message;
  }

  /** Look up `text`; the list is ready when this resolves (null if a later
   *  input overtook it). In a wide window the top row's entry follows. */
  lookup(text: string, started = performance.now(), show = !this.narrow) {
    this.listing = this.list(text, started, show);
    return this.listing;
  }

  private async list(text: string, started: number, show: boolean) {
    this.text = text;
    clearTimeout(this.showTimer);
    const token = this.queries.next();
    if (!text.trim()) {
      this.words = []; this.selection = -1; this.listOpen = false; this.searched = '';
      return [];
    }
    try {
      const words = await api.words(text, LIMIT);
      if (!this.queries.current(token)) return null;
      this.words = words;
      this.searched = text;
      this.selection = words.length ? 0 : -1;
      this.listOpen = true;
      await tick();
      await frame();
      if (!this.queries.current(token)) return null;
      this.measure('words', started);
      if (show && words.length) {
        this.showTimer = setTimeout(() => {
          if (this.queries.current(token)) void this.show(0, true);
        }, SHOW_DELAY);
      }
      return words;
    } catch (error) {
      if (this.queries.current(token)) this.report(error);
      return null;
    }
  }

  /** ↑↓: move through the list; a wide window shows each row at once. */
  move(direction: -1 | 1) {
    this.selection = moveSelection(this.selection, direction, this.words.length);
    this.listOpen = this.words.length > 0;
    if (!this.narrow && this.selection >= 0) void this.show(this.selection, true);
  }

  /** Open a row on purpose (click, Enter): a history stop of its own.
   *  Enter (no index) before the list for the typed text has arrived waits
   *  for it and opens its top row. */
  async choose(index?: number) {
    if (index === undefined) {
      index = this.selection;
      if (this.searched !== this.text) {
        const words = await this.listing;
        if (!words || this.searched !== this.text) return;
        index = 0;
      }
    }
    if (index < 0 || index >= this.words.length) return;
    // A lookup still running must not reopen the list or show its top row.
    this.queries.next();
    clearTimeout(this.showTimer);
    this.selection = index;
    this.listOpen = false;
    await this.show(index, false);
  }

  private async show(index: number, typing: boolean) {
    clearTimeout(this.showTimer);
    const word = this.words[index];
    if (!word) return;
    const focus: Focus = word.anchor ? { kind: 'anchor', number: word.anchor } : { kind: 'top' };
    const replace = typing && this.shownWhileTyping;
    // Entries shown while typing share one history stop until one is chosen.
    if (await this.open(word.entry, focus, { replace })) this.shownWhileTyping = typing;
  }

  /** Show an entry; false if a later request overtook this one or it failed. */
  async open(key: string, focus: Focus = { kind: 'top' }, options: OpenOptions = {}): Promise<boolean> {
    const token = this.entries.next();
    const started = performance.now();
    this.loading = true;
    try {
      const entry = await api.entry(key);
      if (!this.entries.current(token)) return false;
      const left = leave(this.history, this.view());
      const from = options.navigation
        ? options.navigation.visits[options.navigation.index].from
        : options.from ?? null;
      this.history = options.navigation ?? visit(left, {
        key: entry.key, headword: entry.headword, focus, view: null, text: this.text, from,
      }, options.replace);
      this.from = from;
      this.position = options.view ? { kind: 'preserve', ...options.view } : focus;
      this.entryStarted = started;
      this.current = entry;
      this.error = '';
      return true;
    } catch (error) {
      if (this.entries.current(token)) this.report(error);
      return false;
    } finally {
      if (this.entries.current(token)) { this.loading = false; this.travelling = null; }
    }
  }

  /** Alt+← / Alt+→ and the mouse side buttons: entry, place and search box come back. */
  async navigate(direction: -1 | 1) {
    clearTimeout(this.showTimer);
    const left = this.travelling ?? leave(this.history, this.view());
    const next = travel(left, direction);
    if (next === left) return;
    this.travelling = next;
    this.entries.next();
    this.shownWhileTyping = false;
    const target = next.visits[next.index];
    if (target.text !== this.text) await this.lookup(target.text, performance.now(), false);
    // Another press, or a chosen row, overtook this one during the lookup.
    if (this.travelling !== next) return;
    this.listOpen = false;
    const index = this.words.findIndex(word => word.entry === target.key);
    this.selection = index;
    await this.open(target.key, target.focus, { navigation: next, view: target.view });
  }

  async follow(link: Link) {
    if (!this.current) return;
    clearTimeout(this.showTimer);
    const source = this.current;
    const token = this.entries.next();
    try {
      const target = await api.follow(source.key, link);
      if (!this.entries.current(token)) return;
      this.shownWhileTyping = false;
      if (target.kind === 'entry') {
        await this.open(target.key, target.focus, { from: source.headword });
      } else {
        const words = await this.lookup(target.text, performance.now(), false);
        if (words?.length) {
          this.listOpen = false;
          const word = words[0];
          await this.open(word.entry, word.anchor ? { kind: 'anchor', number: word.anchor } : { kind: 'top' },
            { from: source.headword });
        }
      }
    } catch (error) { if (this.entries.current(token)) this.report(error); }
  }

  setTheme(mode: ThemeMode) {
    this.theme = mode;
    applyTheme(mode, document.documentElement, this.storage);
  }

  async updateSettings(settings: Settings) {
    try {
      this.settings = await api.setSettings(settings);
      await this.rerender();
    } catch (error) { this.report(error); }
  }

  /** Render the current entry again (new settings) where the reader is. */
  private async rerender() {
    if (!this.current) return;
    const token = this.entries.next();
    const view = this.view();
    try {
      const entry = await api.entry(this.current.key);
      if (!this.entries.current(token)) return;
      this.position = { kind: 'preserve', ...view };
      this.entryStarted = performance.now();
      this.current = entry;
    } catch (error) { if (this.entries.current(token)) this.report(error); }
  }

  async addPacks(paths: string[]) {
    try {
      const had = this.library.active;
      const added = await api.addPacks(paths);
      this.library = added.library;
      if (added.rejected.length) {
        this.error = added.rejected.map(item => `${item.path}：${item.message}`).join('\n');
      }
      if (added.library.active !== had) await this.switched();
    } catch (error) { this.report(error); }
  }

  async removePack(path: string) {
    try {
      const had = this.library.active;
      this.library = await api.removePack(path);
      if (this.library.active !== had) await this.switched();
    } catch (error) { this.report(error); }
  }

  async movePack(path: string, index: number) {
    try { this.library = await api.movePack(path, index); } catch (error) { this.report(error); }
  }

  /** Switch packs; quick switches (arrow keys in the settings list) only
   *  finish the last one. */
  async usePack(path: string) {
    if (path === (this.switching ?? this.library.active)) return;
    const token = this.packs.next();
    this.switching = path;
    try {
      await api.usePack(path);
      if (!this.packs.current(token)) return;
      await this.refreshLibrary();
      await this.switched();
    } catch (error) {
      if (!this.packs.current(token)) return;
      this.report(error);
      await this.refreshLibrary().catch(() => {});
    } finally {
      if (this.packs.current(token)) this.switching = null;
    }
  }

  /** Another pack is open (or none): start over with its theme. */
  private async switched() {
    this.entries.next();
    this.queries.next();
    this.current = null; this.from = null; this.history = emptyHistory();
    this.words = []; this.selection = -1; this.listOpen = false; this.searched = '';
    this.error = '';
    await this.loadPack();
    if (this.pack && this.text.trim()) await this.lookup(this.text);
    if (this.pack) void this.warmUp();
  }

  async togglePin() {
    if (!isTauri()) return;
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    try {
      await getCurrentWindow().setAlwaysOnTop(!this.pinned);
      this.pinned = !this.pinned;
    } catch (error) { this.report(error); }
  }

  measure(metric: Metric, started: number) {
    record(this.samples[metric], performance.now() - started);
    this.perfVersion++;
  }
}
