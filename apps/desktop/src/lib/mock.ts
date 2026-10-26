// Browser-only stand-in for the Rust commands (no Tauri, no `zdict serve`):
// invented entries, so the UI can be worked on without dictionary data.
import type { Commands, Library, Link, PackInfo, Settings, Word } from './types';
import { renderMock, rows, words } from './mock-data';
import { themeCss } from './mock-theme';

let settings: Settings = { hide_pronunciation: false, hide_examples: false, font_scale: 1 };
const path = '/demo/invented.zdpack';
const info: PackInfo = {
  id: 'invented-demo', title: '小小造词集', entries: words.length, source_lang: 'en', target_langs: ['zh-Hans'],
  switches: { pronunciation: true, example: true },
};
let library: Library = { active: path, packs: [{ path, title: info.title, entries: words.length, bytes: 12_288, error: null }] };
const norm = (text: string) => text.trim().toLocaleLowerCase();
const fail = (code: 'bad_input' | 'not_found' | 'no_pack', message: string): never => { throw { code, message }; };
const open = () => (library.active ? info : fail('no_pack', 'no dictionary pack is open'));

/** Prefix rows, then rows containing the input (spec/pack-v1.md §5, simplified). */
function list(text: string, limit: number): Word[] {
  const key = norm(text);
  if (!key) return [];
  const all = rows();
  const prefix = all.filter(row => norm(row.form).startsWith(key)).sort((a, b) => a.form.length - b.form.length);
  const contains = key.length < 2 ? [] : all
    .filter(row => !prefix.includes(row) && norm(row.form).slice(1).includes(key))
    .map(row => ({ ...row, group: 'contains' as const }));
  return [...prefix.slice(0, limit), ...contains.slice(0, limit)];
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
  pack_info: () => ({ ...open() }),
  library: () => structuredClone(library),
  add_packs: ({ paths }) => ({ library: structuredClone(library), rejected: paths.map(p => ({ path: p, message: '演示模式不能添加词典' })) }),
  remove_pack: ({ path: removed }) => {
    library = { active: library.active === removed ? null : library.active, packs: library.packs.filter(p => p.path !== removed) };
    return structuredClone(library);
  },
  move_pack: () => structuredClone(library),
  use_pack: ({ path: used }) => {
    if (!library.packs.some(p => p.path === used)) fail('not_found', `${used} is not in the library`);
    library = { ...library, active: used };
    return { ...info };
  },
  words: ({ text, limit }) => { open(); return list(text, limit); },
  entry: ({ key }) => {
    open();
    const html = renderMock(key, settings);
    return html === null ? fail('not_found', '没有找到这个词条') : { key, headword: key, homograph: null, html };
  },
  follow: ({ current, link }) => follow(current, link),
  settings: () => ({ ...settings }),
  set_settings: (args) => { settings = { ...args.settings }; return { ...settings }; },
  theme_css: () => { open(); return themeCss; },
};

export async function mockCommand<K extends keyof Commands>(name: K, args: Commands[K][0]): Promise<Commands[K][1]> {
  return handlers[name](args);
}
