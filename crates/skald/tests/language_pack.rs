use skald::{
    CaseMode, LANGUAGE_PACK_FORMAT_VERSION, Options, Seed, from_json, from_language_pack, skald,
};
use std::fs;
use std::path::PathBuf;
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
fn empty_capabilities_use_locale_defaults() {
    let raw = r#"{
      "formatVersion": 1,
      "id": "empty-caps",
      "locale": "nb-NO",
      "contentVersion": "1",
      "capabilities": {},
      "tables": {
        "firstname": {
          "entries": [{"id":"fn-ada","forms":["Ada"],"classes":["female"]}]
        }
      }
    }"#;
    let pack = from_language_pack(raw).unwrap();
    assert_eq!(pack.capabilities.articles, "none");
    assert!(!pack.capabilities.rhyme);
    let err = skald("[a]Ada", &Options::from_pack(pack)).unwrap_err();
    assert!(
        err.to_string().to_lowercase().contains("article")
            || err.to_string().contains("indefinite"),
        "{err}"
    );
}

#[test]
fn table_subs_without_top_level_forms_still_require_form_length() {
    let raw = r#"{
      "formatVersion": 1,
      "id": "subs-only",
      "locale": "nb-NO",
      "contentVersion": "1",
      "tables": {
        "noun": {
          "subs": ["indefinite","definite"],
          "entries": [{"id":"n-hus","forms":["hus"]}]
        }
      }
    }"#;
    let err = from_language_pack(raw).unwrap_err();
    assert!(err.to_string().contains("missing forms"), "{err}");
}

#[test]
fn locale_without_capabilities_is_missing_pack() {
    let err = skald(
        "Ada",
        &Options {
            locale: Some("nb-NO".into()),
            case_mode: Some(CaseMode::None),
            ..Options::default()
        },
    )
    .unwrap_err();
    assert!(err.to_string().contains("missing language pack"), "{err}");
}

#[test]
fn from_pack_binds_locale_and_dictionary() {
    let pack = from_language_pack(&pack("")).unwrap();
    let opts = Options {
        seed: Some(Seed::Int(1)),
        case_mode: Some(CaseMode::None),
        ..Options::from_pack(pack)
    };
    assert_eq!(opts.locale.as_deref(), Some("nb-NO"));
    assert!(opts.capabilities.is_some());
    assert_eq!(skald("<firstname female>", &opts).unwrap(), "Ada");
}

#[test]
fn pack_overlay_keeps_profile_and_adds_tables() {
    let pack = from_language_pack(&pack("")).unwrap();
    let extra = from_json(
        r#"{"tables":{"kaffe_drikke":{"name":"kaffe_drikke","subs":["default"],"entries":[{"forms":["kaffe"],"classes":[]}]}}}"#,
    )
    .unwrap();
    let opts = Options {
        seed: Some(Seed::Int(1)),
        case_mode: Some(CaseMode::None),
        ..Options::from_pack(pack).with_overlay(&extra).unwrap()
    };
    assert_eq!(skald("<firstname female>", &opts).unwrap(), "Ada");
    assert_eq!(skald("<kaffe_drikke>", &opts).unwrap(), "kaffe");
}

#[test]
fn pack_overlay_rejects_lemma_only_noun() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../locales/nb-NO.json");
    let pack = from_language_pack(&fs::read_to_string(path).unwrap()).unwrap();
    let extra = from_json(
        r#"{"tables":{"noun":{"name":"noun","subs":["indefinite","definite","indefinite_pl","definite_pl"],"entries":[{"forms":["katt"],"classes":["m"]}]}}}"#,
    )
    .unwrap();
    let err = Options::from_pack(pack).with_overlay(&extra).unwrap_err();
    assert!(err.to_string().contains("forms"), "{err}");
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
