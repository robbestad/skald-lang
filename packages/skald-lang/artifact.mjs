import { createHash } from "node:crypto";
import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, isAbsolute, join, parse as parsePath, relative, resolve, sep } from "node:path";
import { RUN_PROFILE, canonicalSeed, dictionaryJson as engineDictionaryJson } from "./index.js";

const RUNTIME_VERSION = JSON.parse(
  readFileSync(join(dirname(fileURLToPath(import.meta.url)), "package.json"), "utf8"),
).version;

export const ARTIFACT_FORMAT_VERSION = 2;
export const ARTIFACT_FORMAT_LEGACY = 1;
export const RECEIPT_FORMAT_VERSION = 2;
export const RECEIPT_FORMAT_LEGACY = 1;

export function sha256Hex(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

export function patternHash(pattern) {
  return `sha256:${sha256Hex(Buffer.from(pattern, "utf8"))}`;
}

export function fileHash(bytes) {
  return `sha256:${sha256Hex(bytes)}`;
}

export function sidecarPath(patternPath) {
  return `${patternPath}.json`;
}

export function normalizeSeed(seed) {
  if (seed == null || seed === "") return null;
  if (typeof seed === "object") {
    const type = seed.type ?? seed.kind;
    if (!type || seed.value == null || seed.value === "") return null;
    return { type, value: String(seed.value) };
  }
  const encoded = canonicalSeed(seed);
  if (encoded.startsWith("text:")) return { type: "text", value: encoded.slice(5) };
  return { type: "u64", value: encoded };
}

export function seedRecord(seed) {
  return normalizeSeed(seed);
}

function seedFileLabel(seed) {
  const normalized = normalizeSeed(seed);
  if (!normalized) return null;
  if (normalized.type === "text") return `text-${sha256Hex(normalized.value).slice(0, 16)}`;
  return normalized.value;
}

export function receiptPath(patternPath, runSeed, manifestSeed) {
  const parsed = parsePath(patternPath);
  const run = normalizeSeed(runSeed);
  const manifest = normalizeSeed(manifestSeed);
  const same = (run == null && manifest == null)
    || (run != null && manifest != null && run.type === manifest.type && run.value === manifest.value);
  if (same || run == null) {
    return join(parsed.dir, `${parsed.name}.receipt.json`);
  }
  return join(parsed.dir, `${parsed.name}.seed-${seedFileLabel(run)}.receipt.json`);
}

export function resolveDependencyPath(baseDir, depPath) {
  if (!depPath || isAbsolute(depPath) || !baseDir) return depPath;
  return resolve(baseDir, depPath);
}

export function storedDependencyPath(artifactPath, given) {
  const abs = isAbsolute(given) ? resolve(given) : resolve(given);
  const base = dirname(resolve(artifactPath));
  const rel = relative(base, abs);
  return (rel || given).split(sep).join("/");
}

export function looksLikeLanguagePackText(src) {
  try {
    const obj = JSON.parse(src);
    return Boolean(obj && obj.formatVersion != null && obj.locale && obj.capabilities);
  } catch {
    return false;
  }
}

export function manifestForPattern(pattern, {
  seed,
  caseMode,
  nsfw = false,
  story = false,
  runtimeVersion,
  locale = "en-US",
  dictionaryJson,
  dependencies = [],
  dictOnly = false,
} = {}) {
  let seedObj = null;
  if (seed != null && seed !== "") {
    const encoded = canonicalSeed(seed);
    seedObj = encoded.startsWith("text:")
      ? { type: "text", value: encoded.slice(5) }
      : { type: "u64", value: encoded };
  }
  const dictBytes = Buffer.from(
    typeof dictionaryJson === "string"
      ? dictionaryJson
      : dictionaryJson != null
        ? JSON.stringify(dictionaryJson)
        : engineDictionaryJson(),
    "utf8",
  );
  return {
    formatVersion: ARTIFACT_FORMAT_VERSION,
    runtimeVersion: runtimeVersion ?? RUNTIME_VERSION,
    runProfile: RUN_PROFILE,
    locale,
    patternHash: patternHash(pattern),
    ...(seedObj ? { seed: seedObj } : {}),
    ...(caseMode ? { case: caseMode } : {}),
    nsfw: Boolean(nsfw),
    story: Boolean(story),
    ...(dependencies.length ? { dependencies } : {}),
    dictionaryHash: fileHash(dictBytes),
    ...(dictOnly ? { dictOnly: true } : {}),
  };
}

export function writeManifest(path, manifest) {
  writeFileSync(path, `${JSON.stringify(manifest, null, 2)}\n`);
}

export function writeReceipt(path, receipt) {
  writeFileSync(path, `${JSON.stringify(receipt, null, 2)}\n`);
}

export function readManifest(path, { runtimeVersion } = {}) {
  const manifest = JSON.parse(readFileSync(path, "utf8"));
  if (manifest.formatVersion !== ARTIFACT_FORMAT_VERSION && manifest.formatVersion !== ARTIFACT_FORMAT_LEGACY) {
    throw new Error(`unsupported artifact formatVersion ${manifest.formatVersion}`);
  }
  if (manifest.runProfile !== RUN_PROFILE) {
    throw new Error(`run profile ${manifest.runProfile} does not match ${RUN_PROFILE}`);
  }
  const expected = runtimeVersion ?? RUNTIME_VERSION;
  if (manifest.runtimeVersion !== expected) {
    throw new Error(`runtimeVersion ${manifest.runtimeVersion} does not match ${expected}`);
  }
  return manifest;
}

export function replayLocked(manifest) {
  return manifest.formatVersion >= ARTIFACT_FORMAT_VERSION && Boolean(manifest.dictionaryHash);
}

export function verifyPattern(pattern, manifest) {
  const got = patternHash(pattern);
  if (got !== manifest.patternHash) {
    throw new Error(`pattern hash mismatch: manifest ${manifest.patternHash} file ${got}`);
  }
}

export function verifyLock(manifest, dictionaryJson, { baseDir } = {}) {
  for (const dep of manifest.dependencies ?? []) {
    const depPath = resolveDependencyPath(baseDir, dep.path);
    const bytes = readFileSync(depPath);
    const got = fileHash(bytes);
    if (got !== dep.hash) {
      throw new Error(`dependency hash mismatch for ${dep.path}: manifest ${dep.hash} file ${got}`);
    }
  }
  if (manifest.dictionaryHash) {
    if (dictionaryJson == null) {
      throw new Error("dictionary hash mismatch: effective dictionary JSON was not provided");
    }
    const bytes = Buffer.from(typeof dictionaryJson === "string" ? dictionaryJson : JSON.stringify(dictionaryJson), "utf8");
    const got = fileHash(bytes);
    if (got !== manifest.dictionaryHash) {
      throw new Error(`dictionary hash mismatch: manifest ${manifest.dictionaryHash} file ${got}`);
    }
  }
}

export function readReceipt(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

function stableChannels(channels) {
  const obj = channels && typeof channels === "object" ? channels : {};
  return JSON.stringify(
    Object.fromEntries(Object.entries(obj).sort(([a], [b]) => a.localeCompare(b))),
  );
}

export function verifyReceipt(receipt, text, pattern, channels = {}) {
  const expected = patternHash(pattern);
  if (receipt.patternHash !== expected) {
    throw new Error(`receipt pattern hash mismatch: receipt ${receipt.patternHash} file ${expected}`);
  }
  if (receipt.runProfile !== RUN_PROFILE) {
    throw new Error(`receipt run profile ${receipt.runProfile} does not match ${RUN_PROFILE}`);
  }
  if (receipt.formatVersion === RECEIPT_FORMAT_LEGACY) {
    if (receipt.text === text) return { replayed: true, legacy: true };
    return { replayed: false, legacy: true };
  }
  if (receipt.formatVersion !== RECEIPT_FORMAT_VERSION) {
    throw new Error(`unsupported receipt formatVersion ${receipt.formatVersion}`);
  }
  if (receipt.text !== text) {
    throw new Error("receipt text mismatch");
  }
  if (stableChannels(receipt.channels) !== stableChannels(channels)) {
    throw new Error("receipt channels mismatch");
  }
  return { replayed: true, legacy: false };
}

export function receiptExists(patternPath) {
  return existsSync(receiptPath(patternPath));
}
