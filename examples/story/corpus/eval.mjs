#!/usr/bin/env node
/** Blind eval harness. Not a CI gate. Use --mock offline or --approve-expensive for a live model. */

import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
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

export const EVAL_DIMENSIONS = [
  "schema",
  "repair",
  "grammar",
  "causality",
  "referents",
  "repetition",
  "form",
  "ending",
];

export function loadCorpusIndex(root = here) {
  return JSON.parse(readFileSync(resolve(root, "index.json"), "utf8"));
}

function firstAlternative(text) {
  return String(text ?? "").replace(/\{([^{}|]+)(?:\|[^{}]*)*\}/gu, "$1");
}

function llmOnlyFromDraft(draft) {
  return (draft.beats ?? [])
    .map((beat) => firstAlternative(beat).replace(/<[^<>]+>/g, "").replace(/[ \t]{2,}/g, " ").trim())
    .join("\n");
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

function structuralScores(artifact) {
  const raw = artifact.text?.includes("<");
  const empty = Object.values(artifact.cast ?? {}).some((name) => !name);
  return {
    schema: artifact.ok ? 2 : 0,
    repair: artifact.ok ? 2 : 0,
    grammar: raw ? 0 : 2,
    referents: empty ? 0 : 2,
    causality: null,
    repetition: null,
    form: null,
    ending: null,
  };
}

export function buildBlindPacket({ briefs, samples, seed = 1 }) {
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
      manifest.push({ id, briefId: brief.id, condition: row.condition, scores: row.scores ?? null });
      samplesOut.push({
        id,
        briefId: brief.id,
        text: row.text,
      });
    }
  }
  return {
    packet: {
      locale: "en-US",
      seed,
      promptVersion: PROMPT_VERSION,
      skaldVersion: "2.1.0",
      dimensions: EVAL_DIMENSIONS,
      notes: "Do not use an AI-detector score. Glue ratio is observation, not a gate.",
      samples: samplesOut,
    },
    manifest,
  };
}

export function runMockEval({ corpusRoot = here } = {}) {
  const index = loadCorpusIndex(corpusRoot);
  const samples = [];
  for (const brief of index.briefs) {
    if (!brief.draft) continue;
    const doc = JSON.parse(readFileSync(resolve(corpusRoot, brief.draft), "utf8"));
    const { request, draft } = splitStoryDocument(doc);
    const hybrid = renderStory({ explain }, { ...request, seed: request.seed ?? 1 }, draft, {
      registry: PALETTES,
    });
    samples.push({
      briefId: brief.id,
      condition: "hybrid",
      text: hybrid.artifact.text,
      scores: structuralScores(hybrid.artifact),
    });
    samples.push({
      briefId: brief.id,
      condition: "llm-only",
      text: llmOnlyFromDraft(draft),
      scores: null,
    });
  }
  return buildBlindPacket({ briefs: index.briefs, samples, seed: 1 });
}

function main(argv = process.argv.slice(2)) {
  if (!argv.includes("--mock") && !argv.includes("--approve-expensive")) {
    process.stderr.write(
      "Usage: node eval.mjs --mock [--out packet.json] [--manifest key.json]\n       node eval.mjs --approve-expensive --provider <name> --model <id> --reasoning <level>\n",
    );
    process.exit(1);
  }
  if (argv.includes("--approve-expensive")) {
    process.stderr.write("live eval is not wired in this harness; run loop per brief and score the blind packet by hand\n");
    process.exit(2);
  }
  const { packet, manifest } = runMockEval();
  const outFlag = argv.indexOf("--out");
  const manifestFlag = argv.indexOf("--manifest");
  const packetJson = `${JSON.stringify(packet, null, 2)}\n`;
  if (outFlag >= 0 && argv[outFlag + 1]) {
    const outPath = resolve(argv[outFlag + 1]);
    mkdirSync(dirname(outPath), { recursive: true });
    writeFileSync(outPath, packetJson);
  } else {
    process.stdout.write(packetJson);
  }
  if (manifestFlag >= 0 && argv[manifestFlag + 1]) {
    const keyPath = resolve(argv[manifestFlag + 1]);
    mkdirSync(dirname(keyPath), { recursive: true });
    writeFileSync(keyPath, `${JSON.stringify(manifest, null, 2)}\n`);
  }
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main();
}
