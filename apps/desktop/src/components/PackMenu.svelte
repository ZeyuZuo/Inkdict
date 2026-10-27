<script lang="ts">
  import { tick } from 'svelte';
  import type { Library } from '../lib/types';
  import Icon from './Icon.svelte';
  let { library, title, onuse, onmanage }: {
    library: Library; title: string; onuse: (path: string) => void; onmanage: () => void;
  } = $props();
  let open = $state(false);
  let root: HTMLElement;
  let button: HTMLButtonElement;

  function outside(event: MouseEvent) {
    if (open && !root.contains(event.target as Node)) open = false;
  }
  function choose(path: string) { open = false; onuse(path); }
  // Focus goes into the menu, so Escape reaches this element before the
  // window's handler (which would move focus to the search box).
  async function toggle() {
    open = !open;
    if (!open) return;
    await tick();
    root.querySelector<HTMLElement>('[role="menu"] [aria-checked="true"], [role="menu"] button:not(:disabled)')?.focus();
  }
  function key(event: KeyboardEvent) {
    if (!open || event.key !== 'Escape') return;
    event.stopPropagation();
    open = false;
    button.focus();
  }
</script>

<svelte:window onmousedown={outside} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div bind:this={root} style="min-width:0" onkeydown={key}>
  <button class="pack-button" aria-haspopup="menu" aria-expanded={open} onclick={() => void toggle()} aria-label="切换词典" bind:this={button}>
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
