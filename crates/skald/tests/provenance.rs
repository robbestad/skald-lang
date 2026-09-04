use skald::{CaseMode, Options, PartSource, Seed, explain};

fn opts() -> Options {
    Options {
        seed: Some(Seed::Int(1)),
        case_mode: Some(CaseMode::None),
        ..Default::default()
    }
}

#[test]
fn nested_block_keeps_dictionary_lineage() {
    let out = explain("[case:none]hello {<firstname male>|x}", &opts()).unwrap();
    assert!(
        out.parts.iter().any(|p| p.source == PartSource::Dictionary),
        "{:?}",
        out.parts
    );
    assert!(
        out.parts
            .iter()
            .any(|p| p.source == PartSource::Glue && p.text.contains("hello")),
        "{:?}",
        out.parts
    );
    assert!(
        out.choices.iter().any(|c| c.kind == "block"),
        "{:?}",
        out.choices
    );
}

#[test]
fn article_is_glue_in_front_of_query() {
    let out = explain("[case:none][a]<noun-animal>", &opts()).unwrap();
    assert!(
        out.parts
            .first()
            .is_some_and(|p| p.source == PartSource::Glue && (p.text == "a " || p.text == "an ")),
        "{:?}",
        out.parts
    );
    assert!(
        out.parts.iter().any(|p| p.source == PartSource::Dictionary),
        "{:?}",
        out.parts
    );
}

#[test]
fn named_channel_parts_are_separate() {
    let out = explain("[out:title]{Hi}body", &opts()).unwrap();
    assert_eq!(out.text, "body");
    let title = out
        .parts_by_channel
        .get("title")
        .cloned()
        .unwrap_or_default();
    assert!(
        title.iter().any(|p| p.text.contains("Hi")),
        "{title:?} parts={:?}",
        out.parts_by_channel
    );
    assert!(
        !out.parts.iter().any(|p| p.text.contains("Hi")),
        "title leaked into main {:?}",
        out.parts
    );
}

#[test]
fn let_query_is_not_emitted() {
    let out = explain("[let:who; <firstname male>]X", &opts()).unwrap();
    assert!(
        out.picks
            .iter()
            .any(|p| p.table == "firstname" && !p.emitted),
        "{:?}",
        out.picks
    );
}

#[test]
fn density_queries_count_main_dictionary_parts() {
    let out = explain("[case:none]<firstname male> and <firstname male>", &opts()).unwrap();
    let density = out.density.expect("density");
    assert_eq!(density.queries, 2, "{density:?} parts={:?}", out.parts);
}

#[test]
fn function_body_keeps_dictionary_lineage() {
    let out = explain(
        "[case:none][fn:greet; who]{[who] found [a] <noun-animal>}[greet: Ada]",
        &opts(),
    )
    .unwrap();
    assert!(
        out.parts
            .iter()
            .any(|p| p.source == PartSource::Dictionary && p.table.as_deref() == Some("noun")),
        "{:?}",
        out.parts
    );
}

#[test]
fn unresolved_query_has_span() {
    let out = explain("[case:none]<no_such_table>", &opts()).unwrap();
    assert_eq!(out.unresolved.len(), 1, "{:?}", out.unresolved);
    assert_eq!(out.unresolved[0].kind, "unresolved");
    assert!(out.unresolved[0].span.end > out.unresolved[0].span.start);
}

#[test]
fn replace_keeps_unmatched_as_input_lineage() {
    let out = explain(
        "[case:none][replace: hello world; /world/; {earth}]",
        &opts(),
    )
    .unwrap();
    assert_eq!(out.text, "hello earth");
    assert!(
        out.parts.iter().any(|p| p.text.contains("hello")),
        "{:?}",
        out.parts
    );
}
