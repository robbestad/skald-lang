import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { explain } from "../../packages/skald-lang/index.js";
import { renderStory, setStoryChoiceLock, splitStoryDocument } from "./runner.mjs";
import { observeVariation } from "./corpus/eval.mjs";

const doc = JSON.parse(readFileSync(new URL("./stable-alternatives.json", import.meta.url), "utf8"));
const { request, draft } = splitStoryDocument(doc);
const palettes = { registry: {} };
const original = renderStory({ explain }, request, draft, palettes).artifact;
const selected = original.choices[0].alternativeId;
const observe = (nextRequest, nextDraft = draft) => observeVariation(
  { explain }, nextRequest, nextDraft, palettes, { seeds: [1, 42] },
);

const automatic = observe(request);
assert.equal(automatic.theoreticalCombinations, 2);
assert.equal(automatic.independentGroups, 1);
assert.equal(Object.hasOwn(automatic, "controlledGroups"), false);

for (const locked of [false, true]) {
  const choiceState = setStoryChoiceLock(original, "shirt", locked);
  const nextRequest = { ...request, choiceState };
  const fixed = observe(nextRequest);
  assert.equal(fixed.ok, true);
  assert.equal(fixed.uniqueOutputs, 1);
  assert.equal(fixed.theoreticalCombinations, 1, "duplicate VM slots are a single controlled choice");
  assert.equal(fixed.independentGroups, 0);
  assert.equal(fixed.controlledGroups, 1, "a synchronized group counts once across source occurrences");
  assert.match(fixed.note, /including when unlocked/u);
  for (const row of Object.values(fixed.observedByVariationId)) {
    assert.deepEqual(row.observedAlternativeIds, [selected]);
    assert.deepEqual(row.observed, [selected]);
  }

  const mixed = observe(nextRequest, { ...draft, beats: [...draft.beats, "A {button|ribbon}."] });
  assert.equal(mixed.ok, true);
  assert.equal(mixed.theoreticalCombinations, 2, "the independent, unmarked block still contributes its alternatives");
  assert.equal(mixed.independentGroups, 1);
  assert.equal(mixed.controlledGroups, 1);
}

let calls = 0;
const invalid = observeVariation({ explain() { calls += 1; throw new Error("unexpected render"); } }, {
  ...request,
  choiceState: { formatVersion: 1, groups: { shirt: { alternativeId: "missing", locked: false, rerollCount: 0 } } },
}, draft, palettes, { seeds: [1] });
assert.equal(invalid.ok, false);
assert.equal(calls, 0, "the eval passes saved choice state through preflight before engine execution");

console.log("story choice-control eval tests ok");
