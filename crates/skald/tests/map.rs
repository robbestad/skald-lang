use skald::{CaseMode, Options, PartSource, Seed, explain, skald};

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

fn err(pattern: &str) -> String {
    skald(pattern, &opts()).unwrap_err().to_string()
}

#[test]
fn construct_and_get() {
    assert_eq!(
        run("[let:npc; [map: name; Ada; pet; dog]][npc: name] has [a] [npc: pet]"),
        "Ada has a dog"
    );
}

#[test]
fn construct_does_not_print() {
    assert_eq!(run("[map: name; Ada]X"), "X");
}

#[test]
fn missing_key_is_empty() {
    assert_eq!(run("[let:npc; [map: name; Ada]]([npc: pet])"), "()");
}

#[test]
fn overlay_copies() {
    assert_eq!(
        run("[let:npc; [map: name; Ada]][let:npc; [map: npc; pet; cat]][npc: name] [npc: pet]"),
        "Ada cat"
    );
}

#[test]
fn overlay_overwrites() {
    assert_eq!(
        run("[let:npc; [map: name; Ada]][let:npc; [map: npc; name; Bo]][npc: name]"),
        "Bo"
    );
}

#[test]
fn len_and_join_use_values() {
    assert_eq!(run("[let:npc; [map: one; x; two; y]][len:npc]"), "2");
    assert_eq!(
        run("[let:npc; [map: one; x; two; y]][join:npc; ,\\s; and]"),
        "x and y"
    );
}

#[test]
fn pick_from_values() {
    assert_eq!(run("[let:npc; [map: one; Z; two; Z]][pick:npc]"), "Z");
}

#[test]
fn nested_get() {
    assert_eq!(
        run("[let:npc; [map: pet; [map: name; dog]]][npc: pet; name]"),
        "dog"
    );
}

#[test]
fn pattern_takes_a_map() {
    assert_eq!(
        run(
            "[let:tpl; {[who] found [a] [what]}][let:row; [map: who; Ada; what; hedgehog]][tpl: row]"
        ),
        "Ada found a hedgehog"
    );
}

#[test]
fn pattern_on_a_map_sees_siblings() {
    assert_eq!(
        run(
            "[let:row; [map: who; Ada; what; hedgehog; line; {[who] found [a] [what]}]][row: line]"
        ),
        "Ada found a hedgehog"
    );
}

#[test]
fn pattern_map_bindings_do_not_leak() {
    assert_eq!(
        run("[let:tpl; {[who]}][let:row; [map: who; Ada]][tpl: row][if:who]{leaked}{ok}"),
        "Adaok"
    );
}

#[test]
fn odd_pairs_error() {
    let msg = err("[map: name]");
    assert!(msg.contains("key/value"), "{msg}");
}

#[test]
fn reserved_word_can_be_a_key() {
    assert_eq!(run("[let:npc; [map: a; x]][npc: a]"), "x");
}

#[test]
fn reserved_map_cannot_be_a_fn() {
    let msg = err("[fn:map]{x}");
    assert!(msg.contains("reserved"), "{msg}");
}

#[test]
fn pattern_rejects_a_non_map_arg() {
    let msg = err("[let:tpl; {hi}][tpl: Ada]");
    assert!(msg.contains("[map]"), "{msg}");
}

#[test]
fn empty_map_is_truthy_binding() {
    assert_eq!(run("[let:npc; [map]][if:npc]{yes}{no}"), "yes");
}

#[test]
fn pattern_map_prints_entries_as_dictionary() {
    let out = explain(
        "[let:row; [map: who; <firstname male>; what; <noun-animal>]][let:tpl; {[who] found [a] [what].}][tpl: row]",
        &opts(),
    )
    .unwrap();
    assert!(
        out.parts
            .iter()
            .any(|p| p.source == PartSource::Dictionary && p.table.as_deref() == Some("firstname")),
        "{:?}",
        out.parts
    );
    assert!(
        out.parts
            .iter()
            .any(|p| p.source == PartSource::Dictionary && p.table.as_deref() == Some("noun")),
        "{:?}",
        out.parts
    );
    let density = out.density.expect("density");
    assert!(
        density.glue_ratio < 0.5 && density.queries >= 2,
        "expected query-heavy output, got {density:?} parts={:?}",
        out.parts
    );
}
