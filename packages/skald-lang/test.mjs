import { execFileSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";
import { dirname, join, resolve } from "node:path";
import { skald, compile, explain, output, preflight, dictionaryJson, canonicalSeed, RUN_PROFILE } from "./index.js";
import { fileHash, manifestForPattern, patternHash, sha256Hex } from "./artifact.mjs";

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
if (browserSrc.includes("fetch(")) {
  console.error("browser entry must not fetch the English dictionary");
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
if (sha256Hex(Buffer.from("hello")) !== "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824") {
  console.error("npm sha256 mismatch");
  failed += 1;
}
if (patternHash("hello") !== "sha256:2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824") {
  console.error("pattern hash mismatch", patternHash("hello"));
  failed += 1;
}
const v2 = manifestForPattern("Ada", { caseMode: "none" });
if (v2.formatVersion !== 2 || v2.locale !== "en-US" || !v2.dictionaryHash) {
  console.error("npm manifest should be format 2 with dictionaryHash", v2);
  failed += 1;
}
threw = false;
try {
  manifestForPattern("x", { seed: "18446744073709551616" });
} catch {
  threw = true;
}
if (!threw) {
  console.error("npm manifest must reject overflowing u64 seeds");
  failed += 1;
}
threw = false;
try {
  manifestForPattern("x", { seed: "042" });
} catch {
  threw = true;
}
if (!threw) {
  console.error("npm manifest must reject leading-zero seeds");
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
threw = false;
try {
  skald("[a]Ada", { languagePack: nbPack, locale: "nb-NO", case: "none" });
} catch (err) {
  threw = String(err).includes("indefinite articles");
}
if (!threw) {
  console.error("nb pack should reject English [a]");
  failed += 1;
}
threw = false;
try {
  compile("Ada", { languagePack: nbPack, locale: "nb-NO" }).run({ locale: "nn-NO" });
} catch (err) {
  threw = String(err).includes("compile-time only");
}
if (!threw) {
  console.error("compile().run must reject locale/languagePack");
  failed += 1;
}

const nbCore = JSON.parse(readFileSync(resolve(root, "locales/nb-NO.json"), "utf8"));
const npmNb = JSON.parse(readFileSync(resolve(dirname(fileURLToPath(import.meta.url)), "nb-no.json"), "utf8"));
if (JSON.stringify(nbCore) !== JSON.stringify(npmNb)) {
  console.error("packages/skald-lang/nb-no.json must match locales/nb-NO.json");
  failed += 1;
}
const women = new Set();
for (let seed = 1; seed <= 100; seed += 1) {
  const line = skald("<firstname female :: hero> og <::hero>", {
    languagePack: nbCore,
    locale: "nb-NO",
    seed,
    case: "none",
  });
  const [a, b] = line.split(" og ");
  if (!a || a !== b) {
    console.error("nb-NO bound name desynced", seed, line);
    failed += 1;
    break;
  }
  women.add(a);
}
if (women.size < 2) {
  console.error("nb-NO firstname female should vary", [...women]);
  failed += 1;
}
const boundNoun = skald("<noun animal :: dyr> / <::dyr definite>", {
  languagePack: nbCore,
  locale: "nb-NO",
  seed: 7,
  case: "none",
});
if (!["katt / katten", "hund / hunden", "hest / hesten"].includes(boundNoun)) {
  console.error("nb-NO bound noun forms", boundNoun);
  failed += 1;
}
threw = false;
try {
  skald("[a]katt", { languagePack: nbCore, locale: "nb-NO", case: "none" });
} catch (err) {
  threw = String(err).toLowerCase().includes("article") || String(err).includes("indefinite");
}
if (!threw) {
  console.error("curated nb-NO pack must reject English [a]");
  failed += 1;
}

const nnCore = JSON.parse(readFileSync(resolve(root, "locales/nn-NO.json"), "utf8"));
const npmNn = JSON.parse(readFileSync(resolve(dirname(fileURLToPath(import.meta.url)), "nn-no.json"), "utf8"));
if (JSON.stringify(nnCore) !== JSON.stringify(npmNn)) {
  console.error("packages/skald-lang/nn-no.json must match locales/nn-NO.json");
  failed += 1;
}
const nnPron = skald("<pron nom female>", {
  languagePack: nnCore,
  locale: "nn-NO",
  seed: 1,
  case: "none",
});
if (nnPron !== "ho") {
  console.error("nn-NO female nom should be ho", nnPron);
  failed += 1;
}
const nnPoss = skald("<pron poss n>", {
  languagePack: nnCore,
  locale: "nn-NO",
  seed: 1,
  case: "none",
});
if (nnPoss !== "dess") {
  console.error("nn-NO neuter poss should be dess", nnPoss);
  failed += 1;
}
const nnBound = skald("<noun animal :: dyr> / <::dyr definite_pl>", {
  languagePack: nnCore,
  locale: "nn-NO",
  seed: 7,
  case: "none",
});
if (!["katt / kattane", "hund / hundane", "hest / hestane"].includes(nnBound)) {
  console.error("nn-NO bound noun plurals", nnBound);
  failed += 1;
}
threw = false;
try {
  skald("[a]katt", { languagePack: nnCore, locale: "nn-NO", case: "none" });
} catch (err) {
  threw = String(err).toLowerCase().includes("article") || String(err).includes("indefinite");
}
if (!threw) {
  console.error("curated nn-NO pack must reject English [a]");
  failed += 1;
}

threw = false;
try {
  preflight("<nonexistent_table>");
} catch (err) {
  threw = String(err).includes("PREFLIGHT_UNKNOWN_TABLE");
}
if (!threw) {
  console.error("preflight should reject unknown tables");
  failed += 1;
}
const englishUnknown = skald("<nonexistent_table>", { case: "none" });
if (!englishUnknown.includes("<nonexistent_table>")) {
  console.error("legacy english skald should still emit unknown queries", englishUnknown);
  failed += 1;
}
threw = false;
try {
  skald("<noun imaginary_form>", { languagePack: nbCore, locale: "nb-NO", case: "none" });
} catch (err) {
  threw = String(err).includes("PREFLIGHT_UNKNOWN_FORM");
}
if (!threw) {
  console.error("nb-NO pack should reject unknown forms");
  failed += 1;
}
const tmp = mkdtempSync(join(tmpdir(), "skald-preflight-"));
const badSkald = join(tmp, "bad.skald");
writeFileSync(badSkald, "<nonexistent_table>");
execFileSync("node", [resolve(root, "packages/skald-lang/cli.mjs"), "manifest", badSkald], {
  encoding: "utf8",
});
threw = false;
try {
  execFileSync("node", [resolve(root, "packages/skald-lang/cli.mjs"), "verify", badSkald], {
    encoding: "utf8",
  });
} catch (err) {
  threw = String(err.stderr ?? err).includes("PREFLIGHT_UNKNOWN_TABLE");
}
if (!threw) {
  console.error("npm verify should fail unknown tables");
  failed += 1;
}
threw = false;
try {
  execFileSync("node", [resolve(root, "packages/skald-lang/cli.mjs"), "--locale", "nb-NO", "--case", "none", "Ada"], {
    encoding: "utf8",
  });
} catch (err) {
  threw = String(err.stderr ?? err).includes("missing language pack");
}
if (!threw) {
  console.error("npm CLI nb-NO without pack should fail");
  failed += 1;
}
const nbCli = execFileSync(
  "node",
  [
    resolve(root, "packages/skald-lang/cli.mjs"),
    "--locale",
    "nb-NO",
    "--pack",
    resolve(root, "locales/nb-NO.json"),
    "--seed",
    "1",
    "--case",
    "none",
    "<firstname female>",
  ],
  { encoding: "utf8" },
).trim();
if (!nbCli || nbCli.includes("<")) {
  console.error("npm CLI --pack should render nb-NO", nbCli);
  failed += 1;
}

threw = false;
try {
  preflight("<noun m n>", { languagePack: nbCore, locale: "nb-NO" });
} catch (err) {
  threw = String(err).includes("PREFLIGHT_EMPTY_CANDIDATES");
}
if (!threw) {
  console.error("nb-NO pack should reject empty class intersections");
  failed += 1;
}

const enDictHash = fileHash(dictionaryJson());
const nbDictHash = fileHash(dictionaryJson({ languagePack: nbCore, locale: "nb-NO" }));
if (enDictHash === nbDictHash) {
  console.error("nb-NO dictionary hash should differ from bundled English");
  failed += 1;
}

const cli = resolve(root, "packages/skald-lang/cli.mjs");
const hashDir = mkdtempSync(join(tmpdir(), "skald-dict-hash-"));
const nativeSkald = join(hashDir, "n.skald");
const npmSkald = join(hashDir, "j.skald");
writeFileSync(nativeSkald, "<firstname female>");
writeFileSync(npmSkald, "<firstname female>");
const packPath = resolve(root, "locales/nb-NO.json");
execFileSync(
  "cargo",
  ["run", "-p", "skald", "--bin", "skald", "--quiet", "--", "--pack", packPath, "--locale", "nb-NO", "--seed", "1", "--case", "none", "manifest", nativeSkald],
  { encoding: "utf8", cwd: root },
);
execFileSync("node", [cli, "--pack", packPath, "--locale", "nb-NO", "--seed", "1", "--case", "none", "manifest", npmSkald], {
  encoding: "utf8",
});
const nativeHash = JSON.parse(readFileSync(`${nativeSkald}.json`, "utf8")).dictionaryHash;
const npmHash = JSON.parse(readFileSync(`${npmSkald}.json`, "utf8")).dictionaryHash;
if (nativeHash !== npmHash) {
  console.error("native/npm dictionaryHash mismatch", nativeHash, npmHash);
  failed += 1;
}

const recDir = mkdtempSync(join(tmpdir(), "skald-receipt-"));
const recSkald = join(recDir, "line.skald");
writeFileSync(recSkald, "{A|B|C|D|E|F|G|H}");
execFileSync("node", [cli, "--seed", "1", "--case", "none", "manifest", recSkald], { encoding: "utf8" });
execFileSync("node", [cli, "--case", "none", "run", recSkald], { encoding: "utf8" });
const defaultReceipt = JSON.parse(readFileSync(join(recDir, "line.receipt.json"), "utf8"));
if (defaultReceipt.seed?.value !== "1") {
  console.error("default receipt should store effective seed 1", defaultReceipt.seed);
  failed += 1;
}
const run42 = execFileSync("node", [cli, "--seed", "42", "--case", "none", "run", recSkald], { encoding: "utf8" }).trim();
const seededReceiptPath = join(recDir, "line.seed-42.receipt.json");
if (!existsSync(seededReceiptPath)) {
  console.error("run --seed 42 should write a unique receipt");
  failed += 1;
} else {
  const seededReceipt = JSON.parse(readFileSync(seededReceiptPath, "utf8"));
  if (seededReceipt.seed?.value !== "42" || seededReceipt.text.trim() !== run42) {
    console.error("seed 42 receipt mismatch", seededReceipt);
    failed += 1;
  }
  const defaultAfter = JSON.parse(readFileSync(join(recDir, "line.receipt.json"), "utf8"));
  if (defaultAfter.text !== defaultReceipt.text) {
    console.error("run --seed 42 overwrote the default receipt");
    failed += 1;
  }
}
try {
  execFileSync("node", [cli, "verify", recSkald], { encoding: "utf8" });
  execFileSync("node", [cli, "--seed", "42", "verify", recSkald], { encoding: "utf8" });
} catch (err) {
  console.error("receipt verify should pass", err.stderr ?? err);
  failed += 1;
}

const packDir = mkdtempSync(join(tmpdir(), "skald-pack-recipe-"));
const localPack = join(packDir, "nb-NO.json");
writeFileSync(localPack, JSON.stringify(nbCore));
const packSkald = join(packDir, "nb.skald");
writeFileSync(packSkald, "<firstname female>");
execFileSync(
  "node",
  [cli, "--pack", localPack, "--locale", "nb-NO", "--seed", "1", "--case", "none", "manifest", packSkald],
  { encoding: "utf8" },
);
const sidecar = JSON.parse(readFileSync(`${packSkald}.json`, "utf8"));
if (sidecar.dependencies?.[0]?.path !== "nb-NO.json") {
  console.error("manifest should store pack path relative to the artifact", sidecar.dependencies);
  failed += 1;
}
try {
  const fromElsewhere = execFileSync("node", [cli, "--case", "none", "run", packSkald], {
    encoding: "utf8",
    cwd: tmpdir(),
  }).trim();
  if (!fromElsewhere || fromElsewhere.includes("<")) {
    console.error("run should load the manifest pack without --pack", fromElsewhere);
    failed += 1;
  }
} catch (err) {
  console.error("run without --pack should use the manifest recipe", err.stderr ?? err);
  failed += 1;
}

if (failed) {
  console.error(`${failed} failed`);
  process.exit(1);
}
console.log(`${cases.length} native==wasm goldens ok`);
