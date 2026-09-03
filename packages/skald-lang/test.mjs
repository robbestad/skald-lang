import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { skald, compile, explain } from "./index.js";

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

if (failed) {
  console.error(`${failed} failed`);
  process.exit(1);
}
console.log(`${cases.length} native==wasm goldens ok`);
