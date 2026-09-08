import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import { explain } from "../../packages/skald-lang/index.js";
import {
  buildStoryPattern, inspectStoryDocument, mapPatternSpan, renderStory, rerollStoryChoice, setStoryChoiceLock,
  splitStoryDocument, validateStoryEnvelope,
} from "./runner.mjs";

const palettes = { registry: {} };
const byteSlice = (value, span) => Buffer.from(value).subarray(span.start, span.end).toString("utf8");
const forms = [
  { red: "raud", blue: "blå", green: "grøn" },
  { red: "raude", blue: "blåe", green: "grøne" },
];
const draft = {
  schemaVersion: 1,
  cast: [{ id: "hero", query: "<firstname female>" }],
  beats: [
    "Blå 🐦 <::hero> såg ei {raud|blå|grøn} skjorte. {I dag|I går}.",
    "Dei {grøne|raude|blåe} ermane ved {elva|havet}.",
  ],
};
const variations = [
  { variationId: "shirt-0", beatIndex: 0, pattern: "{raud|blå|grøn}", syncGroup: "shirt", alternativeIds: ["red", "blue", "green"] },
  { variationId: "shirt-1", beatIndex: 1, pattern: "{grøne|raude|blåe}", syncGroup: "shirt", alternativeIds: ["green", "red", "blue"] },
  { variationId: "place", beatIndex: 1, pattern: "{elva|havet}", syncGroup: "place", alternativeIds: ["river", "sea"] },
];
const render = (request, inputDraft = draft) => {
  const run = renderStory({ explain }, { variations, ...request }, inputDraft, palettes);
  assert.equal(run.ok, true, JSON.stringify(run.artifact.diagnostics));
  return run.artifact;
};
const selected = (artifact, group = "shirt") => artifact.choices.find((row) => row.syncGroup === group).alternativeId;
const rerender = (artifact, choiceState) => {
  const saved = splitStoryDocument(artifact);
  return render({ ...saved.request, choiceState }, saved.draft);
};
const withoutShirt = (artifact) => artifact.text
  .replace(`ei ${forms[0][selected(artifact)]} skjorte`, "ei [shirt] skjorte")
  .replace(`Dei ${forms[1][selected(artifact)]} ermane`, "Dei [shirt] ermane");

function assertRenderedEvidence(artifact) {
  const shirt = artifact.choices.filter((row) => row.syncGroup === "shirt");
  assert.equal(shirt.length, 2);
  assert.deepEqual(shirt.map((row) => row.alternativeId), [selected(artifact), selected(artifact)]);
  for (const choice of artifact.choices.filter((row) => row.alternativeId)) {
    const variation = artifact.variations.find((row) => row.variationId === choice.variationId);
    const source = artifact.draft.beats[choice.beatIndex];
    assert.equal(byteSlice(source, choice.beatSpan), variation.pattern, "block spans refer to the original UTF-8 source");
    const literal = byteSlice(source, choice.alternativeSpan);
    if (choice.syncGroup === "shirt") assert.equal(literal, forms[choice.beatIndex][choice.alternativeId]);
    assert.ok(artifact.text.includes(literal), `selected source literal ${literal} must appear in rendered text`);
    assert.match(byteSlice(artifact.pattern, choice.span), /^\{.*\}$/u, "runtime spans refer to executable compiled blocks");
  }
  assert.equal(artifact.channels.main, artifact.text);
  assert.equal(artifact.parts.map((part) => part.text).join(""), artifact.text);
  for (const [channel, parts] of Object.entries(artifact.partsByChannel)) {
    assert.equal(parts.map((part) => part.text).join(""), artifact.channels[channel]);
  }
  const executable = explain(artifact.pattern, { seed: artifact.effectiveSeed, case: "none", story: true, locale: artifact.locale });
  assert.equal(executable.text, artifact.text, "exported pattern must independently reproduce controlled choices");
  assert.deepEqual(executable.channels, artifact.channels);
  assert.deepEqual(executable.parts, artifact.parts);
  assert.deepEqual(executable.partsByChannel, artifact.partsByChannel);
}

// Opt-in keeps existing seed behavior and receipts intact. Enabling controls records every group.
for (const seed of [1, 42]) {
  const base = render({ seed });
  assert.equal(Object.hasOwn(base, "choiceState"), false, "legacy renders must not opt into choice control implicitly");
  const original = structuredClone(base);
  const lockedState = setStoryChoiceLock(base, "shirt");
  assert.deepEqual(base, original, "locking must not mutate the saved artifact");
  assert.equal(lockedState.formatVersion, 1);
  assert.deepEqual(Object.keys(lockedState.groups).sort(), ["place", "shirt"]);
  assert.deepEqual(lockedState.groups.shirt, { alternativeId: selected(base), locked: true, rerollCount: 0 });
  assert.deepEqual(lockedState.groups.place, { alternativeId: selected(base, "place"), locked: false, rerollCount: 0 });
  const locked = rerender(base, lockedState);
  assert.equal(locked.text, base.text);
  assert.deepEqual(locked.cast, base.cast);
  assert.deepEqual(locked.choiceState, lockedState);
  assert.notEqual(locked.replayHash, base.replayHash, "recorded locks belong in replay provenance even when text is unchanged");
  assertRenderedEvidence(locked);

  const edited = structuredClone(draft);
  edited.beats[0] = `{Today|Yesterday} ${edited.beats[0]}`;
  const prefixed = render({ seed, choiceState: lockedState }, edited);
  assert.equal(selected(prefixed), selected(base), "an unrelated preceding random choice cannot move a locked ID");
  assertRenderedEvidence(prefixed);
  assert.throws(() => rerollStoryChoice(locked, "shirt"), /lock/i, "rerolling a locked group requires an explicit unlock");

  const unlockInput = structuredClone(locked);
  const unlockedState = setStoryChoiceLock(locked, "shirt", false);
  assert.deepEqual(locked, unlockInput, "unlocking must not mutate the saved artifact");
  const unlocked = rerender(locked, unlockedState);
  assert.equal(unlocked.text, locked.text, "unlocking preserves a decision until an explicit reroll");
  assert.equal(unlocked.choiceState.groups.shirt.locked, false);

  let current = unlocked;
  for (let attempt = 1; attempt <= 3; attempt += 1) {
    const snapshot = structuredClone(current);
    const state = rerollStoryChoice(current, "shirt");
    assert.deepEqual(current, snapshot, "reroll helpers must not mutate a receipt");
    assert.deepEqual(rerollStoryChoice(current, "shirt"), state, "the same receipt and action have deterministic results");
    assert.notEqual(state.groups.shirt.alternativeId, selected(current), "reroll must choose another ID, not a possibly identical random sample");
    assert.equal(state.groups.shirt.rerollCount, attempt);
    const next = rerender(current, state);
    const repeated = rerender(current, state);
    assert.deepEqual(next, repeated, "repeated rendering of a reroll must reproduce the complete receipt");
    assert.notEqual(selected(next), selected(current));
    assert.equal(withoutShirt(next), withoutShirt(base), "cast, unnamed random choices, other groups, and prose are preserved");
    assert.deepEqual(next.cast, base.cast);
    assert.equal(selected(next, "place"), selected(base, "place"));
    assert.deepEqual(next.choiceState.groups.place, unlocked.choiceState.groups.place);
    assertRenderedEvidence(next);
    current = next;
  }
}

const baseline = render({ seed: 42 });
const validState = setStoryChoiceLock(baseline, "shirt", false);
const controlled = rerender(baseline, rerollStoryChoice(rerender(baseline, validState), "shirt"));

// Rewrites can grow and shrink multiple blocks before a query. Escapes stay literal,
// while every compiled duplicate maps back to the selected original alternative.
const escapedBlocks = [String.raw`{rød\|rosa \{fin\}|\\C blå}`, String.raw`{\\D blåe|rosa\|raud \{fine\}}`];
const escapedForms = [
  { pipe: String.raw`rød\|rosa \{fin\}`, path: String.raw`\\C blå` },
  { pipe: String.raw`rosa\|raud \{fine\}`, path: String.raw`\\D blåe` },
];
const escapedOutput = [
  { pipe: "rød|rosa {fin}", path: "\\C blå" },
  { pipe: "rosa|raud {fine}", path: "\\D blåe" },
];
const escapeDraft = {
  schemaVersion: 1, cast: [],
  beats: [`🧵 ${escapedBlocks[0]}; {x|svært grøn}. <firstname male> vinkar.`, `Så ${escapedBlocks[1]}. <firstname female> svarar.`],
};
const escapeVariations = [
  { variationId: "mark-0", beatIndex: 0, pattern: escapedBlocks[0], syncGroup: "mark", alternativeIds: ["pipe", "path"] },
  { variationId: "tone", beatIndex: 0, pattern: "{x|svært grøn}", syncGroup: "tone", alternativeIds: ["short", "long"] },
  { variationId: "mark-1", beatIndex: 1, pattern: escapedBlocks[1], syncGroup: "mark", alternativeIds: ["path", "pipe"] },
];
const escapeBase = render({ seed: 42, variations: escapeVariations }, escapeDraft);
let escapeRun = rerender(escapeBase, setStoryChoiceLock(escapeBase, "mark", false));
const unpositionedPicks = (artifact) => artifact.picks.map(({ span, ...pick }) => pick);
for (const action of [null, "mark", "tone"]) {
  if (action) escapeRun = rerender(escapeRun, rerollStoryChoice(escapeRun, action));
  const built = buildStoryPattern(escapeDraft, undefined, undefined, escapeVariations, escapeRun.choiceState);
  assert.equal(built.pattern, escapeRun.pattern);
  assert.deepEqual(built.diagnostics, []);
  for (const choice of escapeRun.choices.filter((row) => row.alternativeId)) {
    const source = escapeDraft.beats[choice.beatIndex];
    const variation = escapeVariations.find((row) => row.variationId === choice.variationId);
    const literal = byteSlice(source, choice.alternativeSpan);
    assert.equal(byteSlice(source, choice.beatSpan), variation.pattern);
    if (choice.syncGroup === "mark") {
      assert.equal(literal, escapedForms[choice.beatIndex][choice.alternativeId]);
      assert.ok(escapeRun.text.includes(escapedOutput[choice.beatIndex][choice.alternativeId]));
    }
    const compiledBlock = byteSlice(escapeRun.pattern, choice.span);
    assert.equal(compiledBlock, `{${variation.alternativeIds.map(() => literal).join("|")}}`);
    for (let slot = 0; slot < variation.alternativeIds.length; slot += 1) {
      const start = choice.span.start + 1 + slot * (Buffer.byteLength(literal) + 1);
      const mapped = mapPatternSpan(built.sourceMap, { start, end: start + Buffer.byteLength(literal) });
      assert.equal(mapped.beatIndex, choice.beatIndex);
      assert.deepEqual(mapped.span, choice.alternativeSpan, "every forced runtime slot maps to the chosen original UTF-8 literal");
    }
  }
  assert.equal(escapeRun.picks.length, 2);
  assert.deepEqual(unpositionedPicks(escapeRun), unpositionedPicks(escapeBase), "literal rewrites preserve following dictionary draws");
  for (const pick of escapeRun.picks) {
    const mapped = mapPatternSpan(built.sourceMap, pick.span);
    const query = mapped.beatIndex === 0 ? "<firstname male>" : "<firstname female>";
    assert.equal(byteSlice(escapeRun.pattern, pick.span), query);
    assert.equal(byteSlice(escapeDraft.beats[mapped.beatIndex], mapped.span), query, "query spans after two rewrites and in later beats retain their source location");
  }
  const executable = explain(escapeRun.pattern, { seed: escapeRun.effectiveSeed, case: "none", story: true });
  assert.equal(executable.text, escapeRun.text);
  assert.deepEqual(executable.partsByChannel, escapeRun.partsByChannel);
}

// A two-name version of the existing twin fixture collides at seed 1 and resolves
// at retry seed "2". Choice controls must keep that retry and its cast unchanged.
const retryPalettes = { registry: { twin: { id: "twin", dictionary: { tables: {
  twin: { name: "twin", subs: ["default"], entries: [{ forms: ["Ada"], classes: [] }, { forms: ["Bea"], classes: [] }] },
} } } } };
const retryDraft = {
  schemaVersion: 1,
  cast: [{ id: "a", query: "<twin>" }, { id: "b", query: "<twin>" }],
  beats: ["<::a> and <::b> saw {red|blue}. {Today|Yesterday}."],
};
const retryRequest = {
  seed: 1, paletteIds: ["twin"], policy: { castNameRetries: 3 },
  variations: [{ variationId: "shirt", beatIndex: 0, pattern: "{red|blue}", syncGroup: "shirt", alternativeIds: ["red", "blue"] }],
};
const renderRetry = (request) => {
  const run = renderStory({ explain }, request, retryDraft, retryPalettes);
  assert.equal(run.ok, true, JSON.stringify(run.artifact.diagnostics));
  assert.equal(run.artifact.seed, 1);
  assert.equal(run.artifact.effectiveSeed, "2");
  assert.equal(run.artifact.castNameRetries, 1);
  return run.artifact;
};
const retryBase = renderRetry(retryRequest);
const retryControlled = renderRetry({ ...retryRequest, choiceState: setStoryChoiceLock(retryBase, "shirt", false) });
const retryRerolled = renderRetry({ ...retryRequest, choiceState: rerollStoryChoice(retryControlled, "shirt") });
assert.notEqual(selected(retryControlled), selected(retryRerolled));
assert.deepEqual(retryRerolled.cast, retryBase.cast);
assert.equal(retryRerolled.text.replace(selected(retryRerolled), "[shirt]"), retryBase.text.replace(selected(retryBase), "[shirt]"));
assert.equal(renderRetry(splitStoryDocument(retryRerolled).request).replayHash, retryRerolled.replayHash);

// Invalid or stale controls fail before the VM runs, rather than silently changing a saved decision.
function rejectState(label, choiceState, request = {}, inputDraft = draft) {
  let engineCalls = 0;
  const run = renderStory({ explain() { engineCalls += 1; throw new Error("invalid controls reached the VM"); } }, {
    seed: 42, variations, ...request, choiceState,
  }, inputDraft, palettes);
  assert.equal(run.ok, false, label);
  assert.equal(engineCalls, 0, `${label}: validation must precede engine execution`);
  assert.ok(run.artifact.diagnostics.some((row) => row.code === "STORY_CHOICE_CONFLICT" && row.severity === "error"), `${label}: a conflict diagnostic is required`);
}
for (const malformed of [
  null, undefined, [], "shirt", {},
  { formatVersion: 2, groups: {} },
  { formatVersion: "1", groups: {} },
  { formatVersion: 1, groups: {}, unexpected: true },
  { formatVersion: 1, groups: null },
  { formatVersion: 1, groups: [] },
  { formatVersion: 1, groups: { shirt: null } },
  { formatVersion: 1, groups: { shirt: { alternativeId: "red", locked: "yes", rerollCount: 0 } } },
  { formatVersion: 1, groups: { shirt: { alternativeId: "red", locked: false, rerollCount: 0, unexpected: true } } },
  { formatVersion: 1, groups: { shirt: { alternativeId: "red", locked: false, rerollCount: -1 } } },
  { formatVersion: 1, groups: { shirt: { alternativeId: "red", locked: false, rerollCount: 0.5 } } },
  { formatVersion: 1, groups: { shirt: { alternativeId: "red", locked: false, rerollCount: Number.MAX_SAFE_INTEGER + 1 } } },
  { formatVersion: 1, groups: { shirt: { alternativeId: "red", locked: false } } },
  { formatVersion: 1, groups: { shirt: { alternativeId: "red", rerollCount: 0 } } },
  { formatVersion: 1, groups: { shirt: { locked: false, rerollCount: 0 } } },
]) rejectState(`malformed state ${JSON.stringify(malformed)}`, malformed);

const unknown = structuredClone(validState);
unknown.groups.missing = { alternativeId: "red", locked: false, rerollCount: 0 };
rejectState("unknown group", unknown);
const removedId = structuredClone(validState);
removedId.groups.shirt.alternativeId = "removed";
rejectState("unknown alternative ID", removedId);
for (const locked of [false, true]) {
  const state = structuredClone(validState);
  state.groups.shirt.locked = locked;
  const removed = state.groups.shirt.alternativeId;
  const editedDraft = structuredClone(draft);
  const editedVariations = structuredClone(variations).map((row) => {
    if (row.syncGroup !== "shirt") return row;
    const ids = row.alternativeIds.filter((id) => id !== removed);
    const pattern = `{${ids.map((id) => forms[row.beatIndex][id]).join("|")}}`;
    editedDraft.beats[row.beatIndex] = editedDraft.beats[row.beatIndex].replace(row.pattern, pattern);
    return { ...row, pattern, alternativeIds: ids };
  });
  rejectState(`removed previously selected alternative (locked=${locked})`, state, { variations: editedVariations }, editedDraft);
}
for (const helper of [setStoryChoiceLock, rerollStoryChoice]) {
  assert.throws(() => helper(baseline, "missing"), /group|choice/i);
  assert.throws(() => helper({ ...baseline, ok: false }, "shirt"), /artifact|successful|failed|render/i);
  assert.throws(() => helper({ ...baseline, pattern: `${baseline.pattern} changed` }, "shirt"), /STORY_CHOICE_CONFLICT/);
  assert.throws(() => helper({ ...baseline, choices: [] }, "shirt"), /STORY_CHOICE_CONFLICT/);
  const staleTrace = structuredClone(baseline);
  staleTrace.choices.find((row) => row.syncGroup === "shirt").alternativeId = "removed";
  assert.throws(() => helper(staleTrace, "shirt"), /STORY_CHOICE_CONFLICT/);
}
assert.throws(() => setStoryChoiceLock(baseline, "shirt", "yes"), /STORY_CHOICE_CONFLICT/);

// JSON-envelope and receipt paths carry the same controlled decisions, including immutable provenance.
const envelope = { schemaVersion: 1, seed: 42, draft, variations, choiceState: controlled.choiceState };
assert.equal(validateStoryEnvelope(envelope).ok, true);
assert.equal(inspectStoryDocument(envelope, {}).ok, true);
for (const malformed of [
  { ...controlled.choiceState, unexpected: true },
  { ...controlled.choiceState, groups: { ...controlled.choiceState.groups, shirt: { ...controlled.choiceState.groups.shirt, unexpected: true } } },
]) {
  const invalidEnvelope = { ...envelope, choiceState: malformed };
  assert.equal(validateStoryEnvelope(invalidEnvelope).ok, false, "envelope schema rejects unknown choiceState properties");
  assert.equal(inspectStoryDocument(invalidEnvelope, {}).ok, false, "document inspection rejects unknown choiceState properties");
}
assert.deepEqual(splitStoryDocument(envelope).request.choiceState, controlled.choiceState);
const envelopeBefore = structuredClone(envelope);
const stateBefore = structuredClone(envelope.choiceState);
const replayed = render(splitStoryDocument(envelope).request);
assert.deepEqual(envelope, envelopeBefore, "rendering must not mutate its input envelope, variations, draft, or choice state");
assert.deepEqual(envelope.choiceState, stateBefore);
assert.equal(replayed.replayHash, controlled.replayHash);

const receiptDir = mkdtempSync(resolve(tmpdir(), "skald-choice-control-"));
const hostPath = new URL("./host.mjs", import.meta.url).pathname;
try {
  const receiptPath = resolve(receiptDir, "receipt.json");
  for (const artifact of [baseline, controlled, rerender(controlled, setStoryChoiceLock(controlled, "shirt"))]) {
    writeFileSync(receiptPath, JSON.stringify(artifact));
    const output = JSON.parse(execFileSync(process.execPath, [hostPath, "replay", receiptPath, "--json"], { encoding: "utf8" }));
    assert.equal(output.text, artifact.text);
    assert.equal(output.replayHash, artifact.replayHash);
    assert.deepEqual(output.choiceState, artifact.choiceState);
    assert.deepEqual(output.choices, artifact.choices);
  }
  writeFileSync(receiptPath, JSON.stringify(baseline));
  const lockedPath = resolve(receiptDir, "locked.json");
  const cliLocked = JSON.parse(execFileSync(process.execPath, [hostPath, "lock", receiptPath, "--group", "shirt", "--artifact", lockedPath], { encoding: "utf8" }));
  assert.equal(cliLocked.choiceState.groups.shirt.locked, true);
  assert.equal(cliLocked.text, baseline.text);
  assert.deepEqual(JSON.parse(readFileSync(lockedPath, "utf8")), cliLocked);
  assert.equal(readFileSync(resolve(receiptDir, "locked.skald"), "utf8"), `${cliLocked.pattern}\n`);
  const lockedReroll = spawnSync(process.execPath, [hostPath, "reroll", lockedPath, "--group", "shirt"], { encoding: "utf8" });
  assert.equal(lockedReroll.status, 2);
  assert.ok(JSON.parse(lockedReroll.stdout).diagnostics.some((row) => row.code === "STORY_CHOICE_CONFLICT"));
  const cliUnlocked = JSON.parse(execFileSync(process.execPath, [hostPath, "unlock", lockedPath, "--group", "shirt"], { encoding: "utf8" }));
  assert.equal(cliUnlocked.choiceState.groups.shirt.locked, false);
  assert.equal(cliUnlocked.text, baseline.text);
  writeFileSync(receiptPath, JSON.stringify(cliUnlocked));
  const cliRerolled = JSON.parse(execFileSync(process.execPath, [hostPath, "reroll", receiptPath, "--group", "shirt"], { encoding: "utf8" }));
  assert.notEqual(selected(cliRerolled), selected(cliUnlocked));
  assert.equal(withoutShirt(cliRerolled), withoutShirt(cliUnlocked));
  writeFileSync(receiptPath, JSON.stringify(cliRerolled));
  assert.equal(execFileSync(process.execPath, [hostPath, "pattern", receiptPath], { encoding: "utf8" }), `${cliRerolled.pattern}\n`);
  const missingGroup = spawnSync(process.execPath, [hostPath, "lock", receiptPath], { encoding: "utf8" });
  assert.equal(missingGroup.status, 2);
  assert.ok(JSON.parse(missingGroup.stdout).diagnostics.some((row) => row.code === "STORY_CHOICE_CONFLICT"));
  const tampered = structuredClone(controlled);
  tampered.choiceState.groups.shirt.locked = !tampered.choiceState.groups.shirt.locked;
  writeFileSync(receiptPath, JSON.stringify(tampered));
  const mismatch = spawnSync(process.execPath, [hostPath, "replay", receiptPath, "--json"], { encoding: "utf8" });
  assert.equal(mismatch.status, 2, "changed lock metadata must invalidate a saved receipt even with identical text");
  assert.ok(JSON.parse(mismatch.stdout).diagnostics.some((row) => row.code === "STORY_REPLAY_MISMATCH"));
} finally {
  rmSync(receiptDir, { recursive: true, force: true });
}

console.log("story choice-control tests ok");
