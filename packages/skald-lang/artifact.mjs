import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { RUN_PROFILE } from "./index.js";

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
    const value = String(seed);
    seedObj = /^(0|[1-9]\d*)$/.test(value)
      ? { type: "u64", value }
      : { type: "text", value: value.startsWith("text:") ? value.slice(5) : value };
  }
  return {
    formatVersion: ARTIFACT_FORMAT_VERSION,
    runtimeVersion: runtimeVersion ?? "2.2.0",
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

export function readManifest(path) {
  const manifest = JSON.parse(readFileSync(path, "utf8"));
  if (manifest.formatVersion !== ARTIFACT_FORMAT_VERSION) {
    throw new Error(`unsupported artifact formatVersion ${manifest.formatVersion}`);
  }
  if (manifest.runProfile !== RUN_PROFILE) {
    throw new Error(`run profile ${manifest.runProfile} does not match ${RUN_PROFILE}`);
  }
  return manifest;
}

export function verifyPattern(pattern, manifest) {
  const got = patternHash(pattern);
  if (got !== manifest.patternHash) {
    throw new Error(`pattern hash mismatch: manifest ${manifest.patternHash} file ${got}`);
  }
}
