import type { Settings } from './types';

export interface StorageLike { getItem(key: string): string | null; setItem(key: string, value: string): void }
export const defaults: Settings = {
  hide_chinese: false, hide_pronunciation: false, hide_examples: false, font_scale: 1,
};
export const keys = { settings: 'zdict.settings', theme: 'zdict.theme' };

export function browserStorage(): StorageLike | undefined {
  try { return window.localStorage; } catch { return undefined; }
}
export function read(storage: StorageLike | undefined, key: string): unknown {
  try { return JSON.parse(storage?.getItem(key) ?? 'null'); } catch { return null; }
}
export function write(storage: StorageLike | undefined, key: string, value: unknown): void {
  try { storage?.setItem(key, JSON.stringify(value)); } catch { /* A full/private store must not stop lookup. */ }
}
export function validSettings(value: unknown): value is Settings {
  if (!value || typeof value !== 'object') return false;
  const s = value as Settings;
  return ['hide_chinese', 'hide_pronunciation', 'hide_examples'].every(k => typeof s[k as keyof Settings] === 'boolean')
    && Number.isFinite(s.font_scale) && s.font_scale >= 0.8 && s.font_scale <= 1.6;
}
export function loadSettings(storage?: StorageLike): Settings | null {
  const value = read(storage, keys.settings);
  return validSettings(value) ? { ...value } : null;
}
export function saveSettings(storage: StorageLike | undefined, settings: Settings): void {
  write(storage, keys.settings, settings);
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
