use skald::{CaseMode, Options, Seed, from_language_pack, skald};
use std::collections::HashSet;
use std::sync::Arc;

const NN_NO: &str = include_str!("../../../locales/nn-NO.json");

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
    let npm = include_str!("../../../packages/skald-lang/nn-no.json");
    assert_eq!(
        npm, NN_NO,
        "packages/skald-lang/nn-no.json must match locales/nn-NO.json"
    );
}

#[test]
fn loads_curated_nynorsk_core() {
    let pack = from_language_pack(NN_NO).unwrap();
    assert_eq!(pack.id, "skald-nn-NO-core");
    assert_eq!(pack.locale, "nn-NO");
    assert_eq!(pack.capabilities.articles, "none");
    assert!(pack.dictionary.table("noun").unwrap().entries.len() >= 6);
}

#[test]
fn rejects_english_indefinite_article() {
    let pack = from_language_pack(NN_NO).unwrap();
    let err = skald("[a]katt", &opts(1, &pack)).unwrap_err();
    assert!(
        err.to_string().to_lowercase().contains("article")
            || err.to_string().contains("indefinite"),
        "{err}"
    );
}

#[test]
fn noun_gender_and_nynorsk_plurals() {
    let pack = from_language_pack(NN_NO).unwrap();
    let huset = run("<noun n definite>", 1, &pack);
    assert!(
        huset == "huset" || huset == "eplet" || huset == "barnet",
        "{huset}"
    );
    let definite_pl = run("<noun animal definite_pl>", 1, &pack);
    assert!(
        ["kattane", "hundane", "hestane"].contains(&definite_pl.as_str()),
        "{definite_pl}"
    );
}

#[test]
fn bound_noun_keeps_the_same_lemma_across_forms() {
    let pack = from_language_pack(NN_NO).unwrap();
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
fn declared_pronouns_use_nynorsk_forms() {
    let pack = from_language_pack(NN_NO).unwrap();
    for seed in 1..=40 {
        assert_eq!(run("<pron nom female>", seed, &pack), "ho");
        assert_eq!(run("<pron acc female>", seed, &pack), "henne");
        assert_eq!(run("<pron poss female>", seed, &pack), "hennar");
        assert_eq!(run("<pron acc male>", seed, &pack), "honom");
        assert_eq!(run("<pron poss n>", seed, &pack), "dess");
    }
}

#[test]
fn bound_firstname_replays_the_same_person() {
    let pack = from_language_pack(NN_NO).unwrap();
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
    let pack = from_language_pack(NN_NO).unwrap();
    let mut drinks = HashSet::new();
    let mut saves = HashSet::new();
    for seed in 1..=100 {
        drinks.insert(run("Betal {29|49|79} kr.", seed, &pack));
        saves.insert(run("Lagre {endringane|utkastet}.", seed, &pack));
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
        HashSet::from(["Lagre endringane.".into(), "Lagre utkastet.".into()])
    );
}

#[test]
fn hundred_seed_teaching_fixture_stays_resolved() {
    let pack = from_language_pack(NN_NO).unwrap();
    let pattern = "<firstname female :: elev> opna <noun n definite>.";
    for seed in 1..=100 {
        let line = run(pattern, seed, &pack);
        assert!(!line.contains('<'), "seed {seed}: {line}");
        assert!(
            line.contains(" opna huset.")
                || line.contains(" opna eplet.")
                || line.contains(" opna barnet."),
            "seed {seed}: {line}"
        );
    }
}

#[test]
fn bokmal_and_nynorsk_packs_are_not_interchangeable() {
    let nn = from_language_pack(NN_NO).unwrap();
    let nb = from_language_pack(include_str!("../../../locales/nb-NO.json")).unwrap();
    assert_ne!(
        run("<pron nom female>", 1, &nn),
        run("<pron nom female>", 1, &nb)
    );
    assert_ne!(
        run("<noun animal indefinite_pl>", 1, &nn),
        run("<noun animal indefinite_pl>", 1, &nb)
    );
}
