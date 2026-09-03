use skald::{CaseMode, Options, Seed, explain, lint_story, parse};

const INN: &str = "\
<firstname female :: hero> the {knight|ranger|traveler} and <firstname male :: other> the {liar|thief|priest} {walked|came} to the inn.
<::hero> sat by the {fire|window|door}. <::other> {ordered|asked for} {ale|stew|bread}.
The {innkeeper|boy} brought {a cup|a bowl|a plate} and {left|waited}.
<::other> {said|muttered}, looking at <pron acc female>.
<::hero> {did not answer|drank|stood}.
Outside, the {road|yard} was {dark|quiet|wet}.
{Then|At last} <::hero> {paid|rose|took her pack}. <::other> {smiled|did not follow|watched}.";

const DONT: &str = "\
<firstname female :: hero>, [a] <adj> <noun-job>, <verb.ed> toward the <place>.
<firstname male :: other> <verb-transitive ed> [a] <adj> <noun-animal>.
<::other> <verb.ed> <pron acc female>.";

const NPC: &str = "<firstname male> likes to <verb-transitive> <noun.plural> with <pron poss male> pet <noun-animal> on <timenoun dayofweek plural>.";

fn story_opts() -> Options {
    Options {
        seed: Some(Seed::Int(11)),
        case_mode: Some(CaseMode::None),
        story: true,
        ..Default::default()
    }
}

fn notes_of(pattern: &str) -> Vec<String> {
    let ast = parse(pattern).unwrap();
    lint_story(pattern, &ast)
}

#[test]
fn inn_pattern_is_clean() {
    let notes = notes_of(INN);
    assert!(notes.is_empty(), "{notes:?}");
}

#[test]
fn dont_pattern_flags_adj_verb_and_place() {
    let notes = notes_of(DONT);
    let joined = notes.join("\n");
    assert!(joined.contains("<adj>"), "{joined}");
    assert!(joined.contains("<verb.ed>"), "{joined}");
    assert!(joined.contains("<place>"), "{joined}");
}

#[test]
fn npc_is_silent_without_story_flag() {
    let out = explain(
        NPC,
        &Options {
            seed: Some(Seed::Int(1)),
            case_mode: Some(CaseMode::None),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        !out.notes.iter().any(|n| n.starts_with("story:")),
        "{:?}",
        out.notes
    );
}

#[test]
fn npc_flags_when_story_is_on() {
    let notes = notes_of(NPC);
    assert!(notes.iter().any(|n| n.contains("<verb.ed>")), "{notes:?}");
}

#[test]
fn carriers_and_pron_are_allowed() {
    let notes = notes_of("<firstname female :: hero> looked at <pron acc female>.");
    assert!(notes.is_empty(), "{notes:?}");
}

#[test]
fn neighbor_beats_do_not_cross() {
    let notes = notes_of("<firstname male :: h> sat.\n<verb.ed> later.");
    assert!(!notes.iter().any(|n| n.contains("combines")), "{notes:?}");
}

#[test]
fn explain_json_includes_story_notes() {
    let json = explain(DONT, &story_opts()).unwrap().to_json();
    assert!(json.contains("\"notes\":"), "{json}");
    assert!(json.contains("story:"), "{json}");
}

#[test]
fn story_lint_does_not_change_text() {
    let a = explain(INN, &story_opts()).unwrap();
    let b = explain(
        INN,
        &Options {
            seed: Some(Seed::Int(11)),
            case_mode: Some(CaseMode::None),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(a.text, b.text);
}
