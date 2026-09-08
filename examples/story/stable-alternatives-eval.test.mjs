import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { explain } from "../../packages/skald-lang/index.js";
import { renderStory, splitStoryDocument } from "./runner.mjs";
import { PALETTES } from "./palettes.mjs";
import { observeVariation } from "./corpus/eval.mjs";

const doc = JSON.parse(readFileSync(new URL("./stable-alternatives.json", import.meta.url), "utf8"));
const { request, draft } = splitStoryDocument(doc);
for (const seed of [1, 42]) {
  const run = renderStory({ explain }, { ...request, seed }, draft, { registry: PALETTES });
  assert.equal(run.ok, true, JSON.stringify(run.artifact.diagnostics));
  const color = /The shirt was (red|blue)\./.exec(run.artifact.text)?.[1];
  assert.ok(color, run.artifact.text);
  assert.ok(run.artifact.text.includes(`Later, the same shirt was ${color}.`));
  const report = observeVariation({ explain }, request, draft, { registry: PALETTES }, { seeds: [seed] });
  assert.equal(report.ok, true);
  for (const variation of request.variations) {
    const row = report.observedByVariationId[variation.variationId];
    assert.deepEqual(row.alternativeIds, variation.alternativeIds);
    assert.deepEqual(row.observedAlternativeIds, [color]);
    assert.deepEqual(row.observed, [color], "report must use the source alternative associated with the selected ID");
  }
}

console.log("stable alternative eval tests ok");

await import("./choice-control-eval.test.mjs");
