use skald::{CaseMode, Dictionary, Options, Seed, compile_dictionaries, explain, skald};
use std::sync::Arc;

fn drink_pack() -> Arc<Dictionary> {
    Arc::new(compile_dictionaries(&[(
        "inn_drink.dic",
        "#name inn_drink\n#subs default\n> ale\n> stew\n> bread\n",
    )]))
}

#[test]
fn overlay_keeps_firstname_and_adds_table() {
    let out = skald(
        "[case:none]<firstname female> ordered <inn_drink>.",
        &Options {
            seed: Some(Seed::Int(11)),
            case_mode: Some(CaseMode::None),
            dictionary: Some(drink_pack()),
            merge: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(!out.contains('<'), "{out}");
    assert!(out.contains("ordered"), "{out}");
}

#[test]
fn overlay_replaces_same_table_name() {
    let pack = Arc::new(compile_dictionaries(&[(
        "yn.dic",
        "#name yn\n#subs default\n> only-yes\n",
    )]));
    let out = skald(
        "[case:none]<yn>",
        &Options {
            seed: Some(Seed::Int(1)),
            case_mode: Some(CaseMode::None),
            dictionary: Some(pack),
            merge: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(out, "only-yes");
}

#[test]
fn merge_false_hides_bundled_firstname() {
    let out = skald(
        "[case:none]<firstname female> x <inn_drink>",
        &Options {
            seed: Some(Seed::Int(1)),
            case_mode: Some(CaseMode::None),
            dictionary: Some(drink_pack()),
            merge: false,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(out.contains("<firstname"), "{out}");
}

#[test]
fn overlay_is_seed_stable() {
    let opts = Options {
        seed: Some(Seed::Int(4)),
        case_mode: Some(CaseMode::None),
        dictionary: Some(drink_pack()),
        merge: true,
        ..Default::default()
    };
    assert_eq!(
        skald("[case:none]<inn_drink>", &opts).unwrap(),
        skald("[case:none]<inn_drink>", &opts).unwrap()
    );
}

#[test]
fn overlay_explain_still_traces() {
    let out = explain(
        "[case:none]<inn_drink>",
        &Options {
            seed: Some(Seed::Int(1)),
            case_mode: Some(CaseMode::None),
            dictionary: Some(drink_pack()),
            merge: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(out.picks[0].table, "inn_drink");
}
