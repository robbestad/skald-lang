//! Portable `.skald` sidecar manifests. Native-only; WASM does not hash files.

use crate::error::Error;
use crate::rng::{RUN_PROFILE, Seed};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

pub const ARTIFACT_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestSeed {
    #[serde(rename = "type")]
    pub kind: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    #[serde(rename = "formatVersion")]
    pub format_version: u32,
    #[serde(rename = "runtimeVersion")]
    pub runtime_version: String,
    #[serde(rename = "runProfile")]
    pub run_profile: String,
    pub locale: String,
    #[serde(rename = "patternHash")]
    pub pattern_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<ManifestSeed>,
    #[serde(rename = "case", skip_serializing_if = "Option::is_none")]
    pub case_mode: Option<String>,
    pub nsfw: bool,
    pub story: bool,
}

impl Manifest {
    pub fn for_pattern(
        pattern: &str,
        seed: Option<&Seed>,
        case: Option<&str>,
        nsfw: bool,
        story: bool,
    ) -> Self {
        Self {
            format_version: ARTIFACT_FORMAT_VERSION,
            runtime_version: env!("CARGO_PKG_VERSION").to_string(),
            run_profile: RUN_PROFILE.to_string(),
            locale: "en-US".to_string(),
            pattern_hash: pattern_hash(pattern),
            seed: seed.map(|s| match s {
                Seed::Int(n) => ManifestSeed {
                    kind: "u64".into(),
                    value: n.to_string(),
                },
                Seed::Text(t) => ManifestSeed {
                    kind: "text".into(),
                    value: t.clone(),
                },
            }),
            case_mode: case.map(str::to_string),
            nsfw,
            story,
        }
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for b in digest {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

pub fn pattern_hash(pattern: &str) -> String {
    format!("sha256:{}", sha256_hex(pattern.as_bytes()))
}

pub fn sidecar_path(pattern_path: &Path) -> PathBuf {
    let mut p = pattern_path.as_os_str().to_os_string();
    p.push(".json");
    PathBuf::from(p)
}

pub fn write_manifest(path: &Path, manifest: &Manifest) -> Result<(), Error> {
    let json = serde_json::to_string_pretty(manifest)
        .map_err(|e| Error::runtime(format!("manifest encode: {e}"), None))?;
    fs::write(path, format!("{json}\n"))
        .map_err(|e| Error::runtime(format!("{}: {e}", path.display()), None))
}

pub fn read_manifest(path: &Path) -> Result<Manifest, Error> {
    let src = fs::read_to_string(path)
        .map_err(|e| Error::runtime(format!("{}: {e}", path.display()), None))?;
    let manifest: Manifest = serde_json::from_str(&src)
        .map_err(|e| Error::runtime(format!("{}: {e}", path.display()), None))?;
    if manifest.format_version != ARTIFACT_FORMAT_VERSION {
        return Err(Error::runtime(
            format!(
                "unsupported artifact formatVersion {}",
                manifest.format_version
            ),
            None,
        ));
    }
    if manifest.run_profile != RUN_PROFILE {
        return Err(Error::runtime(
            format!(
                "run profile {} does not match {RUN_PROFILE}",
                manifest.run_profile
            ),
            None,
        ));
    }
    Ok(manifest)
}

pub fn verify_pattern(pattern: &str, manifest: &Manifest) -> Result<(), Error> {
    let got = pattern_hash(pattern);
    if got != manifest.pattern_hash {
        return Err(Error::runtime(
            format!(
                "pattern hash mismatch: manifest {} file {got}",
                manifest.pattern_hash
            ),
            None,
        ));
    }
    Ok(())
}
