<script lang="ts">
  import Icon from './Icon.svelte';
  let { text, disabled, oninput, onkeydown, onclear, onfocus }: {
    text: string; disabled: boolean;
    oninput: (text: string, started: number) => void; onkeydown: (event: KeyboardEvent) => void;
    onclear: () => void; onfocus: () => void;
  } = $props();
  let input: HTMLInputElement;
  export function focus(select = false) { input?.focus(); if (select) input?.select(); }
</script>

<label class="field">
  <Icon name="search" />
  <!-- 200 characters: longer input is not a word, and keys cost time to fold. -->
  <input bind:this={input} value={text} {disabled} maxlength="200" placeholder="查词" aria-label="查词"
    autocomplete="off" autocapitalize="off" spellcheck="false" role="combobox" aria-controls="word-list" aria-expanded={text !== ''}
    oninput={event => oninput(event.currentTarget.value, performance.now())} {onkeydown} {onfocus} />
  {#if text}<button class="clear" aria-label="清空" onclick={() => { onclear(); input.focus(); }}><Icon name="clear" size={14} /></button>{/if}
</label>
