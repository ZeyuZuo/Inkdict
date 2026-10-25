#!/usr/bin/env python3
"""Write the reference outputs the Rust core is checked against.

Runs the Python reference reader of the OxDic data repository over
oxdata/oalecd10.zdpack and writes JSON into oxdata/golden/ (git-ignored:
the outputs contain dictionary text).

    path/to/OxDic/.venv/bin/python scripts/make-golden.py [path/to/OxDic]

The default OxDic path is the parent directory of this repository.
"""
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parent.parent
OXDATA = ROOT / 'oxdata'
PACK = OXDATA / 'oalecd10.zdpack'


def pack_golden(pack):
    """Identification, meta and embedded files (spec/pack-v0.md §1, §2.1, §2.2)."""
    db = pack.db
    files = {name: {'bytes': len(body), 'sha256': hashlib.sha256(body).hexdigest()}
             for name, body in db.execute('SELECT name, body FROM files ORDER BY name')}
    return {
        'application_id': db.execute('PRAGMA application_id').fetchone()[0],
        'user_version': db.execute('PRAGMA user_version').fetchone()[0],
        'meta': pack.meta,
        'fts_langs': pack.langs,
        'files': files,
        'entry_rows': db.execute('SELECT count(*) FROM entries').fetchone()[0],
    }


def write(name, data):
    path = OXDATA / 'golden' / name
    path.parent.mkdir(exist_ok=True)
    path.write_text(json.dumps(data, ensure_ascii=False, indent=1) + '\n', encoding='utf-8')
    print(f'wrote {path}')


def main():
    oxdic = Path(sys.argv[1] if len(sys.argv) > 1 else ROOT.parent).resolve()
    sys.path.insert(0, str(oxdic))
    from oxdic.pack.reader import Pack

    if not PACK.is_file():
        sys.exit(f'missing {PACK} (run scripts/sync-oxdata.sh first)')
    pack = Pack(PACK)
    try:
        write('pack.json', pack_golden(pack))
    finally:
        pack.close()


if __name__ == '__main__':
    main()
