# Maintainer scripts

These compare Inkdict with the Python reference implementation and the Oxford data in the separate, private OxDic repository. Nobody else needs them; `cargo test` runs without them.

| Script | What it does |
| --- | --- |
| `sync-oxdata.sh [path/to/OxDic]` | Copies the built Oxford pack and test data into `oxdata/` (git-ignored) and regenerates `oxdata/golden/` |
| `make-golden.py` | Writes the Python reader's output for a fixed set of inputs to `oxdata/golden/`; the core's tests must match it exactly |
| `bench.py` | Generates seeded benchmark inputs and times the Python reader |
| `compare-packs.py A.inkdict B.inkdict` | Compares two packs table by table (entry bodies after decompression); used to check `inkdict pack` against the Python packer. Needs the `zstandard` module |
