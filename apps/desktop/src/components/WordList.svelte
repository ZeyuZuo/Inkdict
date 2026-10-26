<script lang="ts">
  import { tick } from 'svelte';
  import type { Word } from '../lib/types';
  import { detail, sections } from '../lib/words';
  let { words, searched, selection, onchoose }: {
    words: Word[]; searched: string; selection: number; onchoose: (index: number) => void;
  } = $props();
  let list: HTMLElement;
  const groups = $derived(sections(words, searched));

  // Keep the selected row in view while moving with ↑↓.
  $effect(() => {
    const index = selection;
    void tick().then(() => list?.querySelector(`[data-index="${index}"]`)?.scrollIntoView({ block: 'nearest' }));
  });
</script>

<div class="list" id="word-list" role="listbox" aria-label="查询结果" bind:this={list}>
  {#each groups as section (section.start)}
    {#if section.label}<div class="group" role="presentation">{section.label}</div>{/if}
    {#each section.rows as word, offset (word.entry + '\u0000' + word.form + '\u0000' + word.group)}
      {@const index = section.start + offset}
      <button class="row" class:on={index === selection} role="option" aria-selected={index === selection} data-index={index}
        tabindex="-1" onmousedown={event => event.preventDefault()} onclick={() => onchoose(index)}>
        <!-- The homograph number belongs to the headword: after the form, or after the headword it points to. -->
        <div class="w">{word.form}{#if word.homograph && !word.target}<sup>{word.homograph}</sup>{/if}{#if detail(word)}<span class="to">{detail(word)}{#if word.homograph && word.target}<sup>{word.homograph}</sup>{/if}</span>{/if}</div>
        {#if word.summary}<div class="g" lang={word.summary_lang ?? undefined}>{word.summary}</div>{/if}
      </button>
    {/each}
  {:else}
    {#if searched.trim()}<div class="empty">没有找到 {searched.trim()}</div>{/if}
  {/each}
</div>
