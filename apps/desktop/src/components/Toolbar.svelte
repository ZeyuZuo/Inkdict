<script lang="ts">
  import type { PackInfo } from '../lib/types';
  let { pack, mock, canBack, canForward, settingsOpen, onback, onforward, onsettings }: {
    pack: PackInfo | null; mock: boolean; canBack: boolean; canForward: boolean; settingsOpen: boolean;
    onback: () => void; onforward: () => void; onsettings: () => void;
  } = $props();
</script>

<header class="toolbar">
  <div class="brand"><span class="brand-mark" aria-hidden="true">字</span><strong>ZDict</strong><span class="brand-caption">随手查，慢慢读。</span></div>
  <div class="toolbar-actions">
    <nav aria-label="查词历史">
      <button class="icon-button" aria-label="后退" title="后退（Alt+←）" disabled={!canBack} onclick={onback}>←</button>
      <button class="icon-button" aria-label="前进" title="前进（Alt+→）" disabled={!canForward} onclick={onforward}>→</button>
    </nav>
    <button class="settings-button" aria-expanded={settingsOpen} aria-controls="settings-panel" onclick={onsettings}>设置</button>
  </div>
</header>
<div class="pack-line"><span class="status-dot" aria-hidden="true"></span><span>{pack?.title ?? '尚未打开词典包'}</span>
  {#if pack}<span class="pack-count">{pack.entries.toLocaleString()} 词条</span>{/if}
  {#if mock}<span class="demo-label">演示 · 原创示例</span>{/if}
</div>
