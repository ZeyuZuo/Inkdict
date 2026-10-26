//! Close spellings on the Oxford pack: common misspellings put the word first.

mod common;

use common::load;
use zdict_core::Group;

#[test]
fn common_misspellings_find_the_word_first() {
    let Some((pack, _)) = load("pack.json") else {
        return;
    };
    let cases = [
        ("recieve", "receive"),
        ("accomodate", "accommodate"),
        ("definately", "definitely"),
        ("beleive", "believe"),
        ("occurence", "occurrence"),
        ("seperate", "separate"),
        ("goverment", "government"),
        ("wierd", "weird"),
        ("untill", "until"),
        ("harbuor", "harbour"),
    ];
    for (typed, wanted) in cases {
        let found = pack.words(typed, 5).unwrap();
        let forms: Vec<_> = found.iter().map(|w| (w.group, w.form.as_str())).collect();
        assert_eq!(
            forms.first(),
            Some(&(Group::Close, wanted)),
            "{typed}: {forms:?}"
        );
    }
}
