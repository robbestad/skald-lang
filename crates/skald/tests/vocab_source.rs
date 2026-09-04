use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

fn dic_map(root: &PathBuf) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    fn walk(dir: &PathBuf, root: &PathBuf, out: &mut BTreeMap<String, String>) {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                walk(&path, root, out);
            } else if path.extension().and_then(|s| s.to_str()) == Some("dic") {
                let rel = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                out.insert(rel, fs::read_to_string(&path).unwrap());
            }
        }
    }
    walk(root, root, &mut out);
    out
}

#[test]
fn crate_vocab_is_the_authoritative_copy() {
    let crate_vocab = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vocab");
    let repo_vocab = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vocab");
    let crate_files = dic_map(&crate_vocab);
    let repo_files = dic_map(&repo_vocab);
    assert_eq!(
        crate_files.keys().collect::<Vec<_>>(),
        repo_files.keys().collect::<Vec<_>>(),
        "vocab file lists differ; crates/skald/vocab is authoritative"
    );
    for (name, body) in &crate_files {
        assert_eq!(
            repo_files.get(name),
            Some(body),
            "{name} differs between crate vocab and repo vocab"
        );
    }
}
