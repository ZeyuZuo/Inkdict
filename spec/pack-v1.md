# Inkdict pack format, version 1

Status: **frozen**, 2026-10-27, under the same rules as `v1.md`.

A packer turns a dictionary's **source directory** (§1) into one SQLite file with the extension `.inkdict` (§2–§4; table definitions in `pack-v1.sql`). Readers open it read-only. §5 gives the rules for the word list, so that every application lists the same rows.

Both reference implementations are in this repository: the packer is `crates/inkdict-pack` (the command `inkdict pack`), the reader is `crates/inkdict-core`.

## 1. Source directory

```text
<dictionary>/
  manifest.toml        reading starts here
  types.toml           the type registry (v1.md §3)
  themes/*.toml        themes (v1.md §4)
  entries.jsonl        entries, one per line
  summaries.jsonl      word-list summaries (optional)
  resources/<kind>/    files stored in the pack (optional)
```

A complete example is `samples/en-zh/`. The manifest decides the actual names and locations; the layout above is the convention.

### 1.1 manifest.toml

| Key | Required | Meaning |
|---|---|---|
| `meta.format`, `meta.version` | yes | `inkdict-source`, `1` |
| `pack.id` | yes | The dictionary's id: lowercase letters, digits and `-` |
| `pack.title` | yes | Display name |
| `pack.edition`, `pack.license` | no | Edition; copyright and terms of use |
| `languages.source` | yes | Language of headwords and original text (BCP 47) |
| `languages.targets` | yes | Array of translation languages; `[]` for a monolingual dictionary |
| `files.types` | yes | Path of the type registry |
| `files.themes` | yes | Array of theme paths; the first is the default theme |
| `files.entries` | yes | Path of the entries file |
| `files.summaries` | no | Path of the summaries file |
| `files.resources` | no | Path of the resources directory |
| `lookup.norm` | yes | Lookup key scheme: `fold-1` or `exact-1` (§3.2) |
| `lookup.boost` | no | Array of selectors used for ranking (§3.3) |
| `[[source]]` | no | Online resource sources: `kind` and `template`; for one kind they are tried in the order written (§4) |

Paths are relative to the directory of the manifest.

### 1.2 entries.jsonl

UTF-8, one entry per line: a node tree as in `v1.md` §2. Lines end with LF; a CR just before the LF is removed, so CRLF also works; the last line may lack its LF. Empty lines are not allowed. The order of the lines is the order of entry numbers in the pack. `summaries.jsonl` is split into lines the same way.

### 1.3 summaries.jsonl

One summary per line: the short gloss shown under a row of the word list.

```json
{"entry": "harbour", "anchor": "safe-harbour", "lang": "zh-Hans", "text": "安全港；免责保护"}
```

| Key | Meaning |
|---|---|
| `entry` | The entry key |
| `anchor` | Optional; the `id` of a node in the entry. Without it, the summary is for the whole entry |
| `lang` | Optional; the language of the text, used to choose fonts. Defaults to the `lang` of the entry's root |
| `text` | One line of plain text |

There is at most one summary per `(entry, anchor)`. The dictionary decides how summaries are written and in which language.

### 1.4 resources/

`resources/<kind>/<key>.<extension>`; for example `resources/audio/harbour-gb.wav` is the resource `audio:harbour-gb`. The key is the file name without its last extension and is unique within a kind. The media type follows from the extension:

| Extension | Media type |
|---|---|
| `mp3` `wav` `ogg` `m4a` | `audio/mpeg` `audio/wav` `audio/ogg` `audio/mp4` |
| `jpg` `jpeg` `png` `webp` `svg` | `image/jpeg` `image/jpeg` `image/png` `image/webp` `image/svg+xml` |

Any other extension stops the build. A dictionary with very large media can leave this directory out and use online sources only.

## 2. File format

### 2.1 Identification

| Item | Value |
|---|---|
| `PRAGMA application_id` | `0x494E4B44` (ASCII `INKD`) |
| `PRAGMA user_version` | `1` |
| `meta.format`, `meta.format_version` | `inkdict-pack`, `1` |

A reader checks these first and refuses the file when any of them differs.

### 2.2 meta

| Key | Value |
|---|---|
| `id`, `title`, `edition`, `license` | From the manifest; keys it lacks are left out |
| `source_lang`, `target_langs` | `languages.source`; `languages.targets` as a JSON array |
| `themes` | Theme names as a JSON array, in manifest order; the first is the default |
| `norm`, `unicode_version` | The lookup key scheme; the version of the Unicode tables used for packing (for example `17.0.0`) |
| `compression`, `zstd_level`, `zstd_dict` | `zstd`; the compression level; the name in `files` of the compression dictionary (left out when there is none) |
| `entries`, `resources` | Number of entries; number of files stored in the pack |
| `built_at`, `source_sha256`, `builder` | Build time (RFC 3339, UTC); SHA-256 of `entries.jsonl`; name and version of the packer |

### 2.3 files and entries

`files` holds `types.toml`, `themes/<name>.toml` and the compression dictionary `zstd.dict`. Readers use only these copies, never the source directory or `spec/`.

The columns of `entries`:

| Column | Value |
|---|---|
| `id` | Numbered from 1 in the order of `entries.jsonl`; meaningful only inside this pack |
| `key` | The root's `id`. Cross-references, deep links and history use it |
| `headword` | The text of the first `headword`-kind index node at anchor 0, with layout marks removed and white space collapsed (§3.1); else the first anywhere in the entry; else `key` |
| `homograph` | The text of the first `homograph`-role node at anchor 0 when it is a positive decimal integer (the digits of any script); otherwise NULL |
| `body` | The node tree as compact JSON (UTF-8), compressed with zstd, using `zstd_dict` when present |

## 3. Form index: `lookup`

### 3.1 What is indexed

Every node whose type has the `index` role gives one or more rows:

| Column | Value |
|---|---|
| `norm` | The lookup key (§3.2) |
| `form` | The form as displayed |
| `entry` | `entries.id` |
| `anchor` | Anchor number (§3.4) of the node itself or of its nearest ancestor that has an `id` |
| `kind` | The type's index kind (`v1.md` §3.1) |
| `rank` | Sort value; lower comes first (§3.3) |

A node's **readings** are expanded from its children, at most 32; readings beyond that are dropped in order and reported:

- a string is taken as it is, except that each `(…)` in it may be left out (`by your (own) bootstraps` gives readings with and without `own`);
- a child with the `optional` role may be left out (`the lower middle classes` gives readings with and without `the`);
- a run of children with the `choice` role, separated at most by white-space strings, gives one reading per child (`pull`, `drag` give two);
- any other child contributes its own readings.

Each reading, with the layout marks `ˈ` `ˌ` `·` `‧` `↔` (U+02C8, U+02CC, U+00B7, U+2027, U+2194) removed, white space collapsed and trimmed, is a `form`. Readings whose `norm` is empty are not indexed. Within an entry only one row is kept per `(norm, anchor)`: the one with the lowest `rank`.

### 3.2 Lookup keys

Packing and lookup use the same scheme, recorded in `meta.norm`. The two schemes:

| Step | `fold-1` | `exact-1` |
|---|---|---|
| 1 | Remove the layout marks of §3.1 | Same |
| 2 | NFKD | NFKC |
| 3 | Remove combining marks whose base letter is Latin, Greek, Cyrillic, Hebrew or Arabic (below) | — |
| 4 | NFC | — |
| 5 | Unicode full case folding | Same |
| 6 | Keep only characters of general category L, M or N; when that leaves nothing, keep every non-white-space character of step 5's result instead | Same |

Step 3: the base of a character of general category Mn is the nearest preceding character that is not Mn. The Mn character is removed when its base lies in one of these ranges, or when it has no base:

| Script | Ranges |
|---|---|
| Latin | U+0000–024F, 1E00–1EFF, 2C60–2C7F, A720–A7FF, AB30–AB6F |
| Greek | U+0370–03FF, 1F00–1FFF |
| Cyrillic | U+0400–052F, 1C80–1C8F, 2DE0–2DFF, A640–A69F |
| Hebrew | U+0590–05FF, FB1D–FB4F |
| Arabic | U+0600–06FF, 0750–077F, 0870–08FF, FB50–FDFF, FE70–FEFF |

So under `fold-1`, `Café` = `cafe`, `mother-in-law` = `mother in law` and `كِتَاب` = `كتاب`, while Hindi `किताब` and Japanese `が` keep their marks. Languages whose headwords carry marks that must not be removed use `exact-1`.

The Unicode version affects the result. A reader may use newer tables: for characters assigned in `meta.unicode_version` the result must be the same as at packing time; for characters assigned later it is not specified.

### 3.3 Ranking

```text
rank = class × (B + 1) + importance
```

| class | Rows |
|---|---|
| 0 | `headword` at anchor 0 |
| 1 | `headword` at any other anchor (derived words, headwords inside a part-of-speech group…) |
| 2 | `variant` |
| 3 | `phrase` |
| 4 | `inflection` |
| 5 | `reading` |
| 6 | `hidden` |

`B` is the length of `lookup.boost`. `importance` is the index (from 0) of the first boost selector that matches any node of the entry, or `B` when none does. Selectors are written as in `v1.md` §4.1, without `::` pseudo-elements. Example: `boost = ["badge[scheme=core]"]` ranks entries with a core-vocabulary badge first.

### 3.4 Anchor numbers

All nodes of an entry that have an `id` are numbered in document order (pre-order); the root is 0. The numbers are meaningful only inside the pack; links from outside use `<entry key>#<node id>`.

## 4. Resources

A resource reference `kind:key` (`v1.md` §7) is resolved in order:

1. The row `(kind, key)` of the `resources` table, when present: its `body` and `mime`.
2. Otherwise the templates of that kind in `resource_sources`, by `priority` from 1. In a template, `{key}` is replaced by the key as it is and `{key:uri}` by the key encoded as `encodeURIComponent` does.

`resource_sources` comes from the manifest's `[[source]]` tables; `priority` is the order within a kind. Readers may cache what they fetch.

## 5. The word list (readers)

For the input `q`, the key is `k = norm(q)`. When `k` is empty, no list is shown.

### 5.1 Three groups

| Group | When | Matches | Order |
|---|---|---|---|
| Prefix | Always | `norm` starts with `k` | `norm = k` first, then `length(norm)`, `rank`, `entry` |
| Contains | `k` has at least 2 characters | `norm` contains `k` starting at its 2nd character or later; only `headword`, `variant` and `phrase` rows | `rank`, `length(norm)`, `norm`, `entry` |
| Close spellings | Both groups above are empty and `k` has at least 3 characters | Edit distance to `k` (optimal string alignment: swapping two adjacent characters counts once) at most 1 for 3–4 characters, at most 2 from 5 characters | By key: distance, letter difference, the key's lowest `rank`, length difference to `k`, the key itself; rows of one key by `rank`, `entry` |

The letter difference is the size of the difference between the two keys' character multisets: a swap of adjacent letters counts 0, so for `harbuor` the candidate `harbour` comes before candidates that replace a letter.

The prefix group is a range scan of the `lookup` primary key (`norm >= k AND norm < k || char(0x10FFFF)`). The other two groups may be computed in memory over `lookup.norm`. How many rows each group shows is up to the application.

### 5.2 One row per entry

`lookup` rows are merged into word-list rows before they are shown.

- **Merging**: rows with the same displayed form and the same entry become one row. The displayed form is `form`, except for `reading` and `hidden` rows, whose displayed form is the entry's `headword`; they therefore merge into the headword's row.
- **Representative**: a merged row uses its member with the lowest `rank`; its position in the list is that of its earliest member.
- **No repeats across groups**: a row in an earlier group is not listed again in a later one. This is decided on each group's complete result, before the application's row limit is applied, so the limit never changes which group a row appears in.
- **Different entries never merge**: the homographs `lead¹` and `lead²`, or `left` (headword) and `left` (an inflection of `leave`), are separate rows. Choosing a row opens exactly one entry.

Each row shows:

| Kind of the representative | Shown as |
|---|---|
| `headword`, `phrase` | The displayed form, with the homograph number after it when there is one |
| `variant`, `inflection` | The displayed form → the entry's `headword` (no arrow when they are the same) |
| `reading`, `hidden` | The entry's `headword`; for `reading`, followed by the matched reading (readings with the same key were already reduced to one row at packing time, e.g. `hǎo` and `hào`) |

Below it is the summary: the `summaries` row for the representative's `(entry, anchor)`. Without one, `variant`, `inflection`, `reading` and `hidden` rows, and rows at anchor 0, use `(entry, 0)`, because they are the same word as the headword. Nested headwords and `phrase` rows are different words, so they do not borrow the entry's summary and show none.

### 5.3 Opening a row

Choosing a row opens the representative's entry at the representative's anchor; when that anchor lies inside a tab, that tab is shown (`v1.md` §4.4).

## 6. Build checks

These stop the build: an invalid manifest, type registry or theme; a line that is not valid JSON; a root without `id` or `lang`; a duplicate entry key; a resource file with an unknown extension or a duplicate key.

These are listed in the build report without stopping the build:

- types missing from the registry, children the registry does not allow, attributes that are undeclared or of the wrong type;
- a node `id` used twice in one entry;
- a `link` node whose `entry` is not in the pack, or whose `anchor` is not in the target entry (readers fall back as in `v1.md` §6);
- a resource reference that is neither stored in the pack nor covered by an online source of its kind;
- a summary for a missing entry or anchor, or two summaries for one place;
- a `homograph` node whose text is not a positive integer;
- more than 32 readings; an `index` node all of whose readings are empty.
