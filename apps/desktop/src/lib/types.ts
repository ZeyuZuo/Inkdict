// Command arguments and results, field for field the Rust types
// (apps/desktop/README.md).

export interface PackInfo {
  id: string;
  title: string;
  entries: number | null;
  source_lang: string;
  target_langs: string[];
  /** Display switches the pack offers: a type with that role exists. */
  switches: { pronunciation: boolean; example: boolean };
}

export type Group = 'prefix' | 'contains' | 'close';
export type IndexKind = 'headword' | 'variant' | 'inflection' | 'phrase' | 'reading' | 'hidden';

/** One word list row: one entry (spec/pack-v1.md §5). */
export interface Word {
  group: Group;
  form: string;
  kind: IndexKind;
  entry: string;
  headword: string;
  homograph: number | null;
  anchor: number;
  readings: string[];
  summary: string | null;
  summary_lang: string | null;
  distance: number | null;
  rank: number;
  /** Headword a variant or inflection points to, when it differs from the form. */
  target: string | null;
}

export interface Entry {
  key: string;
  headword: string;
  homograph: number | null;
  html: string;
}
export interface Link {
  entry?: string;
  anchor?: string;
  anchor_type?: string;
  text: string;
}
export type Focus = { kind: 'top' } | { kind: 'anchor'; number: number } | { kind: 'word_family' };
export type LinkTarget = { kind: 'entry'; key: string; focus: Focus } | { kind: 'lookup'; text: string };

export interface Settings {
  hide_pronunciation: boolean;
  hide_examples: boolean;
  font_scale: number;
}

export interface LibraryItem {
  path: string;
  title: string | null;
  entries: number | null;
  bytes: number | null;
  /** Why the file cannot be opened. */
  error: string | null;
}
export interface Library { active: string | null; packs: LibraryItem[] }
export interface Added { library: Library; rejected: { path: string; message: string }[] }

export interface ApiError {
  code: 'no_pack' | 'not_found' | 'bad_input' | 'internal';
  message: string;
}

export interface Commands {
  pack_info: [undefined, PackInfo];
  library: [undefined, Library];
  add_packs: [{ paths: string[] }, Added];
  remove_pack: [{ path: string }, Library];
  move_pack: [{ path: string; index: number }, Library];
  use_pack: [{ path: string }, PackInfo];
  words: [{ text: string; limit: number }, Word[]];
  entry: [{ key: string }, Entry];
  follow: [{ current: string; link: Link }, LinkTarget];
  settings: [undefined, Settings];
  set_settings: [{ settings: Settings }, Settings];
  theme_css: [undefined, string];
}
