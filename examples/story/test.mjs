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
  renderStory,
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

if (failed) {
  console.error(`${failed} story tests failed`);
  process.exit(1);
}
console.log("story runner tests ok");
