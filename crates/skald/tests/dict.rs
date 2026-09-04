use skald::{compile_dic, compile_dictionaries, from_json, to_json};

const SAMPLE: &str = r#"#version 2
#name name
#subs default abbr

#class add female
  > Alice
  > Betty
#class remove female

#class add male
  > Abbot
  > Sean/S
#class remove male

#class add male?
  #class add female?
    > Jordan
  #class remove female?
#class remove male?

#nsfw
> forbidden
#sfw
"#;

#[test]
fn aliases_name_to_firstname() {
    let table = compile_dic(SAMPLE, "unknown");
    assert_eq!(table.name, "firstname");
    assert_eq!(table.subs, vec!["default", "abbr"]);
}

#[test]
fn tracks_nested_classes() {
    let table = compile_dic(SAMPLE, "unknown");
    let jordan = table
        .entries
        .iter()
        .find(|e| e.forms[0] == "Jordan")
        .unwrap();
    assert_eq!(jordan.classes, vec!["male?", "female?"]);
    let alice = table
        .entries
        .iter()
        .find(|e| e.forms[0] == "Alice")
        .unwrap();
    assert_eq!(alice.classes, vec!["female"]);
}

#[test]
fn splits_forms_on_slash() {
    let table = compile_dic(SAMPLE, "unknown");
    let sean = table.entries.iter().find(|e| e.forms[0] == "Sean").unwrap();
    assert_eq!(sean.forms, vec!["Sean", "S"]);
}

#[test]
fn marks_nsfw_context_entries() {
    let table = compile_dic(SAMPLE, "unknown");
    let nsfw = table
        .entries
        .iter()
        .find(|e| e.forms[0] == "forbidden")
        .unwrap();
    assert!(nsfw.classes.iter().any(|c| c == "nsfw"));
    assert!(table.has_nsfw);
    assert!(table.by_class.contains_key("male"));
}

#[test]
fn attaches_extra_class_metadata() {
    let table = compile_dic(
        "#name noun\n#subs singular plural\n> golf ball/golf balls\n  | class round\n",
        "unknown",
    );
    assert!(table.entries[0].classes.iter().any(|c| c == "round"));
}

#[test]
fn keeps_sampa_pronunciation() {
    let table = compile_dic(
        "#name noun\n#subs singular plural\n> beagle/beagles\n  | pron b\"i-gVl/b\"i-gVlz\n> mute\n",
        "unknown",
    );
    assert_eq!(table.entries[0].phones, vec!["b\"i-gVl", "b\"i-gVlz"]);
    assert!(table.entries[1].phones.is_empty());
}

#[test]
fn json_roundtrip_preserves_entries() {
    let dict = compile_dictionaries(&[(
        "noun.dic",
        "#name noun\n#subs singular plural\n> cat/cats\n  | class animal\n> golf ball/golf balls\n",
    )]);
    let json = to_json(&dict);
    assert!(json.contains("golf ball"), "{json}");
    let back = from_json(&json).expect(&json);
    let noun = back.tables.get("noun").expect("noun");
    assert_eq!(noun.subs, vec!["singular", "plural"]);
    assert_eq!(noun.entries[0].forms, vec!["cat", "cats"]);
    assert!(noun.entries[0].classes.iter().any(|c| c == "animal"));
    assert_eq!(noun.entries[1].forms, vec!["golf ball", "golf balls"]);
    assert!(noun.by_class.contains_key("animal"));
}

#[test]
fn json_roundtrip_preserves_phones() {
    let dict = compile_dictionaries(&[(
        "noun.dic",
        "#name noun\n#subs singular plural\n> cat/cats\n  | pron k\"{t/k\"{ts\n",
    )]);
    let json = to_json(&dict);
    assert!(json.contains("k\\\"{t"), "{json}");
    let back = from_json(&json).expect(&json);
    let noun = back.tables.get("noun").expect("noun");
    assert_eq!(noun.entries[0].phones, vec!["k\"{t", "k\"{ts"]);
}
