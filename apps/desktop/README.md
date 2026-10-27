# Desktop app

Tauri 2. The Rust side (`src-tauri/`) calls `inkdict-core` directly, fully offline. The UI is Svelte 5 and TypeScript (Vite) in `src/`; it runs only inside the Tauri WebView and is never deployed on its own.

Entry HTML is produced in Rust (`crates/inkdict-html`, from the core's render tree). The UI inserts it and handles clicks; it implements no theme or layout rules in TypeScript.

## Commands between the UI and Rust

All commands are called with Tauri `invoke`; arguments and results are JSON with snake_case names, matching the types of `inkdict-core`.

| Command | Arguments | Result |
| --- | --- | --- |
| `pack_info` | — | `{ id, title, entries, source_lang, target_langs, switches }`. `switches` is `{ pronunciation, example }`: true when the pack has a type with that role; settings show only those switches. Fails with `no_pack` when no pack is open |
| `library` | — | `Library`: `{ active: string or null, packs: LibraryItem[] }`; each item is `{ path, title, entries, bytes, error }` (`title`, `entries`, `bytes`, `error` may be null). Packs that cannot be opened stay listed, with the reason in `error` |
| `add_packs` | `{ paths: string[] }` (multiple selection in the file dialog) | `{ library: Library, rejected: [{ path, message }] }`. Valid packs are appended in order under their canonical paths; duplicates and invalid files go to `rejected`. When no pack is active, the first added one is opened |
| `remove_pack` | `{ path }` | `Library`. Only removes the entry from the list; the file stays. Removing the active pack opens the first pack that can be opened, or none |
| `move_pack` | `{ path, index }`, `index` from 0; past the end moves it last | `Library`; changes only the display order |
| `use_pack` | `{ path }` | Like `pack_info`; opens that pack of the library and makes it active. Fails with `not_found` when it is not in the library |
| `words` | `{ text, limit }`, `limit` rows per group | The word list (`spec/pack-v1.md` §5): `{ group, form, kind, entry, headword, homograph, anchor, readings, summary, summary_lang, distance, rank, target }[]`. `group` is `prefix`, `contains` or `close`; `kind` is the index kind; `target` is the headword a variant or inflection points to (null when equal to `form`) |
| `entry` | `{ key }` | `{ key, headword, homograph, html }` |
| `follow` | `{ current, link }`, `link` is `{ entry?, anchor?, text }` | `{ kind: "entry", key, focus }` or `{ kind: "lookup", text }`; `focus` is `{ kind: "top" }` or `{ kind: "anchor", number }` |
| `settings` | — | `Settings` |
| `set_settings` | `{ settings }` | `Settings`; later `entry` calls render with them |
| `theme_css` | — | A string: the theme's font and colour variables (light and dark) and the base `.dx-*` rules |

`Settings` is `{ hide_pronunciation: bool, hide_examples: bool, font_scale: number }`. The first two hide types with the `pronunciation` or `example` role (`spec/v1.md` §3.2) through an overlay theme built in Rust; `font_scale` is applied by the UI as a CSS variable.

Settings and the library are saved in `config.json` in the app's configuration directory: `{ "packs": ["/a.inkdict", "/b.inkdict"], "active": "/a.inkdict", "settings": { … } }`. `active` is one of `packs` or null. `--pack` or `INKDICT_PACK` adds a path to the library and makes it active. `inkdict serve` offers the same commands as `POST /<command>` with the same JSON; its library lives in memory only.

Errors: a failed command rejects with `{ code, message }`, `code` being `no_pack`, `not_found`, `bad_input` or `internal`.

## Entry HTML

In the HTML returned by `entry`:

- nodes with an anchor carry `data-anchor="<number>"` (as `anchor` in the word list; 0 is the entry itself);
- a tab group (`spec/v1.md` §4.4) is `<div class="dx-tabs">`: a `<div class="dx-tab-bar" role="tablist">` with one `<button class="dx-tab" role="tab" aria-selected>` per tab, then one `<div class="dx-tab-panel" role="tabpanel">` per tab, `hidden` unless selected. When the anchor to open is inside a tab, the UI selects that tab;
- a play control is `<button class="dx-icon" data-audio="<url> <url>…">`, tried in order;
- files the pack stores come first, at `inkdict-res://localhost/<pack id>/<kind>/<key>` (`http://inkdict-res.localhost/…` on Windows), served from the open dictionary by a Tauri protocol handler; `inkdict serve` serves the same files at `http://127.0.0.1:<port>/resource/…`;
- a picture is `<img src="<first URL>" data-fallback="<url>…">`;
- a link is `<a href="#" class="dx-link" data-link="<the link as JSON, HTML-escaped>">`; on click the UI parses `dataset.link` and passes it to `follow`.

CSS: `theme_css` gives only the theme's variables and the `.dx-*` rules; the page defines `--page`, `--card` (arrow icons are cut out with `--card`) and the base text size.

## Development

```bash
npm install
INKDICT_PACK=/path/to/a.inkdict npm run tauri dev   # a real window (WebKitGTK on Linux); INKDICT_PACK is optional
npm run check && npm test                           # svelte-check, vitest
```

To work on the UI in a browser without a window:

```bash
cargo build --release -p inkdict-cli
../../target/release/inkdict pack ../../spec/samples/en-zh/manifest.toml -o /tmp/sample.inkdict
../../target/release/inkdict --pack /tmp/sample.inkdict serve --port 1430
VITE_INKDICT_BRIDGE=http://127.0.0.1:1430 npx vite --port 1421   # then open http://localhost:1421
```

Without `VITE_INKDICT_BRIDGE` the UI uses a few invented entries from `src/lib/mock.ts`. Development builds expose the UI controller as `window.__inkdict` for scripted checks. Release build: `npm run tauri build`.

## Interface

- Sidebar: search box, word list (prefix matches; "contains …"; close spellings when nothing matches), and at the bottom the dictionary switcher and settings. In a wide window the top row's entry follows the input; ↑↓ show the selected row at once; Enter or a click makes it a history stop.
- Narrower than 720 px: one column; the word list drops down under the search box and closes once a row is chosen; the top bar has "always on top".
- Entries: part-of-speech tabs stick to the top; when the anchor to open is in another tab, that tab is selected; an entry reached through a link shows "‹ source word" at the top.
- History keeps the entry, scroll position, the selected tab of each tab group and the search text.

| Keys | Action |
| --- | --- |
| Typing anywhere | Goes to the search box |
| Ctrl+L / Ctrl+F, Esc | Select the search box; Esc in the search box clears it (in a narrow window it first closes the list) |
| ↑ ↓, Enter | Move in the word list, open |
| Alt+← / Alt+→, mouse side buttons | Back / forward |
| Ctrl+, | Settings |
| Ctrl+Shift+D | Latency statistics (last 200) |

The interface text is Chinese for now; an English interface is planned (`docs/roadmap.md`).

## Benchmark

```bash
INKDICT_BENCH=$PWD/out.json INKDICT_BENCH_QUERIES=$PWD/queries.txt \
INKDICT_BENCH_ENTRIES=$PWD/entries.txt npm run tauri dev
```

The UI types each query key by key and opens each entry (the first 300 lines of each file; `INKDICT_BENCH_LIMIT` changes it), writes the input-to-word-list and open-to-entry times to `out.json` (`words` and `entry`) and exits. Each line's first tab-separated field is a query or an entry key; `scripts/bench.py queries` generates such files from a pack.
