import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { skald, compile, explain, output, canonicalSeed, RUN_PROFILE } from "./index.js";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");

function native(pattern, seed) {
  return execFileSync(
    "cargo",
    [
      "run",
      "-p",
      "skald",
      "--bin",
      "skald",
      "--quiet",
      "--",
      "--seed",
      String(seed),
      "--case",
      "none",
      pattern,
    ],
    { encoding: "utf8", cwd: root },
  ).replace(/\n$/, "");
}

const cases = [
  ["{A|B|C|D}", 42],
  ["[rep:3]{x}", 1],
  ["[sep:\\s][rep:3]{x}", 1],
  ["[numfmt:verbal][rs:3;.]{[rn]}", 1],
  ["[a]ogre", 1],
  ["<firstname male> found [a] <noun-animal>.", 42],
  [
    "<firstname male> likes to <verb-transitive> <noun.plural> with <pron poss male> pet <noun-animal> on <timenoun dayofweek plural>.",
    7,
  ],
  ["<firstname male :: hero> and <::hero>", 11],
  ["[rhyme:perfect]<noun ::~a> / <noun ::~a>", 4],
  ["[x:s;ping][rep:5]{A|B|C}", 1],
  ["[numfmt:roman][n:14;14]", 1],
  ["[replace: hello world; /world/; {earth}]", 1],
  ["[out:title]{[case:title]hello world}[case:none]body", 1],
  [
    "[let:row; [map: who; Ada; what; hedgehog]][let:tpl; {[who] found [a] [what]}][tpl: row]",
    1,
  ],
];

let failed = 0;
for (const [pattern, seed] of cases) {
  const a = native(pattern, seed);
  const b = skald(pattern, { seed, case: "none" });
  if (a !== b) {
    console.error("mismatch");
    console.error(" pattern:", pattern);
    console.error(" native: ", JSON.stringify(a));
    console.error(" wasm:   ", JSON.stringify(b));
    failed += 1;
  }
}

const compiled = compile("<firstname male>", { seed: 9, case: "none" });
const once = compiled.run();
const twice = compiled.run({ seed: 9 });
if (once !== twice || once !== skald("<firstname male>", { seed: 9, case: "none" })) {
  console.error("compile().run mismatch", once, twice);
  failed += 1;
}

const explained = explain("<firstname male :: hero> and <::hero>", {
  seed: 11,
  case: "none",
});
const nativeExplain = execFileSync(
  "cargo",
  [
    "run",
    "-p",
    "skald",
    "--bin",
    "skald",
    "--quiet",
    "--",
    "--seed",
    "11",
    "--case",
    "none",
    "--explain",
    "<firstname male :: hero> and <::hero>",
  ],
  { encoding: "utf8", cwd: root },
).replace(/\n$/, "");
if (JSON.stringify(explained) !== nativeExplain) {
  console.error("explain mismatch");
  console.error(" wasm:   ", JSON.stringify(explained));
  console.error(" native: ", nativeExplain);
  failed += 1;
} else if (explained.picks.length !== 1 || explained.picks[0].table !== "firstname") {
  console.error("explain picks", explained.picks);
  failed += 1;
}

const cliOut = execFileSync(
  process.execPath,
  [
    fileURLToPath(new URL("./cli.mjs", import.meta.url)),
    "--seed",
    "42",
    "--case",
    "none",
    "<firstname male> found [a] <noun-animal>.",
  ],
  { encoding: "utf8" },
).replace(/\n$/, "");
const fromApi = skald("<firstname male> found [a] <noun-animal>.", {
  seed: 42,
  case: "none",
});
if (cliOut !== fromApi) {
  console.error("cli mismatch", JSON.stringify(cliOut), JSON.stringify(fromApi));
  failed += 1;
}

const dont =
  "<firstname female :: hero>, [a] <adj> <noun-job>, <verb.ed> toward the <place>.";
const storyWasm = explain(dont, { seed: 1, case: "none", story: true });
if (!storyWasm.notes?.some((n) => n.startsWith("story:"))) {
  console.error("story lint missing notes", storyWasm.notes);
  failed += 1;
}
const inn =
  "<firstname female :: hero> the {knight|ranger} {walked|came} to the inn.";
if (explain(inn, { seed: 1, case: "none", story: true }).notes?.some((n) => n.startsWith("story:"))) {
  console.error("inn story should be lint-clean");
  failed += 1;
}

const overlayOut = skald("[case:none]<firstname female> ordered <inn_drink>.", {
  seed: 11,
  case: "none",
  dictionary: {
    tables: {
      inn_drink: {
        name: "inn_drink",
        subs: ["default"],
        entries: [{ forms: ["ale"], classes: [] }],
      },
    },
  },
});
if (overlayOut.includes("<") || !overlayOut.includes("ordered")) {
  console.error("overlay merge failed", overlayOut);
  failed += 1;
}

const explainedStory = explain(dont, { seed: 1, case: "none", story: true });
const outputStory = output(dont, { seed: 1, case: "none", story: true });
if (!explainedStory.diagnostics?.length || !outputStory.diagnostics?.length) {
  console.error("story diagnostics missing on explain/output");
  failed += 1;
}

const large = "9007199254740993";
const largeNative = native("{A|B|C|D|E|F|G|H}", large);
const largeWasm = skald("{A|B|C|D|E|F|G|H}", { seed: large, case: "none" });
const largeCli = execFileSync(
  process.execPath,
  [resolve(root, "packages/skald-lang/cli.mjs"), "--seed", large, "--case", "none", "{A|B|C|D|E|F|G|H}"],
  { encoding: "utf8", cwd: root },
).replace(/\n$/, "");
if (largeNative !== largeWasm || largeWasm !== largeCli) {
  console.error("large seed mismatch", { largeNative, largeWasm, largeCli });
  failed += 1;
}
if (canonicalSeed(42) !== "42" || canonicalSeed(large) !== large) {
  console.error("canonicalSeed mismatch", canonicalSeed(42), canonicalSeed(large));
  failed += 1;
}
if (canonicalSeed({ type: "text", value: "42" }) !== "text:42") {
  console.error("explicit text seed encoding failed", canonicalSeed({ type: "text", value: "42" }));
  failed += 1;
}
if (skald("{A|B|C|D|E|F|G|H}", { seed: 42, case: "none" }) === skald("{A|B|C|D|E|F|G|H}", { seed: { type: "text", value: "42" }, case: "none" })) {
  console.error("integer 42 and text:42 should not collide");
  failed += 1;
}
let threw = false;
try {
  canonicalSeed({ type: "u64", value: "18446744073709551616" });
} catch {
  threw = true;
}
if (!threw) {
  console.error("overflowing u64 object seed should be rejected");
  failed += 1;
}
const browserSrc = readFileSync(resolve(root, "packages/skald-lang/browser.js"), "utf8");
if (!browserSrc.includes("canonicalSeed") || !browserSrc.includes("RUN_PROFILE")) {
  console.error("browser entry must export canonicalSeed and RUN_PROFILE");
  failed += 1;
}
threw = false;
try {
  canonicalSeed(9007199254740993);
} catch {
  threw = true;
}
if (!threw) {
  console.error("unsafe JS number should be rejected");
  failed += 1;
}
threw = false;
try {
  skald("x", { seed: "042", case: "none" });
} catch {
  threw = true;
}
if (!threw) {
  console.error("leading-zero seed should be rejected");
  failed += 1;
}
if (RUN_PROFILE !== "skald-pcg32-v1") {
  console.error("run profile", RUN_PROFILE);
  failed += 1;
}
threw = false;
try {
  skald("Ada", { locale: "nb-NO", case: "none" });
} catch (err) {
  threw = String(err).includes("missing language pack");
}
if (!threw) {
  console.error("nb-NO without a pack should be a missing language pack error");
  failed += 1;
}
const nbPack = {
  formatVersion: 1,
  id: "test-nb",
  locale: "nb-NO",
  contentVersion: "0.0.1",
  capabilities: { articles: "none", numbersVerbal: "none", caseTitle: "none", rhyme: false },
  tables: {
    firstname: {
      name: "firstname",
      subs: ["default"],
      entries: [{ id: "fn-ada", forms: ["Ada"], classes: ["female"] }],
    },
  },
};
const nbLine = skald("<firstname female>", { languagePack: nbPack, locale: "nb-NO", seed: 1, case: "none" });
if (!nbLine.includes("Ada")) {
  console.error("language pack should supply Norwegian-pack entries", nbLine);
  failed += 1;
}

if (failed) {
  console.error(`${failed} failed`);
  process.exit(1);
}
console.log(`${cases.length} native==wasm goldens ok`);
