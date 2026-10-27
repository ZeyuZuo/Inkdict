//! Public sample expectations come from spec/samples/README.md and its JSON sources.
#[path = "common/sample.rs"]
mod common;

use inkdict_core::theme::{Theme, ThemeStack};
use inkdict_core::{
    Focus, Group, IndexKind, Link, LinkTarget, Node, Pack, RenderChild, RenderNode, Role,
};

fn pack() -> Pack {
    Pack::open(common::sample_pack()).unwrap()
}

#[test]
fn sample_metadata_and_entry_keys() {
    let pack = pack();
    let meta = pack.meta();
    assert_eq!(meta.id(), "sample-en-zh");
    assert_eq!(meta.title(), "示例英汉双解词典");
    assert_eq!(meta.norm(), inkdict_core::text::Scheme::Fold);
    assert_eq!(meta.source_lang(), "en");
    assert_eq!(meta.target_langs(), ["zh-Hans"]);
    assert_eq!(meta.themes(), ["default"]);
    assert_eq!(meta.entries(), Some(6));
    assert_eq!(meta.resources(), Some(4));
    assert_eq!(pack.prepare_words().unwrap(), 23);
    for (key, headword, homograph) in [
        ("harbour", "harbour", None),
        ("lantern", "lantern", None),
        ("go", "go", None),
        ("lead-1", "lead", Some(1)),
        ("lead-2", "lead", Some(2)),
        ("cafe", "café", None),
    ] {
        let entry = pack.entry(key).unwrap();
        assert_eq!(
            (entry.key.as_str(), entry.headword.as_str(), entry.homograph),
            (key, headword, homograph)
        );
        assert_eq!(entry.root.id.as_deref(), Some(key));
        assert_eq!(pack.entry_by_id(entry.id).unwrap().root, entry.root);
        assert_eq!(
            serde_json::from_str::<Node>(&pack.entry_json(key).unwrap()).unwrap(),
            entry.root
        );
    }
    assert!(matches!(
        pack.entry("missing"),
        Err(inkdict_core::Error::MissingEntry(_))
    ));
}

#[test]
fn sample_lead_order_homographs_and_hidden_forms() {
    let pack = pack();
    let words = pack.words("lead", 20).unwrap();
    let rows: Vec<_> = words
        .iter()
        .map(|w| {
            (
                w.form.as_str(),
                w.entry.as_str(),
                w.homograph,
                w.kind,
                w.anchor,
            )
        })
        .collect();
    assert_eq!(
        rows,
        [
            ("lead", "lead-1", Some(1), IndexKind::Headword, 0),
            ("lead", "lead-2", Some(2), IndexKind::Headword, 0),
            ("leading", "lead-1", Some(1), IndexKind::Inflection, 1),
            ("lead the way", "lead-1", Some(1), IndexKind::Phrase, 4),
            ("lead sb the way", "lead-1", Some(1), IndexKind::Phrase, 4),
        ]
    );
    assert!(
        words
            .iter()
            .all(|w| w.group == Group::Prefix && w.distance.is_none())
    );
    assert!(words[0].rank < words[1].rank, "the core badge boosts lead¹");
    let hidden = pack.words("leads", 20).unwrap();
    assert_eq!(hidden.len(), 2); // Hidden leads plus the optional-sb phrase.
    assert_eq!(
        (
            hidden[0].form.as_str(),
            hidden[0].kind,
            hidden[0].homograph,
            hidden[0].anchor
        ),
        ("lead", IndexKind::Hidden, Some(1), 0)
    );
    assert_eq!(hidden[0].entry, "lead-1");
    assert_eq!(hidden[0].summary.as_deref(), Some("v. 带路；引领"));
}

#[test]
fn sample_fold_choice_and_optional_readings() {
    let pack = pack();
    for query in ["CAFÉ", "cafe", "café"] {
        let words = pack.words(query, 20).unwrap();
        assert_eq!(words.len(), 1, "{query}");
        assert_eq!(
            (
                words[0].form.as_str(),
                words[0].entry.as_str(),
                words[0].kind,
                words[0].group
            ),
            ("café", "cafe", IndexKind::Headword, Group::Prefix)
        );
        assert_eq!(words[0].summary.as_deref(), Some("n. 咖啡馆；小餐馆"));
    }
    for (query, form, key, anchor, summary) in [
        ("go flat", "go flat out", "go", 6, "全力以赴"),
        ("go all", "go all out", "go", 6, "全力以赴"),
        ("lead sb", "lead sb the way", "lead-1", 4, "带路；率先"),
        ("lead the way", "lead the way", "lead-1", 4, "带路；率先"),
    ] {
        let words = pack.words(query, 20).unwrap();
        assert_eq!(words.len(), 1, "{query}");
        let word = &words[0];
        assert_eq!(
            (
                word.form.as_str(),
                word.entry.as_str(),
                word.anchor,
                word.kind,
                word.group
            ),
            (form, key, anchor, IndexKind::Phrase, Group::Prefix)
        );
        assert_eq!(word.summary.as_deref(), Some(summary));
        assert_eq!(word.summary_lang.as_deref(), Some("zh-Hans"));
    }
}

#[test]
fn sample_contains_close_summaries_and_group_limits() {
    let pack = pack();
    let contains = pack.words("arb", 20).unwrap();
    assert_eq!(
        contains.iter().map(|w| w.form.as_str()).collect::<Vec<_>>(),
        ["harbour", "harbor", "safe harbour"]
    );
    assert!(
        contains
            .iter()
            .all(|w| w.group == Group::Contains && w.distance.is_none())
    );
    let close = pack.words("harbur", 20).unwrap();
    assert_eq!(
        close
            .iter()
            .map(|w| (w.form.as_str(), w.distance))
            .collect::<Vec<_>>(),
        [
            ("harbour", Some(1)),
            ("harbor", Some(1)),
            ("harbours", Some(2))
        ]
    );
    assert!(close.iter().all(|w| w.group == Group::Close));
    for (query, form, anchor, summary) in [
        ("harbour", "harbour", 0, "n. 港口；避风港　v. 心怀"),
        ("harbor", "harbor", 0, "n. 港口；避风港　v. 心怀"),
        ("harboured", "harboured", 7, "n. 港口；避风港　v. 心怀"),
        ("went", "went", 1, "v. 去；走"),
        ("safe harbour", "safe harbour", 5, "安全港；免责保护"),
        ("on the go", "on the go", 4, "忙个不停"),
    ] {
        let words = pack.words(query, 20).unwrap();
        let word = &words[0];
        assert_eq!(
            (
                word.form.as_str(),
                word.anchor,
                word.summary.as_deref(),
                word.summary_lang.as_deref()
            ),
            (form, anchor, Some(summary), Some("zh-Hans"))
        );
    }
    // Both groups must survive a limit of one; limiting is after deduplication.
    for query in ["har", "go", "lead", "arb", "harbur"] {
        let all = pack.words(query, 20).unwrap();
        for limit in [0, 1, 2, 3] {
            let actual = pack.words(query, limit).unwrap();
            let expected: Vec<_> = [Group::Prefix, Group::Contains, Group::Close]
                .into_iter()
                .flat_map(|g| {
                    all.iter()
                        .filter(move |w| w.group == g)
                        .take(limit)
                        .cloned()
                })
                .collect();
            assert_eq!(actual, expected, "{query}, limit {limit}");
        }
    }
    let limited = pack.words("har", 1).unwrap();
    assert_eq!(
        limited
            .iter()
            .map(|w| (w.group, w.form.as_str()))
            .collect::<Vec<_>>(),
        [(Group::Prefix, "harbor"), (Group::Contains, "safe harbour")]
    );
}

fn links(node: &Node) -> Vec<&Node> {
    let mut found = Vec::new();
    if node.kind == "target" {
        found.push(node);
    }
    for child in node.child_nodes() {
        found.extend(links(child));
    }
    found
}

#[test]
fn sample_anchors_links_and_missing_target_fallbacks() {
    let pack = pack();
    let harbour = pack.entry("harbour").unwrap();
    let ids: Vec<_> = harbour
        .anchors()
        .iter()
        .map(|n| n.id.as_deref().unwrap())
        .collect();
    assert_eq!(
        ids,
        [
            "harbour",
            "n",
            "n-1",
            "n-1-e1",
            "n-2",
            "safe-harbour",
            "safe-harbour-1",
            "v",
            "v-1",
            "v-1-e1"
        ]
    );
    for (number, id) in ids.iter().enumerate() {
        assert_eq!(harbour.anchor_number(id), Some(number as u32));
        assert_eq!(
            harbour.anchor(number as u32).unwrap().id.as_deref(),
            Some(*id)
        );
    }
    assert!(harbour.anchor(10).is_none());
    assert_eq!(harbour.anchor_number("absent"), None);
    for (from, to, focus) in [
        ("lead-1", "lead-2", Focus::Top),
        ("lead-2", "lead-1", Focus::Anchor(2)),
        ("lantern", "harbour", Focus::Anchor(2)),
    ] {
        let entry = pack.entry(from).unwrap();
        let targets = links(&entry.root);
        assert_eq!(targets.len(), 1);
        assert_eq!(
            pack.resolve_link(&entry, targets[0]).unwrap(),
            LinkTarget::Entry {
                key: to.into(),
                focus
            }
        );
    }
    for (target, anchor, expected) in [
        (
            Some("lead-2"),
            Some("missing"),
            LinkTarget::Entry {
                key: "lead-2".into(),
                focus: Focus::Top,
            },
        ),
        (
            Some("missing"),
            Some("v-1"),
            LinkTarget::Entry {
                key: "lead-1".into(),
                focus: Focus::Anchor(2),
            },
        ),
        (
            None,
            Some("v-1"),
            LinkTarget::Entry {
                key: "lead-1".into(),
                focus: Focus::Anchor(2),
            },
        ),
        (
            Some("missing"),
            Some("missing"),
            LinkTarget::Lookup("café".into()),
        ),
        (None, None, LinkTarget::Lookup("café".into())),
    ] {
        let link = Link {
            entry: target.map(str::to_owned),
            anchor: anchor.map(str::to_owned),
            anchor_type: None,
            text: "café".into(),
        };
        assert_eq!(
            pack.resolve(&pack.entry("lead-1").unwrap(), &link).unwrap(),
            expected
        );
    }
}

#[test]
fn sample_embedded_resources_and_online_audio_fallback() {
    let pack = pack();
    for (reference, mime, source) in [
        ("audio:harbour-gb", "audio/wav", "audio/harbour-gb.wav"),
        ("audio:lantern-gb", "audio/wav", "audio/lantern-gb.wav"),
        ("audio:lantern-us", "audio/wav", "audio/lantern-us.wav"),
        ("image:lantern", "image/svg+xml", "image/lantern.svg"),
    ] {
        let (actual_mime, bytes) = pack.resource(reference).unwrap().unwrap();
        assert_eq!(actual_mime, mime);
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../spec/samples/en-zh/resources")
            .join(source);
        assert_eq!(bytes, std::fs::read(path).unwrap());
        assert!(!bytes.is_empty());
    }
    assert!(pack.resource("audio:harbour-us").unwrap().is_none());
    assert_eq!(
        pack.resource_urls("audio:harbour-us"),
        ["https://media.example.org/sample-en-zh/audio/harbour-us.mp3"]
    );
}

#[test]
fn sample_render_marks_media_the_pack_stores() {
    let pack = pack();
    assert!(pack.has_resource("audio:harbour-gb"));
    assert!(!pack.has_resource("audio:harbour-us"));
    assert!(!pack.has_resource("not a reference"));
    let registry = pack.registry().unwrap();
    let stack = ThemeStack::new(pack.theme("default").unwrap());
    let renderer = pack.renderer(&registry, &stack);
    let harbour = renderer.render(&pack.entry("harbour").unwrap());
    let audio: Vec<_> = nodes(&harbour)
        .into_iter()
        .filter_map(|n| n.audio.as_ref())
        .map(|a| (a.embedded.as_deref(), a.urls.len()))
        .collect();
    assert_eq!(audio, [(Some("audio:harbour-gb"), 1), (None, 1)]);
    let lantern = renderer.render(&pack.entry("lantern").unwrap());
    let image = nodes(&lantern)
        .into_iter()
        .find_map(|n| n.image.as_ref())
        .unwrap();
    assert_eq!(image.embedded.as_deref(), Some("image:lantern"));
    assert!(
        image.urls.is_empty(),
        "the sample has no online image source"
    );
}

fn nodes(root: &RenderNode) -> Vec<&RenderNode> {
    let mut found = vec![root];
    for child in &root.children {
        match child {
            RenderChild::Node(n) => found.extend(nodes(n)),
            RenderChild::Tabs(tabs) => {
                for tab in tabs {
                    if let Some(label) = &tab.label {
                        found.extend(nodes(label));
                    }
                    found.extend(nodes(&tab.panel));
                }
            }
            RenderChild::Text(_) => {}
        }
    }
    found
}

#[test]
fn sample_render_tabs_fonts_and_role_switches() {
    let pack = pack();
    let registry = pack.registry().unwrap();
    let base = pack.theme("default").unwrap();
    assert!(base.validate(&registry).is_empty());
    let mut stack = ThemeStack::new(base);
    let entry = pack.entry("harbour").unwrap();
    let tree = pack.renderer(&registry, &stack).render(&entry);
    let tabs = tree
        .children
        .iter()
        .find_map(|child| match child {
            RenderChild::Tabs(tabs) => Some(tabs),
            _ => None,
        })
        .unwrap();
    assert_eq!(tabs.len(), 2);
    for (tab, label, id, number) in [(&tabs[0], "noun", "n", 1), (&tabs[1], "verb", "v", 7)] {
        assert_eq!(
            tab.label.as_ref().unwrap().children,
            [RenderChild::Text(label.into())]
        );
        assert_eq!(tab.panel.anchor.as_ref().unwrap().id, id);
        assert_eq!(tab.panel.anchor.as_ref().unwrap().number, number);
        assert!(!nodes(&tab.panel).iter().any(|n| n.kind == "pos"));
    }
    let all = nodes(&tree);
    for (kind, lang, token, key) in [
        ("definition", Some("zh-Hans"), "text", "zh-Hans"),
        ("definition", None, "text", "default"),
        ("ipa", None, "phonetic", "default"),
    ] {
        let node = all
            .iter()
            .find(|n| n.kind == kind && n.lang.as_deref() == lang)
            .unwrap();
        let font = node.text.font.as_ref().unwrap();
        assert_eq!((font.token.as_str(), font.key.as_str()), (token, key));
    }
    assert_eq!(all.iter().filter(|n| n.audio.is_some()).count(), 2);
    for role in [Role::Pronunciation, Role::Example] {
        let mut overlay = "[meta]\nformat = \"inkdict-theme\"\nversion = 1\n".to_owned();
        for name in registry.types_with_role(role) {
            overlay.push_str(&format!("[style.\"{name}\"]\ndisplay = \"none\"\n"));
        }
        stack.push(Theme::parse("switch", &overlay).unwrap());
        let hidden = pack.renderer(&registry, &stack).render(&entry);
        let visible = nodes(&hidden);
        assert!(!visible.iter().any(|n| registry.has_role(&n.kind, role)));
        assert!(
            visible
                .iter()
                .any(|n| n.kind == "definition" && n.lang.as_deref() == Some("zh-Hans"))
        );
    }
}
