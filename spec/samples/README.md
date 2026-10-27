# Sample source directory

`en-zh/` is the source directory of a small invented English–Chinese dictionary, before packing. The format is described in [`../pack-v1.md`](../pack-v1.md) §1; the packed tables in [`../pack-v1.sql`](../pack-v1.sql).

Build it with `inkdict pack spec/samples/en-zh/manifest.toml -o sample.inkdict`.

| File | Contents |
|---|---|
| `manifest.toml` | Pack information, languages, file locations, lookup settings, an online resource source |
| `types.toml` | The type registry: types, nesting, attributes, roles |
| `themes/default.toml` | The theme |
| `entries.jsonl` | 6 entries (harbour, lantern, go, lead¹, lead², café), one node tree per line |
| `summaries.jsonl` | Word-list summaries, one per entry or anchor |
| `resources/` | Audio and a picture stored in the pack; `audio:harbour-us` is missing on purpose, to show the fallback to the online source |

It shows:
- two languages (`targets = ["zh-Hans"]`) with fonts chosen per language;
- part-of-speech tabs (the theme's `tabs`);
- the homographs lead¹ and lead² (the `homograph` role), and `boost` ranking lead¹, which has a core-vocabulary badge, first;
- variants, inflections, idioms and a hidden form (`hidden`) in the index;
- `fold-1` lookup keys: café, cafe and CAFÉ all find café;
- reading expansion: `alt` (the `choice` role) gives go all out / go flat out, `opt` (the `optional` role) gives lead sb the way / lead the way;
- links between entries (lead¹ ↔ lead², lantern → an anchor in harbour);
- files stored in the pack, and the fallback to an online source when one is not.

A monolingual dictionary writes `targets = []`; everything else stays the same.

## Rows of `lookup` after packing

| norm | form | entry | anchor | kind |
|---|---|---|---|---|
| harbour | harbour | harbour | root | headword |
| harbor | harbor | harbour | root | variant |
| harbours / harboured / harbouring | same | harbour | v | inflection |
| safeharbour | safe harbour | harbour | safe-harbour | phrase |
| lantern | lantern | lantern | root | headword |
| go | go | go | root | headword |
| goes / went / gone / going | same | go | v | inflection |
| onthego | on the go | go | on-the-go | phrase |
| goallout / goflatout | go all out / go flat out | go | go-all-out | phrase |
| lead | lead | lead¹ | root | headword |
| leads | leads | lead¹ | root | hidden |
| led / leading | same | lead¹ | v | inflection |
| leadsbtheway / leadtheway | lead sb the way / lead the way | lead¹ | lead-the-way | phrase |
| lead | lead | lead² | root | headword |
| cafe | café | café | root | headword (the variant cafe has the same key, so only the headword row is kept) |

The anchor column shows the node `id`; the pack stores its pre-order number.
