#![cfg(not(target_arch = "wasm32"))]

use skald::artifact::{Manifest, pattern_hash, sha256_hex, verify_pattern};

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
