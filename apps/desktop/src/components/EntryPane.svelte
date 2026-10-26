<script lang="ts">
  import { onMount, tick } from 'svelte';
  import type { Entry, Link, PackInfo } from '../lib/types';
  import type { EntryPosition } from '../lib/controller.svelte';
  import type { View } from '../lib/history';
  import { AudioPlayer, fallbackImage } from '../lib/media';
  import * as tabs from '../lib/tabs';
  import Icon from './Icon.svelte';
  let { entry, pack, position, from, started, scale, loading, onfollow, onback, onpaint, onerror, onadd }: {
    entry: Entry | null; pack: PackInfo | null; position: EntryPosition; from: string | null; started: number;
    scale: number; loading: boolean;
    onfollow: (link: Link) => void; onback: () => void; onpaint: (started: number) => void;
    onerror: (message: string) => void; onadd: () => void;
  } = $props();
  let pane: HTMLElement;
  let content: HTMLElement | undefined = $state();
  let generation = 0;
  const player = new AudioPlayer(message => onerror(message));

  export function view(): View {
    return { top: pane?.scrollTop ?? 0, tabs: content ? tabs.state(content) : [] };
  }

  onMount(() => {
    // Image errors do not bubble: capture them in the inserted entry HTML.
    pane.addEventListener('error', fallbackImage, true);
    return () => { pane.removeEventListener('error', fallbackImage, true); player.stop(); };
  });

  $effect(() => {
    const displayed = entry;
    const focus = position;
    const requestStarted = started;
    const token = ++generation;
    player.stop();
    let highlight: HTMLElement | null = null;
    let timer: ReturnType<typeof setTimeout> | undefined;
    void (async () => {
      await tick();
      requestAnimationFrame(() => {
        if (token !== generation || !displayed || !pane || !content) return;
        if (focus.kind === 'preserve') {
          tabs.restore(content, focus.tabs);
          pane.scrollTop = focus.top;
        } else if (focus.kind === 'top') pane.scrollTop = 0;
        else {
          const selector = focus.kind === 'word_family' ? '.dx-word_family' : `[data-anchor="${focus.number}"]`;
          highlight = content.querySelector<HTMLElement>(selector);
          if (highlight) {
            // The anchor may sit in a tab that is not open, or a folded box.
            tabs.reveal(highlight, content);
            highlight.scrollIntoView({ block: 'start' });
            highlight.classList.add('anchor-highlight');
            timer = setTimeout(() => highlight?.classList.remove('anchor-highlight'), 1400);
          } else pane.scrollTop = 0;
        }
        onpaint(requestStarted);
      });
    })();
    return () => { generation++; clearTimeout(timer); highlight?.classList.remove('anchor-highlight'); };
  });

  function clicked(event: MouseEvent) {
    if (!(event.target instanceof Element)) return;
    if (tabs.clicked(event.target)) return;
    const audio = event.target.closest<HTMLElement>('[data-audio]');
    if (audio) { event.preventDefault(); player.play((audio.dataset.audio ?? '').split(/\s+/).filter(Boolean)); return; }
    const link = event.target.closest<HTMLAnchorElement>('a.dx-link[data-link]');
    if (!link) return;
    event.preventDefault();
    try {
      const value: unknown = JSON.parse(link.dataset.link ?? '');
      if (!value || typeof value !== 'object' || !('text' in value) || typeof value.text !== 'string') throw new Error();
      onfollow(value as Link);
    } catch { onerror('词条里的链接格式有误。'); }
  }
  function keyed(event: KeyboardEvent) {
    if (tabs.keyed(event)) event.preventDefault();
  }
</script>

<main class="pane" aria-label="词条" aria-busy={loading} bind:this={pane}>
  {#if entry}
    <div class="column">
      {#if from}<button class="from" onclick={onback}><Icon name="back" size={14} />{from}</button>{/if}
      <!-- Clicks and keys of the entry HTML (links, audio, tabs) are delegated here. -->
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
      <article class="entry" style={`--scale:${scale}`} bind:this={content} onclick={clicked} onkeydown={keyed}>{@html entry.html}</article>
    </div>
  {:else if pack}
    <div class="title-page">
      <h1>{pack.title}</h1>
      {#if pack.entries !== null}<p>{pack.entries.toLocaleString('zh-CN')} 个词条</p>{/if}
    </div>
  {:else}
    <div class="title-page">
      <h1>还没有词典</h1>
      <p>添加一部 .zdpack 词典后就可以查词。</p>
      <button class="button" onclick={onadd}>添加词典…</button>
    </div>
  {/if}
</main>
