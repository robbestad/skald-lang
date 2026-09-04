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
