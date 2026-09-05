import { createHash } from "node:crypto";
import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join, parse as parsePath } from "node:path";
import { RUN_PROFILE, canonicalSeed } from "./index.js";

const RUNTIME_VERSION = JSON.parse(
  readFileSync(join(dirname(fileURLToPath(import.meta.url)), "package.json"), "utf8"),
).version;

export const ARTIFACT_FORMAT_VERSION = 2;
export const ARTIFACT_FORMAT_LEGACY = 1;

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

export function receiptPath(patternPath) {
  const parsed = parsePath(patternPath);
  return join(parsed.dir, `${parsed.name}.receipt.json`);
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
} = {}) {
  let seedObj = null;
  if (seed != null && seed !== "") {
    const encoded = canonicalSeed(seed);
    seedObj = encoded.startsWith("text:")
      ? { type: "text", value: encoded.slice(5) }
      : { type: "u64", value: encoded };
  }
  const dictBytes = dictionaryJson != null
    ? Buffer.from(typeof dictionaryJson === "string" ? dictionaryJson : JSON.stringify(dictionaryJson), "utf8")
    : readFileSync(join(dirname(fileURLToPath(import.meta.url)), "en-us.json"));
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

export function verifyLock(manifest, dictionaryJson) {
  for (const dep of manifest.dependencies ?? []) {
    const bytes = readFileSync(dep.path);
    const got = fileHash(bytes);
    if (got !== dep.hash) {
      throw new Error(`dependency hash mismatch for ${dep.path}: manifest ${dep.hash} file ${got}`);
    }
  }
  if (manifest.dictionaryHash) {
    const bytes = dictionaryJson == null
      ? readFileSync(join(dirname(fileURLToPath(import.meta.url)), "en-us.json"))
      : Buffer.from(typeof dictionaryJson === "string" ? dictionaryJson : JSON.stringify(dictionaryJson), "utf8");
    const got = fileHash(bytes);
    if (got !== manifest.dictionaryHash) {
      throw new Error(`dictionary hash mismatch: manifest ${manifest.dictionaryHash} file ${got}`);
    }
  }
}

export function readReceipt(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

export function verifyReceipt(receipt, text, pattern) {
  const expected = patternHash(pattern);
  if (receipt.patternHash !== expected) {
    throw new Error(`receipt pattern hash mismatch: receipt ${receipt.patternHash} file ${expected}`);
  }
  if (receipt.text !== text) {
    throw new Error("receipt text mismatch");
  }
}

export function receiptExists(patternPath) {
  return existsSync(receiptPath(patternPath));
}
