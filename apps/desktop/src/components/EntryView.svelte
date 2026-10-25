<script lang="ts">
  import { onMount, tick } from 'svelte';
  import type { Entry, Link } from '../lib/types';
  import type { EntryPosition } from '../lib/controller.svelte';
  import { AudioPlayer, fallbackImage } from '../lib/media';
  let { entry, position, started, scale, loading, mock, onfollow, onpaint, onerror, onlookup }: {
    entry: Entry | null; position: EntryPosition; started: number; scale: number; loading: boolean; mock: boolean;
    onfollow: (link: Link) => void; onpaint: (started: number) => void;
    onerror: (message: string) => void; onlookup: (text: string) => void;
  } = $props();
  let pane: HTMLElement;
  let generation = 0;
  const player = new AudioPlayer(message => onerror(message));
  export function scrollTop() { return pane?.scrollTop ?? 0; }

  onMount(() => {
    // Image error does not bubble. Capture catches errors in inserted entry HTML.
    pane.addEventListener('error', fallbackImage, true);
    pane.addEventListener('click', clicked);
    return () => { pane.removeEventListener('error', fallbackImage, true); pane.removeEventListener('click', clicked); player.stop(); };
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
        if (token !== generation || !displayed || !pane) return;
        if (focus.kind === 'preserve') pane.scrollTop = focus.top;
        else if (focus.kind === 'top') pane.scrollTop = 0;
        else {
          const selector = focus.kind === 'word_family' ? '.dx-word_family' : `[data-anchor="${focus.number}"]`;
          highlight = pane.querySelector<HTMLElement>(selector);
          if (highlight) {
            // A requested anchor may sit inside a folded example or box.
            for (let parent = highlight.parentElement; parent && parent !== pane; parent = parent.parentElement) {
              if (parent instanceof HTMLDetailsElement) parent.open = true;
            }
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
    const audio = event.target.closest<HTMLElement>('[data-audio]');
    if (audio) { event.preventDefault(); player.play((audio.dataset.audio ?? '').split(/\s+/).filter(Boolean)); return; }
    const link = event.target.closest<HTMLAnchorElement>('a.dx-link[data-link]');
    if (!link) return;
    event.preventDefault();
    try {
      const value: unknown = JSON.parse(link.dataset.link ?? '');
      if (!value || typeof value !== 'object' || !('text' in value) || typeof value.text !== 'string') throw new Error();
      onfollow(value as Link);
    } catch { onerror('词条链接格式有误。'); }
  }
</script>

<section class="entry-pane" aria-label="词条" aria-busy={loading} bind:this={pane} style={`--entry-font-scale:${scale}`}>
  {#if loading}<div class="entry-loading" role="status">正在打开…</div>{/if}
  {#if entry}
    <div class="entry-content"><div class="entry-eyebrow">{entry.headword}{#if entry.homograph !== null}<sup>{entry.homograph}</sup>{/if}<span>词条</span></div>{@html entry.html}
      {#if mock}<p class="entry-footnote">本词条为界面演示创作，释义、例句与声音均为示例。</p>{/if}
    </div>
  {:else}
    <div class="welcome">
      <span class="welcome-mark" aria-hidden="true">Aa</span>
      <p class="eyebrow">一个词，一点新发现</p>
      <h1>从好奇开始。</h1>
      <p>输入一个词，找到它的意思，<br />也看看它与其他词的联系。</p>
      {#if mock}<div class="starter-words"><span>试着查查</span>{#each ['moonjar', 'hushleaf', 'fernlight'] as word}<button onclick={() => onlookup(word)}>{word} ↗</button>{/each}</div>{/if}
      <p class="welcome-tip"><kbd>/</kbd> 聚焦搜索 <span>·</span> <kbd>Ctrl</kbd> + <kbd>L</kbd></p>
    </div>
  {/if}
</section>
