use skald::{CaseMode, Options, Seed, skald, skald_output};

fn opts() -> Options {
    Options {
        seed: Some(Seed::Int(1)),
        case_mode: Some(CaseMode::None),
        ..Default::default()
    }
}

#[test]
fn named_channel_is_not_in_main() {
    let out = skald_output("pre[out:title]{HEAD}post", &opts()).unwrap();
    assert_eq!(out.text, "prepost");
    assert_eq!(
        out.channels.get("main").map(String::as_str),
        Some("prepost")
    );
    assert_eq!(out.channels.get("title").map(String::as_str), Some("HEAD"));
}

#[test]
fn skald_returns_only_main() {
    assert_eq!(
        skald("pre[out:title]{HEAD}post", &opts()).unwrap(),
        "prepost"
    );
}

#[test]
fn two_channels() {
    let out = skald_output("[out:title]{A}[out:body]{B}", &opts()).unwrap();
    assert_eq!(out.text, "");
    assert_eq!(out.channels.get("title").map(String::as_str), Some("A"));
    assert_eq!(out.channels.get("body").map(String::as_str), Some("B"));
}

#[test]
fn out_with_rep() {
    let out = skald_output("[out:list][rep:3]{x}", &opts()).unwrap();
    assert_eq!(out.channels.get("list").map(String::as_str), Some("xxx"));
    assert_eq!(out.text, "");
}

#[test]
fn missing_name_errors() {
    let err = skald("[out]{x}", &opts()).unwrap_err().to_string();
    assert!(err.contains("[out]"), "{err}");
}

#[test]
fn json_contains_channels() {
    let json = skald_output("[out:title]{Hi}", &opts()).unwrap().to_json();
    assert!(json.contains("\"title\":\"Hi\""), "{json}");
    assert!(json.contains("\"text\":\"\""), "{json}");
    assert!(json.contains("\"picks\":[]"), "{json}");
}

#[test]
fn named_channel_keeps_its_case() {
    let out = skald_output(
        "[out:title]{[case:title]hello world}[case:none]body",
        &opts(),
    )
    .unwrap();
    assert_eq!(out.text, "body");
    assert_eq!(
        out.channels.get("title").map(String::as_str),
        Some("Hello World")
    );
}

#[test]
fn later_case_none_does_not_rewrite_title() {
    let out = skald_output("[case:upper][out:title]{hi}[case:none]body", &opts()).unwrap();
    assert_eq!(out.channels.get("title").map(String::as_str), Some("HI"));
    assert_eq!(out.text, "body");
}
