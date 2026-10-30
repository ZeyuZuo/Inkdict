<script lang="ts">
  import { tick } from 'svelte';
  import type { Library, PackInfo } from '../lib/types';
  import Icon from './Icon.svelte';
  let { library, pack, compact = false, onuse, onpacktheme, onmanage }: {
    library: Library; pack: PackInfo | null;
    /** Narrow window: an icon button in the top bar, the menu opening downwards. */
    compact?: boolean;
    onuse: (path: string) => void; onpacktheme: (name: string) => void; onmanage: () => void;
  } = $props();
  let open = $state(false);
  let root: HTMLElement;
  let button: HTMLButtonElement | undefined = $state();
  const title = $derived(pack?.title ?? '');

  function outside(event: MouseEvent) {
    if (open && !root.contains(event.target as Node)) open = false;
  }
  function choose(path: string) { open = false; onuse(path); }
  function style(name: string) { open = false; onpacktheme(name); }
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
    button?.focus();
  }
</script>

<svelte:window onmousedown={outside} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div bind:this={root} style="min-width:0" onkeydown={key}>
  {#if compact}
    <button class="icon-button" aria-haspopup="menu" aria-expanded={open} onclick={() => void toggle()}
      aria-label="切换词典和样式" title={title || '没有词典'} bind:this={button}><Icon name="book" size={18} /></button>
  {:else}
    <button class="pack-button" aria-haspopup="menu" aria-expanded={open} onclick={() => void toggle()} aria-label="切换词典和样式" bind:this={button}>
      <span>{title || '没有词典'}</span><Icon name="chevron" size={11} />
    </button>
  {/if}
  {#if open}
    <div class="menu" class:down={compact} role="menu">
      {#each library.packs as item (item.path)}
        <button role="menuitemradio" aria-checked={item.path === library.active} disabled={item.error !== null}
          title={item.error ?? item.path} onclick={() => choose(item.path)}>
          <span class="check">{#if item.path === library.active}<Icon name="check" size={14} />{/if}</span>
          <span class="name">{item.title ?? item.path}</span>
        </button>
      {/each}
      <!-- The open dictionary's own themes (spec/v1.md §4); one theme needs no choice. -->
      {#if pack && pack.themes.length > 1}
        <hr />
        <div class="label" id="style-label">词条样式</div>
        <div role="group" aria-labelledby="style-label">
          {#each pack.themes as item (item.name)}
            <button role="menuitemradio" aria-checked={pack.theme === item.name} onclick={() => style(item.name)}>
              <span class="check">{#if pack.theme === item.name}<Icon name="check" size={14} />{/if}</span>
              <span class="name">{item.title}</span>
            </button>
          {/each}
        </div>
      {/if}
      {#if library.packs.length}<hr />{/if}
      <button role="menuitem" onclick={() => { open = false; onmanage(); }}><span class="check"></span>管理词典…</button>
    </div>
  {/if}
</div>
