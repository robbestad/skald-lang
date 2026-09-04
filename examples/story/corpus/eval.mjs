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
    .map((beat) => firstAlternative(beat).replace(/<::[A-Za-z][A-Za-z0-9_]{0,31}>/g, "Someone"))
    .join("\n");
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
  const packet = [];
  let n = 0;
  for (const brief of briefs) {
    const rows = samples.filter((sample) => sample.briefId === brief.id);
    const shuffled = [...rows].sort((a, b) => String(a.condition).localeCompare(String(b.condition)));
    for (const row of shuffled) {
      const id = `S${String(++n).padStart(3, "0")}`;
      manifest.push({ id, briefId: brief.id, condition: row.condition, scores: row.scores ?? null });
      packet.push({
        id,
        briefId: brief.id,
        text: row.text,
      });
    }
  }
  return {
    locale: "en-US",
    seed,
    promptVersion: PROMPT_VERSION,
    skaldVersion: "2.1.0",
    dimensions: EVAL_DIMENSIONS,
    notes: "Do not use an AI-detector score. Glue ratio is observation, not a gate.",
    packet,
    manifest,
  };
}

export function runMockEval({ corpusRoot = here } = {}) {
  const index = loadCorpusIndex(corpusRoot);
  const samples = [];
  for (const brief of index.briefs) {
    const briefText = readFileSync(resolve(corpusRoot, brief.path), "utf8");
    if (!brief.draft) {
      samples.push({
        briefId: brief.id,
        condition: "human",
        text: `(no committed draft; brief only)\n${briefText.trim()}`,
        scores: null,
      });
      continue;
    }
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
      scores: { schema: 2, repair: 2, grammar: 2, referents: 2, causality: null, repetition: null, form: null, ending: null },
    });
  }
  return buildBlindPacket({ briefs: index.briefs, samples, seed: 1 });
}

function main(argv = process.argv.slice(2)) {
  if (!argv.includes("--mock") && !argv.includes("--approve-expensive")) {
    process.stderr.write(
      "Usage: node eval.mjs --mock [--out packet.json]\n       node eval.mjs --approve-expensive --provider <name> --model <id> --reasoning <level>\n",
    );
    process.exit(1);
  }
  if (argv.includes("--approve-expensive")) {
    process.stderr.write("live eval is not wired in this harness; run loop per brief and score the blind packet by hand\n");
    process.exit(2);
  }
  const packet = runMockEval();
  const outFlag = argv.indexOf("--out");
  const json = `${JSON.stringify(packet, null, 2)}\n`;
  if (outFlag >= 0 && argv[outFlag + 1]) {
    const outPath = resolve(argv[outFlag + 1]);
    mkdirSync(dirname(outPath), { recursive: true });
    writeFileSync(outPath, json);
  }
  process.stdout.write(json);
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main();
}
