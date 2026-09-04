use skald::{CaseMode, Options, Seed, from_language_pack, skald};
use std::collections::HashSet;
use std::sync::Arc;

const NB_NO: &str = include_str!("../../../locales/nb-NO.json");

fn opts(seed: u64, pack: &skald::LanguagePack) -> Options {
    Options {
        seed: Some(Seed::Int(seed)),
        case_mode: Some(CaseMode::None),
        dictionary: Some(Arc::new(pack.dictionary.clone())),
        merge: false,
        capabilities: Some(pack.capabilities.clone()),
        ..Options::default()
    }
}

fn run(pattern: &str, seed: u64, pack: &skald::LanguagePack) -> String {
    skald(pattern, &opts(seed, pack)).unwrap_or_else(|e| panic!("{pattern} seed {seed}: {e}"))
}

#[test]
fn npm_copy_matches_canonical_pack() {
    let npm = include_str!("../../../packages/skald-lang/nb-no.json");
    assert_eq!(
        npm, NB_NO,
        "packages/skald-lang/nb-no.json must match locales/nb-NO.json"
    );
}

#[test]
fn loads_curated_bokmal_core() {
    let pack = from_language_pack(NB_NO).unwrap();
    assert_eq!(pack.id, "skald-nb-NO-core");
    assert_eq!(pack.locale, "nb-NO");
    assert_eq!(pack.capabilities.articles, "none");
    assert!(pack.dictionary.table("noun").unwrap().entries.len() >= 6);
    assert_eq!(
        pack.dictionary.table("noun").unwrap().subs,
        ["indefinite", "definite", "indefinite_pl", "definite_pl"]
    );
}

#[test]
fn rejects_english_indefinite_article() {
    let pack = from_language_pack(NB_NO).unwrap();
    let err = skald("[a]katt", &opts(1, &pack)).unwrap_err();
    assert!(
        err.to_string().to_lowercase().contains("article")
            || err.to_string().contains("indefinite"),
        "{err}"
    );
}

#[test]
fn noun_gender_and_definiteness() {
    let pack = from_language_pack(NB_NO).unwrap();
    let huset = run("<noun n definite>", 1, &pack);
    assert!(
        huset == "huset" || huset == "eplet" || huset == "barnet",
        "{huset}"
    );
    let boka = run("<noun f definite>", 1, &pack);
    assert!(boka == "boka" || boka == "jenta", "{boka}");
    let katten = run("<noun animal definite>", 1, &pack);
    assert!(
        ["katten", "hunden", "hesten"].contains(&katten.as_str()),
        "{katten}"
    );
}

#[test]
fn bound_noun_keeps_the_same_lemma_across_forms() {
    let pack = from_language_pack(NB_NO).unwrap();
    let pairs = [("katt", "katten"), ("hund", "hunden"), ("hest", "hesten")];
    for seed in 1..=100 {
        let line = run("<noun animal :: dyr> / <::dyr definite>", seed, &pack);
        assert!(
            pairs.iter().any(|(a, b)| line == format!("{a} / {b}")),
            "seed {seed}: {line}"
        );
    }
}

#[test]
fn declared_pronouns_are_not_inferred_from_names() {
    let pack = from_language_pack(NB_NO).unwrap();
    for seed in 1..=40 {
        assert_eq!(run("<pron nom female>", seed, &pack), "hun");
        assert_eq!(run("<pron acc female>", seed, &pack), "henne");
        assert_eq!(run("<pron poss male>", seed, &pack), "hans");
    }
}

#[test]
fn bound_firstname_replays_the_same_person() {
    let pack = from_language_pack(NB_NO).unwrap();
    let women = HashSet::from(["Kari", "Anne", "Marit", "Ingrid"]);
    for seed in 1..=100 {
        let line = run("<firstname female :: hero> og <::hero>", seed, &pack);
        let parts: Vec<_> = line.split(" og ").collect();
        assert_eq!(parts.len(), 2, "{line}");
        assert_eq!(parts[0], parts[1], "desynced name seed {seed}: {line}");
        assert!(women.contains(parts[0]), "unexpected name {line}");
    }
}

#[test]
fn product_and_ui_closed_blocks_cover_every_alternative() {
    let pack = from_language_pack(NB_NO).unwrap();
    let mut drinks = HashSet::new();
    let mut saves = HashSet::new();
    for seed in 1..=100 {
        drinks.insert(run("Betal {29|49|79} kr.", seed, &pack));
        saves.insert(run("Lagre {endringene|utkastet}.", seed, &pack));
    }
    assert_eq!(
        drinks,
        HashSet::from([
            "Betal 29 kr.".into(),
            "Betal 49 kr.".into(),
            "Betal 79 kr.".into()
        ])
    );
    assert_eq!(
        saves,
        HashSet::from(["Lagre endringene.".into(), "Lagre utkastet.".into()])
    );
}

#[test]
fn hundred_seed_teaching_fixture_stays_resolved() {
    let pack = from_language_pack(NB_NO).unwrap();
    let pattern = "<firstname female :: elev> åpnet <noun n definite>.";
    for seed in 1..=100 {
        let line = run(pattern, seed, &pack);
        assert!(!line.contains('<'), "seed {seed}: {line}");
        assert!(line.ends_with('.'), "{line}");
        assert!(
            line.contains(" åpnet huset.")
                || line.contains(" åpnet eplet.")
                || line.contains(" åpnet barnet."),
            "seed {seed}: {line}"
        );
    }
}
