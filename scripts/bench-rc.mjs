#!/usr/bin/env node
/** Record 3.0-rc timings and sizes. Does not compare to a stored 2.2 gold file. */
import { execFileSync } from "node:child_process";
import { gzipSync } from "node:zlib";
import { existsSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { performance } from "node:perf_hooks";
import { compile, Engine, explain, skald } from "../packages/skald-lang/index.js";
import nb from "../locales/nb-NO.json" with { type: "json" };
import nn from "../locales/nn-NO.json" with { type: "json" };

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const WASM_GZIP_BUDGET = 400_000;

function ms(fn, n = 1) {
  const t0 = performance.now();
  for (let i = 0; i < n; i += 1) fn();
  return (performance.now() - t0) / n;
}

function kb(path) {
  return (statSync(path).size / 1024).toFixed(1);
}

function gzipKb(path) {
  return (gzipSync(readFileSync(path)).length / 1024).toFixed(1);
}

const pattern = "<firstname female :: hero> and <::hero> {walked|came} to the {inn|door}.";
const runMs = ms(() => skald(pattern, { seed: 1, case: "none" }), 200);
const compiled = compile(pattern, { case: "none" });
const compiledRunMs = ms(() => compiled.run({ seed: 1 }), 200);
const explainMs = ms(() => explain(pattern, { seed: 1, case: "none" }), 50);
const nbJson = JSON.stringify(nb);
const packLoadMs = ms(() => Engine.fromLanguagePack(nbJson), 20);
const nbMs = ms(() => skald("<firstname female> åpnet <noun n definite>.", {
  languagePack: nb,
  locale: "nb-NO",
  seed: 1,
  case: "none",
}), 50);
const nnMs = ms(() => skald("<firstname female> opna <noun n definite>.", {
  languagePack: nn,
  locale: "nn-NO",
  seed: 1,
  case: "none",
}), 50);

const heapBefore = process.memoryUsage().heapUsed;
for (let i = 0; i < 50; i += 1) {
  explain(pattern, { seed: i + 1, case: "none" });
}
const heapDeltaKb = ((process.memoryUsage().heapUsed - heapBefore) / 1024).toFixed(1);

const wasm = resolve(root, "packages/skald-lang/pkg/skald_wasm_bg.wasm");
let wasmGzipKb = "n/a";
if (existsSync(wasm)) {
  const gzipBytes = gzipSync(readFileSync(wasm)).length;
  if (gzipBytes >= WASM_GZIP_BUDGET) {
    throw new Error(`wasm gzip ${gzipBytes} exceeds ${WASM_GZIP_BUDGET} byte budget`);
  }
  wasmGzipKb = (gzipBytes / 1024).toFixed(1);
}

let nativeMs = "n/a";
const nativeBin = resolve(root, "target/release/skald");
try {
  if (!existsSync(nativeBin)) {
    execFileSync("cargo", ["build", "-p", "skald", "--quiet", "--release"], {
      cwd: root,
      encoding: "utf8",
    });
  }
  const t0 = performance.now();
  execFileSync(nativeBin, ["--case", "none", "--seed", "1", pattern], {
    cwd: root,
    encoding: "utf8",
  });
  nativeMs = (performance.now() - t0).toFixed(1);
} catch {
  nativeMs = "skipped";
}

const lines = `# 3.0-rc measurements

Package version remains 2.2.0. Snapshot from \`scripts/bench-rc.mjs\` on this
checkout — not a gate, and not a claim that 3.0 is faster than 2.2. There is no
stored 2.2 gold file; the 2.2 contract that still applies is the **400 KB**
gzipped WASM budget. Language packs are separate JSON and are not in that budget.

| Item | Value |
| --- | --- |
| npm \`skald()\` mean (200 runs, en-US) | ${runMs.toFixed(2)} ms |
| npm compiled \`.run()\` mean (200 runs, en-US) | ${compiledRunMs.toFixed(2)} ms |
| npm \`explain()\` mean (50 runs, en-US) | ${explainMs.toFixed(2)} ms |
| \`Engine.fromLanguagePack\` mean (20 loads, nb-NO) | ${packLoadMs.toFixed(2)} ms |
| npm \`skald()\` mean (50 runs, nb-NO pack) | ${nbMs.toFixed(2)} ms |
| npm \`skald()\` mean (50 runs, nn-NO pack) | ${nnMs.toFixed(2)} ms |
| heap delta after 50 \`explain()\` | ${heapDeltaKb} KB |
| native release binary (one pattern, wall) | ${nativeMs} ms |
| \`skald_wasm_bg.wasm\` gzip | ${wasmGzipKb} KB (budget 400) |
| \`en-us.json\` | ${kb(resolve(root, "packages/skald-lang/en-us.json"))} KB |
| \`nb-NO.json\` | ${kb(resolve(root, "locales/nb-NO.json"))} KB |
| \`nb-NO.json\` gzip | ${gzipKb(resolve(root, "locales/nb-NO.json"))} KB |
| \`nn-NO.json\` | ${kb(resolve(root, "locales/nn-NO.json"))} KB |
| \`nn-NO.json\` gzip | ${gzipKb(resolve(root, "locales/nn-NO.json"))} KB |

Language pack files are original curated cores, not ordbank dumps. Re-run:

\`\`\`bash
node scripts/bench-rc.mjs
\`\`\`
`;

const out = resolve(root, "docs/benchmarks-3.0-rc.md");
writeFileSync(out, lines);
process.stdout.write(lines);
