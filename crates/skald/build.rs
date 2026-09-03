use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

fn main() {
    let out = PathBuf::from(env::var("OUT_DIR").unwrap()).join("en_us.rs");
    let mut f = fs::File::create(&out).expect("en_us.rs");

    if env::var("CARGO_FEATURE_EN_US").is_err() {
        writeln!(
            f,
            "fn load_en_us() -> crate::dict::Dictionary {{\n    crate::dict::Dictionary::empty()\n}}"
        )
        .unwrap();
        return;
    }

    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let vocab = manifest.join("../../vocab");
    println!("cargo:rerun-if-changed={}", vocab.display());

    let mut files = Vec::new();
    collect_dic(&vocab, &vocab, &mut files);
    files.sort();
    if files.is_empty() {
        panic!("no .dic files under {}", vocab.display());
    }

    writeln!(
        f,
        "fn load_en_us() -> crate::dict::Dictionary {{\n    crate::dict::compile_dictionaries(&["
    )
    .unwrap();
    for rel in &files {
        let escaped = rel.replace('\\', "/").replace('"', "\\\"");
        writeln!(
            f,
            "        (\"{escaped}\", include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/../../vocab/{escaped}\"))),"
        )
        .unwrap();
    }
    writeln!(f, "    ])\n}}").unwrap();
}

fn collect_dic(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display()));
    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            collect_dic(root, &path, out);
            continue;
        }
        if name.ends_with(".dic") {
            let rel = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            out.push(rel);
        }
    }
}
