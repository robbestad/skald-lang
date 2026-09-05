use std::path::PathBuf;
use std::process::Command;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_skald"))
}

#[test]
fn dict_overlay_keeps_firstname() {
    let dict = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/beats/data/inn.json");
    let out = Command::new(bin())
        .args([
            "--seed",
            "11",
            "--case",
            "none",
            "--dict",
            dict.to_str().unwrap(),
            "[case:none]<firstname female> ordered <inn_drink>.",
        ])
        .output()
        .expect("run skald");
    assert!(out.status.success(), "{:?}", out);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(!text.contains('<'), "{text}");
    assert!(text.contains("ordered"), "{text}");
}

#[test]
fn story_stdin_matches_arg_exit() {
    let pattern = "<firstname female :: hero>, [a] <adj> <noun-job>, <verb.ed> toward the <place>.";
    let arg = Command::new(bin())
        .args(["--story", "--case", "none", pattern])
        .output()
        .expect("arg");
    assert_eq!(arg.status.code(), Some(2));
    let mut child = Command::new(bin())
        .args(["--story", "--case", "none"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn");
    {
        use std::io::Write;
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(pattern.as_bytes())
            .unwrap();
    }
    let piped = child.wait_with_output().expect("piped");
    assert_eq!(piped.status.code(), Some(2), "{piped:?}");
}

#[test]
fn large_seed_is_not_rounded() {
    let pattern = "{A|B|C|D|E|F|G|H}";
    let a = Command::new(bin())
        .args(["--seed", "9007199254740993", "--case", "none", pattern])
        .output()
        .expect("large");
    let b = Command::new(bin())
        .args(["--seed", "9007199254740992", "--case", "none", pattern])
        .output()
        .expect("rounded");
    assert!(a.status.success(), "{:?}", a);
    assert!(b.status.success(), "{:?}", b);
    assert_ne!(
        String::from_utf8_lossy(&a.stdout),
        String::from_utf8_lossy(&b.stdout)
    );
}

#[test]
fn artifact_verify_rejects_tampered_pattern() {
    let dir = std::env::temp_dir();
    let path = dir.join("skald-artifact-hello.skald");
    std::fs::write(&path, "hello").unwrap();
    let wrote = Command::new(bin())
        .args(["manifest", path.to_str().unwrap(), "--case", "none"])
        .output()
        .expect("manifest");
    assert!(wrote.status.success(), "{:?}", wrote);
    let ok = Command::new(bin())
        .args(["verify", path.to_str().unwrap()])
        .output()
        .expect("verify");
    assert!(ok.status.success(), "{:?}", ok);
    std::fs::write(&path, "HELLO").unwrap();
    let bad = Command::new(bin())
        .args(["verify", path.to_str().unwrap()])
        .output()
        .expect("verify tampered");
    assert_eq!(bad.status.code(), Some(1), "{:?}", bad);
}

#[test]
fn locale_without_pack_is_an_error() {
    let out = Command::new(bin())
        .args(["--locale", "nb-NO", "--case", "none", "<firstname female>"])
        .output()
        .expect("locale");
    assert_ne!(out.status.code(), Some(0));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("missing language pack"), "{err}");
}

#[test]
fn pack_locale_fills_norwegian_names() {
    let pack = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../locales/nb-NO.json");
    let out = Command::new(bin())
        .args([
            "--locale",
            "nb-NO",
            "--pack",
            pack.to_str().unwrap(),
            "--seed",
            "1",
            "--case",
            "none",
            "<firstname female>",
        ])
        .output()
        .expect("pack");
    assert!(out.status.success(), "{:?}", out);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(!text.contains('<'), "{text}");
}

#[test]
fn pack_plus_dict_overlay_sees_palette() {
    let pack = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../locales/nb-NO.json");
    let overlay =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/beats/data/kaffe.json");
    let out = Command::new(bin())
        .args([
            "--locale",
            "nb-NO",
            "--pack",
            pack.to_str().unwrap(),
            "--dict",
            overlay.to_str().unwrap(),
            "--seed",
            "1",
            "--case",
            "none",
            "<kaffe_drikke>",
        ])
        .output()
        .expect("pack+dict");
    assert!(out.status.success(), "{:?}", out);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(!text.contains('<'), "{text}");
}

#[test]
fn positional_run_away_is_still_a_pattern() {
    let out = Command::new(bin())
        .args(["--case", "none", "run", "away"])
        .output()
        .expect("run away");
    assert!(out.status.success(), "{:?}", out);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("run away"), "{text}");
}

#[test]
fn artifact_run_applies_manifest_case() {
    let dir = std::env::temp_dir();
    let path = dir.join("skald-artifact-case.skald");
    std::fs::write(&path, "hello").unwrap();
    let wrote = Command::new(bin())
        .args(["manifest", path.to_str().unwrap(), "--case", "upper"])
        .output()
        .expect("manifest");
    assert!(wrote.status.success(), "{:?}", wrote);
    let run = Command::new(bin())
        .args(["run", path.to_str().unwrap()])
        .output()
        .expect("run");
    assert!(run.status.success(), "{:?}", run);
    assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), "HELLO");
}

#[test]
fn leading_zero_seed_is_error() {
    let out = Command::new(bin())
        .args(["--seed", "042", "--case", "none", "x"])
        .output()
        .expect("leading zero");
    assert_eq!(out.status.code(), Some(1), "{:?}", out);
}

#[test]
fn story_file_flag_matches_arg_exit() {
    let dir = std::env::temp_dir();
    let path = dir.join("skald-story-dont.skald");
    let pattern = "<firstname female :: hero>, [a] <adj> <noun-job>, <verb.ed> toward the <place>.";
    std::fs::write(&path, pattern).unwrap();
    let file = Command::new(bin())
        .args(["--story", "--case", "none", "-f", path.to_str().unwrap()])
        .output()
        .expect("file");
    assert_eq!(file.status.code(), Some(2), "{file:?}");
}

#[test]
fn artifact_run_seed_writes_unique_receipt() {
    let dir = std::env::temp_dir().join(format!("skald-receipt-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("line.skald");
    std::fs::write(&path, "{A|B|C|D|E|F|G|H}").unwrap();
    let wrote = Command::new(bin())
        .args([
            "--seed",
            "1",
            "--case",
            "none",
            "manifest",
            path.to_str().unwrap(),
        ])
        .output()
        .expect("manifest");
    assert!(wrote.status.success(), "{:?}", wrote);
    let run_default = Command::new(bin())
        .args(["--case", "none", "run", path.to_str().unwrap()])
        .output()
        .expect("run default");
    assert!(run_default.status.success(), "{:?}", run_default);
    let default_receipt = std::fs::read_to_string(dir.join("line.receipt.json")).unwrap();
    assert!(
        default_receipt.contains("\"value\": \"1\""),
        "{default_receipt}"
    );
    let run_42 = Command::new(bin())
        .args([
            "--seed",
            "42",
            "--case",
            "none",
            "run",
            path.to_str().unwrap(),
        ])
        .output()
        .expect("run 42");
    assert!(run_42.status.success(), "{:?}", run_42);
    let seeded = dir.join("line.seed-42.receipt.json");
    assert!(seeded.exists(), "missing {}", seeded.display());
    let rec_42 = std::fs::read_to_string(&seeded).unwrap();
    assert!(rec_42.contains("\"value\": \"42\""), "{rec_42}");
    let default_after = std::fs::read_to_string(dir.join("line.receipt.json")).unwrap();
    assert_eq!(default_after, default_receipt);
    let verify = Command::new(bin())
        .args(["verify", path.to_str().unwrap()])
        .output()
        .expect("verify default");
    assert!(verify.status.success(), "{:?}", verify);
    let verify_42 = Command::new(bin())
        .args(["--seed", "42", "verify", path.to_str().unwrap()])
        .output()
        .expect("verify 42");
    assert!(verify_42.status.success(), "{:?}", verify_42);
}

#[test]
fn artifact_run_loads_pack_from_manifest() {
    let dir = std::env::temp_dir().join(format!("skald-pack-recipe-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let pack_src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../locales/nb-NO.json");
    let pack = dir.join("nb-NO.json");
    std::fs::copy(&pack_src, &pack).unwrap();
    let path = dir.join("nb.skald");
    std::fs::write(&path, "<firstname female>").unwrap();
    let wrote = Command::new(bin())
        .current_dir(&dir)
        .args([
            "--pack",
            "nb-NO.json",
            "--locale",
            "nb-NO",
            "--seed",
            "1",
            "--case",
            "none",
            "manifest",
            "nb.skald",
        ])
        .output()
        .expect("manifest");
    assert!(wrote.status.success(), "{:?}", wrote);
    let sidecar = std::fs::read_to_string(dir.join("nb.skald.json")).unwrap();
    assert!(sidecar.contains("\"path\": \"nb-NO.json\""), "{sidecar}");
    let run = Command::new(bin())
        .current_dir(std::env::temp_dir())
        .args(["--case", "none", "run", path.to_str().unwrap()])
        .output()
        .expect("run");
    assert!(
        run.status.success(),
        "stderr={} stdout={}",
        String::from_utf8_lossy(&run.stderr),
        String::from_utf8_lossy(&run.stdout)
    );
    let text = String::from_utf8_lossy(&run.stdout);
    assert!(!text.contains('<'), "{text}");
}
