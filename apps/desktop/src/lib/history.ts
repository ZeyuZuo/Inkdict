import type { Focus } from './types';
import { keys, read, write, type StorageLike } from './storage';

export interface Visit { key: string; headword: string; focus: Focus }
export interface History { visits: Visit[]; index: number }
export const emptyHistory = (): History => ({ visits: [], index: -1 });

export function visit(history: History, target: Visit): History {
  const current = history.visits[history.index];
  if (current?.key === target.key && JSON.stringify(current.focus) === JSON.stringify(target.focus)) return history;
  const visits = [...history.visits.slice(0, history.index + 1), target].slice(-100);
  return { visits, index: visits.length - 1 };
}
export function travel(history: History, direction: -1 | 1): History {
  const index = history.index + direction;
  return index >= 0 && index < history.visits.length ? { ...history, index } : history;
}
export function remember(recent: Visit[], target: Visit): Visit[] {
  return [target, ...recent.filter(item => item.key !== target.key)].slice(0, 100);
}
export function loadRecent(storage?: StorageLike): Visit[] {
  const value = read(storage, keys.recent);
  if (!Array.isArray(value)) return [];
  const seen = new Set<string>();
  return value.filter((item): item is Visit => {
    if (!item || typeof item.key !== 'string' || typeof item.headword !== 'string' || seen.has(item.key)) return false;
    seen.add(item.key);
    return true;
  }).slice(0, 100).map(item => ({ key: item.key, headword: item.headword, focus: { kind: 'top' } }));
}
export function saveRecent(storage: StorageLike | undefined, recent: Visit[]): void {
  write(storage, keys.recent, recent.map(({ key, headword }) => ({ key, headword })));
}
