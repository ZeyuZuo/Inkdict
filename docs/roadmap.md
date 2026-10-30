# Roadmap

## Decisions

| | Decision | Why |
| --- | --- | --- |
| D1 | A dictionary is one SQLite file (`.inkdict`) | Random access and prefix lookup come for free; every platform can read it |
| D2 | One Rust core shared by all clients | Lookup, ranking, spelling suggestions and the theme cascade are written once and agree everywhere |
| D3 | Desktop: Tauri 2 with the UI in the system WebView | Small installers; one UI for Linux, Windows and macOS |
| D4 | Desktop UI: TypeScript and Svelte | HTML and CSS are the most mature way to lay out rich text |
| D5 | Android: Kotlin and Compose, calling the core through UniFFI | Native feel, fast start, access to the system's text-selection menu |
| D6 | Native clients only, no web version or server | The core only needs SQLite; dictionary content never has to be served online |
| D7 | No dictionary content in this repository, apart from the sample | Dictionary text and media are copyrighted; the protocol and software are open |

## Done

### Core (M1, M2)

Reading packs, the word list (prefix, contains, close spellings), entries, anchors and links, resources, the theme cascade and render trees, the `inkdict` command-line tool. Checked against a Python reference implementation on the full Oxford Advanced Learner's Dictionary (39,960 entries): identical word lists, lookup keys and HTML.

Release build, on 39,960 entries: word list p95 2.8 ms; render tree p95 0.65 ms.

### Desktop app (M3)

Word list as you type, entries with part-of-speech tabs, pronunciation, cross-references with a "back to" link, history with scroll position, several dictionaries (add, remove, reorder, switch), settings, a narrow single-column layout with always-on-top, light and dark.

Measured in the real WebKitGTK window (release build, from input to the updated screen): word list p50 17 ms / p95 18 ms; opening an entry p50 16 ms / p95 54 ms. Targets were 50 ms and 100 ms.

### Version 0.1

- Protocol v1 frozen and published in English, under CC BY 4.0.
- `inkdict pack`: the packer in Rust, matching the Python reference table by table.
- Tests on the public sample dictionary, so they run anywhere.
- Media stored in a pack shown by the desktop app.

### After 0.1

- **Choosing a dictionary's theme**: when a pack carries several themes, the dictionary menu at the bottom of the sidebar (the book button in a narrow window) lists them under 词条样式; the choice is remembered per dictionary.
- **Dictionary folder**: the desktop app reads every `.inkdict` file in one folder (by default `~/.local/share/win.zeyuz.inkdict/dictionaries` on Linux; it can be changed in the settings). Files put there appear when the app starts or its window is focused again; 添加词典… copies a file into the folder, and deleting a dictionary moves its file to the trash. Dictionaries added by path in 0.1 stay in the list.
- The fold chevron of collapsible blocks shows in every theme (it used a colour token no theme defined).

## Next

- **English interface** for the desktop app, following the system language (the interface is Chinese only today).
- **Look up selected text** from anywhere, and a small lookup window.
- **User themes** in the desktop app (the core already supports a user theme layer).
- **Android** (M4): UniFFI bindings (`crates/inkdict-ffi`), a Compose renderer, the system's `PROCESS_TEXT` menu and share-to-look-up.
- **A second dictionary in another language pair** (M5), to test the protocol beyond English–Chinese.
- **Unicode versions**: packs record `unicode_version`, but the core always uses its own, newer Unicode tables. For characters assigned up to Unicode 14 the result differs only for U+1171E; characters assigned later may not be found in packs built with older tables (see `crates/inkdict-core/src/text.rs`).
