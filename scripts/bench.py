#!/usr/bin/env python3
"""Generate repeatable query files and benchmark the Python reference reader.

    ../.venv/bin/python scripts/bench.py queries
    ../.venv/bin/python scripts/bench.py run words oxdata/bench/words.txt
    ../.venv/bin/python scripts/bench.py run entry oxdata/bench/entry.txt

Both readers warm up once on the first 100 nonblank queries, then time each
call (including result disposal). Percentiles use the nearest-rank method.
"""
import argparse
import json
import math
from pathlib import Path
import random
import re
import sys
import time

ROOT = Path(__file__).resolve().parent.parent
PACK = ROOT / 'oxdata' / 'oalecd10.zdpack'
BENCH = ROOT / 'oxdata' / 'bench'
KINDS = ('words', 'entry')


def write_queries(pack, text):
    """Write the same seeded workload for both readers."""
    rng = random.Random(20261025)
    headwords = [row[0] for row in pack.db.execute(
        'SELECT headword FROM entries WHERE headword != ? ORDER BY headword, key', ('',))]
    keys = [row[0] for row in pack.db.execute('SELECT key FROM entries ORDER BY key')]
    prefixes = [word[:rng.randint(1, len(word))] for word in rng.sample(headwords, 2000)]
    entries = rng.sample(keys, 2000)
    BENCH.mkdir(parents=True, exist_ok=True)
    for kind, queries in zip(KINDS, (prefixes, entries)):
        path = BENCH / f'{kind}.txt'
        path.write_text('\n'.join(queries) + '\n', encoding='utf-8')
        print(f'wrote {path} ({len(queries):,} queries)')
    write_correction_and_render_queries(pack, text, keys)


def edit(word, rng):
    """Apply one typing error; swaps always exchange different neighbours."""
    swaps = [i for i in range(len(word) - 1) if word[i] != word[i + 1]]
    kinds = ['delete', 'insert', 'substitute'] + (['swap'] if swaps else [])
    kind = rng.choice(kinds)
    if kind == 'insert':
        index = rng.randrange(len(word) + 1)
        return word[:index] + rng.choice('abcdefghijklmnopqrstuvwxyz') + word[index:]
    if kind == 'swap':
        index = rng.choice(swaps)
        return word[:index] + word[index + 1] + word[index] + word[index + 2:]
    index = rng.randrange(len(word))
    if kind == 'delete':
        return word[:index] + word[index + 1:]
    letters = 'abcdefghijklmnopqrstuvwxyz'.replace(word[index], '')
    return word[:index] + rng.choice(letters) + word[index + 1:]


def write_correction_and_render_queries(pack, text, keys):
    """Use a separate seed so existing M1 workloads remain byte-identical."""
    rng = random.Random(20261027)
    headwords = sorted({row[0] for row in pack.db.execute('SELECT headword FROM entries')
                        if re.fullmatch('[a-z]{3,}', row[0])})
    known = {row[0] for row in pack.db.execute('SELECT DISTINCT norm FROM lookup')}
    cases = []
    for index, headword in enumerate(rng.sample(headwords, 2000), start=1):
        edits = 2 if len(headword) >= 6 and index % 4 == 0 else 1
        while True:
            typo = headword
            for _ in range(edits):
                typo = edit(typo, rng)
            if text.norm(typo) not in known:
                break
        cases.append(f'{typo}\t{headword}')
    renders = rng.sample(keys, 1000)
    for name, queries in [('typos.tsv', cases), ('render.txt', renders)]:
        path = BENCH / name
        path.write_text('\n'.join(queries) + '\n', encoding='utf-8')
        print(f'wrote {path} ({len(queries):,} queries)')


def statistics(samples, total_seconds):
    """Nearest-rank percentiles of per-call milliseconds."""
    samples.sort()
    return {
        'count': len(samples),
        **{f'p{percent}_ms': samples[math.ceil(percent * len(samples) / 100) - 1]
           for percent in (50, 95, 99)},
        'max_ms': samples[-1],
        'total_seconds': total_seconds,
    }


def run(pack, kind, path, repeat, as_json):
    """Warm up, then time the same reader calls as `zdict bench`."""
    queries = [line.split('\t', 1)[0] if kind == 'words' else line
               for line in path.read_text(encoding='utf-8').splitlines() if line.strip()]
    if not queries:
        raise ValueError(f'{path} contains no queries')
    if any(not query.strip() for query in queries):
        raise ValueError(f'{path} contains an empty query in its first TSV column')
    call = {'words': lambda query: pack.words(query, 20),
            'entry': pack.entry}[kind]
    prepare_ms = None
    if kind == 'words':
        started = time.perf_counter()
        # The reference reader lazily loads lookup rows and entry metadata.
        rows = pack._all_rows()
        if rows:
            pack._entry_info(rows[0][2])
        prepare_ms = (time.perf_counter() - started) * 1000
    for query in queries[:100]:
        call(query)
    samples = []
    total = time.perf_counter()
    for _ in range(repeat):
        for query in queries:
            started = time.perf_counter()
            call(query)
            samples.append((time.perf_counter() - started) * 1000)
    stats = statistics(samples, time.perf_counter() - total)
    if prepare_ms is not None:
        stats['prepare_ms'] = prepare_ms
    if as_json:
        print(json.dumps(stats, ensure_ascii=False, indent=2))
    else:
        for field, value in stats.items():
            print(f'{field}: {value}' if field == 'count' else f'{field}: {value:.3f}')


def positive(value):
    """An argparse repeat count, excluding empty benchmark runs."""
    value = int(value)
    if value < 1:
        raise argparse.ArgumentTypeError('must be at least 1')
    return value


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--oxdic', type=Path, default=ROOT.parent,
                        help='OxDic repository containing the Python reader')
    parser.add_argument('--pack', type=Path, default=PACK)
    subcommands = parser.add_subparsers(dest='command', required=True)
    subcommands.add_parser('queries', help='write deterministic benchmark inputs')
    runner = subcommands.add_parser('run', help='benchmark one kind of reader call')
    runner.add_argument('kind', choices=KINDS)
    runner.add_argument('queries_file', type=Path)
    runner.add_argument('--repeat', type=positive, default=1)
    runner.add_argument('--json', action='store_true')
    args = parser.parse_args()
    sys.path.insert(0, str(args.oxdic.resolve()))
    from oxdic.pack import text
    from oxdic.pack.reader import Pack

    if not args.pack.is_file():
        parser.exit(1, f'missing {args.pack} (run scripts/sync-oxdata.sh first)\n')
    pack = Pack(args.pack)
    try:
        if args.command == 'queries':
            write_queries(pack, text)
        else:
            run(pack, args.kind, args.queries_file, args.repeat, args.json)
    finally:
        pack.close()


if __name__ == '__main__':
    main()
