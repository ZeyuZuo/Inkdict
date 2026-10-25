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
from dataclasses import asdict
from pathlib import Path
import random
import re
import sys
import time
import unicodedata

ROOT = Path(__file__).resolve().parent.parent
OXDATA = ROOT / 'oxdata'
PACK = OXDATA / 'oalecd10.zdpack'
EDGES = [
    'Café', 'CAFÉ', 'cafe', 'mother-in-law', 'mother in law', '&', '%', '  ', '', 'ˈæpl',
    'straße', 'STRASSE', 'ﬁ', 'ﬀ', 'Ⅻ', '①', 'Ａｐｐｌｅ', 'résumé', 'naïve', 'İstanbul', 'ǅ',
    'ǈ', 'ὒ', 'ᾈ', 'x́', 'a\u00a0b', '\u3000苹果', '苹果', '1,000', 'A&E', 'AT&T',
    'Ა', 'ꭰ', 'Ꭰ', 'ﬆ', 'ŉ', 'ǰ', 'ΐ', 'ẞ', 'K', 'Å', 'ë́', '\t a \n',
    '\u001c', '\u0085x', 'Ⓐ', '½', '²', 'a·b', 'ab‧c', 'pull↔up', 'ˌbʊk',
]
FAILED = object()
DROPPED = []


def reference(name, value, function, *args):
    """Record reference failures so the remaining cases can still be written."""
    try:
        return function(*args)
    except Exception as error:
        DROPPED.append((name, value, f'{type(error).__name__}: {error}'))
        return FAILED


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


def chars_golden(text):
    """Python Unicode character classes and changed single-character keys."""
    classes = ('assigned', 'isspace', 'isalnum', 'word', 'combining')
    data = {'unicode_version': unicodedata.unidata_version,
            **{name: [] for name in classes}, 'norm_single': []}
    word = re.compile(r'\w').fullmatch
    for cp in range(0x110000):
        char = chr(cp)
        category = unicodedata.category(char)
        if category == 'Cs':
            continue
        flags = (category != 'Cn', char.isspace(), char.isalnum(),
                 bool(word(char)), bool(unicodedata.combining(char)))
        for name, flag in zip(classes, flags):
            if flag:
                ranges = data[name]
                if ranges and ranges[-1][1] == cp - 1:
                    ranges[-1][1] = cp
                else:
                    ranges.append([cp, cp])
        if flags[0]:
            normalized = reference('chars.json', cp, text.norm, char)
            if normalized is not FAILED and normalized != char:
                data['norm_single'].append([cp, normalized])
    return data


def norm_golden(forms, headwords, text):
    """Keys for every dictionary form and normalization edge case."""
    cases = []
    for value in sorted(set(forms) | set(headwords) | set(EDGES)):
        normalized = reference('norm.json', value, text.norm, value)
        if normalized is not FAILED:
            cases.append([value, normalized])
    return cases


def suggest_golden(pack, headwords, forms, rng):
    """Prefixes, exact forms, uppercase forms and varied result limits."""
    queries = [(value, 20) for value in EDGES]
    for word in rng.sample(headwords, 1000):
        queries.append((word[:rng.randint(1, len(word))], 20))
    selected = rng.sample(forms, 500)
    queries.extend((form, 20) for form in selected)
    queries.extend((form.upper(), 20) for form in rng.sample(selected, 200))
    queries.extend((word, limit) for word in ('a', 'go', 'went', 'apple')
                   for limit in (1, 5, 50))
    cases = []
    for query, limit in queries:
        results = reference('suggest.json', [query, limit], pack.suggest, query, limit)
        if results is not FAILED:
            cases.append({'query': query, 'limit': limit,
                          'results': [asdict(result) for result in results]})
    return {'cases': cases}


def nodes(node, lang=''):
    """Walk dictionary nodes with their inherited language."""
    if isinstance(node, dict):
        lang = node.get('lang', lang)
        yield node, lang
        for child in node.get('children', []):
            yield from nodes(child, lang)


def source_inputs(text):
    """Read Chinese definition runs and all resource attributes in one pass."""
    chinese, refs = {}, set()
    cjk = re.compile(r'[\u3400-\u9fff]+')
    resource = re.compile(r'^(wa|xa|img):')
    with (OXDATA / 'entries.jsonl').open(encoding='utf-8') as source:
        for line in source:
            entry = json.loads(line)
            runs = set()
            for node, lang in nodes(entry):
                for value in node.get('attrs', {}).values():
                    if isinstance(value, str) and resource.match(value):
                        refs.add(value)
                if node.get('type') == 'definition' and lang.startswith('zh'):
                    runs.update(cjk.findall(text.plain(node)))
            if runs:
                chinese[entry['id']] = sorted(runs)
    return chinese, sorted(refs)


def search_golden(pack, headwords, chinese, text, rng):
    """English terms, Chinese snippets, edge queries and column filters."""
    queries = rng.sample([word for word in headwords if word.isalpha()], 300)
    for key in rng.sample(sorted(chinese), 200):
        run = rng.choice(chinese[key])
        length = rng.randint(1, min(3, len(run)))
        start = rng.randint(0, len(run) - length)
        queries.append(run[start:start + length])
    queries.extend(['', 'apple pie', "don't", 'Café', '苹果 apple', '的', 'run*',
                    'AND', 'NOT', '"quoted"', 'e-mail', '苹果', '123', 'ˈæpl', '  '])
    inputs = [(query, None) for query in queries]
    inputs.extend((query, 'example') for query in queries[:100])
    inputs.extend((query, 'text') for query in queries[100:150])
    cases = []
    for query, column in inputs:
        value = [query, column, 20]
        fts_query = reference('search.json', value, text.fts_query, query)
        if fts_query is FAILED:
            continue
        results = reference('search.json', value, pack.search, query, 20, column)
        if results is not FAILED:
            cases.append({'query': query, 'column': column, 'limit': 20,
                          'fts_query': fts_query,
                          'results': [asdict(result) for result in results]})
    return {'cases': cases}


def resources_golden(pack, refs, rng):
    """Unusual resource keys, a sample of ordinary keys and edge refs.
    Ordinary keys use the characters of 'apple#_gb_5'; '#' is in almost all."""
    ordinary, unusual = [], []
    for ref in refs:
        target = ordinary if re.fullmatch(r'[A-Za-z0-9_.#-]*', ref.split(':', 1)[1]) else unusual
        target.append(ref)
    if len(ordinary) < 2000:
        print(f'  resources: only {len(ordinary):,} ordinary refs available; including all')
    selected = set(unusual) | set(rng.sample(ordinary, min(2000, len(ordinary))))
    selected.update(['wa:', 'zz:key', 'img:a b/c?d#e%f', 'wa:中文', "xa:!~*'()"])
    cases = []
    for ref in sorted(selected):
        urls = reference('resources.json', ref, pack.resource_urls, ref)
        if urls is not FAILED:
            cases.append([ref, urls])
    return cases


def anchors_golden(pack, keys, anchor_node, rng):
    """Document-order anchor IDs, including every checked-in example."""
    selected = set(rng.sample(keys, 500))
    for path in sorted((OXDATA / 'examples').glob('*.json')):
        selected.add(json.loads(path.read_text(encoding='utf-8'))['id'])
    cases = []
    for key in sorted(selected):
        entry = reference('anchors.json', key, pack.entry, key)
        if entry is FAILED:
            continue
        anchors = []
        while True:
            try:
                node = anchor_node(entry, len(anchors))
            except IndexError:
                cases.append({'entry': key, 'anchors': anchors})
                break
            except Exception as error:
                DROPPED.append(('anchors.json', [key, len(anchors)],
                                f'{type(error).__name__}: {error}'))
                break
            anchors.append(node['id'])
    return cases


def write(name, data):
    path = OXDATA / 'golden' / name
    path.parent.mkdir(exist_ok=True)
    options = {'indent': 1} if name == 'pack.json' else {'separators': (',', ':')}
    path.write_text(json.dumps(data, ensure_ascii=False, **options) + '\n', encoding='utf-8')
    print(f'wrote {path} ({path.stat().st_size:,} bytes)')


def main():
    started = time.perf_counter()
    rng = random.Random(20261025)
    oxdic = Path(sys.argv[1] if len(sys.argv) > 1 else ROOT.parent).resolve()
    sys.path.insert(0, str(oxdic))
    from oxdic.pack import text
    from oxdic.pack.reader import Pack, anchor_node

    if not PACK.is_file():
        sys.exit(f'missing {PACK} (run scripts/sync-oxdata.sh first)')
    pack = Pack(PACK)
    try:
        write('pack.json', pack_golden(pack))
        headwords = [row[0] for row in pack.db.execute('SELECT headword FROM entries ORDER BY headword, key')]
        forms = [row[0] for row in pack.db.execute('SELECT DISTINCT form FROM lookup ORDER BY form')]
        keys = [row[0] for row in pack.db.execute('SELECT key FROM entries ORDER BY key')]
        chinese, refs = source_inputs(text)
        write('chars.json', chars_golden(text))
        write('norm.json', norm_golden(forms, headwords, text))
        write('suggest.json', suggest_golden(pack, headwords, forms, rng))
        write('search.json', search_golden(pack, headwords, chinese, text, rng))
        write('resources.json', resources_golden(pack, refs, rng))
        write('anchors.json', anchors_golden(pack, keys, anchor_node, rng))
    finally:
        pack.close()
    print(f'Runtime: {time.perf_counter() - started:.2f}s')
    print(f'Dropped inputs: {len(DROPPED)}')
    for name, value, error in DROPPED:
        print(f'  {name}: {value!r}: {error}')


if __name__ == '__main__':
    main()
