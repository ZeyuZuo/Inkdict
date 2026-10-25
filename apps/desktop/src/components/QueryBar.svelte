<script lang="ts">
  import { tick } from 'svelte';
  import type { Column, Result } from '../lib/types';

  let { text, fulltext, column, rows, open, corrected, searching, selection, settingsOpen,
    oninput, onkeydown, onchoose, ontoggle, oncolumn, onsettings, ondismiss, onreopen }: {
    text: string; fulltext: boolean; column: Column; rows: Result[]; open: boolean; corrected: boolean;
    searching: boolean; selection: number; settingsOpen: boolean;
    oninput: (text: string, started: number) => void; onkeydown: (event: KeyboardEvent) => void;
    onchoose: (index: number) => void; ontoggle: () => void; oncolumn: (column: Column) => void;
    onsettings: () => void; ondismiss: () => void; onreopen: () => void;
  } = $props();

  let input: HTMLInputElement;
  let list = $state<HTMLDivElement>();
  export function focus() { input?.focus(); input?.select(); }

  const showList = $derived(open && text.trim() !== '');
  $effect(() => {
    const selected = selection;
    void tick().then(() => list?.querySelector(`#result-${selected}`)?.scrollIntoView({ block: 'nearest' }));
  });

  /** The entry a form belongs to, when the form is not its headword (went: go). */
  function where(item: Result): string {
    return 'form' in item && item.form !== item.headword ? item.headword : '';
  }
  const columns: [Column, string][] = [[null, '全部'], ['text', '释义'], ['example', '例句']];
</script>

<header class="query">
  <div class="query-line">
    <input bind:this={input} class="query-input" value={text}
      aria-label={fulltext ? '全文检索' : '查词'} placeholder={fulltext ? '搜索释义和例句' : '输入单词'}
      role="combobox" aria-autocomplete="list" aria-expanded={showList} aria-controls="lookup-results"
      aria-activedescendant={showList && selection >= 0 ? `result-${selection}` : undefined}
      autocomplete="off" autocapitalize="off" spellcheck="false"
      oninput={event => oninput(event.currentTarget.value, performance.now())} {onkeydown}
      onfocus={onreopen} onclick={onreopen} />
    <button class="scope" class:on={fulltext} aria-pressed={fulltext} title="全文检索（Tab）" onclick={ontoggle}>全文</button>
    <button class="gear" class:on={settingsOpen} aria-label="设置" aria-expanded={settingsOpen} onclick={onsettings}>
      <svg viewBox="0 0 20 20" aria-hidden="true"><path d="M10 6.6a3.4 3.4 0 1 0 0 6.8 3.4 3.4 0 0 0 0-6.8Zm7.2 4.6-1.6.9a5.8 5.8 0 0 1-.6 1.4l.5 1.8-1.4 1.4-1.8-.5c-.4.3-.9.5-1.4.6l-.9 1.6h-2l-.9-1.6a5.8 5.8 0 0 1-1.4-.6l-1.8.5-1.4-1.4.5-1.8a5.8 5.8 0 0 1-.6-1.4l-1.6-.9v-2l1.6-.9c.1-.5.3-1 .6-1.4l-.5-1.8 1.4-1.4 1.8.5c.4-.3.9-.5 1.4-.6l.9-1.6h2l.9 1.6c.5.1 1 .3 1.4.6l1.8-.5 1.4 1.4-.5 1.8c.3.4.5.9.6 1.4l1.6.9v2Z"/></svg>
    </button>
  </div>

  {#if fulltext}
    <div class="scope-columns" role="group" aria-label="检索范围">
      {#each columns as [value, label]}
        <button class:on={column === value} aria-pressed={column === value} onclick={() => oncolumn(value)}>{label}</button>
      {/each}
    </div>
  {/if}

  {#if showList}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="suggestions" onmousedown={event => event.preventDefault()}>
      {#if corrected}<p class="suggestions-note">没有以“{text}”开头的词。拼写接近的有：</p>{/if}
      <div id="lookup-results" role="listbox" aria-label="查询结果" aria-busy={searching} bind:this={list}>
        {#each rows as item, index}
          <button id={`result-${index}`} type="button" role="option" class="row" class:selected={selection === index}
            aria-selected={selection === index} onclick={() => onchoose(index)}>
            <span class="row-word">{'form' in item ? item.form : item.headword}{#if item.homograph !== null}<sup>{item.homograph}</sup>{/if}</span>
            <span class="row-where">{where(item)}</span>
          </button>
        {/each}
      </div>
      {#if !rows.length && !searching}
        <p class="suggestions-note" role="status">{fulltext ? '释义和例句中没有找到这些词。' : '词典里没有这个词。'}</p>
      {/if}
    </div>
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="scrim" role="presentation" onclick={ondismiss}></div>
  {/if}
</header>
