use skald::{Options, Seed, en_us, from_language_pack, preflight_errors, skald};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn nb() -> skald::LanguagePack {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../locales/nb-NO.json");
    from_language_pack(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn english_skald_still_emits_unknown_table() {
    let out = skald("<nonexistent_table>", &Options::default()).unwrap();
    assert!(out.contains("<nonexistent_table>"), "{out}");
}

#[test]
fn preflight_rejects_unknown_table() {
    let err = preflight_errors("<nonexistent_table>", &en_us(), None).unwrap_err();
    assert!(err.to_string().contains("PREFLIGHT_UNKNOWN_TABLE"), "{err}");
}

#[test]
fn preflight_rejects_unknown_form_on_nb_pack() {
    let pack = nb();
    let err = preflight_errors(
        "<noun imaginary_form>",
        &pack.dictionary,
        Some(&pack.capabilities),
    )
    .unwrap_err();
    assert!(err.to_string().contains("PREFLIGHT_UNKNOWN_FORM"), "{err}");
}

#[test]
fn language_pack_run_rejects_unknown_form() {
    let pack = nb();
    let err = skald(
        "<noun imaginary_form>",
        &Options {
            seed: Some(Seed::Int(1)),
            case_mode: Some(skald::CaseMode::None),
            dictionary: Some(std::sync::Arc::new(pack.dictionary)),
            merge: false,
            capabilities: Some(pack.capabilities),
            ..Options::default()
        },
    )
    .unwrap_err();
    assert!(err.to_string().contains("PREFLIGHT_UNKNOWN_FORM"), "{err}");
}

#[test]
fn preflight_rejects_unbound_carrier() {
    let err = preflight_errors("<::hero>", &en_us(), None).unwrap_err();
    assert!(
        err.to_string().contains("PREFLIGHT_UNBOUND_CARRIER"),
        "{err}"
    );
}

#[test]
fn preflight_allows_bound_carrier() {
    preflight_errors("<firstname female :: hero> <::hero>", &en_us(), None).unwrap();
}

#[test]
fn preflight_rejects_empty_class_intersection() {
    let pack = nb();
    let err =
        preflight_errors("<noun m n>", &pack.dictionary, Some(&pack.capabilities)).unwrap_err();
    assert!(
        err.to_string().contains("PREFLIGHT_EMPTY_CANDIDATES"),
        "{err}"
    );
}

#[test]
fn language_pack_run_rejects_empty_class_intersection() {
    let pack = nb();
    let err = skald(
        "<noun m n>",
        &Options {
            seed: Some(Seed::Int(1)),
            case_mode: Some(skald::CaseMode::None),
            dictionary: Some(std::sync::Arc::new(pack.dictionary)),
            merge: false,
            capabilities: Some(pack.capabilities),
            ..Options::default()
        },
    )
    .unwrap_err();
    assert!(
        err.to_string().contains("PREFLIGHT_EMPTY_CANDIDATES"),
        "{err}"
    );
}

#[test]
fn preflight_allows_matching_class() {
    let pack = nb();
    preflight_errors("<noun m>", &pack.dictionary, Some(&pack.capabilities)).unwrap();
}

#[test]
fn preflight_rejects_empty_regex_filter() {
    let pack = nb();
    let err = preflight_errors(
        "<noun ~ /^zzzznotaword/>",
        &pack.dictionary,
        Some(&pack.capabilities),
    )
    .unwrap_err();
    assert!(
        err.to_string().contains("PREFLIGHT_EMPTY_CANDIDATES"),
        "{err}"
    );
}

#[test]
fn language_pack_run_rejects_empty_regex_filter() {
    let pack = nb();
    let err = skald(
        "<noun ~ /^zzzznotaword/>",
        &Options {
            seed: Some(Seed::Int(1)),
            case_mode: Some(skald::CaseMode::None),
            dictionary: Some(std::sync::Arc::new(pack.dictionary)),
            merge: false,
            capabilities: Some(pack.capabilities),
            ..Options::default()
        },
    )
    .unwrap_err();
    assert!(
        err.to_string().contains("PREFLIGHT_EMPTY_CANDIDATES"),
        "{err}"
    );
}

#[test]
fn preflight_allows_matching_regex() {
    let pack = nb();
    preflight_errors(
        "<noun ~ /^katt/>",
        &pack.dictionary,
        Some(&pack.capabilities),
    )
    .unwrap();
}

#[test]
fn english_skald_still_emits_empty_regex() {
    let out = skald("<noun ~ /^zzzznotaword/>", &Options::default()).unwrap();
    assert!(out.contains('<'), "{out}");
}

#[test]
fn artifact_verify_rejects_unknown_table() {
    let dir = tempfile_dir();
    let pattern_path = dir.join("bad.skald");
    fs::write(&pattern_path, "<nonexistent_table>").unwrap();
    let bin = PathBuf::from(env!("CARGO_BIN_EXE_skald"));
    let manifest = Command::new(&bin)
        .args(["--case", "none", "manifest", pattern_path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(manifest.status.success(), "{:?}", manifest);
    let verify = Command::new(&bin)
        .args(["verify", pattern_path.to_str().unwrap()])
        .output()
        .unwrap();
    assert_ne!(verify.status.code(), Some(0), "verify should fail");
    let err = String::from_utf8_lossy(&verify.stderr);
    assert!(
        err.contains("PREFLIGHT_UNKNOWN_TABLE")
            || String::from_utf8_lossy(&verify.stdout).contains("PREFLIGHT_UNKNOWN_TABLE"),
        "stderr={err} stdout={}",
        String::from_utf8_lossy(&verify.stdout)
    );
}

#[test]
fn artifact_verify_rejects_empty_candidates_using_manifest_pack() {
    let dir = tempfile_dir();
    let pack = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../locales/nb-NO.json");
    let pattern_path = dir.join("empty.skald");
    fs::write(&pattern_path, "<noun m n>").unwrap();
    let bin = PathBuf::from(env!("CARGO_BIN_EXE_skald"));
    let manifest = Command::new(&bin)
        .args([
            "--pack",
            pack.to_str().unwrap(),
            "--locale",
            "nb-NO",
            "--case",
            "none",
            "manifest",
            pattern_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(manifest.status.success(), "{:?}", manifest);
    let verify = Command::new(&bin)
        .args(["verify", pattern_path.to_str().unwrap()])
        .output()
        .unwrap();
    assert_ne!(verify.status.code(), Some(0), "verify should fail");
    let err = String::from_utf8_lossy(&verify.stderr);
    assert!(
        err.contains("PREFLIGHT_EMPTY_CANDIDATES"),
        "stderr={err} stdout={}",
        String::from_utf8_lossy(&verify.stdout)
    );
}

#[test]
fn artifact_verify_rejects_empty_regex_using_manifest_pack() {
    let dir = tempfile_dir();
    let pack = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../locales/nb-NO.json");
    let pattern_path = dir.join("regex.skald");
    fs::write(&pattern_path, "<noun ~ /^zzzznotaword/>").unwrap();
    let bin = PathBuf::from(env!("CARGO_BIN_EXE_skald"));
    let manifest = Command::new(&bin)
        .args([
            "--pack",
            pack.to_str().unwrap(),
            "--locale",
            "nb-NO",
            "--case",
            "none",
            "manifest",
            pattern_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(manifest.status.success(), "{:?}", manifest);
    let verify = Command::new(&bin)
        .args(["verify", pattern_path.to_str().unwrap()])
        .output()
        .unwrap();
    assert_ne!(verify.status.code(), Some(0), "verify should fail");
    let err = String::from_utf8_lossy(&verify.stderr);
    assert!(
        err.contains("PREFLIGHT_EMPTY_CANDIDATES"),
        "stderr={err} stdout={}",
        String::from_utf8_lossy(&verify.stdout)
    );
}

fn tempfile_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("skald-preflight-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    dir
}
