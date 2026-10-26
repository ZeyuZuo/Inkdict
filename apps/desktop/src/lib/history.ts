import type { Focus } from './types';

export interface View { top: number; tabs: number[] }

/** One place the reader was: the entry, where in it, and the search box. */
export interface Visit {
  key: string;
  headword: string;
  focus: Focus;
  /** Where the reader was when leaving it: scroll offset and the open
   *  tab of each tab group; restored on return. */
  view: View | null;
  text: string;
  /** Headword of the entry whose link led here. */
  from: string | null;
}
export interface History { visits: Visit[]; index: number }
export const emptyHistory = (): History => ({ visits: [], index: -1 });

const same = (a: Visit, b: Visit) => a.key === b.key && JSON.stringify(a.focus) === JSON.stringify(b.focus);

/** Add a visit after the current one, dropping forward history. With
 *  `replace` it takes the current slot instead: entries shown while typing
 *  are one stop, not one per keystroke. */
export function visit(history: History, target: Visit, replace = false): History {
  const current = history.visits[history.index];
  if (current && same(current, target)) {
    const visits = [...history.visits];
    visits[history.index] = { ...current, text: target.text };
    return { visits, index: history.index };
  }
  const keep = replace && current ? history.index : history.index + 1;
  const visits = [...history.visits.slice(0, keep), target].slice(-100);
  return { visits, index: visits.length - 1 };
}

export function travel(history: History, direction: -1 | 1): History {
  const index = history.index + direction;
  return index >= 0 && index < history.visits.length ? { ...history, index } : history;
}

/** Remember where the reader is in the current visit before leaving it. */
export function leave(history: History, view: View): History {
  const current = history.visits[history.index];
  if (!current) return history;
  const visits = [...history.visits];
  visits[history.index] = { ...current, view };
  return { visits, index: history.index };
}
