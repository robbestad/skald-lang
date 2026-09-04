#!/usr/bin/env node
import { execFileSync, spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, rmSync, unlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { compile, explain } from "../../packages/skald-lang/index.js";
import { createMockModel } from "./mock-model.mjs";
import { createOpenAIModel } from "./adapters/openai.mjs";
import { createOllamaModel } from "./adapters/ollama.mjs";
import { PALETTES } from "./palettes.mjs";
import {
  analyzeStoryDraft,
  applySkaldTransform,
  buildModelPrompt,
  buildNarrativeReviewPrompt,
  buildSkaldCoveragePrompt,
  buildSkaldizePrompt,
  buildStoryPattern,
  createStoryArtifact,
  deterministicSegment,
  diagnostic,
  diagnosticKey,
  expansionPlan,
  inspectStoryDocument,
  joinStoryBeats,
  mapPatternSpan,
  renderStory,
  revisionDiagnostics,
  runStoryLoop,
  splitStoryDocument,
  syncRepeatedChoices,
  validateStoryDraft,
  variationDiagnostics,
  validateStoryEnvelope,
} from "./runner.mjs";
const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const here = dirname(fileURLToPath(import.meta.url));
let failed = 0;

function assert(cond, msg) {
  if (!cond) {
    console.error(msg);
    failed += 1;
  }
}

function golden(name, seed) {
  return readFileSync(resolve(here, "goldens", `${name}.seed-${seed}.txt`), "utf8");
}

function runHost(args) {
  return execFileSync(process.execPath, [resolve(here, "host.mjs"), ...args], {
    encoding: "utf8",
    cwd: root,
  });
}

const inn = JSON.parse(readFileSync(resolve(here, "inn.json"), "utf8"));
const grim = JSON.parse(readFileSync(resolve(here, "grim-fairytale.json"), "utf8"));
const dont = JSON.parse(readFileSync(resolve(here, "dont.json"), "utf8"));
const innDraftFrom = (doc) => doc.draft ?? {
  schemaVersion: doc.schemaVersion ?? 1,
  cast: doc.cast,
  beats: doc.beats,
};
const dontDraft = innDraftFrom(dont);

const innOut = runHost(["render", resolve(here, "inn.json")]);
assert(innOut === golden("inn", 11), `inn golden mismatch\n${innOut}`);

const replayDir = mkdtempSync(resolve(tmpdir(), "skald-story-replay-"));
const replayPath = resolve(replayDir, "artifact.json");
const savedArtifact = JSON.parse(runHost(["render", resolve(here, "inn.json"), "--json"]));
writeFileSync(replayPath, JSON.stringify(savedArtifact));
assert(
  runHost(["replay", replayPath]) === `${savedArtifact.text}\n`,
  "saved artifact should replay without a model",
);
rmSync(replayDir, { recursive: true, force: true });

const exportDir = mkdtempSync(resolve(tmpdir(), "skald-story-export-"));
const manualPatternPath = resolve(exportDir, "manual.skald");
runHost(["pattern", resolve(here, "inn.json"), "--skald", manualPatternPath]);
const manualPattern = readFileSync(manualPatternPath, "utf8");
assert(manualPattern.includes("<::hero>"), "manual pattern export should retain carriers");
assert(
  nativePattern(manualPatternPath).includes("the inn"),
  "manual .skald export should run in the native binary",
);
const artifactPath = resolve(exportDir, "ordinary-tuesday.json");
runHost([
  "render",
  resolve(here, "inn.json"),
  "--json",
  "--artifact",
  artifactPath,
]);
assert(existsSync(artifactPath), "artifact JSON output should be written");
assert(
  existsSync(resolve(exportDir, "ordinary-tuesday.skald")),
  "artifact output should have a sibling .skald file",
);
rmSync(exportDir, { recursive: true, force: true });

const grimOut = runHost(["render", resolve(here, "grim-fairytale.json")]);
assert(grimOut === golden("grim-fairytale", 6), `grim golden mismatch\n${grimOut}`);

try {
  runHost(["render", resolve(here, "dont.json")]);
  assert(false, "dont.json should fail");
} catch (err) {
  assert(err.status === 2, `dont.json exit ${err.status}`);
  assert(String(err.stdout).includes("STORY_OPEN"), `dont diagnostics: ${err.stdout}`);
}

const schemaBad = validateStoryDraft({ schemaVersion: 1, beats: ["x"] });
assert(!schemaBad.ok, "missing cast should fail schema");

const emptyCast = validateStoryDraft({
  schemaVersion: 1,
  cast: [],
  beats: ["Mr. Egg woke at 6:15."],
});
assert(emptyCast.ok, `empty cast ${JSON.stringify(emptyCast.diagnostics)}`);

for (const aside of [
  "Enthusiasm was merely gravity wearing a friendly badge.",
  "Nothing happened, which was not the same as nothing having changed.",
]) {
  const result = analyzeStoryDraft({ schemaVersion: 1, cast: [], beats: [aside] });
  assert(
    result.diagnostics.some((row) => row.code === "STORY_WRITERLY_ASIDE"),
    `writerly aside should be rejected: ${aside}`,
  );
}

const invalidFirstnameFilter = validateStoryDraft({
  schemaVersion: 1,
  cast: [{ id: "child", query: "<firstname female child>" }],
  beats: ["<::child> er registrert."],
});
assert(
  invalidFirstnameFilter.diagnostics.some((d) => d.code === "STORY_CAST"),
  `firstname filter ${JSON.stringify(invalidFirstnameFilter.diagnostics)}`,
);

const extraField = validateStoryDraft({
  schemaVersion: 1,
  cast: [{ id: "hero", query: "<firstname female>" }],
  beats: ["<::hero> sat."],
  seed: 1,
});
assert(!extraField.ok, "seed on draft should fail");

const rebind = analyzeStoryDraft({
  schemaVersion: 1,
  cast: [{ id: "hero", query: "<firstname female>" }],
  beats: ["<firstname female :: hero> sat."],
});
assert(
  rebind.diagnostics.some((d) => d.code === "STORY_CARRIER"),
  `rebind ${JSON.stringify(rebind.diagnostics)}`,
);

const unbound = analyzeStoryDraft({
  schemaVersion: 1,
  cast: [{ id: "hero", query: "<firstname female>" }],
  beats: ["<::other> sat."],
});
assert(
  unbound.diagnostics.some((d) => d.code === "STORY_CARRIER"),
  "unbound carrier",
);

const unusedCast = analyzeStoryDraft({
  schemaVersion: 1,
  cast: [{ id: "kirsten", query: "<firstname female>" }],
  beats: ["Kirsten signerte beretningen."],
});
assert(
  unusedCast.diagnostics.some(
    (d) => d.code === "STORY_CARRIER" && d.message.includes("never recalled"),
  ),
  `unused cast carrier ${JSON.stringify(unusedCast.diagnostics)}`,
);
const twoUnused = inspectStoryDocument(
  {
    schemaVersion: 1,
    draft: {
      schemaVersion: 1,
      cast: [
        { id: "kirsten", query: "<firstname female>" },
        { id: "jo", query: "<firstname male>" },
      ],
      beats: ["Nobody arrived."],
    },
  },
  PALETTES,
);
assert(
  twoUnused.diagnostics.filter(
    (d) => d.code === "STORY_CARRIER" && d.message.includes("never recalled"),
  ).length === 2,
  `distinct unused carriers ${JSON.stringify(twoUnused.diagnostics)}`,
);

for (const tag of ["[ rep:3]{x}", "[r:3]{x}", "[sync:s;locked]{x}", "[repeach]{x}"]) {
  const tagged = analyzeStoryDraft({
    schemaVersion: 1,
    cast: [{ id: "hero", query: "<firstname female>" }],
    beats: [`<::hero> ${tag}`],
  });
  assert(
    tagged.diagnostics.some((d) => d.code === "STORY_ADVANCED_TAG"),
    `advanced tag bypass: ${tag}`,
  );
}

const outerBlock = analyzeStoryDraft({
  schemaVersion: 1,
  cast: [{ id: "hero", query: "<firstname female>" }],
  beats: ["<::hero> chose {outer {a|b}|c|d|e|f|g|h}."],
});
assert(
  outerBlock.diagnostics.some(
    (d) => d.code === "STORY_BLOCK" && d.message.includes("7 alternatives"),
  ),
  `outer block limit ${JSON.stringify(outerBlock.diagnostics)}`,
);

const unicodeQuery = analyzeStoryDraft({
  schemaVersion: 1,
  cast: [{ id: "hero", query: "<firstname female>" }],
  beats: ["Blå 🌨️ <place>."]
});
const placeSpan = unicodeQuery.diagnostics.find((d) => d.code === "STORY_OPEN_PLACE")?.span;
assert(placeSpan?.start === Buffer.byteLength("Blå 🌨️ "), `UTF-8 query span ${JSON.stringify(placeSpan)}`);

const htmlEntity = analyzeStoryDraft({
  schemaVersion: 1,
  cast: [{ id: "hero", query: "<firstname female>" }],
  beats: ["Bilag 5 &#x2013; <::hero> signerte."],
});
const entityDiagnostic = htmlEntity.diagnostics.find((d) => d.code === "STORY_ENTITY");
assert(entityDiagnostic, `HTML entity diagnostic ${JSON.stringify(htmlEntity.diagnostics)}`);
assert(
  entityDiagnostic?.span?.start === Buffer.byteLength("Bilag 5 "),
  `HTML entity UTF-8 span ${JSON.stringify(entityDiagnostic?.span)}`,
);

const commentChar = analyzeStoryDraft({
  schemaVersion: 1,
  cast: [{ id: "hero", query: "<firstname female>" }],
  beats: ["Se bilag #006 for <::hero>."],
});
assert(
  commentChar.diagnostics.some((d) => d.code === "STORY_RESERVED_CHAR"),
  `Skald comment diagnostic ${JSON.stringify(commentChar.diagnostics)}`,
);

const unicodeMap = buildStoryPattern({
  schemaVersion: 1,
  cast: [{ id: "hero", query: "<firstname female>" }],
  beats: ["Blå 🌨️ vei.", "<::hero> ventet."],
});
const second = unicodeMap.sourceMap.beats[1];
const mapped = mapPatternSpan(unicodeMap.sourceMap, {
  start: second.start,
  end: second.start + Buffer.byteLength("<::hero>"),
});
assert(mapped.beatIndex === 1 && mapped.span.start === 0, `UTF-8 source map ${JSON.stringify(mapped)}`);

const innDraft = innDraftFrom(inn);
const innRender = renderStory({ explain }, { seed: 11, paletteIds: [] }, innDraft, {
  registry: PALETTES,
});
assert(innRender.ok, `inn render ${JSON.stringify(innRender.artifact.diagnostics)}`);
assert(innRender.artifact.cast.hero, "resolved hero");
assert(innRender.artifact.cast.other, "resolved other");
assert(innRender.artifact.cast.hero !== innRender.artifact.cast.other, "unique names");

const replay = renderStory({ explain }, { seed: 11, paletteIds: [] }, innDraft, {
  registry: PALETTES,
});
assert(replay.artifact.replayHash === innRender.artifact.replayHash, "replay hash");
assert(replay.artifact.text === innRender.artifact.text, "replay text");

const overlay = renderStory(
  { explain },
  { seed: 4, paletteIds: ["inn"] },
  {
    schemaVersion: 1,
    cast: [{ id: "hero", query: "<firstname female>" }],
    beats: ["<::hero> ordered <inn_drink>."],
  },
  { registry: PALETTES },
);
assert(overlay.ok, `overlay ${JSON.stringify(overlay.artifact.diagnostics)}`);
assert(
  !overlay.artifact.text.includes("<"),
  `overlay raw ${overlay.artifact.text}`,
);

const missingPal = renderStory(
  { explain },
  { seed: 1, paletteIds: ["nope"] },
  innDraft,
  { registry: PALETTES },
);
assert(!missingPal.ok, "unknown palette");

const badDraft = {
  schemaVersion: 1,
  cast: [{ id: "hero", query: "<firstname female>" }],
  beats: ["<::hero> <verb.ed> the <place>."],
};
const goodDraft = {
  schemaVersion: 1,
  cast: [{ id: "hero", query: "<firstname female>" }],
  beats: ["<::hero> the {knight|ranger} {walked|came} to the inn."],
};
const model = createMockModel({ bad: badDraft, good: goodDraft });
const loop = await runStoryLoop(
  { explain },
  { seed: 3, brief: "inn", paletteIds: [], policy: { maxRepairs: 2 } },
  model,
  { registry: PALETTES },
);
assert(loop.ok, `repair loop ${JSON.stringify(loop.artifact.diagnostics)}`);
assert(model.calls === 2, `model calls ${model.calls}`);

const stubborn = createMockModel({ bad: badDraft, good: badDraft });
const failLoop = await runStoryLoop(
  { explain },
  { seed: 3, brief: "inn", paletteIds: [], policy: { maxRepairs: 2 } },
  stubborn,
  { registry: PALETTES },
);
assert(!failLoop.ok, "stubborn draft should fail");
assert(failLoop.artifact.telemetry.modelCalls >= 2, "repair attempts recorded");

function skaldBin() {
  const debug = resolve(root, "target/debug/skald");
  const release = resolve(root, "target/release/skald");
  if (existsSync(debug)) return debug;
  return release;
}

function native(args) {
  return execFileSync(skaldBin(), args, { encoding: "utf8", cwd: root });
}

function nativePattern(path) {
  return native(["--seed", "11", "--case", "none", "-f", path]);
}

const dictOut = native([
  "--seed",
  "11",
  "--case",
  "none",
  "--dict",
  resolve(root, "docs/beats/data/inn.json"),
  "[case:none]<firstname female> ordered <inn_drink>.",
]);
assert(!dictOut.includes("<"), `cli dict ${dictOut}`);
assert(dictOut.includes("ordered"), `cli dict ${dictOut}`);

try {
  native(["--story", "--case", "none", dontDraft.beats[0]]);
  assert(false, "native --story dont should exit 2");
} catch (err) {
  assert(err.status === 2, `native story exit ${err.status}`);
}

const piped = spawnSync(
  skaldBin(),
  ["--story", "--case", "none"],
  { cwd: root, input: dontDraft.beats.join("\n"), encoding: "utf8" },
);
assert(piped.status === 2, `stdin story exit ${piped.status}`);

const tmp = resolve(here, ".dont.tmp.skald");
writeFileSync(tmp, dontDraft.beats[0]);
const fileRun = spawnSync(
  skaldBin(),
  ["--story", "--case", "none", "-f", tmp],
  { encoding: "utf8" },
);
unlinkSync(tmp);
assert(fileRun.status === 2, `file story exit ${fileRun.status}`);

const compiled = compile("<firstname male>", { seed: 1, case: "none" });
const compiledText = compiled.run({
  seed: 1,
  dictionary: { tables: {} },
  merge: false,
});
assert(
  compiledText && !compiledText.includes("<"),
  `compile ignores per-run dictionary: ${compiledText}`,
);

const changedBeats = {
  schemaVersion: 1,
  cast: innDraft.cast,
  beats: [...innDraft.beats.slice(0, -1), "<::hero> left without a word. <::other> stayed."],
};
const after = renderStory({ explain }, { seed: 11, paletteIds: [] }, changedBeats, {
  registry: PALETTES,
});
assert(after.ok, `changed beats ${JSON.stringify(after.artifact.diagnostics)}`);
assert(
  after.artifact.cast.hero === innRender.artifact.cast.hero,
  `prelude cast shifted: ${after.artifact.cast.hero} vs ${innRender.artifact.cast.hero}`,
);

const collideReg = {
  twin: {
    id: "twin",
    dictionary: {
      tables: {
        twin: {
          name: "twin",
          subs: ["default"],
          entries: [{ forms: ["Ada"], classes: [] }],
        },
      },
    },
  },
};
const collide = renderStory(
  { explain },
  {
    seed: 1,
    paletteIds: ["twin"],
    policy: { castNameRetries: 2 },
  },
  {
    schemaVersion: 1,
    cast: [
      { id: "a", query: "<twin>" },
      { id: "b", query: "<twin>" },
    ],
    beats: ["<::a> sat with <::b>."],
  },
  { registry: collideReg },
);
assert(!collide.ok, "name collision should fail");
assert(
  collide.artifact.diagnostics.some((d) => d.code === "STORY_CAST_NAME"),
  `collision codes ${JSON.stringify(collide.artifact.diagnostics)}`,
);

const replaceOnly = renderStory(
  { explain },
  { seed: 1, paletteIds: ["inn"], merge: false },
  {
    schemaVersion: 1,
    cast: [{ id: "hero", query: "<firstname female>" }],
    beats: ["<::hero> ordered <inn_drink>."],
  },
  { registry: PALETTES },
);
assert(!replaceOnly.ok, "replace-only should hide firstname");

const unboundSpan = renderStory(
  { explain },
  { seed: 1, paletteIds: [] },
  {
    schemaVersion: 1,
    cast: [{ id: "hero", query: "<firstname female>" }],
    beats: ["The road was dark.", "<::ghost> waited."],
  },
  { registry: PALETTES },
);
assert(!unboundSpan.ok, "ghost carrier should fail");
const ghost = unboundSpan.artifact.diagnostics.find((d) => d.code === "STORY_CARRIER");
assert(ghost?.beatIndex === 1, `ghost beatIndex ${ghost?.beatIndex}`);

for (const seed of [1, 2, 3, 5, 8, 11, 13, 17, 19, 23]) {
  const run = renderStory({ explain }, { seed, paletteIds: [] }, innDraft, {
    registry: PALETTES,
  });
  assert(run.ok, `seed ${seed} ${JSON.stringify(run.artifact.diagnostics)}`);
  assert(!run.artifact.text.includes("<"), `raw query seed ${seed}: ${run.artifact.text}`);
  assert(run.artifact.cast.hero, `empty hero seed ${seed}`);
  assert(run.artifact.cast.other, `empty other seed ${seed}`);
  assert(
    run.artifact.cast.hero !== run.artifact.cast.other,
    `duplicate names seed ${seed}`,
  );
  const again = renderStory({ explain }, { seed, paletteIds: [] }, innDraft, {
    registry: PALETTES,
  });
  assert(again.artifact.text === run.artifact.text, `replay seed ${seed}`);
  assert(again.artifact.replayHash === run.artifact.replayHash, `hash seed ${seed}`);
  assert(
    (run.artifact.choices ?? []).length > 0,
    `missing block choices seed ${seed}`,
  );
  const heroPick = (run.artifact.picks ?? []).find((p) => p.carrier === "hero");
  assert(heroPick, `missing hero pick seed ${seed}`);
}

const loopLocked = await runStoryLoop(
  { explain },
  { seed: 9, brief: "inn", paletteIds: ["inn"], policy: { maxRepairs: 2 } },
  createMockModel({ bad: badDraft, good: goodDraft }),
  { registry: PALETTES },
  { prompt: "canonical" },
);
assert(loopLocked.artifact.seed === 9, `seed mutated ${loopLocked.artifact.seed}`);
const loopCli = execFileSync(
  process.execPath,
  [resolve(here, "host.mjs"), "loop", "--brief", "Two travelers reach an inn.", "--mock"],
  { encoding: "utf8", cwd: root },
);
const loopDoc = JSON.parse(loopCli);
assert(loopDoc.ok, `host loop ${JSON.stringify(loopDoc.diagnostics)}`);
assert(loopDoc.seed === 11, `loop seed ${loopDoc.seed}`);
const missingProvider = spawnSync(
  process.execPath,
  [resolve(here, "host.mjs"), "loop", "--brief", "A story."],
  { encoding: "utf8", cwd: root },
);
assert(missingProvider.status === 1, "model loop should require explicit provider configuration");
assert(
  missingProvider.stderr.includes("--provider, --model, and --reasoning"),
  "AI loop should name all required provider flags",
);

const originalFetch = globalThis.fetch;
globalThis.fetch = async () => ({
  ok: true,
  async json() {
    return {
      choices: [{ message: { content: JSON.stringify(goodDraft) } }],
      usage: {
        prompt_tokens: 1_000,
        prompt_tokens_details: { cached_tokens: 200 },
        completion_tokens: 100,
        completion_tokens_details: { reasoning_tokens: 25 },
      },
    };
  },
});
const meteredModel = createOpenAIModel({
  apiKey: "test",
  model: "gpt-4.1",
  reviewModel: "gpt-4.1",
  maxModelCalls: 1,
});
await meteredModel.generate({ prompt: "test" });
const measured = meteredModel.getUsage();
assert(measured.requests === 1, `provider request usage ${JSON.stringify(measured)}`);
assert(measured.inputTokens === 1_000, `provider input usage ${JSON.stringify(measured)}`);
assert(measured.cachedInputTokens === 200, `provider cached usage ${JSON.stringify(measured)}`);
assert(measured.outputTokens === 100, `provider output usage ${JSON.stringify(measured)}`);
assert(measured.reasoningTokens === 25, `provider reasoning usage ${JSON.stringify(measured)}`);
assert(measured.estimatedCostUsd === 0.0025, `provider cost ${JSON.stringify(measured)}`);
let budgetStopped = false;
try {
  await meteredModel.generate({ prompt: "test again" });
} catch (error) {
  budgetStopped = String(error).includes("STORY_MODEL_BUDGET");
}
globalThis.fetch = originalFetch;
assert(budgetStopped, "provider should stop before a request beyond maxModelCalls");

globalThis.fetch = async () => ({
  ok: true,
  async json() {
    return {
      message: { content: JSON.stringify(goodDraft) },
      prompt_eval_count: 700,
      eval_count: 80,
    };
  },
});
const localModel = createOllamaModel({
  model: "local-test",
  reasoningEffort: "low",
  maxModelCalls: 1,
});
await localModel.generate({ prompt: "test" });
const localUsage = localModel.getUsage();
assert(localUsage.requests === 1, `local requests ${JSON.stringify(localUsage)}`);
assert(localUsage.inputTokens === 700, `local input usage ${JSON.stringify(localUsage)}`);
assert(localUsage.outputTokens === 80, `local output usage ${JSON.stringify(localUsage)}`);
assert(localUsage.estimatedCostUsd === 0, `local cost ${JSON.stringify(localUsage)}`);
globalThis.fetch = originalFetch;
assert(
  JSON.stringify(loopLocked.artifact.paletteIds) === JSON.stringify(["inn"]),
  `palette mutated ${loopLocked.artifact.paletteIds}`,
);

let receivedBrief;
let receivedTheme;
let receivedWritingStyle;
await runStoryLoop(
  { explain },
  {
    seed: 9,
    narrativeBrief: "municipal double-entry horror",
    theme: "serious administrative dread",
    writingStyle: "close procedural viewpoint with clipped marginal notes",
    paletteIds: [],
  },
  {
    async generate(args) {
      receivedBrief = args.narrativeBrief;
      receivedTheme = args.theme;
      receivedWritingStyle = args.writingStyle;
      return goodDraft;
    },
  },
  { registry: PALETTES },
  { prompt: "canonical" },
);
assert(receivedBrief === "municipal double-entry horror", `narrativeBrief ${receivedBrief}`);
assert(receivedTheme === "serious administrative dread", `theme ${receivedTheme}`);
assert(
  receivedWritingStyle === "close procedural viewpoint with clipped marginal notes",
  `writingStyle ${receivedWritingStyle}`,
);

const bindingPrompt = buildModelPrompt({
  prompt: "canonical",
  narrativeBrief: "Form: numbered audit work papers. No narrator.",
  writingStyle: "clipped documentary fragments",
});
assert(
  bindingPrompt.includes("<narrative-brief>\nForm: numbered audit work papers. No narrator.\n</narrative-brief>"),
  "narrativeBrief should be explicitly delimited",
);
assert(bindingPrompt.includes("<writing-style>clipped documentary fragments</writing-style>"), "writingStyle should be explicitly delimited");
assert(
  bindingPrompt.includes("creatively binding") &&
    bindingPrompt.includes("beats themselves must") &&
    bindingPrompt.includes("be entries in that artifact") &&
    bindingPrompt.includes("Do not summarize") &&
    bindingPrompt.includes("or explain the brief"),
  "model prompt should require formal and narrative realization",
);

const sameLength = expansionPlan("one two three four", 0);
const maxLength = expansionPlan("one two three four", 100);
assert(sameLength.permittedWords === 4, `expansion zero ${JSON.stringify(sameLength)}`);
assert(maxLength.permittedWords === 600, `expansion max ${JSON.stringify(maxLength)}`);

const longExpansionDraft = {
  schemaVersion: 1,
  cast: [{ id: "hero", query: "<firstname female>" }],
  beats: [`<::hero> ${"word ".repeat(700).trim()}.`],
};
const expansionFailure = await runStoryLoop(
  { explain },
  {
    seed: 9,
    narrativeBrief: "A tiny premise.",
    deviation: 20,
    expansion: 100,
    paletteIds: [],
    policy: { maxRepairs: 0, enforceExpansion: true },
  },
  createMockModel({ good: longExpansionDraft }),
  { registry: PALETTES },
  { prompt: "canonical" },
);
assert(!expansionFailure.ok, "opt-in expansion enforcement should apply its ceiling");
assert(
  expansionFailure.artifact.diagnostics.some((d) => d.code === "STORY_EXPANSION"),
  `expansion diagnostic ${JSON.stringify(expansionFailure.artifact.diagnostics)}`,
);
const expansionAllowed = await runStoryLoop(
  { explain },
  {
    seed: 9,
    narrativeBrief: "A tiny premise.",
    expansion: 100,
    paletteIds: [],
    policy: { maxRepairs: 0 },
  },
  createMockModel({ good: longExpansionDraft }),
  { registry: PALETTES },
);
assert(expansionAllowed.ok, "expansion scale should allow longer stories by default");

let invalidScaleThrew = false;
try {
  await runStoryLoop(
    { explain },
    { seed: 9, narrativeBrief: "x", deviation: 101, paletteIds: [] },
    createMockModel({ good: goodDraft }),
    { registry: PALETTES },
  );
} catch (error) {
  invalidScaleThrew = error instanceof RangeError;
}
assert(invalidScaleThrew, "out-of-range creative scale should throw");

const reviewPrompt = buildNarrativeReviewPrompt({
  narrativeBrief: "Numbered work papers. Margin notes become shorter.",
  draft: goodDraft,
});
assert(reviewPrompt.includes("STORY_FORM_DRIFT"), "review prompt should define stable codes");
assert(reviewPrompt.includes("mechanically uniform"), "review prompt should inspect rhythm");

let narrativeReviews = 0;
let receivedIntent;
let receivedRevisionPlan;
const reviewedLoop = await runStoryLoop(
  { explain },
  {
    seed: 9,
    narrativeBrief: "Numbered work papers. Margin notes become shorter.",
    paletteIds: [],
    policy: { maxRepairs: 2 },
  },
  {
    async plan() {
      return {
        anchors: ["numbered work papers"],
        development: ["one discrepancy changes responsibility"],
        comicMechanism: "",
        use: ["compression"],
        avoid: ["ordinary narration"],
        endingEffect: "cold finality",
      };
    },
    async generate(args) {
      receivedIntent = args.storyIntent;
      receivedRevisionPlan = args.revisionPlan ?? receivedRevisionPlan;
      return goodDraft;
    },
    async review(args) {
      narrativeReviews += 1;
      assert(
        args.narrativeBrief === "Numbered work papers. Margin notes become shorter.",
        "review should receive locked narrativeBrief",
      );
      return narrativeReviews === 1
        ? {
            ok: false,
            diagnostics: [{
              code: "STORY_FORM_DRIFT",
              beatIndex: 0,
              message: "The beat is ordinary narration, not a work-paper entry.",
              hint: "Write the beat as a numbered audit entry.",
            }],
            preserve: [0],
            replaceRanges: [],
          }
        : {
            ok: true,
            scores: {
              form: 2,
              identity: 2,
              development: 2,
              theme: 2,
              evidence: 2,
              causality: 2,
              ending: 2,
              rhythm: 2,
              restraint: 2,
            },
            diagnostics: [],
          };
    },
  },
  { registry: PALETTES },
  { prompt: "canonical" },
);
assert(reviewedLoop.ok, `reviewed loop ${JSON.stringify(reviewedLoop.artifact.diagnostics)}`);
assert(reviewedLoop.artifact.telemetry.reviewCalls === 2, "review count should be recorded");
assert(reviewedLoop.artifact.telemetry.planCalls === 1, "plan count should be recorded");
assert(reviewedLoop.artifact.telemetry.modelCalls === 5, "all model calls should be recorded");
assert(reviewedLoop.artifact.telemetry.repairAttempts === 1, "creative repair should be recorded");
assert(receivedIntent?.anchors?.[0] === "numbered work papers", "intent should reach drafting");
assert(receivedRevisionPlan?.preserve?.[0] === 0, "targeted revision should reach drafting");

const revisionDrift = revisionDiagnostics(
  { cast: [], beats: ["keep", "replace me", "also keep"] },
  { cast: [], beats: ["changed illegally", "replacement", "also keep"] },
  { preserve: [0, 2], replaceRanges: [{ start: 1, end: 1, goal: "repair" }] },
);
assert(
  revisionDrift.some((row) => row.code === "STORY_REVISION_DRIFT" && row.beatIndex === 0),
  "targeted revision should reject edits outside replacement ranges",
);
assert(
  !revisionDrift.some((row) => row.beatIndex === 1),
  "targeted revision should allow edits inside replacement ranges",
);
const castDrift = revisionDiagnostics(
  { cast: [], beats: ["keep"] },
  { cast: [{ id: "hero", query: "<firstname female>" }], beats: ["keep", "extra"] },
  { preserve: [0], replaceRanges: [] },
);
assert(castDrift.length === 2, "targeted revision should reject cast and beat-count changes");

const transformed = applySkaldTransform(
  { schemaVersion: 1, cast: [], beats: ["A porter opened the door.", "The porter waited."] },
  {
    cast: [{ id: "porter", query: "<firstname female>" }],
    substitutions: [
      { beatIndex: 0, literal: "A porter", pattern: "<::porter>" },
      { beatIndex: 1, literal: "The porter", pattern: "<::porter>" },
    ],
  },
);
assert(transformed.diagnostics.length === 0, "exact Skald substitutions should apply");
assert(
  transformed.draft.beats[0] === "<::porter> opened the door.",
  "Skald transform should change only the selected literal",
);
assert(
  transformed.draft.beats[1] === "<::porter> waited.",
  "Skald transform should reuse the selected carrier",
);

let stagedCompositions = 0;
const stagedLoop = await runStoryLoop(
  { explain },
  {
    seed: 17,
    narrativeBrief: "A coherent original story whose ending must be prepared.",
    paletteIds: [],
    policy: { maxRepairs: 1 },
  },
  {
    async design() {
      return {
        arc: "cause reaches consequence",
        movements: [{
          purpose: "setup",
          pressure: "the bell rings",
          choice: "the witness answers",
          cost: "the door closes",
          consequence: "knowledge changes",
        }],
        motifs: ["a bell"],
        rhythm: "varied",
        endingSetup: "the bell returns",
      };
    },
    async compose() {
      stagedCompositions += 1;
      return { text: stagedCompositions === 1 ? "First whole manuscript." : "Globally revised manuscript." };
    },
    async segment({ manuscript }) {
      assert(manuscript.text.includes("manuscript"), "segment should receive whole prose");
      return { schemaVersion: 1, cast: [], beats: [manuscript.text] };
    },
    async skaldize({ segmentedDraft }) {
      assert(segmentedDraft.beats[0].includes("manuscript"), "Skald pass should receive segmented prose");
      return { cast: [], substitutions: [] };
    },
    async review() {
      if (stagedCompositions === 1) {
        return {
          ok: false,
          scores: {
            form: 2, identity: 2, development: 2, theme: 2, evidence: 2,
            causality: 1, ending: 2, rhythm: 2, restraint: 2,
          },
          diagnostics: [{
            code: "STORY_CAUSAL_GAP",
            message: "The ending lacks setup across the arc.",
            hint: "Revise the whole manuscript to plant the bell.",
          }],
          revisionScope: "global",
          preserve: [],
          replaceRanges: [],
        };
      }
      return {
        ok: true,
        scores: Object.fromEntries([
          "form", "identity", "development", "theme", "evidence",
          "causality", "ending", "rhythm", "restraint",
        ].map((key) => [key, 2])),
        diagnostics: [],
      };
    },
  },
  { registry: PALETTES },
  { prompt: "canonical" },
);
assert(stagedLoop.ok, "staged composition pipeline should render after global revision");
assert(stagedCompositions === 2, "global review should return to whole-manuscript composition");
assert(stagedLoop.artifact.telemetry.globalRevisions === 1, "global revisions should be recorded");
assert(stagedLoop.artifact.telemetry.segmentCalls === 2, "each manuscript should be segmented");
assert(stagedLoop.artifact.telemetry.skaldizeCalls === 2, "Skald parametrization should run last");
assert(
  stagedLoop.artifact.manuscript?.text === "Globally revised manuscript.",
  "artifact should retain the reviewed whole manuscript",
);

let manuscriptComposes = 0;
let manuscriptSegments = 0;
let coverageReviews = 0;
const manuscriptGate = await runStoryLoop(
  { explain },
  {
    seed: 18,
    narrativeBrief: "Show the meaning through a consequential choice.",
    paletteIds: [],
    policy: { maxRepairs: 1, maxManuscriptRepairs: 1, narrativeReview: false },
  },
  {
    async plan() {
      return { requiredLiterals: ["Mara"] };
    },
    async compose() {
      manuscriptComposes += 1;
      return {
        text: manuscriptComposes === 1
          ? "This proved that friendship was strength."
          : "Mara gave Jo the only key and waited outside.",
      };
    },
    async reviewManuscript({ manuscript }) {
      if (manuscript.text.startsWith("This proved")) {
        return {
          ok: false,
          scores: { change: 1, causality: 1, sceneFunction: 1, dramatization: 0, prose: 1, ending: 1 },
          diagnostics: [{
            code: "STORY_THEME_EXPLAINED",
            excerpt: "This proved that friendship was strength.",
            message: "Narration states the theme.",
            hint: "Replace the thesis with a costly observable choice.",
          }],
        };
      }
      return {
        ok: true,
        scores: { change: 2, causality: 2, sceneFunction: 2, dramatization: 2, prose: 2, ending: 2 },
        diagnostics: [],
      };
    },
    async segment({ manuscript }) {
      manuscriptSegments += 1;
      return {
        schemaVersion: 1,
        cast: [],
        beats: [manuscriptSegments === 1 ? "The segmenter rewrote it." : manuscript.text],
      };
    },
    async skaldize() {
      return { cast: [], substitutions: [] };
    },
    async reviewSkaldization() {
      coverageReviews += 1;
      return coverageReviews === 1
        ? {
            ok: false,
            diagnostics: [{
              code: "STORY_SKALD_COVERAGE",
              beatIndex: 0,
              message: "The verb remains literal.",
              hint: "Add a grammatical closed block.",
            }],
          }
        : { ok: true, diagnostics: [] };
    },
  },
  { registry: PALETTES },
);
assert(manuscriptGate.ok, "manuscript gate should repair before technical stages");
assert(manuscriptComposes === 2, "failed literary review should trigger whole-prose revision");
assert(manuscriptSegments === 1, "rewritten segmentation should use deterministic host fallback");
assert(manuscriptGate.artifact.telemetry.manuscriptReviewCalls === 2, "manuscript reviews should be recorded");
assert(manuscriptGate.artifact.telemetry.manuscriptRepairs === 1, "manuscript repairs should be recorded");
assert(manuscriptGate.artifact.telemetry.skaldCoverageCalls === 2, "Skald coverage should be reviewed");
assert(manuscriptGate.artifact.telemetry.skaldizeRepairs === 1, "incomplete lexical coverage should retry");

const legacyBriefModel = {
  async generate(args) {
    receivedBrief = args.narrativeBrief;
    return goodDraft;
  },
};
const legacyBrief = await runStoryLoop(
  { explain },
  { seed: 9, brief: "legacy premise", paletteIds: [] },
  legacyBriefModel,
  { registry: PALETTES },
  { prompt: "canonical" },
);
assert(receivedBrief === "legacy premise", `legacy brief alias ${receivedBrief}`);
assert(
  legacyBrief.artifact.narrativeBrief === "legacy premise",
  `artifact narrativeBrief ${legacyBrief.artifact.narrativeBrief}`,
);

const drinkDoc = {
  schemaVersion: 1,
  seed: 1,
  paletteIds: ["inn"],
  draft: {
    schemaVersion: 1,
    cast: [{ id: "hero", "query": "<firstname female>" }],
    beats: ["<::hero> ordered <inn_drink>."],
  },
};
const drinkDir = mkdtempSync(resolve(tmpdir(), "skald-story-palette-"));
const drinkPath = resolve(drinkDir, "drink.json");
writeFileSync(drinkPath, JSON.stringify(drinkDoc));
const drinkCheck = spawnSync(
  process.execPath,
  [resolve(here, "host.mjs"), "check", drinkPath],
  { encoding: "utf8", cwd: root },
);
assert(drinkCheck.status === 0, `palette check exit ${drinkCheck.status} ${drinkCheck.stdout}`);
assert(!String(drinkCheck.stdout).includes("STORY_TABLE"), `palette check ${drinkCheck.stdout}`);
const drinkPattern = spawnSync(
  process.execPath,
  [resolve(here, "host.mjs"), "pattern", drinkPath],
  { encoding: "utf8", cwd: root },
);
assert(drinkPattern.status === 0, `palette pattern exit ${drinkPattern.status} ${drinkPattern.stdout}`);
assert(drinkPattern.stdout.includes("<inn_drink>"), `palette pattern ${drinkPattern.stdout}`);
const drinkRender = spawnSync(
  process.execPath,
  [resolve(here, "host.mjs"), "render", drinkPath],
  { encoding: "utf8", cwd: root },
);
assert(drinkRender.status === 0, `palette render exit ${drinkRender.status} ${drinkRender.stdout}`);
assert(!drinkRender.stdout.includes("<"), `palette render raw ${drinkRender.stdout}`);
const drinkBare = {
  ...drinkDoc,
  paletteIds: [],
};
writeFileSync(drinkPath, JSON.stringify(drinkBare));
const drinkBareCheck = spawnSync(
  process.execPath,
  [resolve(here, "host.mjs"), "check", drinkPath],
  { encoding: "utf8", cwd: root },
);
assert(drinkBareCheck.status === 2, "missing palette should fail check");
assert(String(drinkBareCheck.stdout).includes("STORY_TABLE"), `bare check ${drinkBareCheck.stdout}`);
rmSync(drinkDir, { recursive: true, force: true });

const paletteLoop = execFileSync(
  process.execPath,
  [
    resolve(here, "host.mjs"),
    "loop",
    "--brief",
    "Two travelers reach an inn.",
    "--mock",
    "--palette",
    "inn",
    "--seed",
    "9",
  ],
  { encoding: "utf8", cwd: root },
);
const paletteLoopDoc = JSON.parse(paletteLoop);
assert(
  JSON.stringify(paletteLoopDoc.paletteIds) === JSON.stringify(["inn"]),
  `loop --palette ${JSON.stringify(paletteLoopDoc.paletteIds)}`,
);
const unknownPaletteLoop = spawnSync(
  process.execPath,
  [
    resolve(here, "host.mjs"),
    "loop",
    "--brief",
    "Two travelers reach an inn.",
    "--mock",
    "--palette",
    "in",
  ],
  { encoding: "utf8", cwd: root },
);
assert(unknownPaletteLoop.status === 2, `unknown loop palette exit ${unknownPaletteLoop.status}`);
assert(
  String(unknownPaletteLoop.stdout).includes("STORY_PALETTE"),
  `unknown loop palette ${unknownPaletteLoop.stdout}`,
);

const envelopeNested = {
  schemaVersion: 1,
  seed: 11,
  paletteIds: ["inn"],
  narrativeBrief: "Two travelers reach an inn.",
  draft: {
    schemaVersion: 1,
    cast: [{ id: "hero", query: "<firstname female>" }],
    beats: ["<::hero> sat."],
  },
};
const envelopeOk = validateStoryEnvelope(envelopeNested);
assert(envelopeOk.ok, `envelope ${JSON.stringify(envelopeOk.diagnostics)}`);
const draftFromEnvelope = splitStoryDocument(envelopeNested).draft;
assert(validateStoryDraft(draftFromEnvelope).ok, "nested draft should validate");
assert(
  !validateStoryDraft(envelopeNested).ok,
  "envelope fields must not pass StoryDraft validation",
);
const draftSchema = JSON.parse(readFileSync(resolve(here, "story-draft.schema.json"), "utf8"));
const envelopeSchema = JSON.parse(readFileSync(resolve(here, "story.schema.json"), "utf8"));
assert(draftSchema.required.includes("cast") && draftSchema.required.includes("beats"), "draft schema requires cast/beats");
assert(!Object.prototype.hasOwnProperty.call(draftSchema.properties, "seed"), "draft schema must omit seed");
assert(!Object.prototype.hasOwnProperty.call(draftSchema.properties, "paletteIds"), "draft schema must omit paletteIds");
assert(Object.prototype.hasOwnProperty.call(envelopeSchema.properties, "seed"), "envelope schema has seed");
assert(Object.prototype.hasOwnProperty.call(envelopeSchema.properties, "paletteIds"), "envelope schema has paletteIds");
assert(Object.prototype.hasOwnProperty.call(envelopeSchema.properties, "draft"), "envelope schema has nested draft");
assert(!(envelopeSchema.required ?? []).includes("cast"), "envelope must not require top-level cast");
assert(
  (envelopeSchema.oneOf ?? []).some((option) => (option.required ?? []).includes("draft")),
  "envelope schema requires a nested draft or legacy payload",
);
assert(
  !validateStoryEnvelope({ schemaVersion: 1 }).ok,
  "empty envelope without draft should fail",
);
assert(
  validateStoryEnvelope({
    schemaVersion: 1,
    theme: 42,
    draft: { schemaVersion: 1, cast: [], beats: ["Hi."] },
  }).diagnostics.some((row) => row.message.includes("theme")),
  "typed envelope fields should be validated",
);
assert(
  validateStoryEnvelope({
    schemaVersion: 1,
    seed: 1.5,
    draft: { schemaVersion: 1, cast: [], beats: ["Hi."] },
  }).diagnostics.some((row) => row.message.includes("seed")),
  "fractional seed should fail envelope validation",
);
assert(
  validateStoryEnvelope({
    schemaVersion: 1,
    merge: "yes",
    policy: "bad",
    draft: { schemaVersion: 1, cast: [], beats: ["Hi."] },
  }).diagnostics.length >= 2,
  "merge and policy types should fail independently",
);

const inspectedInn = inspectStoryDocument(inn, PALETTES);
assert(inspectedInn.ok, `inn inspect ${JSON.stringify(inspectedInn.diagnostics)}`);
const innCheck = spawnSync(
  process.execPath,
  [resolve(here, "host.mjs"), "check", resolve(here, "inn.json")],
  { encoding: "utf8", cwd: root },
);
const innCheckDoc = JSON.parse(innCheck.stdout);
const innCheckKeys = innCheckDoc.diagnostics.map((row) => diagnosticKey(row));
assert(innCheckKeys.length === new Set(innCheckKeys).size, "check diagnostics should not duplicate");

const duplicated = createStoryArtifact(
  { seed: 1, paletteIds: [] },
  { schemaVersion: 1, cast: [{ id: "hero", query: "<firstname female>" }], beats: ["<::hero> sat."] },
  {
    text: "",
    diagnostics: [diagnostic("STORY_TABLE", "query table 'inn_drink' is not allowed", {
      beatIndex: 0,
      span: { start: 10, end: 21 },
    })],
  },
  {
    diagnostics: [diagnostic("STORY_TABLE", "query table 'inn_drink' is not allowed", {
      beatIndex: 0,
      span: { start: 10, end: 21 },
    })],
  },
);
assert(duplicated.diagnostics.length === 1, `deduped diagnostics ${duplicated.diagnostics.length}`);
const missingTable = renderStory(
  { explain },
  { seed: 1, paletteIds: [] },
  {
    schemaVersion: 1,
    cast: [{ id: "hero", query: "<firstname female>" }],
    beats: ["<::hero> ordered <inn_drink>."],
  },
  { registry: PALETTES },
);
const tableKeys = missingTable.artifact.diagnostics.map((row) => diagnosticKey(row));
assert(tableKeys.length === new Set(tableKeys).size, `render diagnostics duplicated ${tableKeys}`);
assert(
  missingTable.artifact.diagnostics.filter((row) => row.code === "STORY_TABLE").length === 1,
  "STORY_TABLE should appear once",
);

const dialogueMs = {
  text: '"Stay," she said.\n\n  He did not.\n',
};
const sliced = deterministicSegment(dialogueMs);
assert(sliced, "dialogue manuscript should segment");
assert(sliced.beats.join("") === dialogueMs.text, `slice join ${JSON.stringify(sliced.beats)}`);
assert(joinStoryBeats(sliced.beats) === dialogueMs.text, "joinStoryBeats should reconstruct manuscript");
const literalRender = renderStory(
  { explain },
  { seed: 1, paletteIds: [] },
  sliced,
  { registry: PALETTES },
);
assert(literalRender.ok, `literal whitespace render ${JSON.stringify(literalRender.artifact.diagnostics)}`);
assert(
  literalRender.artifact.text === dialogueMs.text,
  `whitespace invariant\n${JSON.stringify(literalRender.artifact.text)}\n${JSON.stringify(dialogueMs.text)}`,
);
const mrEgg = deterministicSegment({ text: "Mr. Egg woke at 6:15. Mrs. Pike waited." });
assert(mrEgg?.beats.length === 2, `Mr. title split ${JSON.stringify(mrEgg?.beats)}`);
assert(joinStoryBeats(mrEgg.beats) === "Mr. Egg woke at 6:15. Mrs. Pike waited.", "title-aware slices");

let localComposes = 0;
let localRevises = 0;
let localSegments = 0;
const localBeats = [
  "Keep the opening intact. ",
  "Replace this middle sentence. ",
  "Keep the ending intact.",
];
const localLoop = await runStoryLoop(
  { explain },
  {
    seed: 21,
    narrativeBrief: "A three-beat story with a local repair.",
    paletteIds: [],
    policy: { maxRepairs: 1, manuscriptReview: false, skaldCoverageReview: false },
  },
  {
    async compose() {
      localComposes += 1;
      return { text: localBeats.join("") };
    },
    async segment({ manuscript }) {
      localSegments += 1;
      assert(manuscript.text === localBeats.join(""), "local segment must see the original manuscript");
      return { schemaVersion: 1, cast: [], beats: [...localBeats] };
    },
    async skaldize() {
      return { cast: [], substitutions: [] };
    },
    async review() {
      if (localRevises === 0) {
        return {
          ok: false,
          scores: {
            form: 1, identity: 2, development: 2, theme: 2, evidence: 2,
            causality: 2, ending: 2, rhythm: 2, restraint: 2,
          },
          diagnostics: [{
            code: "STORY_FORM_DRIFT",
            beatIndex: 1,
            message: "The middle beat is inert.",
            hint: "Replace only the middle beat.",
          }],
          revisionScope: "local",
          preserve: [0, 2],
          replaceRanges: [{ start: 1, end: 1, goal: "Replace the middle beat." }],
        };
      }
      return {
        ok: true,
        scores: Object.fromEntries([
          "form", "identity", "development", "theme", "evidence",
          "causality", "ending", "rhythm", "restraint",
        ].map((key) => [key, 2])),
        diagnostics: [],
      };
    },
    async revise(args) {
      localRevises += 1;
      assert(args.revisionPlan?.scope === "local", `local revise plan ${JSON.stringify(args.revisionPlan)}`);
      const next = structuredClone(args.failingDraft);
      next.beats[1] = "The middle beat now moves. ";
      return next;
    },
  },
  { registry: PALETTES },
  { prompt: "canonical" },
);
assert(localLoop.ok, `local staged loop ${JSON.stringify(localLoop.artifact.diagnostics)}`);
assert(localComposes === 1, `local compose count ${localComposes}`);
assert(localRevises === 1, `local revise count ${localRevises}`);
assert(localSegments === 1, `local segment count ${localSegments}`);
assert(localLoop.artifact.telemetry.globalRevisions === 0, "local repair must not count as global");
assert(localLoop.artifact.telemetry.localRevisions === 1, "local revisions should be recorded");
assert(localLoop.artifact.draft.beats[0] === localBeats[0], "frozen opening beat");
assert(localLoop.artifact.draft.beats[2] === localBeats[2], "frozen ending beat");
assert(localLoop.artifact.draft.beats[1] === "The middle beat now moves. ", "local beat should change");
assert(
  localLoop.artifact.manuscript?.text === localBeats.join(""),
  "local repair must keep the original manuscript",
);

let driftedComposes = 0;
const driftedLocal = await runStoryLoop(
  { explain },
  {
    seed: 22,
    narrativeBrief: "A three-beat story with illegal local edits.",
    paletteIds: [],
    policy: { maxRepairs: 1, manuscriptReview: false, skaldCoverageReview: false },
  },
  {
    async compose() {
      driftedComposes += 1;
      return { text: localBeats.join("") };
    },
    async segment() {
      return { schemaVersion: 1, cast: [], beats: [...localBeats] };
    },
    async skaldize() {
      return { cast: [], substitutions: [] };
    },
    async review() {
      return {
        ok: false,
        scores: {
          form: 1, identity: 2, development: 2, theme: 2, evidence: 2,
          causality: 2, ending: 2, rhythm: 2, restraint: 2,
        },
        diagnostics: [{
          code: "STORY_FORM_DRIFT",
          beatIndex: 1,
          message: "The middle beat is inert.",
        }],
        revisionScope: "local",
        preserve: [0, 2],
        replaceRanges: [{ start: 1, end: 1, goal: "Replace the middle beat." }],
      };
    },
    async revise(args) {
      const next = structuredClone(args.failingDraft);
      next.beats[0] = "Illegally rewritten opening. ";
      next.beats[1] = "The middle beat now moves. ";
      return next;
    },
  },
  { registry: PALETTES },
);
assert(!driftedLocal.ok, "illegal local edits should fail");
assert(driftedComposes === 1, `drift compose count ${driftedComposes}`);
assert(
  driftedLocal.artifact.diagnostics.some((row) => row.code === "STORY_REVISION_DRIFT" && row.beatIndex === 0),
  `local drift ${JSON.stringify(driftedLocal.artifact.diagnostics)}`,
);

let localCoverageComposes = 0;
let localCoverageReviews = 0;
const coveredLocalBeats = [
  "Keep the {opening|start} intact. ",
  "Replace this {middle|center} sentence. ",
  "Keep the {ending|close} intact.",
];
const localCoverageLoop = await runStoryLoop(
  { explain },
  {
    seed: 23,
    narrativeBrief: "A three-beat story whose local repair must stay parametrized.",
    paletteIds: [],
    policy: { maxRepairs: 1, manuscriptReview: false },
  },
  {
    async compose() {
      localCoverageComposes += 1;
      return { text: coveredLocalBeats.join("") };
    },
    async segment() {
      return { schemaVersion: 1, cast: [], beats: [...coveredLocalBeats] };
    },
    async skaldize() {
      return { cast: [], substitutions: [] };
    },
    async reviewSkaldization({ draft }) {
      localCoverageReviews += 1;
      const literal = (draft.beats ?? []).some((beat, index) =>
        index === 1 && !beat.includes("{") && !beat.includes("<"),
      );
      if (literal) {
        return {
          ok: false,
          diagnostics: [{
            code: "STORY_SKALD_COVERAGE",
            beatIndex: 1,
            message: "The revised middle beat is entirely literal.",
          }],
        };
      }
      return { ok: true, diagnostics: [] };
    },
    async review() {
      if (localCoverageReviews < 2) {
        return {
          ok: false,
          scores: {
            form: 1, identity: 2, development: 2, theme: 2, evidence: 2,
            causality: 2, ending: 2, rhythm: 2, restraint: 2,
          },
          diagnostics: [{
            code: "STORY_FORM_DRIFT",
            beatIndex: 1,
            message: "The middle beat is inert.",
          }],
          revisionScope: "local",
          preserve: [0, 2],
          replaceRanges: [{ start: 1, end: 1, goal: "Replace the middle beat." }],
        };
      }
      return {
        ok: true,
        scores: Object.fromEntries([
          "form", "identity", "development", "theme", "evidence",
          "causality", "ending", "rhythm", "restraint",
        ].map((key) => [key, 2])),
        diagnostics: [],
      };
    },
    async revise(args) {
      const next = structuredClone(args.failingDraft);
      next.beats[1] = "The middle beat is now plain prose. ";
      return next;
    },
  },
  { registry: PALETTES },
);
assert(!localCoverageLoop.ok, "local literal revision should fail coverage");
assert(localCoverageComposes === 1, `coverage compose count ${localCoverageComposes}`);
assert(localCoverageReviews >= 2, `coverage after local ${localCoverageReviews}`);
assert(
  localCoverageLoop.artifact.diagnostics.some((row) => row.code === "STORY_SKALD_COVERAGE"),
  `local coverage ${JSON.stringify(localCoverageLoop.artifact.diagnostics)}`,
);

const selectivePrompt = buildSkaldizePrompt({
  manuscript: { text: "Mara opened the door." },
  segmentedDraft: { schemaVersion: 1, cast: [], beats: ["Mara opened the door."] },
});
assert(
  selectivePrompt.includes("selective parametrization"),
  "default skaldize prompt should be selective",
);
assert(
  !selectivePrompt.includes("Parametrize every eligible content-word"),
  "default skaldize prompt must not demand full lexical coverage",
);
const fullPrompt = buildSkaldizePrompt({
  manuscript: { text: "Mara opened the door." },
  segmentedDraft: { schemaVersion: 1, cast: [], beats: ["Mara opened the door."] },
  policy: { fullLexicalCoverage: true },
});
assert(
  fullPrompt.includes("Parametrize every eligible content-word"),
  "fullLexicalCoverage should restore the old skaldize contract",
);
const selectiveCoverage = buildSkaldCoveragePrompt({
  segmentedDraft: { schemaVersion: 1, cast: [], beats: ["Mara opened the door."] },
  transform: { cast: [], substitutions: [] },
  draft: { schemaVersion: 1, cast: [], beats: ["Mara opened the door."] },
});
assert(
  selectiveCoverage.includes("STORY_SKALD_OVERREACH"),
  "default coverage audit should reject frozen-word substitutions",
);
assert(
  selectiveCoverage.includes("Do not fail merely because a plot verb"),
  "default coverage audit must allow literal plot verbs",
);

const overreach = variationDiagnostics(
  { schemaVersion: 1, cast: [], beats: ["Mara opened the door."] },
  { schemaVersion: 1, cast: [], beats: ["<::hero> opened the door."] },
  {},
  { requiredLiterals: ["Mara"] },
);
assert(
  overreach.some((row) => row.code === "STORY_SKALD_OVERREACH" && row.message.includes("Mara")),
  `required literal overreach ${JSON.stringify(overreach)}`,
);

const repeatedBlocks = {
  schemaVersion: 1,
  cast: [],
  beats: ["She ordered {ale|stew|bread}.", "He ordered {ale|stew|bread}."],
};
const synced = syncRepeatedChoices(repeatedBlocks.beats);
assert(synced.synced === 1, `synced groups ${synced.synced}`);
assert(
  synced.beats.every((beat) => beat.includes("[sync:choice1;locked]{ale|stew|bread}")),
  `synced beats ${JSON.stringify(synced.beats)}`,
);
const syncedPattern = buildStoryPattern(repeatedBlocks);
assert(
  (syncedPattern.pattern.match(/\[sync:choice1;locked\]\{ale\|stew\|bread\}/g) || []).length === 2,
  `compiled sync pattern ${syncedPattern.pattern}`,
);
for (const seed of [1, 2, 3, 5, 8, 11]) {
  const run = renderStory({ explain }, { seed, paletteIds: [] }, repeatedBlocks, {
    registry: PALETTES,
  });
  assert(run.ok, `synced render seed ${seed} ${JSON.stringify(run.artifact.diagnostics)}`);
  const drinks = (run.artifact.text.match(/ordered (\w+)/g) ?? []).map((row) => row.split(" ")[1]);
  assert(drinks.length === 2 && drinks[0] === drinks[1], `desynced drinks seed ${seed}: ${run.artifact.text}`);
}

let defaultSkaldPrompt;
const selectiveLoop = await runStoryLoop(
  { explain },
  {
    seed: 24,
    narrativeBrief: "A porter waits at a door.",
    paletteIds: [],
    policy: { maxRepairs: 0, narrativeReview: false, skaldCoverageReview: false },
  },
  {
    async compose() {
      return { text: "A porter waited." };
    },
    async segment({ manuscript }) {
      return { schemaVersion: 1, cast: [], beats: [manuscript.text] };
    },
    async skaldize({ prompt }) {
      defaultSkaldPrompt = prompt;
      return { cast: [], substitutions: [] };
    },
  },
  { registry: PALETTES },
);
assert(selectiveLoop.ok, `selective loop ${JSON.stringify(selectiveLoop.artifact.diagnostics)}`);
assert(
  defaultSkaldPrompt.includes("selective parametrization"),
  "staged skaldize should receive the selective prompt by default",
);

let fullSkaldPrompt;
await runStoryLoop(
  { explain },
  {
    seed: 25,
    narrativeBrief: "A porter waits at a door.",
    paletteIds: [],
    policy: {
      maxRepairs: 0,
      narrativeReview: false,
      skaldCoverageReview: false,
      fullLexicalCoverage: true,
    },
  },
  {
    async compose() {
      return { text: "A porter waited." };
    },
    async segment({ manuscript }) {
      return { schemaVersion: 1, cast: [], beats: [manuscript.text] };
    },
    async skaldize({ prompt }) {
      fullSkaldPrompt = prompt;
      return { cast: [], substitutions: [] };
    },
  },
  { registry: PALETTES },
);
assert(
  fullSkaldPrompt.includes("Parametrize every eligible content-word"),
  "policy.fullLexicalCoverage should reach the skaldize prompt",
);

const fullFlag = spawnSync(
  process.execPath,
  [
    resolve(here, "host.mjs"),
    "loop",
    "--brief",
    "Two travelers reach an inn.",
    "--mock",
    "--full-lexical-coverage",
  ],
  { encoding: "utf8", cwd: root },
);
assert(fullFlag.status === 0, `full lexical coverage flag exit ${fullFlag.status} ${fullFlag.stderr}`);
const fullFlagDoc = JSON.parse(fullFlag.stdout);
assert(
  fullFlagDoc.policy?.fullLexicalCoverage === true,
  `loop flag should lock fullLexicalCoverage ${JSON.stringify(fullFlagDoc.policy)}`,
);

if (failed) {
  console.error(`${failed} story tests failed`);
  process.exit(1);
}
console.log("story runner tests ok");
