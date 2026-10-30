// Browser-only stand-in for the Rust commands (no Tauri, no `inkdict serve`):
// invented entries, so the UI can be worked on without dictionary data.
import type { Commands, Library, Link, PackInfo, Settings, Word } from './types';
import { renderMock, rows, words } from './mock-data';
import { themeCss } from './mock-theme';

let settings: Settings = { hide_pronunciation: false, hide_examples: false, font_scale: 1 };
const folder = '/home/user/.local/share/win.zeyuz.inkdict/dictionaries';
const path = `${folder}/invented.inkdict`;
const info: PackInfo = {
  id: 'invented-demo', title: '小小造词集', entries: words.length, source_lang: 'en', target_langs: ['zh-Hans'],
  switches: { pronunciation: true, example: true },
  theme: 'default',
  themes: [{ name: 'default', title: 'Default' }, { name: 'plain', title: 'Plain' }],
};
let library: Library = { folder, active: path, packs: [{ path, in_folder: true, title: info.title, entries: words.length, bytes: 12_288, error: null }] };
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
  const number = { sense: 1, moment: 2, family: 3 }[link.anchor ?? ''] ?? null;
  if (target) return { kind: 'entry', key: target.key, focus: number !== null ? { kind: 'anchor', number } : { kind: 'top' } };
  if (number !== null) return { kind: 'entry', key: current, focus: { kind: 'anchor', number } };
  return { kind: 'lookup', text: link.text };
}

// One dispatch table keeps parameter names and result shapes aligned with invoke.
const handlers: { [K in keyof Commands]: (args: Commands[K][0]) => Commands[K][1] } = {
  pack_info: () => ({ ...open() }),
  library: () => structuredClone(library),
  rescan: () => structuredClone(library),
  set_folder: ({ path: folder }) => {
    if (!folder.startsWith('/')) fail('bad_input', 'dictionary folder must be an absolute path');
    if (folder !== library.folder) library = { folder, active: null, packs: [] };
    return structuredClone(library);
  },
  show_folder: () => {},
  add_packs: ({ paths }) => ({ library: structuredClone(library), rejected: paths.map(p => ({ path: p, message: '演示模式不能添加词典' })) }),
  remove_pack: ({ path: removed }) => {
    library = { ...library, active: library.active === removed ? null : library.active, packs: library.packs.filter(p => p.path !== removed) };
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
    const html = renderMock(key, settings, info.theme);
    return html === null ? fail('not_found', '没有找到这个词条') : { key, headword: key, homograph: null, html };
  },
  follow: ({ current, link }) => follow(current, link),
  settings: () => ({ ...settings }),
  set_settings: (args) => { settings = { ...args.settings }; return { ...settings }; },
  set_theme: ({ name }) => {
    open();
    if (!info.themes.some(theme => theme.name === name)) fail('bad_input', `unknown theme ${name}`);
    info.theme = name;
    return { ...info };
  },
  theme_css: () => {
    open();
    return info.theme === 'plain' ? `${themeCss}\n.dx-entry{--c-muted:#28704b}.dx-link{color:#28704b}.dx-marker{display:none}` : themeCss;
  },
};

export async function mockCommand<K extends keyof Commands>(name: K, args: Commands[K][0]): Promise<Commands[K][1]> {
  return handlers[name](args);
}
