import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { api, apiError } from '../src/lib/api';

const settings = { hide_pronunciation: true, hide_examples: false, font_scale: 1.2 };

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
beforeEach(() => {
  vi.stubGlobal('window', { __TAURI_INTERNALS__: {} });
  vi.mocked(invoke).mockReset().mockResolvedValue(undefined);
});
afterEach(() => vi.unstubAllGlobals());

describe('Tauri command contract', () => {
  it('passes every command and argument using contract names', async () => {
    const link = { entry: 'moonjar', anchor_type: 'wfg', text: '词族' };
    await api.packInfo(); await api.library();
    await api.addPacks(['/a.zdpack', '/b.zdpack']); await api.removePack('/a.zdpack');
    await api.movePack('/b.zdpack', 0); await api.usePack('/b.zdpack');
    await api.words('moon'); await api.words('moon', 5);
    await api.entry('moonjar'); await api.follow('moonjar', link);
    await api.settings(); await api.setSettings(settings); await api.themeCss();
    expect(vi.mocked(invoke).mock.calls).toEqual([
      ['pack_info', undefined], ['library', undefined],
      ['add_packs', { paths: ['/a.zdpack', '/b.zdpack'] }], ['remove_pack', { path: '/a.zdpack' }],
      ['move_pack', { path: '/b.zdpack', index: 0 }], ['use_pack', { path: '/b.zdpack' }],
      ['words', { text: 'moon', limit: 30 }], ['words', { text: 'moon', limit: 5 }],
      ['entry', { key: 'moonjar' }], ['follow', { current: 'moonjar', link }],
      ['settings', undefined], ['set_settings', { settings }], ['theme_css', undefined],
    ]);
  });
  it('preserves structured command errors', async () => {
    const failure = { code: 'no_pack', message: '尚未打开词典包' };
    vi.mocked(invoke).mockRejectedValueOnce(failure);
    await expect(api.packInfo()).rejects.toEqual(failure);
  });
  it('normalizes unexpected errors to the contract shape', () => {
    expect(apiError(new Error('broken'))).toEqual({ code: 'internal', message: 'broken' });
    expect(apiError('broken')).toEqual({ code: 'internal', message: 'broken' });
  });
});

describe('browser stand-in', () => {
  it('lists prefix rows, then rows containing the input, one group each', async () => {
    vi.stubGlobal('window', {});
    const { mockCommand } = await import('../src/lib/mock');
    const rows = await mockCommand('words', { text: 'moon', limit: 10 });
    expect(rows.map(row => [row.group, row.form])).toEqual([['prefix', 'moonjar'], ['prefix', 'moonjar moment']]);
    const contains = await mockCommand('words', { text: 'leaf', limit: 10 });
    expect(contains.every(row => row.group === 'contains')).toBe(true);
    expect(await mockCommand('words', { text: ' ', limit: 10 })).toEqual([]);
  });
});
