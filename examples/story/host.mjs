#!/usr/bin/env node
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { explain } from "../../packages/skald-lang/index.js";
import { createMockModel } from "./mock-model.mjs";
import { PALETTES } from "./palettes.mjs";
import {
  analyzeStoryDraft,
  renderStory,
  runStoryLoop,
  validateStoryDraft,
} from "./runner.mjs";

const here = dirname(fileURLToPath(import.meta.url));

function splitDoc(doc) {
  const seed = doc.seed;
  const paletteIds = doc.paletteIds ?? [];
  const policy = doc.policy ?? {};
  const brief = doc.brief;
  const draft = {
    schemaVersion: doc.schemaVersion ?? 1,
    cast: doc.cast,
    beats: doc.beats,
  };
  return {
    request: { seed, paletteIds, policy, brief },
    draft,
  };
}

function load(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

function printJson(value) {
  process.stdout.write(JSON.stringify(value, null, 2) + "\n");
}

async function main(argv = process.argv.slice(2)) {
  let mode = "render";
  let path = argv[0];
  if (argv[0] === "check" || argv[0] === "render" || argv[0] === "loop") {
    mode = argv[0];
    path = argv[1];
  }
  if (mode === "loop") {
    const briefFlag = argv.indexOf("--brief");
    const brief =
      briefFlag >= 0
        ? argv[briefFlag + 1]
        : path && !path.startsWith("-")
          ? readFileSync(path, "utf8")
          : "";
    if (!brief.trim()) {
      process.stderr.write(
        "Usage: node host.mjs loop [--brief <text> | <brief.md>] [--mock]\n",
      );
      process.exit(1);
    }
    const prompt = readFileSync(resolve(here, "prompt.md"), "utf8");
    const good = splitDoc(load(resolve(here, "inn.json"))).draft;
    const bad = {
      schemaVersion: 1,
      cast: good.cast,
      beats: ["<::hero> <verb.ed> the <place>."],
    };
    const model = createMockModel({ bad, good });
    const { ok, artifact } = await runStoryLoop(
      { explain },
      { brief, seed: 11, paletteIds: [], policy: { maxRepairs: 2 } },
      model,
      { registry: PALETTES },
      { prompt },
    );
    printJson(artifact);
    process.exit(ok ? 0 : 2);
  }
  if (!path) {
    process.stderr.write(
      "Usage: node host.mjs [check|render] <story.json>\n       node host.mjs loop [--brief <text> | <brief.md>] [--mock]\n",
    );
    process.exit(1);
  }
  const doc = load(path);
  const { request, draft } = splitDoc(doc);
  if (mode === "check") {
    const schema = validateStoryDraft(draft);
    const analysis = analyzeStoryDraft(draft, {
      ...request.policy,
    });
    const diagnostics = [...schema.diagnostics, ...analysis.diagnostics];
    const ok = diagnostics.length === 0;
    printJson({ ok, diagnostics });
    process.exit(ok ? 0 : 2);
  }
  const { ok, artifact } = renderStory({ explain }, request, draft, {
    registry: PALETTES,
  });
  if (argv.includes("--json")) {
    printJson(artifact);
    process.exit(ok ? 0 : 2);
  }
  if (!ok) {
    printJson({ ok: false, diagnostics: artifact.diagnostics, notes: artifact.notes });
    process.exit(2);
  }
  const text = artifact.text.endsWith("\n") ? artifact.text : `${artifact.text}\n`;
  process.stdout.write(text);
}

main();
