use skald::{CaseMode, Options, Seed, compile_dictionaries, skald};
use std::sync::Arc;

fn run_dict(pattern: &str, dict: Arc<skald::Dictionary>, seed: u64) -> String {
    skald(
        pattern,
        &Options {
            seed: Some(Seed::Int(seed)),
            case_mode: Some(CaseMode::None),
            dictionary: Some(dict),
            ..Default::default()
        },
    )
    .unwrap_or_else(|e| panic!("{e}"))
}

fn run(pattern: &str) -> String {
    skald(
        pattern,
        &Options {
            seed: Some(Seed::Int(1)),
            case_mode: Some(CaseMode::None),
            ..Default::default()
        },
    )
    .unwrap_or_else(|e| panic!("{e}"))
}

fn err(pattern: &str) -> String {
    skald(
        pattern,
        &Options {
            seed: Some(Seed::Int(1)),
            case_mode: Some(CaseMode::None),
            ..Default::default()
        },
    )
    .unwrap_err()
    .to_string()
}

fn animals() -> Arc<skald::Dictionary> {
    Arc::new(compile_dictionaries(&[(
        "noun.dic",
        "#name noun\n#subs singular plural\n#class add animal\n> cat/cats\n> dog/dogs\n> bat/bats\n#class remove animal\n",
    )]))
}

#[test]
fn let_does_not_print() {
    assert_eq!(run("[let:hero; Alice]Y"), "Y");
}

#[test]
fn let_lookup_prints_the_binding() {
    assert_eq!(
        run("[case:none][let:hero; Alice][hero] walked"),
        "Alice walked"
    );
}

#[test]
fn let_binds_a_query_entry() {
    let dict = animals();
    let out = run_dict("[case:none][let:pet; <noun-animal>]saw [a][pet]", dict, 1);
    assert!(
        out.starts_with("saw a ") || out.starts_with("saw an "),
        "{out}"
    );
    assert!(!out.contains('<'));
}

#[test]
fn collect_and_oxford_join_three() {
    assert_eq!(
        run("[let:xs; [collect:3; a]][join:xs; ,\\s; and]"),
        "a, a, and a"
    );
}

#[test]
fn join_two_items_skips_the_comma() {
    assert_eq!(
        run("[let:xs; [collect:2; a]][join:xs; ,\\s; and]"),
        "a and a"
    );
}

#[test]
fn join_one_item_is_bare() {
    assert_eq!(run("[let:xs; [collect:1; a]][join:xs; ,\\s; and]"), "a");
}

#[test]
fn join_without_conjunction() {
    assert_eq!(run("[let:xs; [collect:3; x]][join:xs; -]"), "x-x-x");
}

#[test]
fn len_of_a_list_and_a_string() {
    assert_eq!(run("[let:xs; [collect:3; a]][len:xs]"), "3");
    assert_eq!(run("[len:hello]"), "5");
}

#[test]
fn pick_from_a_singleton_list() {
    assert_eq!(run("[let:xs; [collect:4; Z]][pick:xs]"), "Z");
}

#[test]
fn collect_unique_queries() {
    let dict = animals();
    let out = run_dict(
        "[case:none][let:pets; [collect:3; <noun-animal ::!p>]][join:pets; ,\\s; and]",
        dict,
        2,
    );
    let parts: Vec<&str> = if out.contains(", and ") {
        let (head, last) = out.split_once(", and ").unwrap();
        let mut v: Vec<&str> = head.split(", ").collect();
        v.push(last);
        v
    } else if out.contains(" and ") {
        out.split(" and ").collect()
    } else {
        vec![&out[..]]
    };
    let set: std::collections::HashSet<_> = parts.iter().copied().collect();
    assert_eq!(set.len(), parts.len(), "{out}");
    assert!(!out.contains('<'));
}

#[test]
fn reserved_let_name_errors() {
    let msg = err("[let:rep; x]");
    assert!(msg.contains("reserved"), "{msg}");
}

#[test]
fn unknown_tag_still_errors() {
    let msg = err("[nope]");
    assert!(msg.contains("Unknown tag"), "{msg}");
}

#[test]
fn if_sees_a_let_binding() {
    assert_eq!(run("[let:hero; A][if:hero]{yes}{no}"), "yes");
}

#[test]
fn join_inline_collect() {
    assert_eq!(run("[join: [collect:3; b]; ,\\s; and]"), "b, b, and b");
}
