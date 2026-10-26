import { describe, expect, it } from 'vitest';
import { emptyHistory, leave, travel, visit, type Visit } from '../src/lib/history';
import { moveSelection, selectedIndex } from '../src/lib/selection';
import { applyTheme, keys, loadTheme, type StorageLike } from '../src/lib/storage';
import { percentile, record, summarize } from '../src/lib/perf';
import { detail, sections } from '../src/lib/words';
import type { Word } from '../src/lib/types';

function memory(): StorageLike {
  const map = new Map<string, string>();
  return { getItem: key => map.get(key) ?? null, setItem: (key, value) => { map.set(key, value); } };
}
const target = (key: string, text = ''): Visit => ({ key, headword: key, focus: { kind: 'top' }, view: null, text, from: null });

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
    expect(visit(next, focused).visits).toHaveLength(2);
  });
  it('lets entries shown while typing share one stop', () => {
    let history = visit(emptyHistory(), target('go', 'go'));
    history = visit(history, target('harbour', 'h'));
    history = visit(history, target('harbinger', 'harb'), true);
    expect(history.visits.map(v => [v.key, v.text])).toEqual([['go', 'go'], ['harbinger', 'harb']]);
    // The same entry again only updates the search text.
    history = visit(history, target('harbinger', 'harbi'), true);
    expect(history.visits.map(v => v.text)).toEqual(['go', 'harbi']);
  });
  it('remembers where the reader left a visit', () => {
    const history = leave(visit(emptyHistory(), target('a')), { top: 240, tabs: [1] });
    expect(history.visits[0].view).toEqual({ top: 240, tabs: [1] });
    expect(leave(emptyHistory(), { top: 1, tabs: [] })).toEqual(emptyHistory());
  });
  it('caps history at 100 visits', () => {
    let history = emptyHistory();
    for (let i = 0; i < 105; i++) history = visit(history, target(String(i)));
    expect(history.visits).toHaveLength(100);
    expect(history.visits[0].key).toBe('5');
    expect(history.index).toBe(99);
  });
});

describe('word list', () => {
  const word = (form: string, group: Word['group'], extra: Partial<Word> = {}): Word => ({
    group, form, kind: 'headword', entry: form, headword: form, homograph: null, anchor: 0, readings: [],
    summary: null, summary_lang: null, distance: null, rank: 0, target: null, ...extra,
  });
  it('labels the contains and close groups and keeps row positions', () => {
    const found = sections([word('harbour', 'prefix'), word('harbours', 'prefix'), word('safe harbour', 'contains')], ' harbour ');
    expect(found.map(s => [s.group, s.label, s.start, s.rows.length])).toEqual([
      ['prefix', '', 0, 2], ['contains', '含有 harbour', 2, 1],
    ]);
    expect(sections([word('harbour', 'close')], 'harbuor')[0].label).toBe('拼写相近');
    expect(sections([], 'x')).toEqual([]);
  });
  it('says where variants point and which readings matched', () => {
    expect(detail(word('went', 'prefix', { kind: 'inflection', headword: 'go', target: 'go' }))).toBe('→ go');
    expect(detail(word('和', 'prefix', { kind: 'reading', readings: ['hé', 'huó'] }))).toBe('hé huó');
    expect(detail(word('go', 'prefix'))).toBe('');
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

describe('theme persistence', () => {
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
  it('ignores corrupt data and unavailable storage', () => {
    const storage = memory();
    storage.setItem(keys.theme, '{');
    expect(loadTheme(storage)).toBe('system');
    const blocked: StorageLike = { getItem() { throw new Error(); }, setItem() { throw new Error(); } };
    expect(loadTheme(blocked)).toBe('system');
    expect(() => applyTheme('dark', { dataset: {} } as HTMLElement, blocked)).not.toThrow();
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
