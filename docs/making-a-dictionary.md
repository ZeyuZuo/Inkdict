# Making a dictionary

This guide builds a small German–English glossary from nothing, packs it into an `.inkdict` file and opens it in the Inkdict desktop app. Every file below is complete; you can copy them as they are. The rules behind each step are in [`spec/v1.md`](../spec/v1.md) and [`spec/pack-v1.md`](../spec/pack-v1.md); a larger example with more features is [`spec/samples/en-zh/`](../spec/samples/en-zh/).

Only pack and share dictionaries whose content you have the right to distribute.

## What you need

The `inkdict` command-line tool. Download it from the [releases page](https://github.com/ZeyuZuo/Inkdict/releases), or build it from this repository with Rust 1.95:

```bash
cargo build --release -p inkdict-cli     # the tool is target/release/inkdict
```

## 1. The source directory

A dictionary starts as a directory of plain text files:

```text
tiny-de-en/
  manifest.toml        what the dictionary is and where its files are
  types.toml           the node types its entries use
  themes/default.toml  how entries look
  entries.jsonl        the entries, one per line
  summaries.jsonl      a short gloss per entry for the word list (optional)
```

### manifest.toml

```toml
[meta]
format = "inkdict-source"
version = 1

[pack]
id = "tiny-de-en"
title = "Tiny German–English Glossary"
edition = "1"
license = "CC0-1.0"

[languages]
source = "de"
targets = ["en"]

[files]
types = "types.toml"
themes = ["themes/default.toml"]
entries = "entries.jsonl"
summaries = "summaries.jsonl"

[lookup]
norm = "fold-1"
```

- `pack.id` names the dictionary (lowercase letters, digits and `-`); `title` is what people see.
- `languages.source` is the language of the headwords; `targets` lists translation languages, `[]` for a monolingual dictionary.
- `lookup.norm = "fold-1"` makes lookup ignore case and Latin, Greek, Cyrillic, Hebrew and Arabic diacritics: `strasse`, `STRASSE` and `Straße` all find *Straße*, and `hauser` finds *Häuser*. Use `exact-1` for languages whose marks must not be ignored, such as Hindi or Japanese ([`pack-v1.md` §3.2](../spec/pack-v1.md#32-lookup-keys)).

## 2. Node types

An entry is a tree of nodes, and each node has a type that your dictionary defines. Inkdict does not care what you call them: applications only look at the **roles** you give a type. The ones used here:

- `index` with an index kind: the node's text can be looked up. `headword` for the word itself, `inflection` for forms like plurals; there are also `variant`, `phrase`, `reading` and `hidden` ([`v1.md` §3.1](../spec/v1.md#31-index-kinds)).
- `anchor`: a node of this type with an `id` can be the target of a link or of a word-list row.
- `example`: the app's "examples" switch can hide it.

### types.toml

```toml
[meta]
format = "inkdict-types"
version = 1

[types.entry]
kind = "block"
children = ["head", "sense"]
roles = ["anchor"]

[types.head]
kind = "block"
children = ["headword", "gender", "plural"]

[types.headword]
kind = "inline"
children = ["#text"]
roles = ["index"]
index = "headword"

[types.gender]
kind = "inline"
children = ["#text"]
attrs = { value = "enum(m|f|n)" }

[types.plural]
kind = "inline"
children = ["#text"]
roles = ["index"]
index = "inflection"

[types.sense]
kind = "block"
children = ["translation", "example"]
roles = ["anchor"]

[types.translation]
kind = "inline"
children = ["#text"]

[types.example]
kind = "block"
children = ["#text"]
roles = ["example"]
```

`children` lists which types may appear inside a node; `#text` allows plain strings. `attrs` declares attributes and their types (`enum(m|f|n)` here). `kind` (`block` or `inline`) is the layout used when the theme says nothing else.

Other roles you may need later: `audio` and `image` for media, `link` for cross-references, `homograph` for numbered homographs, `pronunciation` for the pronunciation switch, and `choice` and `optional` for phrases with alternatives and optional words ([`v1.md` §3.2](../spec/v1.md#32-roles)). The sample dictionary in `spec/samples/en-zh/` uses all of them.

## 3. Entries

`entries.jsonl` has one entry per line. Each entry is a node tree whose root has an `id` (the entry's key, unique in the dictionary) and a `lang`:

```json
{"type": "entry", "id": "strasse", "lang": "de", "children": [{"type": "head", "children": [{"type": "headword", "text": "Straße"}, {"type": "gender", "attrs": {"value": "f"}, "text": "die"}, {"type": "plural", "text": "Straßen"}]}, {"type": "sense", "id": "s1", "children": [{"type": "translation", "lang": "en", "text": "street, road"}, {"type": "example", "text": "Wir wohnen in einer ruhigen Straße."}]}]}
{"type": "entry", "id": "haus", "lang": "de", "children": [{"type": "head", "children": [{"type": "headword", "text": "Haus"}, {"type": "gender", "attrs": {"value": "n"}, "text": "das"}, {"type": "plural", "text": "Häuser"}]}, {"type": "sense", "id": "s1", "children": [{"type": "translation", "lang": "en", "text": "house"}, {"type": "example", "text": "Das Haus steht am Ende der Straße."}]}]}
```

Content holds only the words themselves: the brackets, "pl." and spacing come from the theme. Nodes inherit `lang` from their parent; the English translations set `lang` to `en` so that a theme can style them differently, and so that fonts suit the language.

When your dictionary already exists in another form (a spreadsheet, XML, a database), write a short script in any language that prints one such JSON object per line. `inkdict pack` checks every entry against `types.toml` and reports what does not fit.

## 4. A theme

```toml
[meta]
format = "inkdict-theme"
version = 1
name = "Default"

[fonts.text]
default = ["Source Serif 4", "serif"]

[colors.light]
fg = "#1b1c1f"
muted = "#6c6e75"

[colors.dark]
fg = "#e5e5e7"
muted = "#9b9da4"

[style.entry]
font = "text"
color = "fg"

[style.headword]
size = 2
weight = "bold"

[style.gender]
before = " "
style = "italic"
color = "muted"

[style.plural]
before = " (pl. "
after = ")"
color = "muted"

[style.sense]
space_before = 0.8

[style.example]
style = "italic"
color = "muted"
indent = 1.2
```

A theme is a list of style rules with CSS-like selectors (`headword`, `sense > example`, `gender[value=f]`) and named fonts and colours, with a light and a dark version of every colour ([`v1.md` §4](../spec/v1.md#4-themes-themesnametoml)). Applications draw exactly what the theme says, so the same dictionary looks the same everywhere.

## 5. Word-list summaries (optional)

```json
{"entry": "strasse", "lang": "en", "text": "street, road"}
{"entry": "haus", "lang": "en", "text": "house"}
```

The word list shows this line under each row. Without summaries the word list shows only the words.

## 6. Pack it

```bash
inkdict pack tiny-de-en/manifest.toml
```

```text
2 entries, 4 lookup rows, 2 summaries
0 resources, 0 links, 0 resource references
tiny-de-en/tiny-de-en.inkdict (36864 bytes)
```

The output is written next to the manifest as `<pack.id>.inkdict` unless you pass `-o`. Problems that stop the build (invalid JSON, a duplicate key, an invalid theme) are printed with the file and line. Content problems (an attribute of the wrong type, a link to a missing entry) are counted under the summary; `--report report.json` saves them all with examples.

## 7. Try it

```bash
export INKDICT_PACK=tiny-de-en/tiny-de-en.inkdict
inkdict words strasse          # Straße, Straßen → Straße
inkdict words haeuser          # a close spelling: Häuser → Haus
inkdict show haus              # the entry as a node tree (--json for JSON)
inkdict themes                 # checks every theme against types.toml
```

Then open the desktop app, click the gear icon at the bottom of the sidebar (settings), choose **添加词典…** (Add dictionary; the interface is in Chinese for now) and pick `tiny-de-en.inkdict`.

## 8. Next steps

- **Pronunciations and pictures.** Give a type the `audio` or `image` role and a `res` attribute holding a reference like `audio:haus`. Put the files in `resources/audio/haus.mp3` and set `files.resources = "resources"`, or add a `[[source]]` URL template for files served online ([`pack-v1.md` §4](../spec/pack-v1.md#4-resources)).
- **Cross-references.** A type with the `link` role and `entry` / `anchor` attributes opens another entry, or a node in it that has an `id`.
- **Ranking.** `lookup.boost` lists selectors; entries that contain a matching node come first among equal matches, e.g. `boost = ["badge[scheme=core]"]`.
- **Tabs.** `tabs = true` on a style draws adjacent nodes, such as parts of speech, as tabs ([`v1.md` §4.4](../spec/v1.md#44-tabs)).
