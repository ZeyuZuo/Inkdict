import { isTauri } from './api';

/** Choose .zdpack files with the system dialog; empty when cancelled.
 *  Only in the app: a browser has no file paths to give. */
export async function pickPacks(): Promise<string[]> {
  if (!isTauri()) return [];
  const { open } = await import('@tauri-apps/plugin-dialog');
  const chosen = await open({ multiple: true, directory: false, filters: [{ name: 'ZDict 词典', extensions: ['zdpack'] }] });
  if (!chosen) return [];
  return Array.isArray(chosen) ? chosen : [chosen];
}

export function size(bytes: number | null): string {
  if (bytes === null) return '';
  if (bytes >= 1 << 30) return `${(bytes / (1 << 30)).toFixed(1)} GB`;
  if (bytes >= 1 << 20) return `${Math.round(bytes / (1 << 20))} MB`;
  return `${Math.max(1, Math.round(bytes / 1024))} KB`;
}

/** The path with the home directory shortened, for display. */
export function shortPath(path: string): string {
  return path.replace(/^\/home\/[^/]+\//, '~/').replace(/^\/Users\/[^/]+\//, '~/').replace(/^[A-Z]:\\Users\\[^\\]+\\/i, '~\\');
}
