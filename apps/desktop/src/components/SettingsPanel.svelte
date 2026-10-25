<script lang="ts">
  import type { Settings } from '../lib/types';
  import type { ThemeMode } from '../lib/storage';
  import { isTauri } from '../lib/api';
  let { settings, theme, busy, mock, onsettings, ontheme, onpack, onclose }: {
    settings: Settings; theme: ThemeMode; busy: boolean; mock: boolean;
    onsettings: (settings: Settings) => void; ontheme: (theme: ThemeMode) => void;
    onpack: (path: string) => Promise<void>; onclose: () => void;
  } = $props();
  let path = $state('');
  let packBusy = $state(false);
  async function openPack(event: SubmitEvent) {
    event.preventDefault(); packBusy = true;
    try { await onpack(path); } finally { packBusy = false; }
  }
  /** Native file dialog (Tauri only); the chosen file opens at once. */
  async function choosePack() {
    const { open } = await import('@tauri-apps/plugin-dialog');
    const chosen = await open({ multiple: false, directory: false, filters: [{ name: 'ZDict 词典包', extensions: ['zdpack'] }] });
    if (typeof chosen !== 'string') return;
    path = chosen; packBusy = true;
    try { await onpack(chosen); } finally { packBusy = false; }
  }
</script>

<section id="settings-panel" class="settings-panel" aria-labelledby="settings-heading">
  <div class="panel-title"><h2 id="settings-heading">阅读设置</h2><button aria-label="关闭设置" onclick={onclose}>×</button></div>
  <div class="settings-grid">
    <fieldset disabled={busy}>
      <legend>词条显示</legend>
      {#each [['hide_chinese', '隐藏中文'], ['hide_pronunciation', '隐藏音标'], ['hide_examples', '隐藏例句']] as [key, label]}
        <label class="toggle-label"><input type="checkbox" checked={settings[key as keyof Settings] as boolean}
          onchange={event => onsettings({ ...settings, [key]: event.currentTarget.checked })} />{label}</label>
      {/each}
      <label class="scale-label" for="font-scale">字号 <output>{settings.font_scale.toFixed(1)}×</output></label>
      <input id="font-scale" type="range" min="0.8" max="1.6" step="0.1" value={settings.font_scale}
        onchange={event => onsettings({ ...settings, font_scale: Number(event.currentTarget.value) })} />
    </fieldset>
    <div class="settings-other">
      <label class="field-label" for="theme-mode">外观</label>
      <select id="theme-mode" value={theme} onchange={event => ontheme(event.currentTarget.value as ThemeMode)}>
        <option value="system">跟随系统</option><option value="light">浅色</option><option value="dark">深色</option>
      </select>
      <form onsubmit={openPack}>
        <label class="field-label" for="pack-path">词典包路径</label>
        <div class="pack-input"><input id="pack-path" bind:value={path} placeholder="/path/to/dictionary.zdpack" required /><button disabled={packBusy || !path.trim()}>打开</button></div>
        {#if isTauri()}<button type="button" class="pack-choose" disabled={packBusy} onclick={choosePack}>选择文件…</button>{/if}
        {#if mock}<p class="small-muted">演示模式使用本地原创词条。</p>{/if}
      </form>
    </div>
  </div>
</section>
