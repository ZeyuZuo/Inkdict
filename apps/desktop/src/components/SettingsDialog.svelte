<script lang="ts">
  import { onMount, tick } from 'svelte';
  import type { Library, PackInfo, Settings } from '../lib/types';
  import type { ThemeMode } from '../lib/storage';
  import { isTauri } from '../lib/api';
  import { shortPath, size } from '../lib/files';
  import Icon from './Icon.svelte';
  let { library, pack, settings, theme, onadd, onaddpath, onremove, onmove, onuse, onsettings, onpacktheme, ontheme, onclose }: {
    library: Library; pack: PackInfo | null; settings: Settings; theme: ThemeMode;
    onadd: () => void; onaddpath: (path: string) => void; onremove: (path: string) => void;
    onmove: (path: string, index: number) => void; onuse: (path: string) => Promise<void>;
    onsettings: (settings: Settings) => void; onpacktheme: (name: string) => void;
    ontheme: (mode: ThemeMode) => void; onclose: () => void;
  } = $props();
  let dialog: HTMLElement;
  let typed = $state('');
  let dragging = $state<string | null>(null);
  /** Pack whose grip moved it with the keyboard: its grip gets focus back. */
  let moved: string | null = null;
  const scales = [0.8, 0.9, 1, 1.1, 1.2, 1.3, 1.4, 1.5, 1.6];
  const scale = $derived(scales.reduce((best, s) => Math.abs(s - settings.font_scale) < Math.abs(best - settings.font_scale) ? s : best));

  onMount(() => dialog.querySelector<HTMLElement>('button, input')?.focus());

  function step(direction: -1 | 1) {
    const next = scales[scales.indexOf(scale) + direction];
    if (next !== undefined) onsettings({ ...settings, font_scale: next });
  }
  function meta(item: Library['packs'][number]) {
    if (item.error) return item.error;
    const parts = [item.entries !== null ? `${item.entries.toLocaleString('zh-CN')} 词条` : '', size(item.bytes), shortPath(item.path)];
    return parts.filter(Boolean).join('，');
  }
  // Reorder by dragging the grip; the row under the pointer takes its place.
  function drop(event: DragEvent, index: number) {
    event.preventDefault();
    if (dragging) onmove(dragging, index);
    dragging = null;
  }
  // ↑↓ on a grip move its pack; the row is re-rendered, so focus follows it.
  function moveKey(event: KeyboardEvent, path: string, index: number) {
    const to = { ArrowUp: index - 1, ArrowDown: index + 1 }[event.key];
    if (to === undefined) return;
    event.preventDefault();
    if (to < 0 || to >= library.packs.length) return;
    moved = path;
    onmove(path, to);
  }
  $effect(() => {
    const index = library.packs.findIndex(item => item.path === moved);
    if (index < 0) return;
    moved = null;
    void tick().then(() => dialog.querySelectorAll<HTMLElement>('.grip')[index]?.focus());
  });
  // The checked radio follows the library, also when a switch failed.
  async function use(path: string) {
    await onuse(path);
    for (const [i, radio] of dialog.querySelectorAll<HTMLInputElement>('input[name="active-pack"]').entries()) {
      radio.checked = library.packs[i]?.path === library.active;
    }
  }
  function trap(event: KeyboardEvent) {
    if (event.key === 'Escape') { event.stopPropagation(); onclose(); return; }
    if (event.key !== 'Tab') return;
    const items = [...dialog.querySelectorAll<HTMLElement>('button:not(:disabled), input:not(:disabled)')];
    const [first, last] = [items[0], items[items.length - 1]];
    if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last.focus(); }
    else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first.focus(); }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="dim" onclick={event => { if (event.target === event.currentTarget) onclose(); }}>
  <div class="dialog" role="dialog" tabindex="-1" aria-modal="true" aria-labelledby="settings-title" bind:this={dialog} onkeydown={trap}>
    <div class="head"><h2 id="settings-title">设置</h2>
      <button class="icon-button" aria-label="关闭" onclick={onclose}><Icon name="clear" size={15} /></button>
    </div>

    <section aria-labelledby="packs-title">
      <h3 id="packs-title">词典</h3>
      {#each library.packs as item, index (item.path)}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="pack" class:dragging={dragging === item.path}
          ondragover={event => event.preventDefault()} ondrop={event => drop(event, index)}>
          <button class="grip" draggable="true" title="拖动或按 ↑↓ 调整顺序" aria-label={`调整 ${item.title ?? item.path} 的顺序`}
            ondragstart={event => { dragging = item.path; event.dataTransfer?.setData('text/plain', item.path); }}
            ondragend={() => { dragging = null; }} onkeydown={event => moveKey(event, item.path, index)}><Icon name="grip" size={14} /></button>
          <input type="radio" name="active-pack" checked={item.path === library.active} disabled={item.error !== null}
            aria-label={`使用 ${item.title ?? item.path}`} onchange={() => void use(item.path)} />
          <div style="min-width:0">
            <div class="name">{item.title ?? shortPath(item.path)}</div>
            <div class="meta" class:bad={item.error !== null} title={item.path}>{meta(item)}</div>
          </div>
          <button class="remove" onclick={() => onremove(item.path)}>移除</button>
        </div>
      {/each}
      <div class="add">
        {#if isTauri()}
          <button class="button" onclick={onadd}>添加词典…</button><span class="hint">可以一次选择多个 .inkdict 文件</span>
        {:else}
          <!-- In a browser there is no file dialog: type the path the bridge can open. -->
          <input bind:value={typed} placeholder="词典文件路径（.inkdict）" aria-label="词典文件路径"
            onkeydown={event => { if (event.key === 'Enter' && typed.trim()) { onaddpath(typed.trim()); typed = ''; } }} />
          <button class="button" disabled={!typed.trim()} onclick={() => { onaddpath(typed.trim()); typed = ''; }}>添加</button>
        {/if}
      </div>
    </section>

    <!-- The open dictionary's own themes (spec/v1.md §4); one theme needs no choice. -->
    {#if pack && pack.themes.length > 1}
      <section aria-labelledby="style-title">
        <h3 id="style-title">词条样式</h3>
        <div class="segments" role="radiogroup" aria-labelledby="style-title"
          style:grid-template-columns={`repeat(${pack.themes.length}, minmax(0, 1fr))`}
          style:width={`min(${pack.themes.length * 90}px, 100%)`}>
          {#each pack.themes as item (item.name)}
            <button role="radio" aria-checked={pack.theme === item.name} onclick={() => onpacktheme(item.name)}>{item.title}</button>
          {/each}
        </div>
      </section>
    {/if}

    <section aria-labelledby="display-title">
      <h3 id="display-title">显示</h3>
      <!-- Only the switches the pack has a role for (spec/v1.md §3.2). -->
      {#if pack?.switches.pronunciation}
        <div class="line"><span id="pron-label">音标</span>
          <button class="switch" role="switch" aria-labelledby="pron-label" aria-checked={!settings.hide_pronunciation}
            onclick={() => onsettings({ ...settings, hide_pronunciation: !settings.hide_pronunciation })}></button></div>
      {/if}
      {#if pack?.switches.example}
        <div class="line"><span id="example-label">例句</span>
          <button class="switch" role="switch" aria-labelledby="example-label" aria-checked={!settings.hide_examples}
            onclick={() => onsettings({ ...settings, hide_examples: !settings.hide_examples })}></button></div>
      {/if}
      <div class="line"><span>字号</span>
        <span class="step"><button aria-label="缩小" disabled={scale <= scales[0]} onclick={() => step(-1)}>−</button>
          <span>{Math.round(scale * 100)}%</span>
          <button aria-label="放大" disabled={scale >= scales[scales.length - 1]} onclick={() => step(1)}>+</button></span></div>
    </section>

    <section aria-labelledby="look-title">
      <h3 id="look-title">外观</h3>
      <div class="segments" role="radiogroup" aria-labelledby="look-title">
        {#each [['system', '跟随系统'], ['light', '浅色'], ['dark', '深色']] as [mode, name]}
          <button role="radio" aria-checked={theme === mode} onclick={() => ontheme(mode as ThemeMode)}>{name}</button>
        {/each}
      </div>
    </section>
  </div>
</div>
