<script lang="ts">
  import { onMount } from 'svelte';
  import { Dictionary } from './lib/controller.svelte';
  import { isTauri } from './lib/api';
  import Toolbar from './components/Toolbar.svelte';
  import SearchBox from './components/SearchBox.svelte';
  import ResultList from './components/ResultList.svelte';
  import EntryView from './components/EntryView.svelte';
  import SettingsPanel from './components/SettingsPanel.svelte';
  import PerfPanel from './components/PerfPanel.svelte';

  const dictionary = new Dictionary();
  const mock = !isTauri();
  let searchBox: SearchBox;
  let entryView: EntryView;
  const empty = $derived(!dictionary.text.trim());
  const selectedId = $derived(dictionary.selection >= 0 ? `result-${dictionary.selection}` : undefined);

  onMount(() => { searchBox.focus(); void dictionary.initialize(); });
  function searchKey(event: KeyboardEvent) {
    if (event.isComposing || event.altKey || event.ctrlKey || event.metaKey) return;
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault(); dictionary.select(event.key === 'ArrowDown' ? 1 : -1);
    } else if (event.key === 'Enter') { event.preventDefault(); void dictionary.choose(); }
  }
  function globalKey(event: KeyboardEvent) {
    if (event.isComposing) return;
    const key = event.key.toLowerCase();
    if ((event.ctrlKey || event.metaKey) && event.shiftKey && key === 'd') {
      event.preventDefault(); dictionary.showPerf = !dictionary.showPerf;
    } else if ((event.ctrlKey || event.metaKey) && event.shiftKey && key === 'f') {
      event.preventDefault(); dictionary.changeMode(); searchBox.focus();
    } else if ((event.ctrlKey || event.metaKey) && key === 'l') {
      event.preventDefault(); searchBox.focus();
    } else if (event.altKey && (key === 'arrowleft' || key === 'arrowright')) {
      event.preventDefault(); void dictionary.navigate(key === 'arrowleft' ? -1 : 1);
    } else if (key === 'escape') {
      event.preventDefault();
      if (dictionary.showSettings) dictionary.showSettings = false;
      else if (dictionary.text) void dictionary.lookup('');
      searchBox.focus();
    } else if (key === '/' && !event.ctrlKey && !event.metaKey && !event.altKey) {
      const target = event.target;
      if (target instanceof HTMLElement && (target.matches('input,textarea,select') || target.isContentEditable)) return;
      event.preventDefault(); searchBox.focus();
    }
  }
  function mouseNavigation(event: MouseEvent) {
    if (event.button !== 3 && event.button !== 4) return;
    event.preventDefault(); void dictionary.navigate(event.button === 3 ? -1 : 1);
  }
  function preventMouseHistory(event: MouseEvent) {
    if (event.button === 3 || event.button === 4) event.preventDefault();
  }
  function closeSettings() { dictionary.showSettings = false; searchBox.focus(); }
  async function starter(text: string) {
    const result = await dictionary.lookup(text);
    if (result) await dictionary.choose(0);
  }
</script>

<svelte:window onkeydown={globalKey} onmousedown={mouseNavigation} onmouseup={preventMouseHistory} onauxclick={preventMouseHistory} />

<div class="app-shell">
  <Toolbar pack={dictionary.pack} {mock} canBack={dictionary.canBack} canForward={dictionary.canForward} settingsOpen={dictionary.showSettings}
    onback={() => void dictionary.navigate(-1)} onforward={() => void dictionary.navigate(1)} onsettings={() => { dictionary.showSettings = !dictionary.showSettings; }} />
  <SearchBox bind:this={searchBox} text={dictionary.text} fulltext={dictionary.fulltext} column={dictionary.column} searching={dictionary.searching} {selectedId}
    oninput={(text, started) => void dictionary.lookup(text, started)} onkeydown={searchKey}
    ontoggle={() => { dictionary.changeMode(); searchBox.focus(); }} oncolumn={column => dictionary.changeColumn(column)} />
  {#if dictionary.showSettings}
    <SettingsPanel settings={dictionary.settings} theme={dictionary.theme} busy={dictionary.settingsBusy} {mock}
      onsettings={settings => void dictionary.updateSettings(settings, entryView.scrollTop())} ontheme={mode => dictionary.setTheme(mode)}
      onpack={path => dictionary.openPack(path)} onclose={closeSettings} />
  {/if}
  {#if dictionary.error}<div class="error-banner" role="alert"><span>{dictionary.error}</span><button aria-label="关闭提示" onclick={() => { dictionary.error = ''; }}>×</button></div>{/if}
  <main class="workspace">
    <ResultList rows={dictionary.rows} recent={dictionary.recent} {empty} corrected={dictionary.corrected} searching={dictionary.searching}
      selection={dictionary.selection} onchoose={index => { dictionary.selection = index; void dictionary.choose(index); }} />
    <EntryView bind:this={entryView} entry={dictionary.current} position={dictionary.position} started={dictionary.entryStarted}
      scale={dictionary.settings.font_scale} loading={dictionary.loading} {mock} onfollow={link => void dictionary.follow(link)}
      onpaint={started => dictionary.measure('entry', started)} onerror={message => { dictionary.error = message; }} onlookup={text => void starter(text)} />
  </main>
  <footer class="app-footer"><span>留一点空间，给新的理解。</span><button onclick={() => { dictionary.showPerf = !dictionary.showPerf; }} title="延迟统计（Ctrl+Shift+D）">延迟统计</button></footer>
</div>
{#if dictionary.showPerf}<PerfPanel samples={dictionary.samples} version={dictionary.perfVersion} onclose={() => { dictionary.showPerf = false; }} />{/if}
