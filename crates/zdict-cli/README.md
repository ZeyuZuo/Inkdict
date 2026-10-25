# zdict

Build with `cargo build --release -p zdict-cli`. Pass `--pack PATH` to any
command, or set `ZDICT_PACK`. All commands accept `--json`.

```sh
export ZDICT_PACK=oxdata/oalecd10.zdpack
target/release/zdict info
target/release/zdict suggest went -n 20
target/release/zdict search 苹果 --column text
target/release/zdict show apple --json
target/release/zdict correct recieve -n 10
target/release/zdict render apple --theme default --overlay custom.toml
target/release/zdict themes
```

`correct` prints `form<TAB>distance<TAB>headword[ hom]<TAB>entry#anchor`;
its JSON output is an array of corrections with flattened suggestion fields.
`render` resolves keys and words like `show` and always prints the render tree
as pretty JSON. It applies the named pack theme (`default` when omitted), then
each `--overlay FILE` in order, with later layers taking priority. `themes`
lists the shipped theme names and their validation problems; it exits nonzero
if any theme is invalid. Its JSON output contains `name` and `problems` per theme.

Generate seeded inputs and compare the Rust and Python readers:

```sh
../.venv/bin/python scripts/bench.py queries
target/release/zdict bench suggest oxdata/bench/suggest.txt --repeat 1
../.venv/bin/python scripts/bench.py run suggest oxdata/bench/suggest.txt --repeat 1
target/release/zdict bench correct oxdata/bench/typos.tsv --json
target/release/zdict bench render oxdata/bench/render.txt --json
target/release/zdict eval correct oxdata/bench/typos.tsv --json
```

Use `search` or `entry` with the matching input file for the other workloads.
Generation reads Chinese definitions directly from the pack and writes only to
`oxdata/bench/`. The default Python reader is imported from the parent OxDic
repository; `--oxdic PATH` and `--pack PATH` can override those defaults before
the Python subcommand.

Each benchmark opens one pack, reads UTF-8 lines (skipping blank lines), and
warms up once on the first 100 queries. It then times every call, including
result construction and disposal, over the requested number of full passes.
Suggestions and searches use a limit of 20; entries decode the node tree.
Corrections use a limit of 10 and accept plain lines or the first TSV column.
Their index is built before warm-up and reported separately as `prepare_ms`.
Rendering times entry decoding and rendering together, with the registry and
default theme stack built once before warm-up. Render inputs must be entry keys.
Percentiles use nearest rank, `ceil(percent * count / 100)`, over sorted
milliseconds. Pack opening, input loading, warm-up, sorting and printing are
excluded from total time. Total seconds includes the timed loop's overhead.
Text output rounds timings to three decimals; JSON retains their precision.

Input generation also writes 2,000 `typo<TAB>headword` cases to `typos.tsv` and
1,000 keys to `render.txt` using a separate seed (20261027). Typo headwords are
distinct lowercase ASCII words of length at least three, sampled from a sorted
list. Each gets one random deletion, insertion, substitution or adjacent swap;
every fourth case gets two edits when its headword has at least six letters.
Typos that normalize to an existing lookup form are retried. The original
suggest/search/entry files retain their existing seed and contents.

`eval correct` reads two nonempty TSV columns (blank lines are skipped), calls
`correct(typo, 5)`, and compares the returned headwords exactly with the expected
headword. It reports top-1 and top-5 counts and percentages, plus top-5 misses
and the first 20 of those misses. JSON includes `count`, `top1`, `top1_percent`,
`top5`, `top5_percent`, `misses`, `miss_percent` and `first_misses` (each with
`typo`, `expected` and `got`). Malformed or empty evaluation input is an error.
