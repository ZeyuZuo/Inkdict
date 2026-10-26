import { invoke } from '@tauri-apps/api/core';
import type { ApiError, Commands, Link, Settings } from './types';

export const isTauri = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export function apiError(error: unknown): ApiError {
  if (error && typeof error === 'object' && 'code' in error && 'message' in error) {
    return error as ApiError;
  }
  return { code: 'internal', message: error instanceof Error ? error.message : String(error) };
}

async function command<K extends keyof Commands>(name: K, args: Commands[K][0]): Promise<Commands[K][1]> {
  try {
    if (isTauri()) return await invoke<Commands[K][1]>(name, args);
    // Browser development against real data: `zdict serve` (crates/zdict-cli).
    const bridge = import.meta.env.VITE_ZDICT_BRIDGE as string | undefined;
    if (bridge) {
      const response = await fetch(`${bridge}/${name}`, { method: 'POST', body: JSON.stringify(args ?? {}) });
      const value = await response.json();
      if (!response.ok) throw value;
      return value as Commands[K][1];
    }
    const { mockCommand } = await import('./mock');
    return await mockCommand(name, args);
  } catch (error) {
    throw apiError(error);
  }
}

export const api = {
  packInfo: () => command('pack_info', undefined),
  library: () => command('library', undefined),
  addPacks: (paths: string[]) => command('add_packs', { paths }),
  removePack: (path: string) => command('remove_pack', { path }),
  movePack: (path: string, index: number) => command('move_pack', { path, index }),
  usePack: (path: string) => command('use_pack', { path }),
  words: (text: string, limit = 30) => command('words', { text, limit }),
  entry: (key: string) => command('entry', { key }),
  follow: (current: string, link: Link) => command('follow', { current, link }),
  settings: () => command('settings', undefined),
  setSettings: (settings: Settings) => command('set_settings', { settings }),
  themeCss: () => command('theme_css', undefined),
};
