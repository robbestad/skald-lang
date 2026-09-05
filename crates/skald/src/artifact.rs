//! Portable `.skald` sidecar manifests. Native-only; WASM does not hash files.

use crate::dict::{Dictionary, en_us, to_json};
use crate::error::Error;
use crate::rng::{RUN_PROFILE, Seed};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const ARTIFACT_FORMAT_VERSION: u32 = 2;
pub const ARTIFACT_FORMAT_LEGACY: u32 = 1;
pub const RECEIPT_FORMAT_VERSION: u32 = 2;
pub const RECEIPT_FORMAT_LEGACY: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestSeed {
    #[serde(rename = "type")]
    pub kind: String,
    pub value: String,
}

impl ManifestSeed {
    pub fn from_seed(seed: &Seed) -> Self {
        match seed {
            Seed::Int(n) => Self {
                kind: "u64".into(),
                value: n.to_string(),
            },
            Seed::Text(t) => Self {
                kind: "text".into(),
                value: t.clone(),
            },
        }
    }

    pub fn encode(&self) -> String {
        if self.kind == "text" {
            format!("text:{}", self.value)
        } else {
            self.value.clone()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestDependency {
    pub path: String,
    pub hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
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
    #[serde(
        rename = "dictOnly",
        default,
        skip_serializing_if = "std::ops::Not::not"
    )]
    pub dict_only: bool,
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
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub channels: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<ManifestSeed>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiptReplay {
    Full,
    LegacySkipped,
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
            seed: seed.map(ManifestSeed::from_seed),
            case_mode: case.map(str::to_string),
            nsfw,
            story,
            dependencies: dependencies.to_vec(),
            dictionary_hash: Some(dictionary_hash(dict)),
            dict_only: false,
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
    receipt_path_for(pattern_path, None, None)
}

/// Default seed uses `<stem>.receipt.json`. A different run seed writes
/// `<stem>.seed-<id>.receipt.json` so `run --seed 42` does not overwrite the
/// default receipt.
pub fn receipt_path_for(
    pattern_path: &Path,
    run_seed: Option<&ManifestSeed>,
    manifest_seed: Option<&ManifestSeed>,
) -> PathBuf {
    let stem = pattern_path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "pattern".into());
    let Some(seed) = run_seed else {
        return pattern_path.with_file_name(format!("{stem}.receipt.json"));
    };
    if Some(seed) == manifest_seed {
        return pattern_path.with_file_name(format!("{stem}.receipt.json"));
    }
    pattern_path.with_file_name(format!(
        "{stem}.seed-{}.receipt.json",
        seed_file_label(seed)
    ))
}

fn seed_file_label(seed: &ManifestSeed) -> String {
    if seed.kind == "text" {
        let hex = sha256_hex(seed.value.as_bytes());
        format!("text-{}", &hex[..16])
    } else {
        seed.value.clone()
    }
}

/// Resolve a manifest dependency against the `.skald` file's directory.
pub fn resolve_dependency_path(base_dir: &Path, dep_path: &str) -> PathBuf {
    let p = Path::new(dep_path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        base_dir.join(p)
    }
}

/// Store a dependency path relative to the `.skald` file so the artifact is portable.
pub fn stored_dependency_path(artifact_path: &Path, given: &str) -> Result<String, Error> {
    let abs = canonicalize_existing(Path::new(given))?;
    let parent = artifact_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let base = canonicalize_existing(parent)?;
    Ok(path_relative_to(&base, &abs))
}

fn canonicalize_existing(path: &Path) -> Result<PathBuf, Error> {
    fs::canonicalize(path).map_err(|e| Error::runtime(format!("{}: {e}", path.display()), None))
}

fn path_relative_to(base: &Path, file: &Path) -> String {
    let base_comps: Vec<_> = base.components().collect();
    let file_comps: Vec<_> = file.components().collect();
    let mut i = 0usize;
    while i < base_comps.len() && i < file_comps.len() && base_comps[i] == file_comps[i] {
        i += 1;
    }
    let mut out = PathBuf::new();
    for _ in i..base_comps.len() {
        out.push("..");
    }
    for c in &file_comps[i..] {
        out.push(c.as_os_str());
    }
    if out.as_os_str().is_empty() {
        return file
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| file.display().to_string());
    }
    out.to_string_lossy().replace('\\', "/")
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

pub fn verify_lock(
    manifest: &Manifest,
    dictionary: &Dictionary,
    base_dir: &Path,
) -> Result<(), Error> {
    for dep in &manifest.dependencies {
        let path = resolve_dependency_path(base_dir, &dep.path);
        let bytes = fs::read(&path)
            .map_err(|e| Error::runtime(format!("{}: {e}", path.display()), None))?;
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

pub fn verify_receipt(
    receipt: &Receipt,
    text: &str,
    channels: &BTreeMap<String, String>,
    pattern: &str,
) -> Result<ReceiptReplay, Error> {
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
    if receipt.run_profile != RUN_PROFILE {
        return Err(Error::runtime(
            format!(
                "receipt run profile {} does not match {RUN_PROFILE}",
                receipt.run_profile
            ),
            None,
        ));
    }
    if receipt.format_version == RECEIPT_FORMAT_LEGACY {
        if receipt.text == text {
            return Ok(ReceiptReplay::Full);
        }
        return Ok(ReceiptReplay::LegacySkipped);
    }
    if receipt.format_version != RECEIPT_FORMAT_VERSION {
        return Err(Error::runtime(
            format!(
                "unsupported receipt formatVersion {}",
                receipt.format_version
            ),
            None,
        ));
    }
    if receipt.text != text {
        return Err(Error::runtime("receipt text mismatch", None));
    }
    if &receipt.channels != channels {
        return Err(Error::runtime("receipt channels mismatch", None));
    }
    Ok(ReceiptReplay::Full)
}

pub fn choose_run_seed() -> Seed {
    use std::time::{SystemTime, UNIX_EPOCH};
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(1);
    Seed::Int(n.max(1))
}
