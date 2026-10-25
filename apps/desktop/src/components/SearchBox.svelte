<script lang="ts">
  import type { Column } from '../lib/types';
  let { text, fulltext, column, searching, selectedId, oninput, onkeydown, ontoggle, oncolumn }: {
    text: string; fulltext: boolean; column: Column; searching: boolean; selectedId?: string;
    oninput: (text: string, started: number) => void; onkeydown: (event: KeyboardEvent) => void;
    ontoggle: () => void; oncolumn: (column: Column) => void;
  } = $props();
  let input: HTMLInputElement;
  export function focus() { input?.focus(); input?.select(); }
</script>

<div class="search-area">
  <div class="search-line">
    <div class="search-field">
      <span class="search-symbol" aria-hidden="true">⌕</span>
      <input bind:this={input} value={text} aria-label={fulltext ? '全文检索' : '查词'} placeholder={fulltext ? '搜索释义或例句…' : '输入要查的词…'}
        role="combobox" aria-autocomplete="list" aria-expanded="true" aria-controls="lookup-results" aria-activedescendant={selectedId}
        autocomplete="off" autocapitalize="off" spellcheck="false"
        oninput={event => oninput(event.currentTarget.value, performance.now())} {onkeydown} />
      {#if text}<button class="clear" aria-label="清空搜索" onclick={() => { oninput('', performance.now()); focus(); }}>×</button>
      {:else}<kbd class="search-shortcut">/</kbd>{/if}
    </div>
    <button class:active={fulltext} class="mode-button" aria-pressed={fulltext} title="全文检索（Ctrl+Shift+F）" onclick={ontoggle}>全文检索</button>
  </div>
  {#if fulltext}
    <div class="filter-row" aria-label="检索范围">
      {#each [[null, '全部'], ['text', '释义'], ['example', '例句']] as [value, label]}
        <button class:active={column === value} aria-pressed={column === value} onclick={() => oncolumn(value as Column)}>{label}</button>
      {/each}
      {#if searching}<span role="status">检索中…</span>{/if}
    </div>
  {/if}
</div>
