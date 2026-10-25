#!/usr/bin/env bash
# Copy the built Oxford pack and test data from the OxDic data repository
# into oxdata/ (git-ignored: dictionary content must not be committed).
#
#   scripts/sync-oxdata.sh [path/to/OxDic]     default: the parent directory
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
oxdic="$(cd "${1:-$root/..}" && pwd)"
out="$root/oxdata"

for file in data/build/oalecd10.zdpack data/build/entries.jsonl; do
    [[ -f "$oxdic/$file" ]] || { echo "missing $oxdic/$file (run pack.py build in OxDic)" >&2; exit 1; }
done

mkdir -p "$out/examples"
cp -p "$oxdic"/data/build/{oalecd10.zdpack,entries.jsonl,convert_report.json,pack_report.json} "$out/"
cp -p "$oxdic"/examples/*.json "$oxdic"/examples/resources.toml "$out/examples/"
# Readable copies of the pack inputs; the pack itself holds the same files.
rm -rf "$out/pack-inputs" && cp -rp "$oxdic/packs/oalecd10" "$out/pack-inputs"
(cd "$out" && sha256sum oalecd10.zdpack entries.jsonl > SHA256SUMS)
# Reference outputs for the Rust core tests, from OxDic's Python reader.
"${PYTHON:-$oxdic/.venv/bin/python}" "$root/scripts/make-golden.py" "$oxdic"
echo "synced into $out:"
ls -l "$out" "$out/examples" "$out/golden"
