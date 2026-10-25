import type { Focus } from './types';

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
