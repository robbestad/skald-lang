#!/usr/bin/env node
/** Record 3.0 timings and sizes versus the stored 2.2 baseline. */
import { execFileSync } from "node:child_process";
import { gzipSync } from "node:zlib";
import { existsSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { performance } from "node:perf_hooks";
import { compile, Engine, explain, skald } from "../packages/skald-lang/index.js";
import { initSync } from "../packages/skald-lang/pkg/skald_wasm.js";
import nb from "../locales/nb-NO.json" with { type: "json" };
import nn from "../locales/nn-NO.json" with { type: "json" };

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const WASM_GZIP_BUDGET = 500_000;

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
const compileMs = ms(() => compile(pattern, { case: "none" }), 50);
const runMs = ms(() => skald(pattern, { seed: 1, case: "none" }), 200);
const compiled = compile(pattern, { case: "none" });
const compiledRunMs = ms(() => compiled.run({ seed: 1 }), 200);
const explainMs = ms(() => explain(pattern, { seed: 1, case: "none" }), 50);
const nbJson = JSON.stringify(nb);
const packLoadMs = ms(() => Engine.fromLanguagePack(nbJson), 20);
const nbMs = ms(() => skald("<firstname female> åpnet {døren|vinduet}.", {
  languagePack: nb,
  locale: "nb-NO",
  seed: 1,
  case: "none",
}), 50);
const nnMs = ms(() => skald("<firstname female> opna {døra|vindauget}.", {
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

const wasmExports = initSync({
  module: readFileSync(resolve(root, "packages/skald-lang/pkg/skald_wasm_bg.wasm")),
});
const wasmMemoryBytes = wasmExports.memory?.buffer?.byteLength ?? null;
const wasmMemoryKb = wasmMemoryBytes == null ? "n/a" : (wasmMemoryBytes / 1024).toFixed(1);

const wasm = resolve(root, "packages/skald-lang/pkg/skald_wasm_bg.wasm");
let wasmGzipKb = "n/a";
let wasmGzipBytes = null;
if (existsSync(wasm)) {
  wasmGzipBytes = execFileSync("gzip", ["-c", wasm]).length;
  if (wasmGzipBytes >= WASM_GZIP_BUDGET) {
    throw new Error(`wasm gzip ${wasmGzipBytes} exceeds ${WASM_GZIP_BUDGET} byte budget`);
  }
  wasmGzipKb = (wasmGzipBytes / 1024).toFixed(1);
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

const baselinePath = resolve(root, "docs/benchmarks-2.2.json");
const baseline = existsSync(baselinePath)
  ? JSON.parse(readFileSync(baselinePath, "utf8"))
  : null;
const wasmDelta = baseline && wasmGzipBytes != null
  ? `${((wasmGzipBytes - baseline.wasmGzipBytes) / 1024).toFixed(1)} KB vs ${baseline.tag}`
  : "n/a";

const pkgVersion = JSON.parse(readFileSync(resolve(root, "packages/skald-lang/package.json"), "utf8")).version;
const lines = `# 3.0 measurements vs 2.2`

Package version is ${pkgVersion}. Snapshot from \`scripts/bench-rc.mjs\` on this
checkout. Baseline: \`docs/benchmarks-2.2.json\` (${baseline ? baseline.tag : "missing"}).
Not a gate. WASM gzip budget remains **500 KB**. Language packs are separate JSON.
JS heap and WASM linear memory are reported separately.

| Item | Value |
| --- | --- |
| npm \`compile()\` mean (50 runs, en-US) | ${compileMs.toFixed(2)} ms |
| npm \`skald()\` mean (200 runs, en-US) | ${runMs.toFixed(2)} ms |
| npm compiled \`.run()\` mean (200 runs, en-US) | ${compiledRunMs.toFixed(2)} ms |
| npm \`explain()\` mean (50 runs, en-US) | ${explainMs.toFixed(2)} ms |
| \`Engine.fromLanguagePack\` mean (20 loads, nb-NO) | ${packLoadMs.toFixed(2)} ms |
| npm \`skald()\` mean (50 runs, nb-NO pack) | ${nbMs.toFixed(2)} ms |
| npm \`skald()\` mean (50 runs, nn-NO pack) | ${nnMs.toFixed(2)} ms |
| JS heap delta after 50 \`explain()\` | ${heapDeltaKb} KB |
| WASM linear memory | ${wasmMemoryKb} KB |
| native release binary (one pattern, wall) | ${nativeMs} ms |
| \`skald_wasm_bg.wasm\` gzip | ${wasmGzipKb} KB (budget 500; ${wasmDelta}) |
| 2.2 npm \`skald()\` mean | ${baseline ? `${baseline.npmSkaldMs} ms` : "n/a"} |
| 2.2 wasm gzip | ${baseline ? `${(baseline.wasmGzipBytes / 1024).toFixed(1)} KB` : "n/a"} |
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

const snapshot = {
  packageVersion: pkgVersion,
  commit: execFileSync("git", ["rev-parse", "HEAD"], { cwd: root, encoding: "utf8" }).trim(),
  platform: `${process.platform}-${process.arch}`,
  node: process.version,
  npmCompileMs: Number(compileMs.toFixed(4)),
  npmSkaldMs: Number(runMs.toFixed(4)),
  npmCompiledRunMs: Number(compiledRunMs.toFixed(4)),
  npmExplainMs: Number(explainMs.toFixed(4)),
  jsHeapDeltaKb: Number(heapDeltaKb),
  wasmMemoryBytes,
  wasmGzipBytes,
};
const baseline301 = resolve(root, "docs/benchmarks-3.0.1.json");
if (!existsSync(baseline301)) {
  throw new Error("docs/benchmarks-3.0.1.json is the frozen 3.0.1 baseline and must exist");
}
writeFileSync(resolve(root, "docs/benchmarks-latest.json"), `${JSON.stringify(snapshot, null, 2)}\n`);
const out = resolve(root, "docs/benchmarks-3.0-rc.md");
writeFileSync(out, lines);
process.stdout.write(lines);
