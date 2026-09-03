#!/usr/bin/env node
import { readFileSync } from "node:fs";
import { explain } from "../../packages/skald-lang/index.js";

function uniqueIds(cast) {
  const seen = new Set();
  for (const row of cast) {
    if (seen.has(row.id)) throw new Error(`duplicate cast id: ${row.id}`);
    seen.add(row.id);
  }
}

function buildPattern(doc) {
  if (!Array.isArray(doc.beats) || doc.beats.length === 0) {
    throw new Error("beats must be a non-empty array");
  }
  if (doc.cast) {
    uniqueIds(doc.cast);
    for (const row of doc.cast) {
      if (!row.id || !row.query) throw new Error("cast entries need id and query");
    }
  }
  return doc.beats.join("\n");
}

function main(argv = process.argv.slice(2)) {
  const path = argv[0];
  if (!path) {
    process.stderr.write("Usage: node host.mjs <story.json>\n");
    process.exit(1);
  }
  const doc = JSON.parse(readFileSync(path, "utf8"));
  const pattern = buildPattern(doc);
  const result = explain(pattern, {
    seed: doc.seed,
    case: "none",
    story: true,
  });
  const storyNotes = (result.notes ?? []).filter((n) => String(n).startsWith("story:"));
  if (storyNotes.length) {
    process.stdout.write(
      JSON.stringify({ ok: false, pattern, notes: storyNotes }, null, 2) + "\n",
    );
    process.exit(2);
  }
  process.stdout.write(result.text.endsWith("\n") ? result.text : `${result.text}\n`);
}

main();
