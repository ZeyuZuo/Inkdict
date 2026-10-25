import { tick } from 'svelte';
import { api, apiError } from './api';
import type { Column, Entry, Focus, Link, PackInfo, Result, Settings } from './types';
import { QueryRunner, Sequence } from './query';
import { emptyHistory, loadRecent, remember, saveRecent, travel, visit, type History, type Visit } from './history';
import { applyTheme, browserStorage, defaults, loadSettings, loadTheme, saveSettings, type ThemeMode } from './storage';
import { createSamples, record, type Metric } from './perf';
import { moveSelection, selectedIndex } from './selection';
import { installTheme } from './theme';

const frame = () => new Promise<void>(resolve => requestAnimationFrame(() => resolve()));
export type EntryPosition = Focus | { kind: 'preserve'; top: number };

export class Dictionary {
  text = $state('');
  fulltext = $state(false);
  column = $state<Column>(null);
  rows = $state<Result[]>([]);
  corrected = $state(false);
  selection = $state(-1);
  searching = $state(false);
  loading = $state(false);
  error = $state('');
  pack = $state<PackInfo | null>(null);
  current = $state<Entry | null>(null);
  position = $state<EntryPosition>({ kind: 'top' });
  entryStarted = $state(0);
  history = $state<History>(emptyHistory());
  recent = $state<Visit[]>([]);
  settings = $state<Settings>({ ...defaults });
  settingsBusy = $state(true);
  theme = $state<ThemeMode>('system');
  showSettings = $state(false);
  showPerf = $state(false);
  perfVersion = $state(0);
  readonly samples = createSamples();
  private storage = browserStorage();
  private queries = new QueryRunner(api);
  private entries = new Sequence();
  private queryEpoch = 0;

  get canBack() { return this.history.index > 0; }
  get canForward() { return this.history.index + 1 < this.history.visits.length; }
  get choices() { return this.text.trim() ? this.rows : this.recent; }

  async initialize() {
    this.recent = loadRecent(this.storage);
    this.setTheme(loadTheme(this.storage));
    window.__zdictPerf = this.samples;
    const results = await Promise.allSettled([
      api.themeCss().then(installTheme),
      api.packInfo().then(pack => { this.pack = pack; }),
      (async () => {
        const saved = loadSettings(this.storage);
        this.settings = saved ? await api.setSettings(saved) : await api.settings();
      })(),
    ]);
    for (const result of results) if (result.status === 'rejected') this.report(result.reason);
    this.settingsBusy = false;
  }

  report(error: unknown) {
    const failure = apiError(error);
    this.error = failure.code === 'no_pack' ? '请在设置中打开一个词典包。' : failure.message;
  }

  async lookup(text: string, started = performance.now()) {
    this.text = text;
    this.rows = [];
    this.selection = -1;
    this.corrected = false;
    this.searching = true;
    this.error = '';
    const epoch = ++this.queryEpoch;
    try {
      const result = await this.queries.run(text, this.fulltext, this.column);
      if (!result || epoch !== this.queryEpoch) return null;
      this.rows = result.rows;
      this.corrected = result.corrected;
      this.searching = false;
      await tick();
      await frame();
      if (epoch !== this.queryEpoch) return null;
      this.measure('suggest', started);
      return result;
    } catch (error) {
      if (epoch === this.queryEpoch) { this.searching = false; this.report(error); }
      return null;
    }
  }

  changeMode() { this.fulltext = !this.fulltext; void this.lookup(this.text); }
  changeColumn(column: Column) { this.column = column; void this.lookup(this.text); }
  select(direction: -1 | 1) { this.selection = moveSelection(this.selection, direction, this.choices.length); }
  async choose(index = selectedIndex(this.selection, this.choices.length)) {
    if (index < 0) return;
    if (!this.text.trim()) {
      const item = this.recent[index];
      if (item) await this.open(item.key, { kind: 'top' });
    } else {
      const item = this.rows[index];
      if (item) await this.open(item.entry, item.anchor ? { kind: 'anchor', number: item.anchor } : { kind: 'top' });
    }
  }

  async open(key: string, focus: Focus = { kind: 'top' }, navigation?: History) {
    const token = this.entries.next();
    const started = performance.now();
    this.loading = true;
    this.error = '';
    try {
      const entry = await api.entry(key);
      if (!this.entries.current(token)) return;
      this.position = focus;
      this.entryStarted = started;
      this.current = entry;
      const target = { key: entry.key, headword: entry.headword, focus };
      this.history = navigation ?? visit(this.history, target);
      this.recent = remember(this.recent, target);
      saveRecent(this.storage, this.recent);
    } catch (error) {
      if (this.entries.current(token)) this.report(error);
    } finally {
      if (this.entries.current(token)) this.loading = false;
    }
  }

  async navigate(direction: -1 | 1) {
    const next = travel(this.history, direction);
    if (next === this.history) return;
    const target = next.visits[next.index];
    await this.open(target.key, target.focus, next);
  }

  async follow(link: Link) {
    if (!this.current) return;
    const token = this.entries.next();
    this.loading = false;
    try {
      const target = await api.follow(this.current.key, link);
      if (!this.entries.current(token)) return;
      if (target.kind === 'entry') await this.open(target.key, target.focus);
      else {
        this.fulltext = false;
        const result = await this.lookup(target.text);
        if (result && this.entries.current(token)) await this.choose(0);
      }
    } catch (error) { if (this.entries.current(token)) this.report(error); }
  }

  setTheme(mode: ThemeMode) {
    this.theme = mode;
    applyTheme(mode, document.documentElement, this.storage);
  }

  async updateSettings(settings: Settings, top: number) {
    if (this.settingsBusy) return;
    this.settingsBusy = true;
    this.error = '';
    try {
      this.settings = await api.setSettings(settings);
      saveSettings(this.storage, this.settings);
      if (this.current) {
        const token = this.entries.next();
        const started = performance.now();
        this.loading = true;
        try {
          const entry = await api.entry(this.current.key);
          if (this.entries.current(token)) {
            this.position = { kind: 'preserve', top };
            this.entryStarted = started;
            this.current = entry;
          }
        } finally {
          if (this.entries.current(token)) this.loading = false;
        }
      }
    } catch (error) { this.report(error); }
    finally { this.settingsBusy = false; }
  }

  async openPack(path: string) {
    try {
      const pack = await api.openPack(path);
      this.entries.next(); this.queries.cancel(); this.queryEpoch++;
      this.loading = false; this.searching = false;
      this.pack = pack; this.current = null; this.history = emptyHistory();
      this.rows = []; this.text = ''; this.error = ''; this.selection = -1;
      installTheme(await api.themeCss());
      this.settings = await api.setSettings(this.settings);
      this.showSettings = false;
    } catch (error) { this.report(error); }
  }

  measure(metric: Metric, started: number) {
    record(this.samples[metric], performance.now() - started);
    this.perfVersion++;
  }
}
