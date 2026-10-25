<script lang="ts">
  import type { Settings } from '../lib/types';
  import type { ThemeMode } from '../lib/storage';
  import { isTauri } from '../lib/api';

  let { settings, theme, busy, packTitle, onsettings, ontheme, onpack, onclose }: {
    settings: Settings; theme: ThemeMode; busy: boolean; packTitle: string;
    onsettings: (settings: Settings) => void; ontheme: (theme: ThemeMode) => void;
    onpack: (path: string) => Promise<void>; onclose: () => void;
  } = $props();

  let path = $state('');
  let packBusy = $state(false);
  const shown: [keyof Settings, string][] = [['hide_chinese', '中文'], ['hide_pronunciation', '音标'], ['hide_examples', '例句']];
  const modes: [ThemeMode, string][] = [['system', '跟随系统'], ['light', '浅色'], ['dark', '深色']];
  const scale = (step: number) => Math.round(Math.min(1.6, Math.max(0.8, settings.font_scale + step)) * 10) / 10;

  async function open(chosen: string) {
    if (!chosen.trim()) return;
    packBusy = true;
    try { await onpack(chosen); } finally { packBusy = false; }
  }
  async function choose() {
    const { open: dialog } = await import('@tauri-apps/plugin-dialog');
    const chosen = await dialog({ multiple: false, directory: false, filters: [{ name: 'ZDict 词典包', extensions: ['zdpack'] }] });
    if (typeof chosen === 'string') await open(chosen);
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="sheet-scrim" role="presentation" onclick={onclose}></div>
<section class="sheet" aria-label="设置">
  <div class="sheet-group">
    <h2>显示</h2>
    {#each shown as [key, label]}
      <label class="switch-row">
        <span>{label}</span>
        <input type="checkbox" role="switch" disabled={busy} checked={!settings[key]}
          onchange={event => onsettings({ ...settings, [key]: !event.currentTarget.checked })} />
      </label>
    {/each}
    <div class="switch-row">
      <span>字号</span>
      <span class="stepper">
        <button aria-label="缩小字号" disabled={busy || settings.font_scale <= 0.8} onclick={() => onsettings({ ...settings, font_scale: scale(-0.1) })}>A</button>
        <output>{Math.round(settings.font_scale * 100)}%</output>
        <button aria-label="放大字号" class="larger" disabled={busy || settings.font_scale >= 1.6} onclick={() => onsettings({ ...settings, font_scale: scale(0.1) })}>A</button>
      </span>
    </div>
  </div>

  <div class="sheet-group">
    <h2>外观</h2>
    <div class="segmented" role="radiogroup" aria-label="外观">
      {#each modes as [value, label]}
        <button role="radio" aria-checked={theme === value} class:on={theme === value} onclick={() => ontheme(value)}>{label}</button>
      {/each}
    </div>
  </div>

  <div class="sheet-group">
    <h2>词典</h2>
    <p class="pack-name">{packTitle || '还没有打开词典包'}</p>
    {#if isTauri()}
      <button class="pack-button" disabled={packBusy} onclick={choose}>{packTitle ? '换一个词典包…' : '打开词典包…'}</button>
    {:else}
      <form class="pack-path" onsubmit={event => { event.preventDefault(); void open(path); }}>
        <input bind:value={path} placeholder="词典包路径（.zdpack）" aria-label="词典包路径" />
        <button disabled={packBusy || !path.trim()}>打开</button>
      </form>
    {/if}
  </div>
</section>
