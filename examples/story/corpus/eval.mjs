#!/usr/bin/env node
/** Blind eval harness. Not a CI gate. Use --mock offline. Live generation is not wired. */

import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { explain } from "../../../packages/skald-lang/index.js";
import { PALETTES } from "../palettes.mjs";
import {
  PROMPT_VERSION,
  renderStory,
  splitStoryDocument,
} from "../runner.mjs";

const here = dirname(fileURLToPath(import.meta.url));

export const EVAL_PROTOCOL_VERSION = "eval-1";
export const ALLOWED_CONDITIONS = ["hybrid", "llm-only", "human", "mad-libs"];
export const MACHINE_DIMENSIONS = ["schema", "repair", "unresolvedQuery", "emptyReferent"];
export const EDITORIAL_DIMENSIONS = [
  "grammar",
  "causality",
  "referents",
  "repetition",
  "form",
  "ending",
  "voice",
];
/** Editorial dimensions only. Machine results are not rater scores. */
export const EVAL_DIMENSIONS = EDITORIAL_DIMENSIONS;

const SAMPLE_KEYS = new Set(["briefId", "condition", "text", "locale", "source", "notes", "generation"]);
const GENERATION_KEYS = new Set([
  "provider",
  "model",
  "reasoning",
  "maxModelCalls",
  "maxCostUsd",
  "promptTokens",
  "completionTokens",
  "costUsd",
]);
const GENERATION_STRINGS = new Set(["provider", "model", "reasoning"]);
const GENERATION_NUMBERS = new Set(["maxModelCalls", "maxCostUsd", "promptTokens", "completionTokens", "costUsd"]);

export function loadCorpusIndex(root = here) {
  return JSON.parse(readFileSync(resolve(root, "index.json"), "utf8"));
}

export function emptyEditorialScores() {
  return Object.fromEntries(EDITORIAL_DIMENSIONS.map((key) => [key, null]));
}

export function machineScores(artifact) {
  const diagnostics = artifact?.diagnostics ?? [];
  const unresolved = diagnostics.some((row) => row.code === "STORY_UNRESOLVED");
  const empty = Object.values(artifact?.cast ?? {}).some((name) => !name);
  return {
    schema: artifact?.ok ? 2 : 0,
    repair: artifact?.ok ? 2 : 0,
    unresolvedQuery: unresolved ? 0 : 2,
    emptyReferent: empty ? 0 : 2,
  };
}

export function loadImportedSamples(root = here, { locale = "en-US" } = {}) {
  const dir = resolve(root, "samples");
  if (!existsSync(dir)) return { samples: [], errors: [] };
  const samples = [];
  const errors = [];
  for (const name of readdirSync(dir).sort()) {
    if (!name.endsWith(".json")) continue;
    const path = resolve(dir, name);
    let doc;
    try {
      doc = JSON.parse(readFileSync(path, "utf8"));
    } catch (err) {
      errors.push({ path: name, reason: `invalid JSON: ${err.message}` });
      continue;
    }
    const checked = validateImportedSample(doc, name, { locale });
    if (!checked.ok) {
      errors.push({ path: name, reason: checked.reason });
      continue;
    }
    samples.push(checked.sample);
  }
  return { samples, errors };
}

function validateGeneration(value, name) {
  if (value == null) return { ok: true, generation: null };
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return { ok: false, reason: `${name} generation must be an object` };
  }
  const unknown = Object.keys(value).filter((key) => !GENERATION_KEYS.has(key));
  if (unknown.length) {
    return { ok: false, reason: `${name} unknown generation fields: ${unknown.join(", ")}` };
  }
  const generation = {};
  for (const key of GENERATION_STRINGS) {
    if (value[key] == null) continue;
    if (typeof value[key] !== "string" || !value[key].trim()) {
      return { ok: false, reason: `${name} generation.${key} must be a non-empty string` };
    }
    generation[key] = value[key].trim();
  }
  for (const key of GENERATION_NUMBERS) {
    if (value[key] == null) continue;
    if (typeof value[key] !== "number" || !Number.isFinite(value[key]) || value[key] < 0) {
      return { ok: false, reason: `${name} generation.${key} must be a finite number >= 0` };
    }
    generation[key] = value[key];
  }
  return { ok: true, generation: Object.keys(generation).length ? generation : null };
}

function validateImportedSample(doc, name, { locale = "en-US" } = {}) {
  if (!doc || typeof doc !== "object" || Array.isArray(doc)) {
    return { ok: false, reason: `${name} must be an object` };
  }
  const unknown = Object.keys(doc).filter((key) => !SAMPLE_KEYS.has(key));
  if (unknown.length) {
    return { ok: false, reason: `${name} unknown fields: ${unknown.join(", ")}` };
  }
  if (typeof doc.briefId !== "string" || !doc.briefId.trim()) {
    return { ok: false, reason: `${name} needs briefId` };
  }
  if (!ALLOWED_CONDITIONS.includes(doc.condition)) {
    return { ok: false, reason: `${name} condition must be one of ${ALLOWED_CONDITIONS.join(", ")}` };
  }
  if (typeof doc.text !== "string" || !doc.text.trim()) {
    return { ok: false, reason: `${name} needs non-empty text` };
  }
  if (doc.locale != null && (typeof doc.locale !== "string" || !doc.locale.trim())) {
    return { ok: false, reason: `${name} locale must be a non-empty string` };
  }
  const sampleLocale = typeof doc.locale === "string" && doc.locale.trim() ? doc.locale.trim() : locale;
  if (sampleLocale !== locale) {
    return { ok: false, reason: `${name} locale ${sampleLocale} does not match corpus ${locale}` };
  }
  const generation = validateGeneration(doc.generation, name);
  if (!generation.ok) return generation;
  return {
    ok: true,
    sample: {
      briefId: doc.briefId.trim(),
      condition: doc.condition,
      text: doc.text,
      locale: sampleLocale,
      source: "imported",
      notes: typeof doc.notes === "string" ? doc.notes : "",
      generation: generation.generation,
      origin: name,
    },
  };
}

export function inventoryCorpus(index, { imported = [] } = {}) {
  return (index.briefs ?? []).map((brief) => {
    const importedFor = imported.filter((row) => row.briefId === brief.id);
    const hasDraft = Boolean(brief.draft);
    return {
      id: brief.id,
      kind: brief.kind,
      path: brief.path,
      hasDraft,
      stateFrom: brief.stateFrom ?? null,
      importedCount: importedFor.length,
      importedConditions: [...new Set(importedFor.map((row) => row.condition))],
      ready: hasDraft || importedFor.length > 0,
    };
  });
}

function seededShuffle(items, seed) {
  let state = (Number(seed) >>> 0) || 1;
  const next = () => {
    state = (Math.imul(state, 1664525) + 1013904223) >>> 0;
    return state / 0x100000000;
  };
  const out = [...items];
  for (let i = out.length - 1; i > 0; i -= 1) {
    const j = Math.floor(next() * (i + 1));
    [out[i], out[j]] = [out[j], out[i]];
  }
  return out;
}

function briefText(root, brief) {
  const path = resolve(root, brief.path);
  return existsSync(path) ? readFileSync(path, "utf8") : "";
}

function omitReasons(row) {
  const reasons = [];
  if (!row.hasDraft) reasons.push("missing-draft");
  if (row.stateFrom && !row.hasDraft && row.importedCount === 0) {
    reasons.push(`needs-state-from:${row.stateFrom}`);
  }
  if (row.importedCount === 0 && !row.hasDraft) reasons.push("no-imported-sample");
  return reasons;
}

export function buildBlindPacket({
  briefs,
  samples,
  seed = 1,
  locale = "en-US",
  omitted = [],
  errors = [],
  inventory = [],
  missingConditions = [],
}) {
  const manifest = [];
  const samplesOut = [];
  let n = 0;
  for (const brief of briefs) {
    const rows = seededShuffle(
      samples.filter((sample) => sample.briefId === brief.id),
      seed + n + 1,
    );
    for (const row of rows) {
      const id = `S${String(++n).padStart(3, "0")}`;
      manifest.push({
        id,
        briefId: brief.id,
        condition: row.condition,
        source: row.source ?? "mock-render",
        origin: row.origin ?? null,
        locale: row.locale ?? locale,
        machine: row.machine ?? null,
        editorial: row.editorial ?? emptyEditorialScores(),
        generation: row.generation ?? null,
        notes: row.notes || null,
      });
      samplesOut.push({
        id,
        briefId: brief.id,
        kind: brief.kind ?? null,
        stateFrom: brief.stateFrom ?? null,
        brief: brief.briefText ?? "",
        text: row.text,
      });
    }
  }
  return {
    packet: {
      locale,
      seed,
      protocolVersion: EVAL_PROTOCOL_VERSION,
      promptVersion: PROMPT_VERSION,
      skaldVersion: "2.2.0",
      dimensions: EVAL_DIMENSIONS,
      machineDimensions: MACHINE_DIMENSIONS,
      notes: [
        "Editorial scores are 0/1/2 or null. Mock does not fill them.",
        "Do not use an AI-detector score. Glue ratio is observation, not a gate.",
        "llm-only means model prose without Skald substitution, not a stripped draft.",
        "Schema/repair/unresolvedQuery/emptyReferent are machine results, not rater scores.",
      ].join(" "),
      samples: samplesOut,
    },
    manifest,
    omitted,
    missingConditions,
    errors,
    inventory,
  };
}

function renderHybrid(root, brief) {
  const doc = JSON.parse(readFileSync(resolve(root, brief.draft), "utf8"));
  const { request, draft } = splitStoryDocument(doc);
  const hybrid = renderStory({ explain }, { ...request, seed: request.seed ?? 1 }, draft, {
    registry: PALETTES,
  });
  return {
    briefId: brief.id,
    condition: "hybrid",
    text: hybrid.artifact.text,
    source: "mock-render",
    origin: brief.draft,
    machine: machineScores(hybrid.artifact),
    editorial: emptyEditorialScores(),
  };
}

export function runMockEval({ corpusRoot = here } = {}) {
  const index = loadCorpusIndex(corpusRoot);
  const corpusLocale = index.locale ?? "en-US";
  const { samples: imported, errors } = loadImportedSamples(corpusRoot, { locale: corpusLocale });
  const unknownBrief = imported.filter((row) => !(index.briefs ?? []).some((brief) => brief.id === row.briefId));
  for (const row of unknownBrief) {
    errors.push({ path: row.origin, reason: `unknown briefId ${row.briefId}` });
  }
  const knownImported = imported.filter((row) => (index.briefs ?? []).some((brief) => brief.id === row.briefId));
  const inventory = inventoryCorpus(index, { imported: knownImported });
  const samples = [];
  const omitted = [];
  const briefs = (index.briefs ?? []).map((brief) => ({
    ...brief,
    briefText: briefText(corpusRoot, brief),
  }));

  for (const brief of briefs) {
    const row = inventory.find((item) => item.id === brief.id);
    if (brief.draft) {
      samples.push(renderHybrid(corpusRoot, brief));
    }
    const importedFor = knownImported.filter((item) => item.briefId === brief.id);
    for (const item of importedFor) {
      samples.push({
        ...item,
        machine: null,
        editorial: emptyEditorialScores(),
      });
    }
    if (!row.ready) {
      omitted.push({
        briefId: brief.id,
        kind: brief.kind,
        stateFrom: brief.stateFrom ?? null,
        reasons: omitReasons(row),
      });
    }
  }

  const missingConditions = inventory
    .filter((row) => row.ready && !row.importedConditions.includes("llm-only") && row.hasDraft)
    .map((row) => ({
      briefId: row.id,
      condition: "llm-only",
      reason: "llm-only-not-imported",
    }));

  return buildBlindPacket({
    briefs,
    samples,
    seed: 1,
    locale: corpusLocale,
    omitted,
    errors,
    inventory,
    missingConditions,
  });
}

function writeJson(path, value) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function main(argv = process.argv.slice(2)) {
  if (!argv.includes("--mock") && !argv.includes("--approve-expensive")) {
    process.stderr.write(
      "Usage: node eval.mjs --mock [--out packet.json] [--manifest key.json] [--report report.json]\n       Live generation is not wired; --approve-expensive exits 2.\n",
    );
    process.exit(1);
  }
  if (argv.includes("--approve-expensive")) {
    process.stderr.write(
      "live eval is not wired in this harness; import samples or score a frozen packet offline\n",
    );
    process.exit(2);
  }
  const { packet, manifest, omitted, missingConditions, errors, inventory } = runMockEval();
  const outFlag = argv.indexOf("--out");
  const manifestFlag = argv.indexOf("--manifest");
  const reportFlag = argv.indexOf("--report");
  const packetJson = `${JSON.stringify(packet, null, 2)}\n`;
  if (outFlag >= 0 && argv[outFlag + 1]) {
    writeJson(resolve(argv[outFlag + 1]), packet);
  } else {
    process.stdout.write(packetJson);
  }
  if (manifestFlag >= 0 && argv[manifestFlag + 1]) {
    writeJson(resolve(argv[manifestFlag + 1]), manifest);
  }
  if (reportFlag >= 0 && argv[reportFlag + 1]) {
    writeJson(resolve(argv[reportFlag + 1]), { omitted, missingConditions, errors, inventory });
  }
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main();
}
