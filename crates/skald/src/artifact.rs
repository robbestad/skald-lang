//! Portable `.skald` sidecar manifests. Native-only; WASM does not hash files.

use crate::dict::{Dictionary, en_us, to_json};
use crate::error::Error;
use crate::rng::{RUN_PROFILE, Seed};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

pub const ARTIFACT_FORMAT_VERSION: u32 = 2;
pub const ARTIFACT_FORMAT_LEGACY: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestSeed {
    #[serde(rename = "type")]
    pub kind: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestDependency {
    pub path: String,
    pub hash: String,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<ManifestDependency>,
    #[serde(rename = "dictionaryHash", skip_serializing_if = "Option::is_none")]
    pub dictionary_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Receipt {
    #[serde(rename = "formatVersion")]
    pub format_version: u32,
    #[serde(rename = "patternHash")]
    pub pattern_hash: String,
    #[serde(rename = "runProfile")]
    pub run_profile: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<ManifestSeed>,
}

impl Manifest {
    pub fn for_pattern(
        pattern: &str,
        seed: Option<&Seed>,
        case: Option<&str>,
        nsfw: bool,
        story: bool,
    ) -> Self {
        Self::for_pattern_locked(pattern, seed, case, nsfw, story, "en-US", None, &[])
    }

    #[allow(clippy::too_many_arguments)]
    pub fn for_pattern_locked(
        pattern: &str,
        seed: Option<&Seed>,
        case: Option<&str>,
        nsfw: bool,
        story: bool,
        locale: &str,
        dictionary: Option<&Dictionary>,
        dependencies: &[ManifestDependency],
    ) -> Self {
        let fallback = en_us();
        let dict = dictionary.unwrap_or(fallback.as_ref());
        Self {
            format_version: ARTIFACT_FORMAT_VERSION,
            runtime_version: env!("CARGO_PKG_VERSION").to_string(),
            run_profile: RUN_PROFILE.to_string(),
            locale: locale.to_string(),
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
            dependencies: dependencies.to_vec(),
            dictionary_hash: Some(dictionary_hash(dict)),
        }
    }

    pub fn replay_locked(&self) -> bool {
        self.format_version >= ARTIFACT_FORMAT_VERSION && self.dictionary_hash.is_some()
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

pub fn file_hash(bytes: &[u8]) -> String {
    format!("sha256:{}", sha256_hex(bytes))
}

pub fn dictionary_hash(dict: &Dictionary) -> String {
    file_hash(to_json(dict).as_bytes())
}

pub fn sidecar_path(pattern_path: &Path) -> PathBuf {
    let mut p = pattern_path.as_os_str().to_os_string();
    p.push(".json");
    PathBuf::from(p)
}

pub fn receipt_path(pattern_path: &Path) -> PathBuf {
    let stem = pattern_path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "pattern".into());
    pattern_path.with_file_name(format!("{stem}.receipt.json"))
}

pub fn write_manifest(path: &Path, manifest: &Manifest) -> Result<(), Error> {
    let json = serde_json::to_string_pretty(manifest)
        .map_err(|e| Error::runtime(format!("manifest encode: {e}"), None))?;
    fs::write(path, format!("{json}\n"))
        .map_err(|e| Error::runtime(format!("{}: {e}", path.display()), None))
}

pub fn write_receipt(path: &Path, receipt: &Receipt) -> Result<(), Error> {
    let json = serde_json::to_string_pretty(receipt)
        .map_err(|e| Error::runtime(format!("receipt encode: {e}"), None))?;
    fs::write(path, format!("{json}\n"))
        .map_err(|e| Error::runtime(format!("{}: {e}", path.display()), None))
}

pub fn read_manifest(path: &Path) -> Result<Manifest, Error> {
    let src = fs::read_to_string(path)
        .map_err(|e| Error::runtime(format!("{}: {e}", path.display()), None))?;
    let manifest: Manifest = serde_json::from_str(&src)
        .map_err(|e| Error::runtime(format!("{}: {e}", path.display()), None))?;
    if manifest.format_version != ARTIFACT_FORMAT_VERSION
        && manifest.format_version != ARTIFACT_FORMAT_LEGACY
    {
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
    let runtime = env!("CARGO_PKG_VERSION");
    if manifest.runtime_version != runtime {
        return Err(Error::runtime(
            format!(
                "runtimeVersion {} does not match {runtime}",
                manifest.runtime_version
            ),
            None,
        ));
    }
    Ok(manifest)
}

pub fn read_receipt(path: &Path) -> Result<Receipt, Error> {
    let src = fs::read_to_string(path)
        .map_err(|e| Error::runtime(format!("{}: {e}", path.display()), None))?;
    serde_json::from_str(&src).map_err(|e| Error::runtime(format!("{}: {e}", path.display()), None))
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

pub fn verify_lock(manifest: &Manifest, dictionary: &Dictionary) -> Result<(), Error> {
    for dep in &manifest.dependencies {
        let bytes =
            fs::read(&dep.path).map_err(|e| Error::runtime(format!("{}: {e}", dep.path), None))?;
        let got = file_hash(&bytes);
        if got != dep.hash {
            return Err(Error::runtime(
                format!(
                    "dependency hash mismatch for {}: manifest {} file {got}",
                    dep.path, dep.hash
                ),
                None,
            ));
        }
    }
    if let Some(expected) = &manifest.dictionary_hash {
        let got = dictionary_hash(dictionary);
        if got != *expected {
            return Err(Error::runtime(
                format!("dictionary hash mismatch: manifest {expected} file {got}"),
                None,
            ));
        }
    }
    Ok(())
}

pub fn verify_receipt(receipt: &Receipt, text: &str, pattern: &str) -> Result<(), Error> {
    if receipt.pattern_hash != pattern_hash(pattern) {
        return Err(Error::runtime(
            format!(
                "receipt pattern hash mismatch: receipt {} file {}",
                receipt.pattern_hash,
                pattern_hash(pattern)
            ),
            None,
        ));
    }
    if receipt.text != text {
        return Err(Error::runtime("receipt text mismatch", None));
    }
    Ok(())
}
