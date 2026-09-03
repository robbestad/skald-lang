use skald::{CaseMode, Options, Seed, skald};

fn run(pattern: &str) -> String {
    run_seed(pattern, 1)
}

fn run_seed(pattern: &str, seed: u64) -> String {
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

#[test]
fn prints_plain_text_in_a_block_once() {
    assert_eq!(run("{Example text}"), "Example text");
}

#[test]
fn repeats_plain_text_with_rep() {
    assert_eq!(run("[rep:3]{x}"), "xxx");
}

#[test]
fn joins_repetitions_with_a_space() {
    assert_eq!(run("[sep:\\s][rep:3]{x}"), "x x x");
}

#[test]
fn joins_repetitions_with_a_newline() {
    assert_eq!(run("[sep:\\n][rep:2]{x}"), "x\nx");
}

#[test]
fn picks_one_alternative_from_a_block() {
    assert_eq!(run("{alpha|alpha|alpha}"), "alpha");
}

#[test]
fn selects_independently_each_repetition() {
    let out = run("[rep:5]{A|B}");
    assert!(
        out.chars().all(|c| c == 'A' || c == 'B') && out.len() == 5,
        "{out}"
    );
}

#[test]
fn expands_backslash_c() {
    let out = run("[rep:8]{\\C}");
    assert_eq!(out.len(), 8);
    assert!(out.chars().all(|c| c.is_ascii_uppercase()), "{out}");
}

#[test]
fn uppercases() {
    assert_eq!(run("[case:upper]hello"), "HELLO");
}

#[test]
fn lowercases() {
    assert_eq!(run("[case:lower]HELLO"), "hello");
}

#[test]
fn title_cases_small_words() {
    assert_eq!(
        run("[case:title]i like all the big butts"),
        "I Like All the Big Butts"
    );
}

#[test]
fn sentence_cases() {
    assert_eq!(
        run("[case:sentence]i. like. big. butts"),
        "I. Like. Big. Butts"
    );
}

#[test]
fn word_cases() {
    assert_eq!(run("[case:word]I LIKE BIG BUTTS"), "I Like Big Butts");
}

#[test]
fn rng_is_deterministic_for_a_seed() {
    assert_eq!(
        run_seed("[rep:12]{A|B|C|D}", 42),
        run_seed("[rep:12]{A|B|C|D}", 42)
    );
    assert_ne!(
        run_seed("[rep:12]{A|B|C|D}", 42),
        run_seed("[rep:12]{A|B|C|D}", 43)
    );
}

#[test]
fn prefixes_a_an() {
    assert_eq!(run("[case:none][a]ogre"), "an ogre");
    assert_eq!(run("[case:none][a]turtle"), "a turtle");
    let out = run("[case:none][a] <noun>");
    assert!(out.starts_with("a ") || out.starts_with("an "), "{out}");
}

#[test]
fn if_takes_else_when_carrier_missing() {
    assert_eq!(run("[case:none][if:hero]{yes}{no}"), "no");
}

#[test]
fn picks_weighted_block_items() {
    assert_eq!(run("{(1000)yes|(0)no}"), "yes");
}

#[test]
fn emits_a_random_integer_in_range() {
    let n: i64 = run("[n:2;2]").parse().unwrap();
    assert_eq!(n, 2);
}

#[test]
fn verbalizes_rn_with_rs() {
    assert_eq!(
        run("[case:none][numfmt:verbal][rs:3;.]{[rn]}"),
        "one.two.three"
    );
}

#[test]
fn locks_two_blocks_together() {
    let out = run("[x:s;locked]{A|B}[x:s;locked]{A|B}");
    assert!(out == "AA" || out == "BB", "{out}");
}

#[test]
fn ping_walks_then_back() {
    assert_eq!(run("[x:s;ping][rep:5]{A|B|C}"), "ABCBA");
}

#[test]
fn pong_walks_the_other_way() {
    assert_eq!(run("[x:s;pong][rep:5]{A|B|C}"), "CBABC");
}

#[test]
fn roman_and_hex_numfmt() {
    assert_eq!(run("[numfmt:roman][n:14;14]"), "XIV");
    assert_eq!(run("[numfmt:hex][n:255;255]"), "FF");
}

#[test]
fn skips_a_block_when_chance_is_zero() {
    assert_eq!(run("[chance:0]{secret}ok"), "ok");
}

#[test]
fn protects_inner_attrs_from_leaking() {
    assert_eq!(run("[rep:3][protect:-]{a}"), "-aaa");
}

#[test]
fn unknown_tag_errors() {
    let err = skald(
        "[nope]hi",
        &Options {
            seed: Some(Seed::Int(1)),
            case_mode: Some(CaseMode::None),
            nsfw: false,
            dictionary: None,
        },
    )
    .unwrap_err();
    assert!(err.to_string().contains("Unknown tag"), "{err}");
}

#[test]
fn unknown_query_stays_in_output() {
    assert_eq!(run("[case:none]<notatoken xyz>"), "<notatoken xyz>");
}

#[test]
fn default_case_capitalizes_first_letter() {
    let out = skald(
        "hello",
        &Options {
            seed: Some(Seed::Int(1)),
            case_mode: None,
            nsfw: false,
            dictionary: None,
        },
    )
    .unwrap();
    assert_eq!(out, "Hello");
}
