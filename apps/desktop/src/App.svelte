<script lang="ts">
  import { onMount } from 'svelte';
  import { Dictionary } from './lib/controller.svelte';
  import { isTauri } from './lib/api';
  import { runBench } from './lib/bench';
  import { pickPacks } from './lib/files';
  import SearchField from './components/SearchField.svelte';
  import WordList from './components/WordList.svelte';
  import EntryPane from './components/EntryPane.svelte';
  import PackMenu from './components/PackMenu.svelte';
  import SettingsDialog from './components/SettingsDialog.svelte';
  import PerfPanel from './components/PerfPanel.svelte';
  import Icon from './components/Icon.svelte';

  const dictionary = new Dictionary();
  let search: SearchField;
  let pane: EntryPane;
  let top: HTMLElement | undefined = $state();

  onMount(() => {
    dictionary.view = () => pane?.view() ?? { top: 0, tabs: [] };
    // Development builds: the controller for scripted checks in a browser.
    if (import.meta.env.DEV) (window as unknown as { __inkdict: Dictionary }).__inkdict = dictionary;
    // One column below this width: the list drops down under the search box.
    const narrow = matchMedia('(max-width: 720px)');
    const update = () => { dictionary.narrow = narrow.matches; };
    update();
    narrow.addEventListener('change', update);
    search.focus();
    void dictionary.initialize().then(() => runBench(dictionary));
    return () => narrow.removeEventListener('change', update);
  });

  /** Keys that belong to an input method: WebKit sends the key that
   *  commits a composition after compositionend, as keyCode 229. */
  const composing = (event: KeyboardEvent) => event.isComposing || event.keyCode === 229;

  function searchKey(event: KeyboardEvent) {
    if (composing(event) || event.ctrlKey || event.metaKey || event.altKey) return;
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      dictionary.move(event.key === 'ArrowDown' ? 1 : -1);
    } else if (event.key === 'Enter') {
      event.preventDefault();
      void dictionary.choose();
    } else if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      if (dictionary.narrow && dictionary.listOpen) dictionary.listOpen = false;
      else void dictionary.lookup('');
    }
  }

  function globalKey(event: KeyboardEvent) {
    if (composing(event)) return;
    const key = event.key.toLowerCase();
    const mod = event.ctrlKey || event.metaKey;
    const target = event.target;
    const typing = target instanceof HTMLElement && (target.matches('input,textarea,select') || target.isContentEditable);
    if (mod && event.shiftKey && key === 'd') {
      event.preventDefault(); dictionary.showPerf = !dictionary.showPerf;
    } else if (mod && (key === 'l' || key === 'f')) {
      event.preventDefault(); search.focus(true);
    } else if (mod && key === ',') {
      event.preventDefault(); dictionary.showSettings = true;
    } else if (event.altKey && (key === 'arrowleft' || key === 'arrowright')) {
      event.preventDefault(); void dictionary.navigate(key === 'arrowleft' ? -1 : 1);
    } else if (key === 'escape' && !dictionary.showSettings) {
      search.focus(true);
    } else if (!typing && !mod && !event.altKey && event.key.length === 1 && event.key !== ' ' && !dictionary.showSettings) {
      // Start typing anywhere: the keystroke goes to the search box.
      search.focus();
    }
  }

  function mouseNavigation(event: MouseEvent) {
    if (event.button !== 3 && event.button !== 4) return;
    event.preventDefault(); void dictionary.navigate(event.button === 3 ? -1 : 1);
  }
  function preventMouseHistory(event: MouseEvent) {
    if (event.button === 3 || event.button === 4) event.preventDefault();
  }
  function outside(event: MouseEvent) {
    if (dictionary.narrow && dictionary.listOpen && top && !top.contains(event.target as Node)) dictionary.listOpen = false;
  }
  async function add() {
    const paths = await pickPacks();
    if (paths.length) await dictionary.addPacks(paths);
  }
  function closeSettings() { dictionary.showSettings = false; search.focus(); }
</script>

<svelte:window onkeydown={globalKey} onmousedown={event => { mouseNavigation(event); outside(event); }}
  onmouseup={preventMouseHistory} onauxclick={preventMouseHistory} />

{#snippet field()}
  <SearchField bind:this={search} text={dictionary.text} disabled={!dictionary.pack}
    oninput={(text, started) => void dictionary.lookup(text, started)} onkeydown={searchKey}
    onclear={() => void dictionary.lookup('')}
    onfocus={() => { if (dictionary.narrow && dictionary.words.length) dictionary.listOpen = true; }} />
{/snippet}
{#snippet list()}
  <WordList words={dictionary.words} searched={dictionary.searched} selection={dictionary.selection}
    onchoose={index => void dictionary.choose(index)} />
{/snippet}

<div class="app" class:narrow={dictionary.narrow}>
  {#if dictionary.narrow}
    <div class="top" bind:this={top}>
      {@render field()}
      {#if isTauri()}
        <button class="icon-button" class:on={dictionary.pinned} aria-pressed={dictionary.pinned}
          aria-label="固定在最前" title="固定在最前" onclick={() => void dictionary.togglePin()}><Icon name="pin" /></button>
      {/if}
      <button class="icon-button" aria-label="设置" title="设置" onclick={() => { dictionary.showSettings = true; }}><Icon name="gear" size={18} /></button>
      {#if dictionary.listOpen && dictionary.searched.trim()}<div class="drop">{@render list()}</div>{/if}
    </div>
  {:else}
    <aside class="side">
      {@render field()}
      {@render list()}
      <div class="foot">
        <PackMenu library={dictionary.library} title={dictionary.pack?.title ?? ''}
          onuse={path => void dictionary.usePack(path)} onmanage={() => { dictionary.showSettings = true; }} />
        <button class="icon-button" aria-label="设置" title="设置" onclick={() => { dictionary.showSettings = true; }}><Icon name="gear" size={18} /></button>
      </div>
    </aside>
  {/if}

  <EntryPane bind:this={pane} entry={dictionary.current} pack={dictionary.pack} position={dictionary.position}
    from={dictionary.from} started={dictionary.entryStarted} scale={dictionary.settings.font_scale} loading={dictionary.loading}
    onfollow={link => void dictionary.follow(link)} onback={() => void dictionary.navigate(-1)}
    onpaint={started => dictionary.measure('entry', started)}
    onerror={message => { dictionary.error = message; }} onadd={() => { if (isTauri()) void add(); else dictionary.showSettings = true; }} />
</div>

{#if dictionary.showSettings}
  <SettingsDialog library={dictionary.library} pack={dictionary.pack} settings={dictionary.settings} theme={dictionary.theme}
    onadd={() => void add()} onaddpath={path => void dictionary.addPacks([path])}
    onremove={path => void dictionary.removePack(path)} onmove={(path, index) => void dictionary.movePack(path, index)}
    onuse={path => dictionary.usePack(path)} onsettings={settings => void dictionary.updateSettings(settings)}
    onpacktheme={name => void dictionary.setPackTheme(name)} ontheme={mode => dictionary.setTheme(mode)} onclose={closeSettings} />
{/if}

{#if dictionary.error}
  <div class="notice" role="alert"><span>{dictionary.error}</span><button aria-label="关闭提示" onclick={() => { dictionary.error = ''; }}>×</button></div>
{/if}
{#if dictionary.showPerf}<PerfPanel samples={dictionary.samples} version={dictionary.perfVersion} onclose={() => { dictionary.showPerf = false; }} />{/if}
