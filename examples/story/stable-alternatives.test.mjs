import assert from "node:assert/strict";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import { execFileSync } from "node:child_process";
import { explain } from "../../packages/skald-lang/index.js";
import {
  applySkaldTransform, buildStoryPattern, inspectStoryDocument, mapPatternSpan,
  mergeStoryVariations, renderStory, splitStoryDocument, syncRepeatedChoices,
  validateStoryEnvelope,
} from "./runner.mjs";

const bytes = (value) => Buffer.byteLength(value, "utf8");
const byteSlice = (value, span) => Buffer.from(value).subarray(span.start, span.end).toString("utf8");
const draftOf = (beats, cast = []) => ({ schemaVersion: 1, cast, beats });
const variation = (beatIndex, pattern, alternativeIds, extra = {}) => ({
  variationId: `color-${beatIndex}`, beatIndex, pattern, syncGroup: "color", alternativeIds, ...extra,
});
const render = (draft, variations, seed = 1) => renderStory({ explain }, { seed, variations }, draft, { registry: {} });
const pattern = (forms, ids) => `{${ids.map((id) => forms[id]).join("|")}}`;

// Reordering either or both source lists cannot change the compiled choice or its identity.
for (const forms of [
  [{ red: "red", blue: "blue" }, { red: "red", blue: "blue" }],
  [{ red: "raud", blue: "blå" }, { red: "raude", blue: "blåe" }],
]) {
  const exercised = new Set();
  for (const seed of [1, 42]) {
    let expected;
    for (const firstIds of [["red", "blue"], ["blue", "red"]]) {
      for (const secondIds of [["red", "blue"], ["blue", "red"]]) {
        const ids = [firstIds, secondIds];
        const patterns = forms.map((row, i) => pattern(row, ids[i]));
        const draft = draftOf(patterns.map((row) => `The shirt was ${row}.`));
        const variations = patterns.map((row, i) => variation(i, row, ids[i]));
        const run = render(draft, variations, seed);
        assert.equal(run.ok, true, JSON.stringify(run.artifact.diagnostics));
        expected ??= { text: run.artifact.text, pattern: run.artifact.pattern, id: run.artifact.choices[0].alternativeId };
        exercised.add(expected.id);
        assert.equal(run.artifact.text, expected.text, `seed ${seed}`);
        assert.equal(run.artifact.pattern, expected.pattern, `canonical compilation for seed ${seed}`);
        for (const choice of run.artifact.choices) {
          assert.equal(choice.alternativeId, expected.id);
          assert.equal(choice.variationId, variations[choice.beatIndex].variationId);
          assert.equal(byteSlice(draft.beats[choice.beatIndex], choice.alternativeSpan), forms[choice.beatIndex][expected.id]);
        }
      }
    }
  }
  assert.deepEqual([...exercised].sort(), ["blue", "red"]);
}

// Multiple blocks in a beat retain separate targets even when canonicalization makes their text identical.
const adjacentDraft = draftOf(["A {red|blue} shirt and a {blue|red} hat."]);
const adjacentVariations = [
  variation(0, "{red|blue}", ["red", "blue"]),
  variation(0, "{blue|red}", ["blue", "red"], { variationId: "hat", syncGroup: "hat" }),
];
const adjacent = render(adjacentDraft, adjacentVariations);
assert.equal(adjacent.ok, true, JSON.stringify(adjacent.artifact.diagnostics));
assert.deepEqual(adjacent.artifact.choices.map((row) => row.variationId), ["color-0", "hat"]);
assert.match(adjacent.artifact.pattern, /\[sync:color;locked\]\{blue\|red\}.*\[sync:hat;locked\]\{blue\|red\}/u);

// The source map includes Unicode byte offsets, unequal alternatives, multiple tags, and a cast prelude.
const unicodeDraft = draftOf(["Blå 🐦 {lys rød|mørkeblå}; {gått|kome}. <::hero> såg."], [{ id: "hero", query: "<firstname female>" }]);
const unicodeVariations = [
  variation(0, "{lys rød|mørkeblå}", ["red", "blue"]),
  variation(0, "{gått|kome}", ["go", "come"], { variationId: "motion", syncGroup: "motion" }),
];
const unicodeBuilt = buildStoryPattern(unicodeDraft, undefined, undefined, unicodeVariations);
assert.deepEqual(unicodeBuilt.diagnostics, []);
for (const [canonicalBlock, selectedLiteral] of [["{mørkeblå|lys rød}", "mørkeblå"], ["{kome|gått}", "gått"]]) {
  const compiledStart = unicodeBuilt.pattern.indexOf(selectedLiteral, unicodeBuilt.pattern.indexOf(canonicalBlock));
  const mapped = mapPatternSpan(unicodeBuilt.sourceMap, {
    start: bytes(unicodeBuilt.pattern.slice(0, compiledStart)),
    end: bytes(unicodeBuilt.pattern.slice(0, compiledStart + selectedLiteral.length)),
  });
  assert.equal(mapped.beatIndex, 0);
  assert.equal(byteSlice(unicodeDraft.beats[0], mapped.span), selectedLiteral);
  assert.equal(mapped.span.start, bytes(unicodeDraft.beats[0].slice(0, unicodeDraft.beats[0].indexOf(selectedLiteral))));
}
const recallStart = unicodeBuilt.pattern.indexOf("<::hero>");
const mappedRecall = mapPatternSpan(unicodeBuilt.sourceMap, {
  start: bytes(unicodeBuilt.pattern.slice(0, recallStart)),
  end: bytes(unicodeBuilt.pattern.slice(0, recallStart + "<::hero>".length)),
});
assert.equal(byteSlice(unicodeDraft.beats[0], mappedRecall.span), "<::hero>");
for (const seed of [1, 42]) {
  const run = render(unicodeDraft, unicodeVariations, seed);
  assert.equal(run.ok, true, JSON.stringify(run.artifact.diagnostics));
  for (const choice of run.artifact.choices.filter((row) => row.alternativeId)) {
    const original = unicodeVariations.find((row) => row.variationId === choice.variationId);
    assert.equal(byteSlice(unicodeDraft.beats[0], choice.beatSpan), original.pattern);
    const literal = byteSlice(unicodeDraft.beats[0], choice.alternativeSpan);
    assert.ok(run.artifact.text.includes(literal));
    assert.equal(byteSlice(run.artifact.pattern, choice.span).startsWith("{"), true);
  }
}

// Escaped separators/braces and literal backslashes remain whole alternatives when moved.
const escapedPattern = String.raw`{rød\|rosa \{fin\}|\\C blå}`;
const escaped = render(draftOf([escapedPattern]), [variation(0, escapedPattern, ["red", "blue"])]);
assert.equal(escaped.ok, true, JSON.stringify(escaped.artifact.diagnostics));
assert.equal(escaped.artifact.pattern, String.raw`[sync:color;locked]{\\C blå|rød\|rosa \{fin\}}`);

const baseDraft = draftOf(["The shirt was {red|blue}.", "Still {blue|red}."]);
const baseVariations = [variation(0, "{red|blue}", ["red", "blue"]), variation(1, "{blue|red}", ["blue", "red"])];
const invalid = (label, change, draft = baseDraft) => {
  const rows = structuredClone(baseVariations);
  change(rows);
  let rendered = false;
  const run = renderStory({ explain() { rendered = true; throw new Error("must not render invalid metadata"); } }, { seed: 1, variations: rows }, draft, { registry: {} });
  assert.equal(run.ok, false, label);
  assert.equal(rendered, false, `${label}: validation must happen before rendering`);
  assert.ok(run.artifact.diagnostics.some((row) => row.code === "STORY_ALTERNATIVE_ID"), `${label}: ${JSON.stringify(run.artifact.diagnostics)}`);
};
for (const ids of [null, undefined, [], ["red"], ["red", "red"], ["red", "blue", "green"], ["red", ""], ["red", "two words"], ["red", "3blue"], ["red", 3], ["red", "a".repeat(33)]]) {
  invalid(`invalid ID list ${JSON.stringify(ids)}`, (rows) => { rows[0].alternativeIds = ids; });
}
invalid("conflicting ID sets", (rows) => { rows[1].alternativeIds = ["green", "red"]; });
invalid("mixed identified and positional group", (rows) => { delete rows[1].alternativeIds; });
invalid("missing variationId", (rows) => { delete rows[0].variationId; });
invalid("duplicate variationId", (rows) => { rows[1].variationId = rows[0].variationId; });
invalid("empty syncGroup", (rows) => { rows[0].syncGroup = ""; });
invalid("invalid syncGroup", (rows) => { rows[0].syncGroup = "x;deck"; });
invalid("invalid beatIndex", (rows) => { rows[0].beatIndex = 42; });
invalid("missing target", (rows) => { rows[0].pattern = "{red|green}"; });
invalid("stale start", (rows) => { rows[0].start = 0; });
invalid("invalid end", (rows) => { rows[0].end = 1; });
invalid("overlapping metadata", (rows) => { rows.push({ ...rows[0], variationId: "other", syncGroup: "other" }); });
invalid("unidentified overlapping metadata", (rows) => { rows.push({ beatIndex: 0, pattern: "{red|blue}", variationId: "other", syncGroup: "other" }); });
invalid("ambiguous repeated pattern", () => {}, draftOf(["{red|blue} and {red|blue}.", "Still {blue|red}."]));
for (const unsupported of ["{red|blue", "{red|blue}}", "{{red|blue}", "{red|blue} {red|blue}", "{red|{blue|green}}", "{(2)red|blue}", String.raw`{\s(2)red|blue}`, "{<noun>|blue}", String.raw`{\C|blue}`, String.raw`{\d|blue}`]) {
  const draft = draftOf([unsupported]);
  const compiled = buildStoryPattern(draft, undefined, undefined, [variation(0, unsupported, ["red", "blue"])]);
  assert.ok(compiled.diagnostics.some((row) => row.code === "STORY_ALTERNATIVE_ID"), unsupported);
}
invalid("nested target", () => {}, draftOf(["{{red|blue}|green}.", "Still {blue|red}."]));
const manual = renderStory({ explain }, { seed: 1, policy: { allowAdvancedTags: true }, variations: baseVariations }, draftOf(["[sync:color;locked]{red|blue}.", "Still {blue|red}."]), { registry: {} });
assert.equal(manual.ok, false);
assert.ok(manual.artifact.diagnostics.some((row) => row.code === "STORY_ALTERNATIVE_ID"));

// Explicit offsets can disambiguate repeated source blocks.
const repeated = "{red|blue} and {red|blue}.";
const located = render(draftOf([repeated]), [
  variation(0, "{red|blue}", ["red", "blue"], { start: 0 }),
  variation(0, "{red|blue}", ["red", "blue"], { variationId: "later", start: repeated.lastIndexOf("{red|blue}") }),
]);
assert.equal(located.ok, true, JSON.stringify(located.artifact.diagnostics));
assert.equal(located.artifact.choices.length, 2);

// Metadata survives application, merging, JSON envelopes, and saved CLI receipts.
const applied = applySkaldTransform(draftOf(["The shirt was red.", "Still red."]), {
  substitutions: baseVariations.map((row) => ({ ...row, literal: "red" })),
});
assert.deepEqual(applied.diagnostics, []);
assert.deepEqual(applied.substitutions.map((row) => row.alternativeIds), baseVariations.map((row) => row.alternativeIds));
assert.deepEqual(mergeStoryVariations([], applied.substitutions, applied.draft), applied.substitutions);
const missingAfterRepair = mergeStoryVariations(applied.substitutions, [], draftOf(["The shirt disappeared.", "Still red."]));
assert.equal(missingAfterRepair.length, 2, "identified targets must fail validation instead of silently losing metadata after a repair");
assert.ok(syncRepeatedChoices(["The shirt disappeared.", "Still red."], missingAfterRepair).diagnostics.length);
const envelope = { schemaVersion: 1, seed: 42, draft: applied.draft, variations: applied.substitutions };
assert.equal(validateStoryEnvelope(envelope).ok, true);
assert.equal(inspectStoryDocument(envelope, {}).ok, true);
const saved = render(applied.draft, applied.substitutions, 42).artifact;
const split = splitStoryDocument(JSON.parse(JSON.stringify(saved)));
const replayed = renderStory({ explain }, split.request, split.draft, { registry: {} });
assert.equal(replayed.ok, true);
assert.equal(replayed.artifact.replayHash, saved.replayHash);
assert.deepEqual(replayed.artifact.choices, saved.choices);
const badTransform = applySkaldTransform(draftOf(["A red shirt."]), { substitutions: [{ beatIndex: 0, literal: "red", pattern: "{red|blue}", syncGroup: "color", alternativeIds: ["red", "blue"] }] });
assert.ok(badTransform.diagnostics.some((row) => row.code === "STORY_ALTERNATIVE_ID"), "transforms cannot manufacture a missing identified variationId");
const receiptDir = mkdtempSync(resolve(tmpdir(), "skald-stable-receipt-"));
try {
  const receiptPath = resolve(receiptDir, "receipt.json");
  writeFileSync(receiptPath, JSON.stringify(saved));
  const output = execFileSync(process.execPath, [new URL("./host.mjs", import.meta.url).pathname, "replay", receiptPath, "--json"], { encoding: "utf8" });
  assert.equal(JSON.parse(output).replayHash, saved.replayHash);
} finally {
  rmSync(receiptDir, { recursive: true, force: true });
}
console.log("stable alternative compiler tests ok");
