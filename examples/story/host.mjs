#!/usr/bin/env node
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, extname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { explain } from "../../packages/skald-lang/index.js";
import { createMockModel } from "./mock-model.mjs";
import { PALETTES } from "./palettes.mjs";
import {
  buildStoryPattern,
  inspectStoryDocument,
  renderStory,
  runStoryLoop,
  splitStoryDocument,
} from "./runner.mjs";

const here = dirname(fileURLToPath(import.meta.url));

function load(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

function printJson(value) {
  process.stdout.write(JSON.stringify(value, null, 2) + "\n");
}

function stringFlag(argv, name) {
  const index = argv.indexOf(name);
  return index >= 0 ? argv[index + 1] : undefined;
}

function writeOutputs(argv, artifact) {
  const artifactPath = stringFlag(argv, "--artifact");
  const explicitSkaldPath = stringFlag(argv, "--skald");
  const skaldPath = explicitSkaldPath ?? (artifactPath
    ? artifactPath.slice(0, artifactPath.length - extname(artifactPath).length) + ".skald"
    : undefined);
  if (artifactPath) writeFileSync(artifactPath, JSON.stringify(artifact, null, 2) + "\n");
  if (skaldPath && artifact.pattern) writeFileSync(skaldPath, `${artifact.pattern}\n`);
  return { artifactPath, skaldPath };
}

async function main(argv = process.argv.slice(2)) {
  let mode = "render";
  let path = argv[0];
  if (argv[0] === "check" || argv[0] === "render" || argv[0] === "replay" || argv[0] === "pattern" || argv[0] === "loop") {
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
    const writingStyle = stringFlag(argv, "--writing-style");
    const mock = argv.includes("--mock");
    const provider = stringFlag(argv, "--provider");
    const modelName = stringFlag(argv, "--model");
    const reasoning = stringFlag(argv, "--reasoning");
    const reviewModel = stringFlag(argv, "--review-model") ?? modelName;
    const maxModelCalls = numberFlag("--max-model-calls", Infinity);
    const maxCostUsd = numberFlag("--max-cost-usd", Infinity);
    const seed = numberFlag("--seed", 11);
    const paletteIds = [];
    for (let i = 0; i < argv.length; i += 1) {
      if (argv[i] === "--palette" && argv[i + 1] && !String(argv[i + 1]).startsWith("-")) {
        paletteIds.push(argv[i + 1]);
      }
    }
    if (!brief.trim()) {
      process.stderr.write(
        "Usage: node host.mjs loop [--brief <text> | <brief.md>] --provider <name> --model <id> --reasoning <level> [--seed <n>] [--palette <id>] [--deviation 0-100] [--expansion 0-100] [--theme <text>]\n       node host.mjs loop [--brief <text> | <brief.md>] --mock [--palette <id>]\n",
      );
      process.exit(1);
    }
    const prompt = readFileSync(resolve(here, "prompt.md"), "utf8");
    const good = splitStoryDocument(load(resolve(here, "inn.json"))).draft;
    const bad = {
      schemaVersion: 1,
      cast: good.cast,
      beats: ["<::hero> <verb.ed> the <place>."],
    };
    let storyModel;
    if (mock) {
      storyModel = createMockModel({ bad, good });
    } else {
      if (!provider || !modelName || !reasoning) {
        process.stderr.write("model loop requires --provider, --model, and --reasoning\n");
        process.exit(1);
      }
      if (provider !== "openai" && provider !== "ollama") {
        process.stderr.write(`unsupported provider '${provider}'; available: openai, ollama\n`);
        process.exit(1);
      }
      if (provider === "openai") {
        const { createOpenAIModel } = await import("./adapters/openai.mjs");
        storyModel = createOpenAIModel({
          model: modelName,
          reviewModel,
          reasoningEffort: reasoning,
          maxModelCalls,
          maxCostUsd,
        });
      } else {
        const { createOllamaModel } = await import("./adapters/ollama.mjs");
        storyModel = createOllamaModel({
          model: modelName,
          reviewModel,
          reasoningEffort: reasoning,
          baseUrl: stringFlag(argv, "--base-url"),
          maxModelCalls,
          contextSize: numberFlag("--context-size", 16_384),
        });
      }
    }
    let ok;
    let artifact;
    try {
      ({ ok, artifact } = await runStoryLoop(
        { explain },
        {
          narrativeBrief: brief,
          deviation,
          expansion,
          theme,
          writingStyle,
          seed,
          provider: mock ? "mock" : provider,
          model: mock ? "mock" : modelName,
          reasoning: mock ? null : reasoning,
          paletteIds,
          policy: { maxRepairs: 2, maxModelCalls, maxCostUsd },
        },
        storyModel,
        { registry: PALETTES },
        { prompt },
      ));
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      if (!message.startsWith("STORY_MODEL_BUDGET:")) throw error;
      ok = false;
      artifact = {
        ok,
        schemaVersion: 1,
        seed,
        narrativeBrief: brief,
        provider,
        model: modelName,
        reasoning,
        diagnostics: [{ code: "STORY_MODEL_BUDGET", severity: "error", message }],
        telemetry: { providerUsage: storyModel.getUsage?.() ?? null },
      };
    }
    writeOutputs(argv, artifact);
    printJson(artifact);
    process.exit(ok ? 0 : 2);
  }
  if (!path) {
    process.stderr.write(
      "Usage: node host.mjs [check|render|replay] <story-or-artifact.json>\n       node host.mjs pattern <story-or-artifact.json> --skald <name.skald>\n       node host.mjs loop [--brief <text> | <brief.md>] --provider <name> --model <id> --reasoning <level> [--palette <id>] [--artifact <name.json>]\n       node host.mjs loop [--brief <text> | <brief.md>] --mock [--palette <id>]\n",
    );
    process.exit(1);
  }
  const doc = load(path);
  const { request, draft } = splitStoryDocument(doc);
  if (mode === "replay" && !doc.draft) {
    process.stderr.write("replay requires a saved StoryArtifact containing draft\n");
    process.exit(2);
  }
  if (mode === "check") {
    const inspected = inspectStoryDocument(doc, PALETTES);
    printJson({ ok: inspected.ok, diagnostics: inspected.diagnostics });
    process.exit(inspected.ok ? 0 : 2);
  }
  if (mode === "pattern") {
    const inspected = inspectStoryDocument(doc, PALETTES);
    if (!inspected.ok) {
      printJson({ ok: false, diagnostics: inspected.diagnostics });
      process.exit(2);
    }
    const pattern = buildStoryPattern(inspected.draft).pattern;
    const skaldPath = stringFlag(argv, "--skald");
    if (skaldPath) writeFileSync(skaldPath, `${pattern}\n`);
    else process.stdout.write(`${pattern}\n`);
    return;
  }
  const { ok, artifact } = renderStory({ explain }, request, draft, {
    registry: PALETTES,
  });
  writeOutputs(argv, artifact);
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

main().catch((error) => {
  const message = error instanceof Error ? error.message : String(error);
  const code = message.startsWith("STORY_MODEL_BUDGET:")
    ? "STORY_MODEL_BUDGET"
    : "STORY_MODEL";
  printJson({ ok: false, diagnostics: [{ code, severity: "error", message }] });
  process.exit(2);
});
