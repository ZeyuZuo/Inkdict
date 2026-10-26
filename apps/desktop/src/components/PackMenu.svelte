<script lang="ts">
  import type { Library } from '../lib/types';
  import Icon from './Icon.svelte';
  let { library, title, onuse, onmanage }: {
    library: Library; title: string; onuse: (path: string) => void; onmanage: () => void;
  } = $props();
  let open = $state(false);
  let root: HTMLElement;

  function outside(event: MouseEvent) {
    if (open && !root.contains(event.target as Node)) open = false;
  }
  function choose(path: string) { open = false; onuse(path); }
</script>

<svelte:window onmousedown={outside} onkeydown={event => { if (open && event.key === 'Escape') { event.stopPropagation(); open = false; } }} />

<div bind:this={root} style="min-width:0">
  <button class="pack-button" aria-haspopup="menu" aria-expanded={open} onclick={() => { open = !open; }} aria-label="切换词典">
    <span>{title || '没有词典'}</span><Icon name="chevron" size={11} />
  </button>
  {#if open}
    <div class="menu" role="menu">
      {#each library.packs as pack (pack.path)}
        <button role="menuitemradio" aria-checked={pack.path === library.active} disabled={pack.error !== null}
          title={pack.error ?? pack.path} onclick={() => choose(pack.path)}>
          <span class="check">{#if pack.path === library.active}<Icon name="check" size={14} />{/if}</span>
          <span class="name">{pack.title ?? pack.path}</span>
        </button>
      {/each}
      {#if library.packs.length}<hr />{/if}
      <button role="menuitem" onclick={() => { open = false; onmanage(); }}><span class="check"></span>管理词典…</button>
    </div>
  {/if}
</div>
