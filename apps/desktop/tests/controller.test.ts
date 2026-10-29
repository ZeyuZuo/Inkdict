import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { api } from '../src/lib/api';
import { Dictionary } from '../src/lib/controller.svelte';
import { installTheme } from '../src/lib/theme';

vi.mock('../src/lib/theme', () => ({ installTheme: vi.fn() }));

beforeEach(async () => {
  vi.stubGlobal('window', {});
  vi.stubEnv('VITE_INKDICT_BRIDGE', '');
  await api.setTheme('default');
  await api.setSettings({ hide_pronunciation: true, hide_examples: false, font_scale: 1.2 });
  vi.mocked(installTheme).mockClear();
});
afterEach(() => { vi.unstubAllGlobals(); vi.unstubAllEnvs(); });

describe('pack theme selection with the browser mock', () => {
  it('updates the pack, installs its CSS and re-renders at the reading position', async () => {
    const dictionary = new Dictionary();
    dictionary.pack = await api.packInfo();
    await dictionary.open('moonjar');
    const original = dictionary.current!.html;
    const css = await api.themeCss();
    const history = structuredClone(dictionary.history);
    dictionary.view = () => ({ top: 240, tabs: [1] });

    await dictionary.setPackTheme('plain');

    expect(dictionary.pack!.theme).toBe('plain');
    expect(dictionary.pack!.themes).toEqual([
      { name: 'default', title: 'Default' }, { name: 'plain', title: 'Plain' },
    ]);
    expect(installTheme).toHaveBeenCalledExactlyOnceWith(await api.themeCss());
    expect(await api.themeCss()).not.toBe(css);
    expect(dictionary.current!.key).toBe('moonjar');
    expect(dictionary.current!.html).not.toBe(original);
    expect(dictionary.current!.html).not.toContain('dx-marker');
    expect(dictionary.current!.html).not.toContain('dx-pronunciation');
    expect(dictionary.position).toEqual({ kind: 'preserve', top: 240, tabs: [1] });
    expect(dictionary.history).toEqual(history);
    expect(dictionary.error).toBe('');
  });

  it('reports unknown themes and leaves the current pack and rendering alone', async () => {
    const dictionary = new Dictionary();
    dictionary.pack = await api.packInfo();
    await dictionary.open('moonjar');
    const original = dictionary.current;
    const css = await api.themeCss();

    await dictionary.setPackTheme('missing');

    expect(dictionary.error).toBe('unknown theme missing');
    expect(dictionary.pack!.theme).toBe('default');
    expect(dictionary.current).toBe(original);
    expect(await api.themeCss()).toBe(css);
    expect(installTheme).not.toHaveBeenCalled();
  });
});
