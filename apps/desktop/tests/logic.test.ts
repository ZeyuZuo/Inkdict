import { describe, expect, it } from 'vitest';
import { emptyHistory, loadRecent, remember, saveRecent, travel, visit, type Visit } from '../src/lib/history';
import { moveSelection, selectedIndex } from '../src/lib/selection';
import { applyTheme, defaults, keys, loadSettings, loadTheme, saveSettings, type StorageLike } from '../src/lib/storage';
import { percentile, record, summarize } from '../src/lib/perf';

function memory(): StorageLike {
  const map = new Map<string, string>();
  return { getItem: key => map.get(key) ?? null, setItem: (key, value) => { map.set(key, value); } };
}
const target = (key: string): Visit => ({ key, headword: key, focus: { kind: 'top' } });

describe('history', () => {
  it('moves back and forward without exceeding its bounds', () => {
    const a = visit(emptyHistory(), target('a'));
    const b = visit(a, target('b'));
    expect(travel(a, -1)).toBe(a);
    expect(travel(b, 1)).toBe(b);
    expect(travel(b, -1).visits[travel(b, -1).index].key).toBe('a');
    expect(travel(travel(b, -1), 1)).toEqual(b);
  });
  it('truncates forward history on a new visit, retaining anchor focus', () => {
    const history = visit(visit(emptyHistory(), target('a')), target('b'));
    const focused: Visit = { ...target('c'), focus: { kind: 'anchor', number: 2 } };
    const next = visit(travel(history, -1), focused);
    expect(next.visits.map(v => v.key)).toEqual(['a', 'c']);
    expect(next.visits[next.index].focus).toEqual(focused.focus);
    expect(visit(next, focused)).toBe(next);
  });
  it('caps history and recent entries at 100 and deduplicates recent keys', () => {
    let history = emptyHistory();
    let recent: Visit[] = [];
    for (let i = 0; i < 105; i++) { history = visit(history, target(String(i))); recent = remember(recent, target(String(i))); }
    expect(history.visits).toHaveLength(100);
    expect(history.visits[0].key).toBe('5');
    expect(history.index).toBe(99);
    recent = remember(recent, target('80'));
    expect(recent).toHaveLength(100);
    expect(recent[0].key).toBe('80');
    const storage = memory();
    saveRecent(storage, recent);
    expect(loadRecent(storage)).toEqual(recent);
  });
  it('ignores corrupt persisted history', () => {
    const storage = memory(); storage.setItem(keys.recent, '[null, {"key":3}]');
    expect(loadRecent(storage)).toEqual([]);
  });
});

describe('keyboard selection', () => {
  it('handles an empty list and Enter without a selection', () => {
    expect(moveSelection(-1, 1, 0)).toBe(-1);
    expect(selectedIndex(-1, 0)).toBe(-1);
    expect(selectedIndex(-1, 3)).toBe(0);
  });
  it('starts at the appropriate end and clamps at list boundaries', () => {
    expect(moveSelection(-1, 1, 3)).toBe(0);
    expect(moveSelection(-1, -1, 3)).toBe(2);
    expect(moveSelection(0, -1, 3)).toBe(0);
    expect(moveSelection(2, 1, 3)).toBe(2);
    expect(moveSelection(0, 1, 3)).toBe(1);
    expect(selectedIndex(8, 2)).toBe(1);
  });
});

describe('settings and theme persistence', () => {
  it('round trips every setting including font scale', () => {
    const storage = memory();
    const settings = { hide_chinese: true, hide_pronunciation: true, hide_examples: true, font_scale: 1.4 };
    expect(loadSettings(storage)).toBeNull();
    saveSettings(storage, settings);
    expect(loadSettings(storage)).toEqual(settings);
    expect(defaults.font_scale).toBe(1);
  });
  it('ignores corrupt data, out-of-range scales, and unavailable storage', () => {
    const storage = memory();
    for (const invalid of ['{', 'null', JSON.stringify({ ...defaults, font_scale: 9 }), JSON.stringify({ ...defaults, hide_chinese: 'true' })]) {
      storage.setItem(keys.settings, invalid); expect(loadSettings(storage)).toBeNull();
    }
    const blocked: StorageLike = { getItem() { throw new Error(); }, setItem() { throw new Error(); } };
    expect(loadSettings(blocked)).toBeNull();
    expect(() => saveSettings(blocked, defaults)).not.toThrow();
  });
  it('removes the theme attribute for system mode and persists explicit modes', () => {
    const storage = memory();
    const root = { dataset: {} } as HTMLElement;
    applyTheme('dark', root, storage);
    expect(root.dataset.theme).toBe('dark'); expect(loadTheme(storage)).toBe('dark');
    applyTheme('light', root, storage);
    expect(root.dataset.theme).toBe('light');
    applyTheme('system', root, storage);
    expect(root.dataset.theme).toBeUndefined(); expect(loadTheme(storage)).toBe('system');
  });
});

describe('latency samples', () => {
  it('computes nearest-rank percentiles without mutating samples', () => {
    const samples = [40, 10, 30, 20];
    expect(percentile([], .95)).toBe(0);
    expect(percentile(samples, .5)).toBe(20);
    expect(percentile(samples, .95)).toBe(40);
    expect(percentile([7], .5)).toBe(7);
    expect(summarize(samples)).toEqual({ count: 4, p50: 20, p95: 40, max: 40 });
    expect(samples).toEqual([40, 10, 30, 20]);
  });
  it('retains only the last 200 valid samples', () => {
    const samples: number[] = [];
    for (let i = 0; i <= 205; i++) record(samples, i);
    record(samples, NaN); record(samples, -1);
    expect(samples).toHaveLength(200); expect(samples[0]).toBe(6);
    expect(percentile(samples, .95)).toBe(195);
  });
});
