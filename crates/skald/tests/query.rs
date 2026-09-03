use skald::{CaseMode, Dictionary, Options, Seed, compile_dictionaries, en_us, skald};
use std::sync::Arc;

fn run_dict(pattern: &str, dict: Arc<Dictionary>, seed: u64) -> String {
    skald(
        pattern,
        &Options {
            seed: Some(Seed::Int(seed)),
            case_mode: Some(CaseMode::None),
            nsfw: false,
            dictionary: Some(dict),
        },
    )
    .unwrap_or_else(|e| panic!("{e}"))
}

fn run(pattern: &str, seed: u64) -> String {
    skald(
        pattern,
        &Options {
            seed: Some(Seed::Int(seed)),
            case_mode: Some(CaseMode::None),
            nsfw: false,
            dictionary: None,
        },
    )
    .unwrap_or_else(|e| panic!("{e}"))
}

fn pets(nsfw: bool) -> Arc<Dictionary> {
    let src = if nsfw {
        "#name pet\n#subs default\n> capybara\n#nsfw\n> forbidden\n#sfw\n"
    } else {
        "#name pet\n#subs default\n> capybara\n"
    };
    Arc::new(compile_dictionaries(&[("pet.dic", src)]))
}

#[test]
fn bundled_english_has_core_tables() {
    let dict = en_us();
    for name in [
        "firstname",
        "noun",
        "verb",
        "pron",
        "timenoun",
        "yn",
        "adj",
        "place",
    ] {
        assert!(dict.tables.contains_key(name), "missing {name}");
    }
    assert!(dict.tables.len() >= 30);
}

#[test]
fn classic_pattern_fills_queries() {
    let sentence = run(
        "<firstname male> likes to <verb-transitive> <noun.plural> with <pron poss male> pet <noun-animal> on <timenoun dayofweek plural>.",
        7,
    );
    assert!(!sentence.contains('<'), "{sentence}");
    assert!(!sentence.contains("undefined"), "{sentence}");
    assert!(sentence.len() > 20, "{sentence}");
}

#[test]
fn filters_by_class() {
    let dict = Arc::new(compile_dictionaries(&[(
        "yn.dic",
        "#name yn\n#subs default\n#class add yes\n> yep\n#class remove yes\n#class add no\n> nope\n#class remove no\n",
    )]));
    for seed in 0..12 {
        assert_eq!(
            run_dict("[case:none]<yn yes>", Arc::clone(&dict), seed),
            "yep"
        );
        assert_eq!(
            run_dict("[case:none]<yn no>", Arc::clone(&dict), seed),
            "nope"
        );
    }
}

#[test]
fn returns_plural_nouns() {
    let word = run("[case:none]<noun-animal plural>", 3);
    assert!(!word.contains('<'), "{word}");
    assert!(word.len() > 1, "{word}");
}

#[test]
fn recalls_a_match_carrier() {
    let out = run("[case:none]<firstname male :: hero> and <::hero>", 11);
    let mut parts = out.split(" and ");
    let a = parts.next().unwrap();
    let b = parts.next().unwrap();
    assert_eq!(a, b);
    assert!(!a.is_empty());
}

#[test]
fn if_sees_a_bound_carrier() {
    let hit = run("[case:none]<firstname :: x>[if:x]{yes}{no}", 1);
    assert!(hit.ends_with("yes"), "{hit}");
}

#[test]
fn custom_dictionary() {
    let dict = pets(false);
    assert_eq!(run_dict("[case:none]<pet>", dict, 1), "capybara");
}

#[test]
fn omits_nsfw_unless_flagged() {
    let dict = pets(true);
    for seed in 0..20 {
        assert_eq!(
            run_dict("[case:none]<pet>", Arc::clone(&dict), seed),
            "capybara"
        );
    }
    let wild: std::collections::HashSet<String> = (0..40)
        .map(|seed| {
            skald(
                "[case:none]<pet>",
                &Options {
                    seed: Some(Seed::Int(seed)),
                    case_mode: Some(CaseMode::None),
                    nsfw: true,
                    dictionary: Some(Arc::clone(&dict)),
                },
            )
            .unwrap()
        })
        .collect();
    assert!(wild.contains("forbidden"), "{wild:?}");
}

#[test]
fn unique_carrier_does_not_repeat() {
    let out = run("[case:none]<yn yes ::!a> <yn yes ::!a> <yn yes ::!a>", 4);
    let words: Vec<_> = out.split_whitespace().collect();
    let set: std::collections::HashSet<_> = words.iter().copied().collect();
    assert_eq!(set.len(), words.len(), "{out}");
}

#[test]
fn carrier_keeps_the_entry_for_another_form() {
    let dict = Arc::new(compile_dictionaries(&[(
        "noun.dic",
        "#name noun\n#subs singular plural\n> cat/cats\n> dog/dogs\n",
    )]));
    let out = run_dict("[case:none]<noun ::pet> and <::pet plural>", dict, 1);
    assert!(out == "cat and cats" || out == "dog and dogs", "{out}");
}

#[test]
fn plural_sub_follows_last_number() {
    let dict = Arc::new(compile_dictionaries(&[(
        "noun.dic",
        "#name noun\n#subs singular plural\n> cat/cats\n",
    )]));
    assert_eq!(
        run_dict("[case:none][n:1;1] <noun..plural>", Arc::clone(&dict), 1),
        "1 cat"
    );
    assert_eq!(
        run_dict("[case:none][n:2;2] <noun..plural>", dict, 1),
        "2 cats"
    );
}

#[test]
fn article_on_a_real_noun() {
    let out = run("[case:none][a]<noun animal>", 2);
    assert!(out.starts_with("a ") || out.starts_with("an "), "{out}");
    assert!(!out.contains('<'));
}

fn fruit() -> Arc<Dictionary> {
    Arc::new(compile_dictionaries(&[(
        "pet.dic",
        "#name pet\n#subs default\n> apple\n> apricot\n> banana\n> cat\n",
    )]))
}

#[test]
fn regex_keeps_matching_forms() {
    let dict = fruit();
    for seed in 0..20 {
        let w = run_dict("[case:none]<pet ~ /^a/>", Arc::clone(&dict), seed);
        assert!(w == "apple" || w == "apricot", "seed {seed}: {w}");
    }
}

#[test]
fn regex_negation_drops_matches() {
    let dict = fruit();
    for seed in 0..20 {
        let w = run_dict("[case:none]<pet !~ /^a/>", Arc::clone(&dict), seed);
        assert!(w == "banana" || w == "cat", "seed {seed}: {w}");
    }
}

#[test]
fn regex_no_match_stays_raw() {
    let dict = fruit();
    let w = run_dict("[case:none]<pet ~ /^zzz/>", dict, 1);
    assert!(w.contains('<'), "{w}");
}

#[test]
fn invalid_regex_is_a_runtime_error() {
    let dict = fruit();
    let err = skald(
        "<pet ~ /[/>",
        &Options {
            seed: Some(Seed::Int(1)),
            case_mode: Some(CaseMode::None),
            nsfw: false,
            dictionary: Some(dict),
        },
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("regex"), "{err}");
}

fn rhyming() -> Arc<Dictionary> {
    Arc::new(compile_dictionaries(&[(
        "noun.dic",
        "#name noun\n#subs singular plural\n\
         > cat/cats\n  | pron k\"{t/k\"{ts\n\
         > bat/bats\n  | pron b\"{t/b\"{ts\n\
         > dog/dogs\n  | pron d\"Og/d\"Ogz\n\
         > dude\n  | pron d\"ud\n\
         > net/nets\n  | pron n\"Et/n\"Ets\n",
    )]))
}

fn perfect_pair() -> Arc<Dictionary> {
    Arc::new(compile_dictionaries(&[(
        "noun.dic",
        "#name noun\n#subs singular plural\n\
         > cat/cats\n  | pron k\"{t/k\"{ts\n\
         > bat/bats\n  | pron b\"{t/b\"{ts\n",
    )]))
}

fn err_dict(pattern: &str, dict: Arc<Dictionary>) -> String {
    skald(
        pattern,
        &Options {
            seed: Some(Seed::Int(1)),
            case_mode: Some(CaseMode::None),
            nsfw: false,
            dictionary: Some(dict),
        },
    )
    .unwrap_err()
    .to_string()
}

#[test]
fn perfect_rhyme_pairs_cat_and_bat() {
    let dict = perfect_pair();
    for seed in 0..40 {
        let out = run_dict(
            "[case:none][rhyme:perfect]<noun ::~a> <noun ::~a>",
            Arc::clone(&dict),
            seed,
        );
        let mut parts = out.split_whitespace();
        let a = parts.next().unwrap();
        let b = parts.next().unwrap();
        let set = ["cat", "bat"];
        assert!(set.contains(&a), "seed {seed}: {out}");
        assert!(set.contains(&b), "seed {seed}: {out}");
        assert_ne!(a, b, "seed {seed}: {out}");
    }
}

#[test]
fn rhyme_ampersand_carrier_matches_tilde() {
    let dict = perfect_pair();
    let out = run_dict("[case:none][rhyme:perfect]<noun ::&a> <noun ::&a>", dict, 3);
    let words: Vec<_> = out.split_whitespace().collect();
    assert_eq!(words.len(), 2, "{out}");
    assert_ne!(words[0], words[1], "{out}");
    assert!(["cat", "bat"].contains(&words[0]), "{out}");
}

#[test]
fn slant_rhyme_pairs_final_consonants() {
    let dict = rhyming();
    let mut hits = 0;
    for seed in 0..40 {
        let out = run_dict(
            "[case:none][rhyme:slant]<noun ::~s> <noun ::~s>",
            Arc::clone(&dict),
            seed,
        );
        if out.contains('<') {
            continue;
        }
        hits += 1;
        let mut parts = out.split_whitespace();
        let a = parts.next().unwrap();
        let b = parts.next().unwrap();
        let t_coda = ["cat", "bat", "net"];
        assert!(
            t_coda.contains(&a) && t_coda.contains(&b) && a != b,
            "seed {seed}: {out}"
        );
    }
    assert!(hits > 0, "expected at least one slant pair");
}

#[test]
fn alliteration_pairs_leading_consonants() {
    let dict = rhyming();
    let mut hits = 0;
    for seed in 0..40 {
        let out = run_dict(
            "[case:none][rhyme:alliteration]<noun ::~h> <noun ::~h>",
            Arc::clone(&dict),
            seed,
        );
        if out.contains('<') {
            continue;
        }
        hits += 1;
        let mut parts = out.split_whitespace();
        let a = parts.next().unwrap();
        let b = parts.next().unwrap();
        let d_onset = ["dog", "dude"];
        assert!(
            d_onset.contains(&a) && d_onset.contains(&b) && a != b,
            "seed {seed}: {out}"
        );
    }
    assert!(hits > 0, "expected at least one alliteration pair");
}

#[test]
fn rhyme_without_phones_is_a_runtime_error() {
    let dict = pets(false);
    let err = err_dict("<pet ::~a>", dict);
    assert!(err.contains("pronunciation"), "{err}");
}

#[test]
fn unknown_rhyme_mode_is_a_runtime_error() {
    let err = skald(
        "[rhyme:forced]x",
        &Options {
            seed: Some(Seed::Int(1)),
            case_mode: Some(CaseMode::None),
            nsfw: false,
            dictionary: None,
        },
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("rhyme mode"), "{err}");
}

#[test]
fn rhyme_exhaustion_stays_raw() {
    let dict = perfect_pair();
    let out = run_dict(
        "[case:none][rhyme:perfect]<noun ::~a> <noun ::~a> <noun ::~a>",
        dict,
        1,
    );
    assert!(out.contains('<'), "{out}");
}

#[test]
fn bundled_perfect_rhyme_is_stable() {
    let out = run("[rhyme:perfect]<noun ::~a> / <noun ::~a>", 4);
    assert_eq!(out, "baboon / harpoon");
}

#[test]
fn default_rhyme_mode_is_perfect() {
    let dict = perfect_pair();
    let out = run_dict("[case:none]<noun ::~a> <noun ::~a>", dict, 8);
    let mut parts = out.split_whitespace();
    let a = parts.next().unwrap();
    let b = parts.next().unwrap();
    assert!(["cat", "bat"].contains(&a), "{out}");
    assert!(["cat", "bat"].contains(&b), "{out}");
}
