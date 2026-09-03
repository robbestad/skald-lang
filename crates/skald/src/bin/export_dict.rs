use skald::{en_us, to_json};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::process;

fn main() {
    let json = to_json(&en_us());
    let dest = env::args().nth(1);
    match dest.as_deref() {
        None | Some("-") => {
            let mut out = io::stdout();
            out.write_all(json.as_bytes()).unwrap();
            if !json.ends_with('\n') {
                out.write_all(b"\n").unwrap();
            }
        }
        Some(path) => {
            if let Err(err) = fs::write(path, json.as_bytes()) {
                eprintln!("{path}: {err}");
                process::exit(1);
            }
            let tables = en_us().tables.len();
            eprintln!("wrote {path} ({tables} tables, {} bytes)", json.len());
        }
    }
}
