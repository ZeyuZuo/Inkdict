import type { Group, Word } from './types';

export interface Section { group: Group; label: string; start: number; rows: Word[] }

/** Consecutive rows of one group, with the heading shown above it. The
 *  prefix group needs none: it is what the reader typed. */
export function sections(words: Word[], text: string): Section[] {
  const found: Section[] = [];
  words.forEach((word, index) => {
    const last = found[found.length - 1];
    if (last?.group === word.group) last.rows.push(word);
    else found.push({ group: word.group, label: label(word.group, text), start: index, rows: [word] });
  });
  return found;
}

export function label(group: Group, text: string): string {
  const shown = text.trim();
  if (group === 'contains') return `含有 ${shown}`;
  if (group === 'close') return `拼写相近`;
  return '';
}

/** What a row says after its form: the headword it points to, or the readings. */
export function detail(word: Word): string {
  if (word.target) return `→ ${word.target}`;
  if (word.kind === 'reading' && word.readings.length) return word.readings.join(' ');
  return '';
}
