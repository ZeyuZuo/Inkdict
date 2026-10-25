import { invoke } from '@tauri-apps/api/core';
import type { ApiError, Column, Commands, Link, Settings } from './types';

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
    const { mockCommand } = await import('./mock');
    return await mockCommand(name, args);
  } catch (error) {
    throw apiError(error);
  }
}

export const api = {
  packInfo: () => command('pack_info', undefined),
  openPack: (path: string) => command('open_pack', { path }),
  suggest: (text: string, limit = 30) => command('suggest', { text, limit }),
  correct: (text: string, limit = 10) => command('correct', { text, limit }),
  search: (text: string, limit = 30, column: Column = null) => command('search', { text, limit, column }),
  entry: (key: string) => command('entry', { key }),
  follow: (current: string, link: Link) => command('follow', { current, link }),
  settings: () => command('settings', undefined),
  setSettings: (settings: Settings) => command('set_settings', { settings }),
  themeCss: () => command('theme_css', undefined),
};
