#!/usr/bin/env node
import { execFileSync, spawnSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { compile, explain } from "../../packages/skald-lang/index.js";
import { createMockModel } from "./mock-model.mjs";
import { PALETTES } from "./palettes.mjs";
import {
  analyzeStoryDraft,
  applySkaldTransform,
  buildModelPrompt,
  buildNarrativeReviewPrompt,
  buildStoryPattern,
  expansionPlan,
  mapPatternSpan,
  renderStory,
  revisionDiagnostics,
  runStoryLoop,
  validateStoryDraft,
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

const innOut = runHost(["render", resolve(here, "inn.json")]);
assert(innOut === golden("inn", 11), `inn golden mismatch\n${innOut}`);

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

const innDraft = { schemaVersion: 1, cast: inn.cast, beats: inn.beats };
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
  native(["--story", "--case", "none", dont.beats[0]]);
  assert(false, "native --story dont should exit 2");
} catch (err) {
  assert(err.status === 2, `native story exit ${err.status}`);
}

const piped = spawnSync(
  skaldBin(),
  ["--story", "--case", "none"],
  { cwd: root, input: dont.beats.join("\n"), encoding: "utf8" },
);
assert(piped.status === 2, `stdin story exit ${piped.status}`);

const tmp = resolve(here, ".dont.tmp.skald");
const { writeFileSync, unlinkSync } = await import("node:fs");
writeFileSync(tmp, dont.beats[0]);
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
  cast: inn.cast,
  beats: [...inn.beats.slice(0, -1), "<::hero> left without a word. <::other> stayed."],
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
assert(
  JSON.stringify(loopLocked.artifact.paletteIds) === JSON.stringify(["inn"]),
  `palette mutated ${loopLocked.artifact.paletteIds}`,
);

let receivedBrief;
let receivedTheme;
await runStoryLoop(
  { explain },
  {
    seed: 9,
    narrativeBrief: "municipal double-entry horror",
    theme: "serious administrative dread",
    paletteIds: [],
  },
  {
    async generate(args) {
      receivedBrief = args.narrativeBrief;
      receivedTheme = args.theme;
      return goodDraft;
    },
  },
  { registry: PALETTES },
  { prompt: "canonical" },
);
assert(receivedBrief === "municipal double-entry horror", `narrativeBrief ${receivedBrief}`);
assert(receivedTheme === "serious administrative dread", `theme ${receivedTheme}`);

const bindingPrompt = buildModelPrompt({
  prompt: "canonical",
  narrativeBrief: "Form: numbered audit work papers. No narrator.",
});
assert(
  bindingPrompt.includes("<narrative-brief>\nForm: numbered audit work papers. No narrator.\n</narrative-brief>"),
  "narrativeBrief should be explicitly delimited",
);
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

const expansionFailure = await runStoryLoop(
  { explain },
  {
    seed: 9,
    narrativeBrief: "A tiny premise.",
    deviation: 20,
    expansion: 100,
    paletteIds: [],
    policy: { maxRepairs: 0 },
  },
  createMockModel({
    good: {
      schemaVersion: 1,
      cast: [{ id: "hero", query: "<firstname female>" }],
      beats: [`<::hero> ${"word ".repeat(700).trim()}.`],
    },
  }),
  { registry: PALETTES },
  { prompt: "canonical" },
);
assert(!expansionFailure.ok, "explicit expansion should enforce its safety ceiling");
assert(
  expansionFailure.artifact.diagnostics.some((d) => d.code === "STORY_EXPANSION"),
  `expansion diagnostic ${JSON.stringify(expansionFailure.artifact.diagnostics)}`,
);

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
const stagedLiteralDraft = {
  schemaVersion: 1,
  cast: [],
  beats: ["A whole story happened."],
};
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
        movements: [{ purpose: "setup", change: "knowledge", consequence: "choice" }],
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
      return stagedLiteralDraft;
    },
    async skaldize({ segmentedDraft }) {
      assert(segmentedDraft === stagedLiteralDraft, "Skald pass should receive segmented prose");
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

if (failed) {
  console.error(`${failed} story tests failed`);
  process.exit(1);
}
console.log("story runner tests ok");
