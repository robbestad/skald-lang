use skald::{CaseMode, Options, Seed, compile_dictionaries, parse, skald};
use std::sync::Arc;

fn opts() -> Options {
    Options {
        seed: Some(Seed::Int(1)),
        case_mode: Some(CaseMode::None),
        ..Default::default()
    }
}

fn run(pattern: &str) -> String {
    skald(pattern, &opts()).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn replace_literal_body() {
    assert_eq!(
        run("[replace: hello world; /world/; {earth}]"),
        "hello earth"
    );
}

#[test]
fn replace_binds_match() {
    assert_eq!(run("[replace: cat bat; /[a-z]+/; {[m]!}]"), "cat! bat!");
}

#[test]
fn replace_binds_groups() {
    assert_eq!(
        run("[replace: ab-cd; /([a-z]+)-([a-z]+)/; {[m2]-[m1]}]"),
        "cd-ab"
    );
}

#[test]
fn replace_keeps_unmatched() {
    assert_eq!(run("[replace: abc; /b/; {X}]"), "aXc");
}

#[test]
fn replace_regex_may_contain_semicolon() {
    assert_eq!(run("[replace: a;b;c; /;/; {-}]"), "a-b-c");
}

#[test]
fn replace_as_let_value_does_not_print() {
    assert_eq!(run("[let:msg; [replace: hi; /hi/; {yo}]]X[msg]"), "Xyo");
}

#[test]
fn let_block_is_a_pattern_rerun() {
    let out = run("[let:coin; {A|A}][coin][coin]");
    assert_eq!(out, "AA");
}

#[test]
fn replace_uses_a_pattern_binding() {
    assert_eq!(
        run("[let:bang; {[m]!}][replace: hi there; /[a-z]+/; bang]"),
        "hi! there!"
    );
}

#[test]
fn replace_missing_pattern_errors() {
    let err = skald("[replace: hello;; {x}]", &opts())
        .unwrap_err()
        .to_string();
    assert!(err.contains("[replace]"), "{err}");
}

#[test]
fn reserved_replace_cannot_be_a_fn() {
    let err = skald("[fn:replace]{x}", &opts()).unwrap_err().to_string();
    assert!(err.contains("reserved"), "{err}");
}

#[test]
fn backtick_replacer_points_at_replace() {
    let err = skald("[`a`: x]", &opts()).unwrap_err().to_string();
    assert!(err.contains("[replace"), "{err}");
}

#[test]
fn parses_replace_with_block_body() {
    let nodes = parse("[replace: in; /a/; {[m]}]").unwrap();
    match &nodes[0] {
        skald::Node::Tag(t) => {
            assert_eq!(t.name, "replace");
            assert_eq!(t.args.len(), 3);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn replace_on_a_query() {
    let dict = Arc::new(compile_dictionaries(&[(
        "pet.dic",
        "#name pet\n#subs default\n> cat\n",
    )]));
    let out = skald(
        "[replace: saw a <pet>; /cat/; {dog}]",
        &Options {
            dictionary: Some(dict),
            ..opts()
        },
    )
    .unwrap();
    assert_eq!(out, "saw a dog");
}
