#!/usr/bin/env python3
"""Write the reference outputs the Rust core is checked against.

Runs the Python reference reader of the OxDic data repository over
oxdata/oalecd10.inkdict and writes JSON into oxdata/golden/ (git-ignored:
the outputs contain dictionary text).

    path/to/OxDic/.venv/bin/python scripts/make-golden.py [path/to/OxDic]

The default OxDic path is the parent directory of this repository.
"""
from collections import Counter
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
PACK = OXDATA / 'oalecd10.inkdict'
EDGES = [
    'Café', 'CAFÉ', 'cafe', 'mother-in-law', 'mother in law', '&', '%', '  ', '', 'ˈæpl',
    'straße', 'STRASSE', 'ﬁ', 'ﬀ', 'Ⅻ', '①', 'Ａｐｐｌｅ', 'résumé', 'naïve', 'İstanbul', 'ǅ',
    'ǈ', 'ὒ', 'ᾈ', 'x́', 'a\u00a0b', '\u3000苹果', '苹果', '1,000', 'A&E', 'AT&T',
    'Ა', 'ꭰ', 'Ꭰ', 'ﬆ', 'ŉ', 'ǰ', 'ΐ', 'ẞ', 'K', 'Å', 'ë́', '\t a \n',
    '\u001c', '\u0085x', 'Ⓐ', '½', '²', 'a·b', 'ab‧c', 'pull↔up', 'ˌbʊk',
]
# Words of other scripts: fold-1 folds some marks and must keep others.
SCRIPTS = ['كِتَاب', 'שָׁלוֹם', 'Ἑλλάς', 'Ёлка', 'किताब', 'कि', 'が', 'ｶﾞ', 'ภาษาไทย', '한국어', 'Tiếng Việt',
           'ĳ', 'Æsir', 'ǅemal']
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
    """Identification, meta and embedded files (spec/pack-v1.md §2)."""
    db = pack.db
    files = {name: {'bytes': len(body), 'sha256': hashlib.sha256(body).hexdigest()}
             for name, body in db.execute('SELECT name, body FROM files ORDER BY name')}
    return {
        'application_id': db.execute('PRAGMA application_id').fetchone()[0],
        'user_version': db.execute('PRAGMA user_version').fetchone()[0],
        'meta': pack.meta,
        'files': files,
        'entry_rows': db.execute('SELECT count(*) FROM entries').fetchone()[0],
        'summary_rows': db.execute('SELECT count(*) FROM summaries').fetchone()[0],
        'resource_rows': db.execute('SELECT count(*) FROM resources').fetchone()[0],
    }


def chars_golden(text):
    """Python Unicode character classes the lookup keys use (spec/pack-v1.md §3.2)
    and single characters whose key differs from the character itself."""
    classes = ('assigned', 'isspace', 'nonspacing', 'kept')
    data = {'unicode_version': unicodedata.unidata_version,
            **{name: [] for name in classes}, 'norm_single': [], 'exact_single': []}
    for cp in range(0x110000):
        char = chr(cp)
        category = unicodedata.category(char)
        if category == 'Cs':
            continue
        flags = (category != 'Cn', char.isspace(), category == 'Mn', category[0] in 'LMN')
        for name, flag in zip(classes, flags):
            if flag:
                ranges = data[name]
                if ranges and ranges[-1][1] == cp - 1:
                    ranges[-1][1] = cp
                else:
                    ranges.append([cp, cp])
        if flags[0]:
            for scheme, name in ((text.FOLD, 'norm_single'), (text.EXACT, 'exact_single')):
                normalized = reference('chars.json', [cp, scheme], text.norm, char, scheme)
                if normalized is not FAILED and normalized != char:
                    data[name].append([cp, normalized])
    return data


def norm_golden(forms, headwords, text):
    """fold-1 and exact-1 keys for every dictionary form and edge case."""
    cases = []
    for value in sorted(set(forms) | set(headwords) | set(EDGES) | set(SCRIPTS)):
        fold = reference('norm.json', value, text.norm, value, text.FOLD)
        exact = reference('norm.json', value, text.norm, value, text.EXACT)
        if fold is not FAILED and exact is not FAILED:
            cases.append([value, fold, exact])
    return cases


def typo(word, rng):
    """One random edit: swap, delete, insert or replace a letter."""
    letters = 'abcdefghijklmnopqrstuvwxyz'
    i = rng.randrange(len(word))
    edit = rng.choice(('swap', 'delete', 'insert', 'replace') if len(word) > 1 else ('insert', 'replace'))
    if edit == 'swap':
        i = min(i, len(word) - 2)
        return word[:i] + word[i + 1] + word[i] + word[i + 2:]
    if edit == 'delete':
        return word[:i] + word[i + 1:]
    if edit == 'insert':
        return word[:i] + rng.choice(letters) + word[i:]
    return word[:i] + rng.choice(letters) + word[i + 1:]


def words_golden(pack, headwords, forms, rng):
    """The word list (spec/pack-v1.md §5): prefixes, exact and uppercase forms,
    typing errors for the close group, homographs and varied limits."""
    queries = [(value, 20) for value in EDGES]
    for word in rng.sample(headwords, 1000):
        queries.append((word[:rng.randint(1, len(word))], 20))
    selected = rng.sample(forms, 500)
    queries.extend((form, 20) for form in selected)
    queries.extend((form.upper(), 20) for form in rng.sample(selected, 200))
    plain = [word for word in headwords if word.isalpha() and word.isascii() and len(word) >= 3]
    queries.extend((typo(word.lower(), rng), 20) for word in rng.sample(plain, 300))
    queries.extend((word, limit) for word in ('a', 'go', 'went', 'apple', 'lead', 'left', 'book', 'harboured')
                   for limit in (1, 5, 50))
    cases = []
    for query, limit in queries:
        results = reference('words.json', [query, limit], pack.words, query, limit)
        if results is not FAILED:
            cases.append({'query': query, 'limit': limit,
                          'results': [asdict(result) for result in results]})
    groups = Counter(result['group'] for case in cases for result in case['results'])
    print(f'  words: {len(cases):,} queries; rows by group: {dict(groups)}')
    return {'cases': cases}


def nodes(node, lang=''):
    """Walk dictionary nodes with their inherited language."""
    if isinstance(node, dict):
        lang = node.get('lang', lang)
        yield node, lang
        for child in node.get('children', []):
            yield from nodes(child, lang)


def resource_refs():
    """Every Oxford resource reference in the entries."""
    refs = set()
    resource = re.compile(r'^(wa|xa|img):')
    with (OXDATA / 'entries.jsonl').open(encoding='utf-8') as source:
        for line in source:
            for node, _ in nodes(json.loads(line)):
                for value in node.get('attrs', {}).values():
                    if isinstance(value, str) and resource.match(value):
                        refs.add(value)
    return sorted(refs)


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
        stored = reference('resources.json', ref, pack.resource, ref)
        if urls is not FAILED and stored is not FAILED:
            cases.append([ref, urls, stored is not None])
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


class PackResolver:
    """A resource's first online URL from the pack's own sources, as Rust's Preview markup
    prints it; a resource stored in the pack has no URL there."""

    def __init__(self, pack):
        self.pack = pack

    def url(self, ref):
        return next(iter(self.pack.resource_urls(ref)), None)


def html_golden(pack, keys, registry, theme, renderer):
    """Example HTML plus independent samples of ordinary and special entries."""
    rng = random.Random(20261026)
    selected = set(rng.sample(keys, 500))

    def special(node, path):
        if not isinstance(node, dict):
            return False
        if node['type'] == 'image':
            return True
        path = path + [renderer.frame(node, path)]
        style = theme.own_style(path)
        if style.get('collapse', 'none') != 'none' or style.get('tabs'):
            return True
        return any(special(child, path) for child in node.get('children', []))

    special_keys = []
    for key in keys:
        entry = reference('html.json', key, pack.entry, key)
        if entry is not FAILED and special(entry, []):
            special_keys.append(key)
    selected.update(rng.sample(special_keys, 300) if len(special_keys) > 300 else special_keys)
    examples = {}
    for path in sorted((OXDATA / 'examples').glob('*.json')):
        entry = json.loads(path.read_text(encoding='utf-8'))
        examples[entry['id']] = entry
    selected.update(examples)

    cases = []
    for key in sorted(selected):
        entry = examples[key] if key in examples else reference('html.json', key, pack.entry, key)
        if entry is FAILED:
            continue
        rendered = reference('html.json', key, renderer.entry, entry)
        if rendered is not FAILED:
            cases.append({'entry': key, 'html': rendered})
    errors = theme.validate(registry)
    print(f'  html: {len(cases):,} cases; theme_errors: {errors}')
    return {'theme_errors': errors, 'cases': cases}


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
    from oxdic.preview import Renderer
    from oxdic.protocol import Registry, Theme

    if not PACK.is_file():
        sys.exit(f'missing {PACK} (run scripts/sync-oxdata.sh first)')
    pack = Pack(PACK)
    try:
        write('pack.json', pack_golden(pack))
        headwords = [row[0] for row in pack.db.execute('SELECT headword FROM entries ORDER BY headword, key')]
        forms = [row[0] for row in pack.db.execute('SELECT DISTINCT form FROM lookup ORDER BY form')]
        keys = [row[0] for row in pack.db.execute('SELECT key FROM entries ORDER BY key')]
        write('chars.json', chars_golden(text))
        write('norm.json', norm_golden(forms, headwords, text))
        write('words.json', words_golden(pack, headwords, forms, rng))
        write('resources.json', resources_golden(pack, resource_refs(), rng))
        write('anchors.json', anchors_golden(pack, keys, anchor_node, rng))
        types_path = OXDATA / 'pack-inputs/types.toml'
        theme_path = OXDATA / 'pack-inputs/themes/default.toml'
        assert types_path.read_bytes() == pack.file('types.toml'), 'types.toml differs from pack'
        assert theme_path.read_bytes() == pack.file('themes/default.toml'), 'default theme differs from pack'
        registry, theme = Registry(types_path), Theme(theme_path)
        renderer = Renderer(registry, theme, PackResolver(pack))
        write('html.json', html_golden(pack, keys, registry, theme, renderer))
    finally:
        pack.close()
    print(f'Runtime: {time.perf_counter() - started:.2f}s')
    print(f'Dropped inputs: {len(DROPPED)}')
    for name, value, error in DROPPED:
        print(f'  {name}: {value!r}: {error}')


if __name__ == '__main__':
    main()
