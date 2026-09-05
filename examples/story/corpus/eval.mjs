#!/usr/bin/env node
/** Blind eval harness. Not a CI gate. Use --mock offline. Live generation is not wired. */

import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { explain } from "../../../packages/skald-lang/index.js";
import { PALETTES } from "../palettes.mjs";
import nbNO from "../../../locales/nb-NO.json" with { type: "json" };
import nnNO from "../../../locales/nn-NO.json" with { type: "json" };

const LANGUAGE_PACKS = { "nb-NO": nbNO, "nn-NO": nnNO };
import {
  PROMPT_VERSION,
  renderStory,
  scanBlocks,
  splitStoryDocument,
} from "../runner.mjs";
import { sha256Hex } from "../sha256.mjs";

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

const SAMPLE_KEYS = new Set(["briefId", "condition", "text", "textHash", "locale", "source", "notes", "generation", "editorial"]);
const TEXT_HASH_RE = /^sha256:[0-9a-f]{64}$/;

export function sampleTextHash(text) {
  return `sha256:${sha256Hex(text)}`;
}
export const VARIATION_SEEDS = [1, 2, 3, 5, 8, 11, 13, 17, 19, 23];
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

function validateEditorial(value, name) {
  if (value == null) return { ok: true, editorial: emptyEditorialScores() };
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return { ok: false, reason: `${name} editorial must be an object` };
  }
  const unknown = Object.keys(value).filter((key) => !EDITORIAL_DIMENSIONS.includes(key));
  if (unknown.length) {
    return { ok: false, reason: `${name} unknown editorial fields: ${unknown.join(", ")}` };
  }
  const editorial = emptyEditorialScores();
  for (const key of EDITORIAL_DIMENSIONS) {
    if (value[key] == null) continue;
    if (value[key] !== 0 && value[key] !== 1 && value[key] !== 2) {
      return { ok: false, reason: `${name} editorial.${key} must be 0, 1, 2, or null` };
    }
    editorial[key] = value[key];
  }
  return { ok: true, editorial };
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

function leafSurfaces(alternatives) {
  let count = 0;
  for (const alt of alternatives) {
    const inner = scanBlocks(alt).filter((block) => block.depth === 1 && block.alternatives.length >= 2);
    if (inner.length === 0) {
      count += 1;
      continue;
    }
    count += theoreticalCombinations(inner.map((block) => ({
      alternatives: block.alternatives,
      leaves: leafSurfaces(block.alternatives),
    })));
  }
  return count;
}

export function choiceGroupsFromPattern(pattern) {
  const source = String(pattern ?? "");
  const occurrences = [];
  for (const block of scanBlocks(source)) {
    if (block.depth !== 1 || block.alternatives.length < 2) continue;
    const sync = source.slice(0, block.start).match(/\[sync:([A-Za-z][A-Za-z0-9_]{0,31});locked\]$/)?.[1] ?? null;
    occurrences.push({
      sync,
      alternatives: block.alternatives.map((row) => row.trim()),
      text: block.text,
      start: block.start,
      leaves: leafSurfaces(block.alternatives),
    });
  }
  const groups = [];
  const seenSync = new Set();
  for (const row of occurrences) {
    if (row.sync) {
      if (seenSync.has(row.sync)) continue;
      seenSync.add(row.sync);
    }
    groups.push(row);
  }
  return groups;
}

export function theoreticalCombinations(groups) {
  return (groups ?? []).reduce((product, group) => product * Math.max(1, group.leaves ?? group.alternatives.length), 1);
}

function patternOccurrences(pattern) {
  return scanBlocks(String(pattern ?? "")).filter((block) => block.depth === 1 && block.alternatives.length >= 2);
}

function variationOccurrenceIndex(pattern, draft, variation) {
  const local = scanBlocks(String(variation?.pattern ?? "")).find((block) => (
    block.depth === 1 && block.alternatives.length >= 2
  ));
  if (!local) return -1;
  const beats = draft?.beats ?? [];
  let earlier = 0;
  for (let i = 0; i < (variation.beatIndex ?? 0); i += 1) {
    earlier += scanBlocks(String(beats[i] ?? "")).filter((block) => block.depth === 1 && block.text === local.text).length;
  }
  const beat = String(beats[variation.beatIndex] ?? "");
  const from = Number.isInteger(variation.start) ? variation.start : beat.indexOf(local.text);
  earlier += scanBlocks(beat.slice(0, Math.max(from, 0))).filter((block) => block.depth === 1 && block.text === local.text).length;
  const groups = patternOccurrences(pattern);
  let seen = 0;
  for (let i = 0; i < groups.length; i += 1) {
    if (groups[i].text !== local.text) continue;
    if (seen === earlier) return i;
    seen += 1;
  }
  return -1;
}

export function pairwiseManuscriptVariant(manuscript, variant) {
  const a = String(manuscript ?? "");
  const b = String(variant ?? "");
  const wordsA = a.match(/[\p{L}\p{N}]+/gu) ?? [];
  const wordsB = b.match(/[\p{L}\p{N}]+/gu) ?? [];
  const setA = new Set(wordsA);
  const setB = new Set(wordsB);
  let shared = 0;
  for (const word of setA) {
    if (setB.has(word)) shared += 1;
  }
  return {
    identical: a === b,
    manuscriptChars: a.length,
    variantChars: b.length,
    manuscriptWords: wordsA.length,
    variantWords: wordsB.length,
    sharedWordTypes: shared,
    onlyManuscript: [...setA].filter((word) => !setB.has(word)).length,
    onlyVariant: [...setB].filter((word) => !setA.has(word)).length,
  };
}

export function observeVariation(api, request, draft, palettes, { seeds = VARIATION_SEEDS } = {}) {
  const runs = [];
  for (const seed of seeds) {
    const run = renderStory(api, { ...request, seed }, draft, palettes);
    runs.push({
      seed,
      ok: run.ok,
      text: run.artifact.text,
      pattern: run.artifact.pattern,
      choices: run.artifact.choices ?? [],
      variations: run.artifact.variations ?? [],
      manuscript: run.artifact.manuscript?.text ?? request.manuscript?.text ?? null,
    });
  }
  const texts = runs.map((row) => row.text);
  const unique = [...new Set(texts)];
  const groups = choiceGroupsFromPattern(runs.find((row) => row.pattern)?.pattern ?? "");
  const byVariation = {};
  const catalog = runs[0]?.variations ?? [];
  for (const variation of catalog) {
    const local = scanBlocks(String(variation.pattern ?? "")).find((block) => (
      block.depth === 1 && block.alternatives.length >= 2
    ));
    const alternatives = (local?.alternatives ?? []).map((row) => row.trim());
    const seen = new Set();
    for (const run of runs) {
      const index = variationOccurrenceIndex(run.pattern, draft, variation);
      const pick = run.choices[index]?.alternative;
      if (index >= 0 && Number.isInteger(pick) && alternatives[pick] != null) {
        seen.add(alternatives[pick]);
      }
    }
    byVariation[variation.variationId] = {
      role: variation.role ?? null,
      alternatives,
      observed: [...seen],
    };
  }
  const manuscript = runs.find((row) => row.manuscript)?.manuscript ?? null;
  const pairwise = manuscript
    ? unique.map((text) => pairwiseManuscriptVariant(manuscript, text))
    : unique.slice(1).map((text) => pairwiseManuscriptVariant(unique[0] ?? "", text));
  return {
    seeds: seeds.length,
    ok: runs.every((row) => row.ok),
    uniqueOutputs: unique.length,
    collisions: Math.max(0, seeds.length - unique.length),
    collisionRate: seeds.length ? (seeds.length - unique.length) / seeds.length : 0,
    theoreticalCombinations: theoreticalCombinations(groups),
    independentGroups: groups.length,
    observedByVariationId: byVariation,
    hasManuscript: Boolean(manuscript),
    pairwise,
    note: "theoreticalCombinations multiplies independent closed {a|b} groups after sync. It does not count dictionary queries, cast retries, or weighted/identical surfaces, so it is not the size of the full variation space. Pairwise is manuscript→variant when a manuscript exists, otherwise first unique text vs later uniques.",
  };
}

export function summarizeEditorial(manifest) {
  const rows = manifest ?? [];
  const scored = rows.filter((row) => EDITORIAL_DIMENSIONS.some((key) => row.editorial?.[key] != null));
  return {
    samples: rows.length,
    scored: scored.length,
    unscored: rows.length - scored.length,
    byCondition: Object.fromEntries(
      ALLOWED_CONDITIONS.map((condition) => {
        const subset = rows.filter((row) => row.condition === condition);
        return [condition, {
          samples: subset.length,
          scored: subset.filter((row) => EDITORIAL_DIMENSIONS.some((key) => row.editorial?.[key] != null)).length,
        }];
      }),
    ),
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
  const overlayOnly = doc.condition === "hybrid" && (doc.text == null || doc.text === "");
  if (!overlayOnly && (typeof doc.text !== "string" || !doc.text.trim())) {
    return { ok: false, reason: `${name} needs non-empty text` };
  }
  let textHash = null;
  if (doc.textHash != null) {
    if (typeof doc.textHash !== "string" || !TEXT_HASH_RE.test(doc.textHash)) {
      return { ok: false, reason: `${name} textHash must be sha256:<64 hex chars>` };
    }
    textHash = doc.textHash;
  }
  if (overlayOnly && !textHash) {
    return { ok: false, reason: `${name} overlay needs textHash of the assessed hybrid` };
  }
  if (!overlayOnly && textHash) {
    const got = sampleTextHash(doc.text);
    if (got !== textHash) {
      return { ok: false, reason: `${name} textHash does not match text` };
    }
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
  const editorial = validateEditorial(doc.editorial, name);
  if (!editorial.ok) return editorial;
  return {
    ok: true,
    sample: {
      briefId: doc.briefId.trim(),
      condition: doc.condition,
      text: overlayOnly ? "" : doc.text,
      textHash,
      overlay: overlayOnly,
      locale: sampleLocale,
      source: "imported",
      notes: typeof doc.notes === "string" ? doc.notes : "",
      generation: generation.generation,
      editorial: editorial.editorial,
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
      skaldVersion: "3.0.1",
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
  const locale = request.locale ?? "en-US";
  const hybrid = renderStory({ explain }, {
    ...request,
    seed: request.seed ?? 1,
    locale,
    languagePack: request.languagePack ?? LANGUAGE_PACKS[locale] ?? null,
  }, draft, {
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
    const importedFor = knownImported.filter((item) => item.briefId === brief.id);
    const overlays = importedFor.filter((item) => item.overlay);
    const importedTexts = importedFor.filter((item) => !item.overlay);
    if (brief.draft) {
      const hybrid = renderHybrid(corpusRoot, brief);
      const overlay = overlays.find((item) => item.condition === "hybrid");
      if (overlay) {
        const got = sampleTextHash(hybrid.text);
        if (overlay.textHash !== got) {
          errors.push({
            path: overlay.origin,
            reason: `stale editorial overlay for ${brief.id}: textHash ${overlay.textHash} does not match ${got}`,
          });
        } else {
          hybrid.editorial = overlay.editorial;
        }
      }
      samples.push(hybrid);
    }
    for (const item of importedTexts) {
      samples.push({
        ...item,
        machine: null,
        editorial: item.editorial ?? emptyEditorialScores(),
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

  const packet = buildBlindPacket({
    briefs,
    samples,
    seed: 1,
    locale: corpusLocale,
    omitted,
    errors,
    inventory,
    missingConditions,
  });
  const variation = [];
  for (const brief of briefs) {
    if (!brief.draft) continue;
    const doc = JSON.parse(readFileSync(resolve(corpusRoot, brief.draft), "utf8"));
    const { request, draft } = splitStoryDocument(doc);
    const locale = request.locale ?? corpusLocale;
    variation.push({
      briefId: brief.id,
      ...observeVariation({ explain }, {
        ...request,
        locale,
        languagePack: request.languagePack ?? LANGUAGE_PACKS[locale] ?? null,
      }, draft, { registry: PALETTES }),
    });
  }
  return {
    ...packet,
    variation,
    editorial: summarizeEditorial(packet.manifest),
  };
}

function writeJson(path, value) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function main(argv = process.argv.slice(2)) {
  if (!argv.includes("--mock") && !argv.includes("--approve-expensive")) {
    process.stderr.write(
      "Usage: node eval.mjs --mock [--root corpusDir] [--out packet.json] [--manifest key.json] [--report report.json]\n       Live generation is not wired; --approve-expensive exits 2.\n",
    );
    process.exit(1);
  }
  if (argv.includes("--approve-expensive")) {
    process.stderr.write(
      "live eval is not wired in this harness; import samples or score a frozen packet offline\n",
    );
    process.exit(2);
  }
  const rootFlag = argv.indexOf("--root");
  const corpusRoot = rootFlag >= 0 && argv[rootFlag + 1] ? resolve(argv[rootFlag + 1]) : here;
  const { packet, manifest, omitted, missingConditions, errors, inventory, variation, editorial } = runMockEval({
    corpusRoot,
  });
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
    writeJson(resolve(argv[reportFlag + 1]), {
      protocolVersion: EVAL_PROTOCOL_VERSION,
      omitted,
      missingConditions,
      errors,
      inventory,
      editorial,
      variation,
      notes: "This report is not the blind packet. It must not be shown to raters.",
    });
  }
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main();
}
