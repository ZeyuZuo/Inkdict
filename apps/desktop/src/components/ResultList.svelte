<script lang="ts">
  import { tick } from 'svelte';
  import type { Result } from '../lib/types';
  import type { Visit } from '../lib/history';
  let { rows, recent, empty, corrected, searching, selection, onchoose }: {
    rows: Result[]; recent: Visit[]; empty: boolean; corrected: boolean; searching: boolean;
    selection: number; onchoose: (index: number) => void;
  } = $props();
  let list: HTMLDivElement;
  const choices = $derived(empty ? recent : rows);
  $effect(() => {
    const selected = selection;
    void tick().then(() => list?.querySelector(`#result-${selected}`)?.scrollIntoView({ block: 'nearest' }));
  });
</script>

<aside class="results-pane" aria-label={empty ? '最近查过' : '查询结果'}>
  <div class="list-heading"><span>{empty ? '最近查过' : corrected ? '你是不是要找：' : '查询结果'}</span><span>{choices.length || ''}</span></div>
  <div id="lookup-results" class="result-scroll" role="listbox" aria-label={empty ? '最近查过' : '查询结果'} aria-busy={searching} bind:this={list}>
    {#each choices as item, index}
      <button id={`result-${index}`} type="button" class="result" class:selected={selection === index} role="option"
        aria-selected={selection === index} onclick={() => onchoose(index)}>
        <span class="result-word">{'form' in item ? item.form : item.headword}{#if 'homograph' in item && item.homograph !== null}<sup>{item.homograph}</sup>{/if}</span>
        <span class="result-meta">
          {#if 'lang' in item}<span>{item.lang}</span>{:else if 'form' in item && item.form !== item.headword}<span>{item.headword}</span>{:else}<span>{empty ? '最近查阅' : '词条'}</span>{/if}
          {#if 'anchor' in item && item.anchor !== 0}<span>词条内 · #{item.anchor}</span>{/if}
        </span>
      </button>
    {/each}
    {#if !choices.length}
      <p class="list-empty" role="status">{empty ? '查过的词会留在这里。' : searching ? '正在查找…' : '没有找到匹配的词。'}</p>
    {/if}
  </div>
  <div class="list-hint"><kbd>↑</kbd><kbd>↓</kbd> 选择 <span><kbd>Enter</kbd> 打开</span></div>
</aside>
