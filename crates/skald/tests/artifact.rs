#![cfg(not(target_arch = "wasm32"))]

use skald::artifact::{
    ARTIFACT_FORMAT_LEGACY, Manifest, ManifestSeed, dictionary_hash, pattern_hash, receipt_path,
    receipt_path_for, sha256_hex, verify_pattern,
};
use skald::en_us;
use std::path::Path;

#[test]
fn sha256_hello_is_stable() {
    assert_eq!(
        sha256_hex(b"hello"),
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
    );
}

#[test]
fn pattern_hash_is_raw_bytes() {
    assert_eq!(
        pattern_hash("hello"),
        "sha256:2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
    );
    assert_ne!(pattern_hash("hello"), pattern_hash("hello\n"));
}

#[test]
fn verify_detects_tampering() {
    let manifest = Manifest::for_pattern("Ada", None, Some("none"), false, false);
    assert!(verify_pattern("Ada", &manifest).is_ok());
    assert!(verify_pattern("Eve", &manifest).is_err());
}

#[test]
fn read_manifest_rejects_other_runtime_versions() {
    let dir = std::env::temp_dir();
    let path = dir.join("skald-runtime-mismatch.skald.json");
    let mut manifest = Manifest::for_pattern("Ada", None, Some("none"), false, false);
    manifest.runtime_version = "0.0.0".into();
    std::fs::write(&path, serde_json::to_string_pretty(&manifest).unwrap()).unwrap();
    let err = skald::artifact::read_manifest(&path).unwrap_err();
    assert!(err.to_string().contains("runtimeVersion"), "{err}");
}

#[test]
fn format_2_locks_dictionary_hash() {
    let manifest = Manifest::for_pattern("Ada", None, Some("none"), false, false);
    assert_eq!(manifest.format_version, 2);
    assert_eq!(manifest.locale, "en-US");
    let expected = dictionary_hash(en_us().as_ref());
    assert_eq!(manifest.dictionary_hash.as_deref(), Some(expected.as_str()));
    assert!(manifest.replay_locked());
}

#[test]
fn format_1_imports_without_claiming_lock() {
    let dir = std::env::temp_dir();
    let path = dir.join("skald-format1.skald.json");
    let mut manifest = Manifest::for_pattern("Ada", None, Some("none"), false, false);
    manifest.format_version = ARTIFACT_FORMAT_LEGACY;
    manifest.dictionary_hash = None;
    std::fs::write(&path, serde_json::to_string_pretty(&manifest).unwrap()).unwrap();
    let loaded = skald::artifact::read_manifest(&path).unwrap();
    assert!(!loaded.replay_locked());
    verify_pattern("Ada", &loaded).unwrap();
}

#[test]
fn verify_receipt_rejects_wrong_profile() {
    let receipt = skald::artifact::Receipt {
        format_version: 2,
        pattern_hash: pattern_hash("Ada"),
        run_profile: "not-the-profile".into(),
        text: "Ada".into(),
        channels: Default::default(),
        seed: None,
    };
    let err =
        skald::artifact::verify_receipt(&receipt, "Ada", &Default::default(), "Ada").unwrap_err();
    assert!(err.to_string().contains("run profile"), "{err}");
}

#[test]
fn format_1_receipt_with_presentation_json_is_legacy() {
    let receipt = skald::artifact::Receipt {
        format_version: 1,
        pattern_hash: pattern_hash("Ada"),
        run_profile: skald::RUN_PROFILE.into(),
        text: "{\"text\":\"Ada\",\"channels\":{}}".into(),
        channels: Default::default(),
        seed: None,
    };
    let status =
        skald::artifact::verify_receipt(&receipt, "Ada", &Default::default(), "Ada").unwrap();
    assert_eq!(status, skald::artifact::ReceiptReplay::LegacySkipped);
}

#[test]
fn format_2_receipt_checks_channels() {
    let mut channels = std::collections::BTreeMap::new();
    channels.insert("title".into(), "A".into());
    let receipt = skald::artifact::Receipt {
        format_version: 2,
        pattern_hash: pattern_hash("[out:title]{A|B}"),
        run_profile: skald::RUN_PROFILE.into(),
        text: String::new(),
        channels: channels.clone(),
        seed: None,
    };
    assert_eq!(
        skald::artifact::verify_receipt(&receipt, "", &channels, "[out:title]{A|B}").unwrap(),
        skald::artifact::ReceiptReplay::Full
    );
    let err =
        skald::artifact::verify_receipt(&receipt, "", &Default::default(), "[out:title]{A|B}")
            .unwrap_err();
    assert!(err.to_string().contains("channels"), "{err}");
}

#[test]
fn receipt_path_is_unique_per_seed() {
    let path = Path::new("/tmp/line.skald");
    let default = ManifestSeed {
        kind: "u64".into(),
        value: "1".into(),
    };
    let other = ManifestSeed {
        kind: "u64".into(),
        value: "42".into(),
    };
    assert_eq!(
        receipt_path_for(path, Some(&default), Some(&default)),
        receipt_path(path)
    );
    let seeded = receipt_path_for(path, Some(&other), Some(&default));
    assert!(
        seeded.ends_with("line.seed-42.receipt.json"),
        "{}",
        seeded.display()
    );
}
