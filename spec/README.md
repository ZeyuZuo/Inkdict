# Inkdict protocol

| File | Contents |
| --- | --- |
| [`v1.md`](v1.md) | Entry content, the type registry, themes, languages, links, resource references |
| [`pack-v1.md`](pack-v1.md) | The source directory, packing, the `.inkdict` file format, the word-list rules for readers |
| [`pack-v1.sql`](pack-v1.sql) | The `.inkdict` table definitions |
| [`samples/en-zh/`](samples/en-zh/) | The complete source directory of a small invented English–Chinese dictionary |

Version 1 is frozen (2026-10-27). Later revisions may only add what it allows to be added, or make an unclear rule explicit; they never change the meaning of an existing type, field or rule. A change in meaning starts version 2. (An earlier draft, v0, was never published as a stable format.)

A dictionary's own type registry and themes are not here: they belong to the dictionary and travel inside its `.inkdict` file.

This directory (the protocol documents and `samples/`) is published under CC BY 4.0; see [`LICENSE`](LICENSE). Implementing readers, packers or dictionaries that follow the protocol needs no further permission.
