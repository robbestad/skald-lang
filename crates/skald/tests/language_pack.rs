use skald::{
    CaseMode, LANGUAGE_PACK_FORMAT_VERSION, Options, Seed, from_json, from_language_pack, skald,
};
use std::sync::Arc;

fn pack(extra: &str) -> String {
    format!(
        r#"{{
  "formatVersion": {LANGUAGE_PACK_FORMAT_VERSION},
  "id": "test-nb",
  "locale": "nb-NO",
  "contentVersion": "0.0.1",
  "capabilities": {{"articles":"none","numbersVerbal":"none","caseTitle":"none","rhyme":false}},
  "tables": {{
    "firstname": {{
      "name": "firstname",
      "subs": ["default"],
      "entries": [{{"id":"fn-ada","forms":["Ada"],"classes":["female"]}}]
    }}
  }}{extra}
}}"#
    )
}

#[test]
fn loads_a_strict_pack() {
    let pack = from_language_pack(&pack("")).unwrap();
    assert_eq!(pack.locale, "nb-NO");
    assert_eq!(pack.id, "test-nb");
    assert_eq!(pack.capabilities.articles, "none");
    assert_eq!(
        pack.dictionary.table("firstname").unwrap().entries[0]
            .id
            .as_deref(),
        Some("fn-ada")
    );
}

#[test]
fn rejects_unknown_top_level_fields() {
    let err = from_language_pack(&pack(r#", "extra": true"#)).unwrap_err();
    assert!(
        err.to_string().contains("unknown language pack fields"),
        "{err}"
    );
}

#[test]
fn rejects_duplicate_entry_ids() {
    let raw = r#"{
      "formatVersion": 1,
      "id": "dup",
      "locale": "nb-NO",
      "contentVersion": "1",
      "tables": {
        "firstname": {
          "entries": [
            {"id":"x","forms":["Ada"]},
            {"id":"x","forms":["Eve"]}
          ]
        }
      }
    }"#;
    let err = from_language_pack(raw).unwrap_err();
    assert!(err.to_string().contains("duplicate entry id"), "{err}");
}

#[test]
fn rejects_short_forms_instead_of_padding() {
    let raw = r#"{
      "formatVersion": 1,
      "id": "forms",
      "locale": "nb-NO",
      "contentVersion": "1",
      "forms": {"noun": ["indefinite","definite"]},
      "tables": {
        "noun": {
          "entries": [{"id":"n-hus","forms":["hus"]}]
        }
      }
    }"#;
    let err = from_language_pack(raw).unwrap_err();
    assert!(err.to_string().contains("missing forms"), "{err}");
}

#[test]
fn unknown_locale_is_an_error() {
    let raw = r#"{
      "formatVersion": 1,
      "id": "xx",
      "locale": "sv-SE",
      "contentVersion": "1",
      "tables": {"firstname": {"entries": [{"forms":["Ada"]}]}}
    }"#;
    let err = from_language_pack(raw).unwrap_err();
    assert!(err.to_string().contains("unknown locale"), "{err}");
}

#[test]
fn legacy_tables_json_still_loads() {
    let dict = from_json(r#"{"tables":{"firstname":{"name":"firstname","subs":["default"],"entries":[{"forms":["Ada"],"classes":[]}]}}}"#).unwrap();
    assert!(dict.table("firstname").is_some());
}

#[test]
fn missing_entry_id_is_an_error() {
    let raw = r#"{
      "formatVersion": 1,
      "id": "noid",
      "locale": "nb-NO",
      "contentVersion": "1",
      "tables": {"firstname": {"entries": [{"forms":["Ada"]}]}}
    }"#;
    let err = from_language_pack(raw).unwrap_err();
    assert!(err.to_string().contains("stable id"), "{err}");
}

#[test]
fn declared_forms_become_table_subs() {
    let raw = r#"{
      "formatVersion": 1,
      "id": "forms",
      "locale": "nb-NO",
      "contentVersion": "1",
      "forms": {"noun": ["indefinite","definite"]},
      "tables": {
        "noun": {
          "entries": [{"id":"n-hus","forms":["hus","huset"]}]
        }
      }
    }"#;
    let pack = from_language_pack(raw).unwrap();
    assert_eq!(
        pack.dictionary.table("noun").unwrap().subs,
        ["indefinite", "definite"]
    );
    let line = skald(
        "<noun definite>",
        &Options {
            seed: Some(Seed::Int(1)),
            case_mode: Some(CaseMode::None),
            dictionary: Some(Arc::new(pack.dictionary)),
            merge: false,
            capabilities: Some(pack.capabilities),
            ..Options::default()
        },
    )
    .unwrap();
    assert_eq!(line, "huset");
}

#[test]
fn articles_none_rejects_a_tag() {
    let pack = from_language_pack(&pack("")).unwrap();
    let err = skald(
        "[a]Ada",
        &Options {
            case_mode: Some(CaseMode::None),
            dictionary: Some(Arc::new(pack.dictionary)),
            merge: false,
            capabilities: Some(pack.capabilities),
            ..Options::default()
        },
    )
    .unwrap_err();
    assert!(err.to_string().contains("indefinite articles"), "{err}");
}
