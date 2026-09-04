import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { RUN_PROFILE, canonicalSeed } from "./index.js";

const RUNTIME_VERSION = JSON.parse(
  readFileSync(join(dirname(fileURLToPath(import.meta.url)), "package.json"), "utf8"),
).version;

export const ARTIFACT_FORMAT_VERSION = 1;

export function sha256Hex(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

export function patternHash(pattern) {
  return `sha256:${sha256Hex(Buffer.from(pattern, "utf8"))}`;
}

export function sidecarPath(patternPath) {
  return `${patternPath}.json`;
}

export function manifestForPattern(pattern, { seed, caseMode, nsfw = false, story = false, runtimeVersion } = {}) {
  let seedObj = null;
  if (seed != null && seed !== "") {
    const encoded = canonicalSeed(seed);
    seedObj = encoded.startsWith("text:")
      ? { type: "text", value: encoded.slice(5) }
      : { type: "u64", value: encoded };
  }
  return {
    formatVersion: ARTIFACT_FORMAT_VERSION,
    runtimeVersion: runtimeVersion ?? RUNTIME_VERSION,
    runProfile: RUN_PROFILE,
    locale: "en-US",
    patternHash: patternHash(pattern),
    ...(seedObj ? { seed: seedObj } : {}),
    ...(caseMode ? { case: caseMode } : {}),
    nsfw: Boolean(nsfw),
    story: Boolean(story),
  };
}

export function writeManifest(path, manifest) {
  writeFileSync(path, `${JSON.stringify(manifest, null, 2)}\n`);
}

export function readManifest(path, { runtimeVersion } = {}) {
  const manifest = JSON.parse(readFileSync(path, "utf8"));
  if (manifest.formatVersion !== ARTIFACT_FORMAT_VERSION) {
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

export function verifyPattern(pattern, manifest) {
  const got = patternHash(pattern);
  if (got !== manifest.patternHash) {
    throw new Error(`pattern hash mismatch: manifest ${manifest.patternHash} file ${got}`);
  }
}
