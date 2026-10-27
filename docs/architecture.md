# Architecture

## In one sentence

`inkdict-core` is a **library**, not a server: every client compiles it in, and only the interface differs. There is no web version and no backend.

```text
                    spec/ (the protocol)
                          │
              crates/inkdict-core (Rust)
   reads .inkdict · word list · entries · themes · render trees
               ┌──────────┴───────────┐
            UniFFI          crates/inkdict-html + inkdict-app
               │                      │
         apps/android            apps/desktop
     Kotlin + Compose (planned)   Tauri 2: Rust + a Svelte UI in the WebView
           offline                    offline

   crates/inkdict-pack: source directory → .inkdict   (used by `inkdict pack`)
```

## Where lookup happens

| Client | Where |
| --- | --- |
| Desktop | On the computer: the Rust side of the Tauri app calls the core; the UI in the WebView only displays and handles input |
| Android (planned) | On the phone: the core reads the local `.inkdict` directly |

No client needs a server. The only network use is media a pack does not store itself: audio and pictures are then fetched from the pack's online templates (`spec/pack-v1.md` §4).

## What the core does

1. **Open a pack**: check `application_id` and `user_version`, read `meta` (including `norm` and `unicode_version`) and `files` (type registry, themes, zstd dictionary).
2. **Entries**: fetch a body by key, decompress it, parse the node tree.
3. **Word list**: normalise the input with the pack's scheme (`fold-1` or `exact-1`) and return the prefix, contains and close-spelling groups, one row per entry, with summaries (`spec/pack-v1.md` §5). Close spellings use an in-memory edit-distance search over the lookup keys.
4. **Themes**: apply the theme's selectors and cascade to every node and produce a **render tree** with final styles, so that each renderer only draws and none reimplements the cascade.
5. **Resources and links**: a resource reference is looked up in the pack's `resources` table, then turned into online URLs from the templates; links resolve as in `spec/v1.md` §6.

The packer (`inkdict-pack`) shares the core's lookup-key, registry and theme code, so packing and reading cannot disagree.

## Storage

The core uses `rusqlite` with a bundled SQLite and opens packs read-only (`immutable=1`). There is no storage abstraction; normalisation and ranking are plain functions independent of the database.

## Renderers

Two are planned: HTML for the desktop app (`crates/inkdict-html`) and Compose for Android. Both consume only the render tree. Colours, sizes and spacing come from the theme, never from client code. Clients touch entry structure only through the roles in the type registry (for example the pronunciation and examples switches), never through a dictionary's type names.

The desktop app's logic (dictionary library, settings, commands) lives in `crates/inkdict-app`. Both the Tauri layer (`apps/desktop/src-tauri`) and `inkdict serve` (for developing the UI in a browser) use it. Files a pack stores itself reach the WebView through the `inkdict-res` protocol.

## Themes and customisation

How an entry looks is decided entirely by theme files (TOML, `spec/v1.md` §4). Themes stack, upper layers first (`spec/v1.md` §4.3):

| Layer | Source | Status |
| --- | --- | --- |
| The dictionary's theme | `themes/<name>.toml` inside the pack | Used by the desktop app |
| A user theme | A TOML file of the user's | Supported by the core and by `inkdict render --overlay`; not yet in the desktop app |
| Settings | An overlay generated from the app's switches | Hides pronunciations / examples by role; text size is set by the app |

A theme can change fonts, sizes, colours (a light and a dark set), whether any type is shown, block or inline layout, generated text such as numbers and brackets, spacing and indentation, backgrounds and corners, collapsing, icons and tabs. It cannot change the content, add arbitrary CSS or scripts, or lay entries out in ways the model does not describe (such as columns). The limits are deliberate: one theme must draw the same in HTML and in Compose, which runs no CSS, and having no scripts keeps dictionaries from carrying code.

## Tests and reference data

The sample dictionary `spec/samples/en-zh/` is in the repository. Tests build it with `inkdict-pack` and cover reading, the word list, links, resources, rendering, HTML, the CLI and the app's logic; they need no other data and always run.

Maintainers also compare the core with a Python reference implementation (in the OxDic repository, which holds the Oxford Advanced Learner's Dictionary data and is private). `scripts/sync-oxdata.sh` copies that data into the git-ignored `oxdata/`:

| File | Used for |
| --- | --- |
| `oxdata/oalecd10.inkdict` | Reading, word-list and rendering tests on 39,960 real entries |
| `oxdata/entries.jsonl` | All entries uncompressed: render-tree and decoding tests |
| `oxdata/examples/*.json` | Five reference entries |
| `oxdata/*_report.json`, `SHA256SUMS` | Build reports and checksums |
| `oxdata/golden/*.json` | Output of the Python reader (`scripts/make-golden.py`) that the core must match exactly |

Without `oxdata/`, these extra tests print one `skipped:` line and pass; the sample tests still run. `scripts/compare-packs.py` compares two packs table by table, for checking `inkdict pack` against the Python packer.
