#!/usr/bin/env node
import { createInterface } from "node:readline";
import { existsSync, readFileSync } from "node:fs";
import { stdin as stdinFd, stdout, stderr } from "node:process";
import { skald, output, explain, preflight, dictionaryJson } from "./index.js";
import { dirname, resolve as resolvePath } from "node:path";
import {
  fileHash,
  looksLikeLanguagePackText,
  manifestForPattern,
  readManifest,
  readReceipt,
  receiptPath,
  replayLocked,
  resolveDependencyPath,
  seedRecord,
  sidecarPath,
  storedDependencyPath,
  verifyLock,
  verifyPattern,
  verifyReceipt,
  writeManifest,
  writeReceipt,
  RECEIPT_FORMAT_VERSION,
} from "./artifact.mjs";

const VERSION = "3.0.2";

function printHelp() {
  process.stderr.write(`Usage: skald-lang [options] <pattern>
       skald-lang [options] -f <file>
       skald-lang [options] -e <pattern>
       skald-lang                 (REPL, or read stdin if piped)

Generate procedural text from a Skald pattern.

Artifact commands (sidecar is <file>.json next to the .skald):
  skald-lang manifest <file.skald>   Write/update the sidecar
  skald-lang inspect <file.skald>    Show the sidecar
  skald-lang verify <file.skald>     Check pattern hash
  skald-lang run <file.skald>        Verify, then render (optional --seed writes a unique receipt)

Options:
  -s, --seed <value>   Seed the generator (integer or string)
  -e, --eval <pattern> Pattern on the command line
  -f, --file <path>    Read the pattern from a file
      --case <mode>    none|first|word|title|upper|lower|sentence
      --nsfw           Include NSFW dictionary entries
      --channels       Print JSON with text and named channels
      --explain        Print JSON with text, channels, and dictionary picks
      --prove          Like --explain, plus glue vs dictionary parts and density
      --story          Explain JSON plus story-lint notes (exit 2 if any story notes)
      --dict <path>    Overlay dictionary JSON (repeatable; left to right)
      --dict-only      Ignore bundled English; use only --dict files
      --locale <id>    en-US (default), nb-NO, or nn-NO
      --pack <path>    Language pack JSON (required for nb-NO / nn-NO)
  -h, --help           Show this help
  -v, --version        Show version

REPL commands: :seed, :case, :prove, :story, :channels, :help, :quit
`);
}

function parseArgs(argv) {
  const out = {
    seed: undefined,
    file: undefined,
    eval: undefined,
    caseMode: undefined,
    nsfw: false,
    channels: false,
    explain: false,
    story: false,
    help: false,
    version: false,
    dicts: [],
    dictOnly: false,
    locale: undefined,
    pack: undefined,
    rest: [],
  };
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === "-h" || arg === "--help") out.help = true;
    else if (arg === "-v" || arg === "--version") out.version = true;
    else if (arg === "--nsfw") out.nsfw = true;
    else if (arg === "--channels") out.channels = true;
    else if (arg === "--explain" || arg === "--prove") out.explain = true;
    else if (arg === "--story") {
      out.story = true;
      out.explain = true;
    }
    else if (arg === "-s" || arg === "--seed") out.seed = argv[++i];
    else if (arg === "-e" || arg === "--eval") out.eval = argv[++i];
    else if (arg === "-f" || arg === "--file") out.file = argv[++i];
    else if (arg === "--case") out.caseMode = argv[++i];
    else if (arg === "--dict") {
      const path = argv[++i];
      if (!path) throw new Error("--dict needs a path");
      out.dicts.push(path);
    } else if (arg === "--dict-only") out.dictOnly = true;
    else if (arg === "--locale") out.locale = argv[++i];
    else if (arg === "--pack") {
      const path = argv[++i];
      if (!path) throw new Error("--pack needs a path");
      out.pack = path;
    } else if (arg.startsWith("-")) throw new Error(`Unknown option: ${arg}`);
    else out.rest.push(arg);
  }
  return out;
}

function seedOf(value) {
  if (value === undefined) return undefined;
  return value;
}

function loadDicts(args) {
  const locale = args.locale;
  if (args.pack) {
    const languagePack = JSON.parse(readFileSync(args.pack, "utf8"));
    const packLocale = languagePack.locale;
    if (locale && packLocale && locale !== packLocale) {
      throw new Error(`language pack locale ${packLocale} does not match ${locale}`);
    }
    const out = {
      languagePack,
      locale: locale ?? packLocale,
      merge: false,
    };
    if (args.dicts.length) {
      const tables = {};
      for (const path of args.dicts) {
        const raw = JSON.parse(readFileSync(path, "utf8"));
        Object.assign(tables, raw.tables ?? raw);
      }
      out.dictionary = { tables };
    }
    return out;
  }
  if (locale && locale !== "en-US") {
    throw new Error(`missing language pack for ${locale}`);
  }
  if (!args.dicts.length && !args.dictOnly) return locale ? { locale } : {};
  const tables = {};
  for (const path of args.dicts) {
    const raw = JSON.parse(readFileSync(path, "utf8"));
    Object.assign(tables, raw.tables ?? raw);
  }
  return {
    dictionary: { tables },
    merge: !args.dictOnly,
    ...(locale ? { locale } : {}),
  };
}

function render(pattern, args) {
  const options = {
    seed: seedOf(args.seed),
    nsfw: args.nsfw,
    case: args.caseMode,
    story: args.story,
    ...loadDicts(args),
  };
  if (args.explain || args.story) return JSON.stringify(explain(pattern, options));
  if (args.channels) return JSON.stringify(output(pattern, options));
  return skald(pattern, options);
}

function storyExit(text) {
  try {
    const parsed = JSON.parse(text);
    const diags = parsed.diagnostics ?? [];
    const notes = parsed.notes ?? [];
    if (diags.some((d) => d.severity === "error")) return 2;
    if (notes.some((n) => String(n).startsWith("story:"))) return 2;
  } catch {
    return 0;
  }
  return 0;
}

function printOut(text) {
  process.stdout.write(text.endsWith("\n") ? text : `${text}\n`);
}

function encodeManifestSeed(seed) {
  if (!seed?.value) return undefined;
  return seed.type === "text" ? `text:${seed.value}` : seed.value;
}

function applyManifestRunOptions(args, manifest, argv = []) {
  if (!args.seed && manifest.seed?.value) args.seed = encodeManifestSeed(manifest.seed);
  if (!args.caseMode && manifest.case) args.caseMode = manifest.case;
  if (!argv.includes("--nsfw")) args.nsfw = Boolean(manifest.nsfw);
  if (!argv.includes("--story")) {
    args.story = Boolean(manifest.story);
    if (args.story) args.explain = true;
  }
}

function applyManifestLanguage(args, manifest, artifactPath) {
  if (!args.locale && manifest.locale) args.locale = manifest.locale;
  if (manifest.dictOnly) args.dictOnly = true;
  if (args.pack || args.dicts.length) return;
  const baseDir = dirname(resolvePath(artifactPath));
  for (const dep of manifest.dependencies ?? []) {
    const depPath = resolveDependencyPath(baseDir, dep.path);
    const src = readFileSync(depPath, "utf8");
    if (looksLikeLanguagePackText(src) && !args.pack) args.pack = depPath;
    else args.dicts.push(depPath);
  }
}

function artifactCommand(args, argv = []) {
  const cmd = args.rest[0];
  if (!["run", "inspect", "verify", "manifest"].includes(cmd)) return null;
  const path = args.file ?? args.rest[1];
  if (!args.file && !(typeof path === "string" && path.endsWith(".skald"))) return null;
  args.rest.shift();
  if (!path) throw new Error(`skald-lang ${cmd} needs a .skald file`);
  const pattern = readFileSync(path, "utf8");
  const side = sidecarPath(path);
  if (cmd === "manifest") {
    const depPaths = [...(args.pack ? [args.pack] : []), ...args.dicts];
    const dependencies = depPaths.map((dictPath) => ({
      path: storedDependencyPath(path, dictPath),
      hash: fileHash(readFileSync(dictPath)),
    }));
    const loaded = loadDicts(args);
    const manifest = manifestForPattern(pattern, {
      seed: args.seed,
      caseMode: args.caseMode,
      nsfw: args.nsfw,
      story: args.story,
      runtimeVersion: VERSION,
      locale: loaded.locale ?? args.locale ?? "en-US",
      dictionaryJson: dictionaryJson(loaded),
      dependencies,
      dictOnly: args.dictOnly,
    });
    writeManifest(side, manifest);
    stdout.write(`${side}\n`);
    return 0;
  }
  const manifest = readManifest(side);
  if (cmd === "inspect") {
    stdout.write(`${JSON.stringify(manifest, null, 2)}\n`);
    return 0;
  }
  const cliSeed = args.seed !== undefined;
  const cliCase = args.caseMode !== undefined;
  applyManifestRunOptions(args, manifest, argv);
  applyManifestLanguage(args, manifest, path);
  if (cmd === "run" && replayLocked(manifest)) {
    const caseKey = (mode) => (mode == null || mode === "default" || mode === "first" ? "first" : mode);
    const caseChanged = cliCase && caseKey(args.caseMode) !== caseKey(manifest.case);
    const nsfwChanged = argv.includes("--nsfw") && args.nsfw !== Boolean(manifest.nsfw);
    const storyChanged = argv.includes("--story") && args.story !== Boolean(manifest.story);
    if (caseChanged || nsfwChanged || storyChanged) {
      throw new Error("locked artifact run rejects recipe overrides; pass --seed for a new instance or update the manifest");
    }
  }
  if (cmd === "run" && args.seed === undefined) {
    args.seed = String(BigInt(Date.now()) * 1000n + BigInt(process.hrtime.bigint() % 1000n));
  }
  const loaded = loadDicts(args);
  const dictJson = dictionaryJson(loaded);
  verifyPattern(pattern, manifest);
  preflight(pattern, { ...loaded, nsfw: args.nsfw });
  verifyLock(manifest, dictJson, { baseDir: dirname(resolvePath(path)) });
  const runOutput = (current) => {
    const out = output(pattern, {
      ...current,
      seed: args.seed,
      nsfw: args.nsfw,
      case: args.caseMode,
      story: false,
    });
    if (out.unresolved?.length) {
      throw new Error(`UNRESOLVED_QUERY: <${out.unresolved[0].raw}>`);
    }
    return out;
  };
  if (cmd === "verify") {
    const rec = cliSeed ? receiptPath(path, args.seed, manifest.seed) : receiptPath(path);
    if (existsSync(rec)) {
      const receipt = readReceipt(rec);
      if (receipt.seed) args.seed = encodeManifestSeed(receipt.seed);
      const out = runOutput(loadDicts(args));
      const check = verifyReceipt(receipt, out.text, pattern, out.channels ?? {});
      if (check.replayed) stdout.write(`ok ${manifest.patternHash} receipt\n`);
      else stdout.write(`ok ${manifest.patternHash} (legacy receipt; recipe verified, receipt not replayed)\n`);
      return 0;
    }
    if (replayLocked(manifest)) stdout.write(`ok ${manifest.patternHash} (recipe; no receipt)\n`);
    else stdout.write(`ok ${manifest.patternHash} (formatVersion ${manifest.formatVersion}; replay not locked)\n`);
    return 0;
  }
  const out = runOutput(loaded);
  const display = render(pattern, args);
  const seed = seedRecord(args.seed);
  const recPath = cliSeed ? receiptPath(path, seed, manifest.seed) : receiptPath(path);
  writeReceipt(recPath, {
    formatVersion: RECEIPT_FORMAT_VERSION,
    patternHash: manifest.patternHash,
    runProfile: manifest.runProfile,
    text: out.text,
    channels: out.channels ?? {},
    ...(seed ? { seed } : {}),
  });
  printOut(display);
  return args.story ? storyExit(display) : 0;
}

function handleReplCmd(line, args) {
  const [name, ...rest] = line.split(/\s+/);
  const arg = rest.join(" ").trim();
  switch (name) {
    case "q":
    case "quit":
    case "exit":
      return "quit";
    case "help":
    case "h":
      printHelp();
      return "ok";
    case "seed":
      args.seed = arg || undefined;
      stderr.write(arg ? `seed: ${arg}\n` : "seed: (none)\n");
      return "ok";
    case "case":
      args.caseMode = arg || undefined;
      stderr.write(arg ? `case: ${arg}\n` : "case: default\n");
      return "ok";
    case "prove":
    case "explain":
      args.explain = !args.explain;
      args.channels = false;
      stderr.write(`prove: ${args.explain ? "on" : "off"}\n`);
      return "ok";
    case "channels":
      args.channels = !args.channels;
      args.explain = false;
      stderr.write(`channels: ${args.channels ? "on" : "off"}\n`);
      return "ok";
    case "story":
      args.story = !args.story;
      if (args.story) {
        args.explain = true;
        args.channels = false;
      }
      stderr.write(`story: ${args.story ? "on" : "off"}\n`);
      return "ok";
    default:
      stderr.write(`unknown command :${line} — try :help\n`);
      return "ok";
  }
}

async function repl(args) {
  stderr.write(`skald-lang ${VERSION} — type a pattern, or :help\n`);
  const rl = createInterface({ input: stdinFd, output: stderr, prompt: "skald> " });
  rl.prompt();
  for await (const raw of rl) {
    const line = raw.trim();
    if (!line) {
      rl.prompt();
      continue;
    }
    if (line.startsWith(":")) {
      if (handleReplCmd(line.slice(1).trim(), args) === "quit") {
        rl.close();
        return 0;
      }
      rl.prompt();
      continue;
    }
    if (line === "exit" || line === "quit") {
      rl.close();
      return 0;
    }
    try {
      printOut(render(line, args));
    } catch (err) {
      stderr.write(`${err instanceof Error ? err.message : err}\n`);
    }
    rl.prompt();
  }
  stderr.write("\n");
  return 0;
}

function readStdin() {
  return readFileSync(0, "utf8");
}

function main(argv = process.argv.slice(2)) {
  try {
    const args = parseArgs(argv);
    if (args.help) {
      printHelp();
      return 0;
    }
    if (args.version) {
      stdout.write(`${VERSION}\n`);
      return 0;
    }
    const artifactCode = artifactCommand(args, argv);
    if (artifactCode != null) return artifactCode;
    let pattern = args.eval ?? (args.rest.length ? args.rest.join(" ") : undefined);
    if (args.file) pattern = readFileSync(args.file, "utf8");
    if (pattern) {
      const text = render(pattern, args);
      printOut(text);
      return args.story ? storyExit(text) : 0;
    }
    if (process.stdin.isTTY) {
      return repl(args);
    }
    const piped = readStdin();
    if (!piped.trim()) {
      printHelp();
      return 1;
    }
    const text = render(piped, args);
    printOut(text);
    return args.story ? storyExit(text) : 0;
  } catch (err) {
    stderr.write(`${err instanceof Error ? err.message : err}\n`);
    return 1;
  }
}

const code = main();
if (code && typeof code.then === "function") {
  code.then((c) => process.exit(c));
} else {
  process.exit(code);
}
