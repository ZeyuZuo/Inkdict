# zdict

Build with `cargo build --release -p zdict-cli`. Pass `--pack PATH` to any
command, or set `ZDICT_PACK`. All commands accept `--json`.

```sh
export ZDICT_PACK=oxdata/oalecd10.zdpack
target/release/zdict info
target/release/zdict suggest went -n 20
target/release/zdict search 苹果 --column text
target/release/zdict show apple --json
```

Generate seeded inputs and compare the Rust and Python readers:

```sh
../.venv/bin/python scripts/bench.py queries
target/release/zdict bench suggest oxdata/bench/suggest.txt --repeat 1
../.venv/bin/python scripts/bench.py run suggest oxdata/bench/suggest.txt --repeat 1
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
Percentiles use nearest rank, `ceil(percent * count / 100)`, over sorted
milliseconds. Pack opening, input loading, warm-up, sorting and printing are
excluded from total time. Total seconds includes the timed loop's overhead.
Text output rounds timings to three decimals; JSON retains their precision.
