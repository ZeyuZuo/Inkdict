import type { Commands, Correction, Hit, Link, Settings } from './types';
import { defaults } from './storage';
import { renderMock, suggestions, words } from './mock-data';
import { themeCss } from './mock-theme';

let settings: Settings = { ...defaults };
const info = { id: 'invented-demo', title: '小小造词集', entries: words.length, source_lang: 'en', target_lang: 'zh-CN' };
const norm = (text: string) => text.trim().toLocaleLowerCase();
const fail = (code: 'bad_input' | 'not_found', message: string): never => { throw { code, message }; };

function distance(a: string, b: string): number {
  const rows = Array.from({ length: a.length + 1 }, (_, i) => [i, ...Array(b.length).fill(0)]);
  for (let j = 0; j <= b.length; j++) rows[0][j] = j;
  for (let i = 1; i <= a.length; i++) for (let j = 1; j <= b.length; j++) {
    rows[i][j] = Math.min(rows[i - 1][j] + 1, rows[i][j - 1] + 1, rows[i - 1][j - 1] + Number(a[i - 1] !== b[j - 1]));
    if (i > 1 && j > 1 && a[i - 1] === b[j - 2] && a[i - 2] === b[j - 1]) {
      rows[i][j] = Math.min(rows[i][j], rows[i - 2][j - 2] + 1);
    }
  }
  return rows[a.length][b.length];
}
function follow(current: string, link: Link): Commands['follow'][1] {
  if (!words.some(w => w.key === current)) return fail('not_found', '当前词条不存在');
  const target = words.find(w => w.key === link.entry);
  const number = link.anchor === 'moment' ? 2 : link.anchor === 'sense' ? 1 : null;
  if (target) return { kind: 'entry', key: target.key, focus: number !== null ? { kind: 'anchor', number }
    : link.anchor_type === 'wfg' ? { kind: 'word_family' } : { kind: 'top' } };
  if (number !== null) return { kind: 'entry', key: current, focus: { kind: 'anchor', number } };
  return { kind: 'lookup', text: link.text };
}

// One dispatch table keeps parameter names and result shapes aligned with invoke.
const handlers: { [K in keyof Commands]: (args: Commands[K][0]) => Commands[K][1] } = {
  pack_info: () => ({ ...info }),
  open_pack: ({ path }) => path.trim() ? { ...info } : fail('bad_input', '请输入词典包路径'),
  suggest: ({ text, limit }) => norm(text) ? suggestions().filter(s => norm(s.form).startsWith(norm(text)))
    .sort((a, b) => a.form.length - b.form.length).slice(0, limit) : [],
  correct: ({ text, limit }) => suggestions().filter(s => s.anchor === 0)
    .map(s => ({ ...s, distance: distance(norm(text), s.form) } satisfies Correction))
    .filter(s => s.distance <= (text.length < 5 ? 1 : 2)).sort((a, b) => a.distance - b.distance).slice(0, limit),
  search: ({ text, limit, column }) => {
    if (!norm(text)) return [];
    const hits: Hit[] = [];
    for (const w of words) {
      const rows = [
        { value: `${w.word} ${w.definition}`, lang: 'en', anchor: 1, column: 'text' },
        { value: w.zh, lang: 'zh-CN', anchor: 1, column: 'text' },
        { value: w.example, lang: 'en', anchor: 1, column: 'example' },
        { value: `${w.word} moment A quiet moment to notice something small.`, lang: 'en', anchor: 2, column: 'text' },
      ];
      for (const row of rows) if ((!column || row.column === column) && norm(text).split(/\s+/).every(t => norm(row.value).includes(t))) {
        hits.push({ entry: w.key, headword: w.word, homograph: null, anchor: row.anchor, lang: row.lang, score: -1 });
      }
    }
    return hits.slice(0, limit);
  },
  entry: ({ key }) => {
    const html = renderMock(key, settings);
    return html === null ? fail('not_found', '没有找到这个词条') : { key, headword: key, homograph: null, html };
  },
  follow: ({ current, link }) => follow(current, link),
  settings: () => ({ ...settings }),
  set_settings: (args) => { settings = { ...args.settings }; return { ...settings }; },
  theme_css: () => themeCss,
};

export async function mockCommand<K extends keyof Commands>(name: K, args: Commands[K][0]): Promise<Commands[K][1]> {
  return handlers[name](args);
}
