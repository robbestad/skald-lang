use skald::{CaseMode, Options, Seed, compile_dictionaries, explain, skald, skald_output};
use std::sync::Arc;

fn opts() -> Options {
    Options {
        seed: Some(Seed::Int(11)),
        case_mode: Some(CaseMode::None),
        nsfw: false,
        dictionary: None,
    }
}

fn opts_dict(dict: Arc<skald::Dictionary>) -> Options {
    Options {
        seed: Some(Seed::Int(1)),
        case_mode: Some(CaseMode::None),
        nsfw: false,
        dictionary: Some(dict),
    }
}

#[test]
fn lists_dictionary_picks() {
    let out = explain("[case:none]<firstname male :: hero> and <::hero>", &opts())
        .unwrap_or_else(|e| panic!("{e}"));
    let mut parts = out.text.split(" and ");
    let a = parts.next().unwrap();
    let b = parts.next().unwrap();
    assert_eq!(a, b);
    assert_eq!(out.picks.len(), 1, "{out:?}");
    let pick = &out.picks[0];
    assert_eq!(pick.table, "firstname");
    assert_eq!(pick.value, a);
    assert_eq!(pick.carrier.as_deref(), Some("hero"));
    assert!(pick.classes.iter().any(|c| c == "male"), "{pick:?}");
    assert!(pick.span.end > pick.span.start);
}

#[test]
fn unique_queries_each_record_a_pick() {
    let dict = Arc::new(compile_dictionaries(&[(
        "pet.dic",
        "#name pet\n#subs default\n> apple\n> banana\n> cherry\n",
    )]));
    let out = explain("[case:none]<pet ::!p> <pet ::!p>", &opts_dict(dict)).unwrap();
    assert_eq!(out.picks.len(), 2, "{:?}", out.picks);
    assert_ne!(out.picks[0].value, out.picks[1].value);
    assert_eq!(out.picks[0].table, "pet");
}

#[test]
fn skald_output_does_not_trace() {
    let out = skald_output("<firstname male>", &opts()).unwrap();
    assert!(out.picks.is_empty(), "{:?}", out.picks);
    assert!(!out.text.is_empty());
}

#[test]
fn json_includes_picks() {
    let json = explain("<firstname male :: hero>", &opts())
        .unwrap()
        .to_json();
    assert!(json.contains("\"table\":\"firstname\""), "{json}");
    assert!(json.contains("\"carrier\":\"hero\""), "{json}");
    assert!(json.contains("\"picks\":["), "{json}");
}

#[test]
fn unknown_tag_suggests_a_neighbor() {
    let err = skald("[cae]hi", &opts()).unwrap_err().to_string();
    assert!(err.contains("Did you mean [case]"), "{err}");
}

#[test]
fn unknown_tag_without_neighbor_has_no_hint() {
    let err = skald("[zzzzzz]hi", &opts()).unwrap_err().to_string();
    assert!(err.contains("Unknown tag"), "{err}");
    assert!(!err.contains("Did you mean"), "{err}");
}

#[test]
fn unknown_rhyme_mode_suggests_perfect() {
    let err = skald("[rhyme:perfct]x", &opts()).unwrap_err().to_string();
    assert!(err.contains("Did you mean 'perfect'"), "{err}");
}

#[test]
fn collect_records_each_query() {
    let dict = Arc::new(compile_dictionaries(&[(
        "pet.dic",
        "#name pet\n#subs default\n> apple\n",
    )]));
    let out = explain("[let:xs; [collect:3; <pet>]][join:xs; ,]", &opts_dict(dict)).unwrap();
    assert_eq!(out.picks.len(), 3, "{:?}", out.picks);
}
