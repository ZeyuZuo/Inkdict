// Per-viewer conveniences only; settings live in the app's config.json (Rust).
export interface StorageLike { getItem(key: string): string | null; setItem(key: string, value: string): void }
export const keys = { theme: 'zdict.theme' };

export function browserStorage(): StorageLike | undefined {
  try { return window.localStorage; } catch { return undefined; }
}
export function read(storage: StorageLike | undefined, key: string): unknown {
  try { return JSON.parse(storage?.getItem(key) ?? 'null'); } catch { return null; }
}
export function write(storage: StorageLike | undefined, key: string, value: unknown): void {
  try { storage?.setItem(key, JSON.stringify(value)); } catch { /* A full/private store must not stop lookup. */ }
}
export type ThemeMode = 'system' | 'light' | 'dark';
export function loadTheme(storage?: StorageLike): ThemeMode {
  const value = read(storage, keys.theme);
  return value === 'light' || value === 'dark' ? value : 'system';
}
export function applyTheme(mode: ThemeMode, root: HTMLElement, storage?: StorageLike): void {
  if (mode === 'system') delete root.dataset.theme;
  else root.dataset.theme = mode;
  write(storage, keys.theme, mode);
}
