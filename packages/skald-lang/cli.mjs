#!/usr/bin/env node
import { readFileSync } from "node:fs";
import { skald, output, explain } from "./index.js";

const VERSION = "1.0.0";

function printHelp() {
  process.stderr.write(`Usage: skald-lang [options] <pattern>
       skald-lang [options] -f <file>
       skald-lang [options] -e <pattern>

Generate procedural text from a Skald pattern.

Options:
  -s, --seed <value>   Seed the generator (integer or string)
  -e, --eval <pattern> Pattern on the command line
  -f, --file <path>    Read the pattern from a file
      --case <mode>    none|first|word|title|upper|lower|sentence
      --nsfw           Include NSFW dictionary entries
      --channels       Print JSON with text and named channels
      --explain        Print JSON with text, channels, and dictionary picks
  -h, --help           Show this help
  -v, --version        Show version
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
    help: false,
    version: false,
    rest: [],
  };
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === "-h" || arg === "--help") out.help = true;
    else if (arg === "-v" || arg === "--version") out.version = true;
    else if (arg === "--nsfw") out.nsfw = true;
    else if (arg === "--channels") out.channels = true;
    else if (arg === "--explain") out.explain = true;
    else if (arg === "-s" || arg === "--seed") out.seed = argv[++i];
    else if (arg === "-e" || arg === "--eval") out.eval = argv[++i];
    else if (arg === "-f" || arg === "--file") out.file = argv[++i];
    else if (arg === "--case") out.caseMode = argv[++i];
    else if (arg.startsWith("-")) throw new Error(`Unknown option: ${arg}`);
    else out.rest.push(arg);
  }
  return out;
}

function seedOf(value) {
  if (value === undefined) return undefined;
  if (/^\d+$/.test(value)) return Number(value);
  return value;
}

function main(argv = process.argv.slice(2)) {
  try {
    const args = parseArgs(argv);
    if (args.help) {
      printHelp();
      return 0;
    }
    if (args.version) {
      process.stdout.write(`${VERSION}\n`);
      return 0;
    }
    let pattern = args.eval ?? (args.rest.length ? args.rest.join(" ") : undefined);
    if (args.file) pattern = readFileSync(args.file, "utf8");
    if (!pattern) {
      printHelp();
      return 1;
    }
    const options = {
      seed: seedOf(args.seed),
      nsfw: args.nsfw,
      case: args.caseMode,
    };
    const result = args.explain
      ? JSON.stringify(explain(pattern, options))
      : args.channels
        ? JSON.stringify(output(pattern, options))
        : skald(pattern, options);
    process.stdout.write(result.endsWith("\n") ? result : `${result}\n`);
    return 0;
  } catch (err) {
    process.stderr.write(`${err instanceof Error ? err.message : err}\n`);
    return 1;
  }
}

process.exit(main());
