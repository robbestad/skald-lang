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
  const narrativeBrief = doc.narrativeBrief ?? doc.brief;
  const deviation = doc.deviation;
  const expansion = doc.expansion;
  const theme = doc.theme;
  const merge = doc.merge;
  const draft = doc.draft ?? {
    schemaVersion: doc.schemaVersion ?? 1,
    cast: doc.cast,
    beats: doc.beats,
  };
  return {
    request: { seed, paletteIds, policy, narrativeBrief, deviation, expansion, theme, merge },
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
  if (argv[0] === "check" || argv[0] === "render" || argv[0] === "replay" || argv[0] === "loop") {
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
    const numberFlag = (name, fallback) => {
      const index = argv.indexOf(name);
      return index >= 0 ? Number(argv[index + 1]) : fallback;
    };
    const deviation = numberFlag("--deviation", undefined);
    const expansion = numberFlag("--expansion", undefined);
    const themeFlag = argv.indexOf("--theme");
    const theme = themeFlag >= 0 ? argv[themeFlag + 1] : undefined;
    if (!brief.trim()) {
      process.stderr.write(
        "Usage: node host.mjs loop [--brief <text> | <brief.md>] [--deviation 0-100] [--expansion 0-100] [--theme <text>] [--mock]\n",
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
      {
        narrativeBrief: brief,
        deviation,
        expansion,
        theme,
        seed: 11,
        paletteIds: [],
        policy: { maxRepairs: 2 },
      },
      model,
      { registry: PALETTES },
      { prompt },
    );
    printJson(artifact);
    process.exit(ok ? 0 : 2);
  }
  if (!path) {
    process.stderr.write(
      "Usage: node host.mjs [check|render|replay] <story-or-artifact.json>\n       node host.mjs loop [--brief <text> | <brief.md>] [--deviation 0-100] [--expansion 0-100] [--theme <text>] [--mock]\n",
    );
    process.exit(1);
  }
  const doc = load(path);
  const { request, draft } = splitDoc(doc);
  if (mode === "replay" && !doc.draft) {
    process.stderr.write("replay requires a saved StoryArtifact containing draft\n");
    process.exit(2);
  }
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
