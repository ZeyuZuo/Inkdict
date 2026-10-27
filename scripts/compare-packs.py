#!/usr/bin/env python3
"""Compare two .inkdict packs table by table; exit 1 when they differ.

    compare-packs.py A.inkdict B.inkdict

Used to check `inkdict pack` against the Python reference packer (OxDic).
Entry bodies are compared after decompression, so packs compressed with
different zstd dictionaries still match. Meta keys that record how and when
a pack was built are not compared. Needs the `zstandard` module.
"""
import sqlite3
import sys

import zstandard

# Allowed to differ between builders (spec/pack-v1.md §2.2).
BUILD_META = {'built_at', 'builder', 'unicode_version'}
TABLES = {
    'lookup': 'SELECT norm, rank, entry, anchor, form, kind FROM lookup ORDER BY norm, rank, entry, anchor',
    'summaries': 'SELECT entry, anchor, lang, text FROM summaries ORDER BY entry, anchor',
    'resources': 'SELECT kind, key, mime, body FROM resources ORDER BY kind, key',
    'resource_sources': 'SELECT kind, priority, template FROM resource_sources ORDER BY kind, priority',
}
SAMPLES = 5


def open_pack(path):
    db = sqlite3.connect(f'file:{path}?mode=ro', uri=True)
    files = dict(db.execute('SELECT name, body FROM files'))
    meta = dict(db.execute('SELECT key, value FROM meta'))
    dictionary = files.get(meta.get('zstd_dict', ''))
    data = zstandard.ZstdCompressionDict(dictionary) if dictionary else None
    return db, files, meta, zstandard.ZstdDecompressor(dict_data=data)


def entries(db, decompressor):
    for row in db.execute('SELECT id, key, headword, homograph, body FROM entries ORDER BY id'):
        yield row[:4] + (decompressor.decompress(row[4]),)


def compare_rows(name, left, right, problems):
    differing = 0
    for number, (a, b) in enumerate(zip(left, right)):
        if a != b:
            differing += 1
            if differing <= SAMPLES:
                print(f'  {name} row {number}: {short(a)} != {short(b)}')
    if len(left) != len(right):
        print(f'  {name}: {len(left)} rows vs {len(right)}')
    if differing or len(left) != len(right):
        problems.append(name)
    else:
        print(f'{name}: {len(left)} rows match')


def short(row):
    text = repr(row)
    return text if len(text) < 200 else text[:200] + '…'


def main(a_path, b_path):
    (a, a_files, a_meta, a_zstd), (b, b_files, b_meta, b_zstd) = open_pack(a_path), open_pack(b_path)
    problems = []
    for pragma in ('application_id', 'user_version'):
        left, right = a.execute(f'PRAGMA {pragma}').fetchone(), b.execute(f'PRAGMA {pragma}').fetchone()
        if left != right:
            print(f'  {pragma}: {left} != {right}')
            problems.append(pragma)
    compare_rows('entries', list(entries(a, a_zstd)), list(entries(b, b_zstd)), problems)
    for name, query in TABLES.items():
        compare_rows(name, a.execute(query).fetchall(), b.execute(query).fetchall(), problems)
    skip = {a_meta.get('zstd_dict'), b_meta.get('zstd_dict')}
    left = sorted((k, v) for k, v in a_files.items() if k not in skip)
    right = sorted((k, v) for k, v in b_files.items() if k not in skip)
    compare_rows('files', left, right, problems)
    left = sorted((k, v) for k, v in a_meta.items() if k not in BUILD_META)
    right = sorted((k, v) for k, v in b_meta.items() if k not in BUILD_META)
    compare_rows('meta', left, right, problems)
    for key in sorted(BUILD_META):
        print(f'  {key}: {a_meta.get(key)!r} / {b_meta.get(key)!r}')
    print('DIFFERENT: ' + ', '.join(problems) if problems else 'SAME')
    return 1 if problems else 0


if __name__ == '__main__':
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    sys.exit(main(sys.argv[1], sys.argv[2]))
