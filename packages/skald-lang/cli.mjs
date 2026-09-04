#!/usr/bin/env node
import { createInterface } from "node:readline";
import { readFileSync } from "node:fs";
import { stdin as stdinFd, stdout, stderr } from "node:process";
import { skald, output, explain } from "./index.js";

const VERSION = "2.2.0";

function printHelp() {
  process.stderr.write(`Usage: skald-lang [options] <pattern>
       skald-lang [options] -f <file>
       skald-lang [options] -e <pattern>
       skald-lang                 (REPL, or read stdin if piped)

Generate procedural text from a Skald pattern.

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
    else if (arg.startsWith("-")) throw new Error(`Unknown option: ${arg}`);
    else out.rest.push(arg);
  }
  return out;
}

function seedOf(value) {
  if (value === undefined) return undefined;
  return value;
}

function loadDicts(args) {
  if (!args.dicts.length && !args.dictOnly) return {};
  const tables = {};
  for (const path of args.dicts) {
    const raw = JSON.parse(readFileSync(path, "utf8"));
    Object.assign(tables, raw.tables ?? raw);
  }
  return {
    dictionary: { tables },
    merge: !args.dictOnly,
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
