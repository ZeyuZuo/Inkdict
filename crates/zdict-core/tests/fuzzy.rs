//! Spelling correction on the Oxford pack.

mod common;

use std::time::Instant;

use common::load;

#[test]
fn common_misspellings_find_the_word_first() {
    let Some((pack, _)) = load("pack.json") else {
        return;
    };
    let started = Instant::now();
    let keys = pack.prepare_correction().unwrap();
    eprintln!("correction index: {keys} keys in {:?}", started.elapsed());
    let cases = [
        ("recieve", "receive"),
        ("accomodate", "accommodate"),
        ("definately", "definitely"),
        ("teh", "the"),
        ("beleive", "believe"),
        ("occurence", "occurrence"),
        ("seperate", "separate"),
        ("goverment", "government"),
        ("wierd", "weird"),
        ("untill", "until"),
    ];
    for (typed, wanted) in cases {
        let found = pack.correct(typed, 5).unwrap();
        let forms: Vec<_> = found.iter().map(|c| c.suggestion.form.as_str()).collect();
        assert_eq!(forms.first(), Some(&wanted), "{typed}: {forms:?}");
    }
}
