import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { api, apiError } from '../src/lib/api';
import { defaults } from '../src/lib/storage';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
beforeEach(() => {
  vi.stubGlobal('window', { __TAURI_INTERNALS__: {} });
  vi.mocked(invoke).mockReset().mockResolvedValue(undefined);
});
afterEach(() => vi.unstubAllGlobals());

describe('Tauri command contract', () => {
  it('passes every command and argument using contract names', async () => {
    const link = { entry: 'moonjar', anchor_type: 'wfg', text: '词族' };
    await api.packInfo(); await api.openPack('/tmp/demo.zdpack');
    await api.suggest('moon'); await api.correct('monjar');
    await api.search('quiet'); await api.search('wish', 10, 'example');
    await api.entry('moonjar'); await api.follow('moonjar', link);
    await api.settings(); await api.setSettings(defaults); await api.themeCss();
    expect(vi.mocked(invoke).mock.calls).toEqual([
      ['pack_info', undefined], ['open_pack', { path: '/tmp/demo.zdpack' }],
      ['suggest', { text: 'moon', limit: 30 }], ['correct', { text: 'monjar', limit: 10 }],
      ['search', { text: 'quiet', limit: 30, column: null }], ['search', { text: 'wish', limit: 10, column: 'example' }],
      ['entry', { key: 'moonjar' }], ['follow', { current: 'moonjar', link }],
      ['settings', undefined], ['set_settings', { settings: defaults }], ['theme_css', undefined],
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
