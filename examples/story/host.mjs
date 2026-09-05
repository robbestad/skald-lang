#!/usr/bin/env node
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, extname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { explain } from "../../packages/skald-lang/index.js";
import { createMockModel } from "./mock-model.mjs";
import { PALETTES } from "./palettes.mjs";
import nbNO from "../../locales/nb-NO.json" with { type: "json" };
import nnNO from "../../locales/nn-NO.json" with { type: "json" };
import {
  buildStoryPattern,
  composeStatePatch,
  extractStoryState,
  inspectStoryDocument,
  mergePalettes,
  renderStory,
  runStoryLoop,
  splitStoryDocument,
} from "./runner.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const LANGUAGE_PACKS = { "nb-NO": nbNO, "nn-NO": nnNO };

function withLanguagePack(request) {
  const locale = request.locale ?? request.storyState?.locale ?? "en-US";
  if (locale === "en-US") return { ...request, locale };
  return {
    ...request,
    locale,
    languagePack: request.languagePack ?? LANGUAGE_PACKS[locale] ?? null,
  };
}

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

function repeatableFlag(argv, name) {
  const values = [];
  for (let i = 0; i < argv.length; i += 1) {
    if (argv[i] === name && argv[i + 1] && !String(argv[i + 1]).startsWith("-")) {
      values.push(argv[i + 1]);
    }
  }
  return values;
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
  if (argv[0] === "check" || argv[0] === "render" || argv[0] === "replay" || argv[0] === "pattern" || argv[0] === "loop" || argv[0] === "state") {
    mode = argv[0];
    path = argv[1];
  }
  if (mode === "loop") {
    const briefFlag = argv.indexOf("--brief");
    let envelope = null;
    let brief = "";
    if (briefFlag >= 0) {
      brief = argv[briefFlag + 1] ?? "";
    } else if (path && !path.startsWith("-")) {
      if (String(path).endsWith(".json")) envelope = load(path);
      else brief = readFileSync(path, "utf8");
    }
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
    const seedRaw = stringFlag(argv, "--seed");
    const fromEnvelope = envelope ? splitStoryDocument(envelope).request : {};
    const seed = seedRaw === undefined ? (fromEnvelope.seed ?? 11) : seedRaw;
    const statePath = stringFlag(argv, "--state");
    const patchPath = stringFlag(argv, "--patch");
    if (statePath && fromEnvelope.storyState) {
      printJson({ ok: false, diagnostics: [{ code: "STORY_SCHEMA", severity: "error", message: "cannot combine --state with envelope storyState" }] });
      process.exit(2);
    }
    if (patchPath && fromEnvelope.statePatch) {
      printJson({ ok: false, diagnostics: [{ code: "STORY_SCHEMA", severity: "error", message: "cannot combine --patch with envelope statePatch" }] });
      process.exit(2);
    }
    const storyState = statePath ? load(statePath) : fromEnvelope.storyState;
    const composedPatch = composeStatePatch({
      patch: patchPath ? load(patchPath) : fromEnvelope.statePatch,
      closedThreads: repeatableFlag(argv, "--closed-thread"),
    });
    if (!composedPatch.ok) {
      printJson({ ok: false, diagnostics: composedPatch.diagnostics });
      process.exit(2);
    }
    const paletteIds = [...(fromEnvelope.paletteIds ?? [])];
    for (let i = 0; i < argv.length; i += 1) {
      if (argv[i] === "--palette" && argv[i + 1] && !String(argv[i + 1]).startsWith("-")) {
        paletteIds.push(argv[i + 1]);
      }
    }
    const narrativeBrief = brief.trim() || fromEnvelope.narrativeBrief || "";
    if (!narrativeBrief) {
      process.stderr.write(
        "Usage: node host.mjs loop [--brief <text> | <brief.md> | <request.json>] --provider <name> --model <id> --reasoning <level> [--seed <n>] [--palette <id>] [--state <state.json>] [--patch <patch.json>] [--closed-thread <text>] [--deviation 0-100] [--expansion 0-100] [--theme <text>] [--full-lexical-coverage]\n       node host.mjs loop [--brief <text> | <brief.md> | <request.json>] --mock [--palette <id>] [--state <state.json>] [--patch <patch.json>] [--closed-thread <text>] [--full-lexical-coverage]\n",
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
    const paletteCheck = mergePalettes(PALETTES, paletteIds);
    if (!paletteCheck.ok) {
      printJson({ ok: false, diagnostics: paletteCheck.diagnostics });
      process.exit(2);
    }
    let ok;
    let artifact;
    try {
      ({ ok, artifact } = await runStoryLoop(
        { explain },
        withLanguagePack({
          ...fromEnvelope,
          narrativeBrief,
          deviation: deviation ?? fromEnvelope.deviation,
          expansion: expansion ?? fromEnvelope.expansion,
          theme: theme ?? fromEnvelope.theme,
          writingStyle: writingStyle ?? fromEnvelope.writingStyle,
          seed,
          provider: mock ? "mock" : (provider ?? fromEnvelope.provider),
          model: mock ? "mock" : (modelName ?? fromEnvelope.model),
          reasoning: mock ? null : (reasoning ?? fromEnvelope.reasoning),
          paletteIds,
          storyState,
          statePatch: composedPatch.patch,
          policy: {
            ...(fromEnvelope.policy ?? {}),
            maxRepairs: 2,
            maxModelCalls,
            maxCostUsd,
            fullLexicalCoverage: argv.includes("--full-lexical-coverage") || fromEnvelope.policy?.fullLexicalCoverage === true,
          },
        }),
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
        narrativeBrief,
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
      "Usage: node host.mjs [check|render|replay] <story-or-artifact.json>\n       node host.mjs pattern <story-or-artifact.json> --skald <name.skald>\n       node host.mjs state <artifact.json> [--patch <patch.json>] [--closed-thread <text>]\n       node host.mjs loop [--brief <text> | <brief.md> | <request.json>] --provider <name> --model <id> --reasoning <level> [--palette <id>] [--state <state.json>] [--patch <patch.json>] [--closed-thread <text>] [--full-lexical-coverage] [--artifact <name.json>]\n       node host.mjs loop [--brief <text> | <brief.md> | <request.json>] --mock [--palette <id>] [--state <state.json>] [--patch <patch.json>] [--closed-thread <text>] [--full-lexical-coverage]\n",
    );
    process.exit(1);
  }
  const doc = load(path);
  const { request, draft } = splitStoryDocument(doc);
  if (mode === "state") {
    const composedPatch = composeStatePatch({
      patch: stringFlag(argv, "--patch") ? load(stringFlag(argv, "--patch")) : null,
      closedThreads: repeatableFlag(argv, "--closed-thread"),
    });
    if (!composedPatch.ok) {
      printJson({ ok: false, diagnostics: composedPatch.diagnostics });
      process.exit(2);
    }
    const extracted = extractStoryState(doc, composedPatch.patch, { caller: true });
    if (!extracted.ok) {
      printJson({ ok: false, diagnostics: extracted.diagnostics });
      process.exit(2);
    }
    printJson(extracted.state);
    return;
  }
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
    const built = buildStoryPattern(
      inspected.draft,
      inspected.draft.cast,
      undefined,
      inspected.request.variations ?? [],
    );
    if ((built.diagnostics ?? []).length) {
      printJson({ ok: false, diagnostics: built.diagnostics });
      process.exit(2);
    }
    const pattern = built.pattern;
    const skaldPath = stringFlag(argv, "--skald");
    if (skaldPath) writeFileSync(skaldPath, `${pattern}\n`);
    else process.stdout.write(`${pattern}\n`);
    return;
  }
  const { ok, artifact } = renderStory({ explain }, withLanguagePack(request), draft, {
    registry: PALETTES,
  });
  if (mode === "replay") {
    if (doc.text != null && artifact.text !== doc.text) {
      printJson({
        ok: false,
        diagnostics: [{
          code: "STORY_REPLAY_MISMATCH",
          severity: "error",
          message: "replay text does not match the saved artifact",
        }],
      });
      process.exit(2);
    }
    if (doc.replayHash && artifact.replayHash !== doc.replayHash) {
      printJson({
        ok: false,
        diagnostics: [{
          code: "STORY_REPLAY_MISMATCH",
          severity: "error",
          message: "replay hash does not match the saved artifact",
        }],
      });
      process.exit(2);
    }
  }
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
