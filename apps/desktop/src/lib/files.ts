import { isTauri } from './api';

/** Choose .inkdict files with the system dialog; empty when cancelled.
 *  Only in the app: a browser has no file paths to give. */
export async function pickPacks(): Promise<string[]> {
  if (!isTauri()) return [];
  const { open } = await import('@tauri-apps/plugin-dialog');
  const chosen = await open({ multiple: true, directory: false, filters: [{ name: 'Inkdict 词典', extensions: ['inkdict'] }] });
  if (!chosen) return [];
  return Array.isArray(chosen) ? chosen : [chosen];
}

/** Choose a directory with the system dialog; null when cancelled or in a browser. */
export async function pickFolder(): Promise<string | null> {
  if (!isTauri()) return null;
  const { open } = await import('@tauri-apps/plugin-dialog');
  const chosen = await open({ directory: true, multiple: false });
  return typeof chosen === 'string' ? chosen : null;
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
