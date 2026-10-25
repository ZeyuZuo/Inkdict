export interface PackInfo {
  id: string;
  title: string;
  entries: number;
  source_lang: string;
  target_lang: string;
}

export interface Suggestion {
  form: string;
  kind: string;
  entry: string;
  headword: string;
  homograph: number | null;
  anchor: number;
}
export interface Correction extends Suggestion { distance: number }
export interface Hit {
  entry: string;
  headword: string;
  homograph: number | null;
  anchor: number;
  lang: string;
  score: number;
}
export type Result = Suggestion | Hit;
export type Column = null | 'text' | 'example';
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
  hide_chinese: boolean;
  hide_pronunciation: boolean;
  hide_examples: boolean;
  font_scale: number;
}
export interface ApiError {
  code: 'no_pack' | 'not_found' | 'bad_input' | 'internal';
  message: string;
}
export interface Commands {
  pack_info: [undefined, PackInfo];
  open_pack: [{ path: string }, PackInfo];
  suggest: [{ text: string; limit: number }, Suggestion[]];
  correct: [{ text: string; limit: number }, Correction[]];
  search: [{ text: string; limit: number; column: Column }, Hit[]];
  entry: [{ key: string }, Entry];
  follow: [{ current: string; link: Link }, LinkTarget];
  settings: [undefined, Settings];
  set_settings: [{ settings: Settings }, Settings];
  theme_css: [undefined, string];
}
