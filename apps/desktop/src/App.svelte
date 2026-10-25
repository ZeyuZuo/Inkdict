<script lang="ts">
  import { onMount } from 'svelte';
  import { Dictionary } from './lib/controller.svelte';
  import { isTauri } from './lib/api';
  import { runBench } from './lib/bench';
  import QueryBar from './components/QueryBar.svelte';
  import EntryView from './components/EntryView.svelte';
  import SettingsPanel from './components/SettingsPanel.svelte';
  import PerfPanel from './components/PerfPanel.svelte';

  const dictionary = new Dictionary();
  const mock = !isTauri() && !import.meta.env.VITE_ZDICT_BRIDGE;
  let query: QueryBar;
  let entryView: EntryView;

  onMount(() => { query.focus(); void dictionary.initialize().then(() => runBench(dictionary)); });

  function queryKey(event: KeyboardEvent) {
    if (event.isComposing || event.ctrlKey || event.metaKey || event.altKey) return;
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      dictionary.listOpen = dictionary.text.trim() !== '';
      dictionary.select(event.key === 'ArrowDown' ? 1 : -1);
    } else if (event.key === 'Enter') {
      event.preventDefault(); void dictionary.choose();
    } else if (event.key === 'Tab' && !event.shiftKey) {
      event.preventDefault(); dictionary.changeMode();
    }
  }

  function globalKey(event: KeyboardEvent) {
    if (event.isComposing) return;
    const key = event.key.toLowerCase();
    const mod = event.ctrlKey || event.metaKey;
    const target = event.target;
    const typing = target instanceof HTMLElement && (target.matches('input,textarea,select') || target.isContentEditable);
    if (mod && event.shiftKey && key === 'd') {
      event.preventDefault(); dictionary.showPerf = !dictionary.showPerf;
    } else if (mod && key === 'l') {
      event.preventDefault(); query.focus();
    } else if (event.altKey && (key === 'arrowleft' || key === 'arrowright')) {
      // Kept without any button: return to the entry a link left.
      event.preventDefault(); void dictionary.navigate(key === 'arrowleft' ? -1 : 1);
    } else if (key === 'escape') {
      event.preventDefault();
      if (dictionary.listOpen) dictionary.listOpen = false;
      else if (dictionary.showSettings) dictionary.showSettings = false;
      else if (dictionary.text) void dictionary.lookup('');
      query.focus();
    } else if (!typing && !mod && !event.altKey && event.key.length === 1 && event.key !== ' ') {
      // Start typing anywhere: the keystroke goes to the query.
      query.focus();
    }
  }

  function mouseNavigation(event: MouseEvent) {
    if (event.button !== 3 && event.button !== 4) return;
    event.preventDefault(); void dictionary.navigate(event.button === 3 ? -1 : 1);
  }
  function preventMouseHistory(event: MouseEvent) {
    if (event.button === 3 || event.button === 4) event.preventDefault();
  }
  function closeSettings() { dictionary.showSettings = false; query.focus(); }
  async function starter(text: string) {
    const result = await dictionary.lookup(text);
    if (result) await dictionary.choose(0);
  }
</script>

<svelte:window onkeydown={globalKey} onmousedown={mouseNavigation} onmouseup={preventMouseHistory} onauxclick={preventMouseHistory} />

<div class="app">
  <QueryBar bind:this={query} text={dictionary.text} fulltext={dictionary.fulltext} column={dictionary.column}
    rows={dictionary.choices} open={dictionary.listOpen} corrected={dictionary.corrected} searching={dictionary.searching}
    selection={dictionary.selection} settingsOpen={dictionary.showSettings}
    oninput={(text, started) => void dictionary.lookup(text, started)} onkeydown={queryKey}
    onchoose={index => { dictionary.selection = index; void dictionary.choose(index); }}
    ontoggle={() => { dictionary.changeMode(); query.focus(); }} oncolumn={column => dictionary.changeColumn(column)}
    onsettings={() => { dictionary.showSettings = !dictionary.showSettings; }}
    ondismiss={() => { dictionary.listOpen = false; }}
    onreopen={() => { if (dictionary.text.trim()) dictionary.listOpen = true; }} />

  {#if dictionary.showSettings}
    <SettingsPanel settings={dictionary.settings} theme={dictionary.theme} busy={dictionary.settingsBusy}
      packTitle={dictionary.pack?.title ?? ''}
      onsettings={settings => void dictionary.updateSettings(settings, entryView.scrollTop())} ontheme={mode => dictionary.setTheme(mode)}
      onpack={path => dictionary.openPack(path)} onclose={closeSettings} />
  {/if}

  {#if dictionary.error}
    <div class="notice" role="alert"><span>{dictionary.error}</span><button aria-label="关闭提示" onclick={() => { dictionary.error = ''; }}>×</button></div>
  {/if}

  <EntryView bind:this={entryView} entry={dictionary.current} pack={dictionary.pack} position={dictionary.position}
    started={dictionary.entryStarted} scale={dictionary.settings.font_scale} loading={dictionary.loading} {mock}
    onfollow={link => void dictionary.follow(link)} onpaint={started => dictionary.measure('entry', started)}
    onerror={message => { dictionary.error = message; }} onlookup={text => void starter(text)} />
</div>
{#if dictionary.showPerf}<PerfPanel samples={dictionary.samples} version={dictionary.perfVersion} onclose={() => { dictionary.showPerf = false; }} />{/if}
