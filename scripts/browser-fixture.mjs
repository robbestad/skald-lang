#!/usr/bin/env node
/** Offline browser-entry + language-pack fixture. No network. */
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { Engine } from "../packages/skald-lang/engine.js";
import nb from "../locales/nb-NO.json" with { type: "json" };

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const browserSrc = readFileSync(resolve(root, "packages/skald-lang/browser.js"), "utf8");
if (browserSrc.includes("fetch(")) {
  throw new Error("browser.js must load en-us.json as a module, not via fetch");
}

const engine = Engine.fromLanguagePack(JSON.stringify(nb));
const line = engine.run("<firstname female>", "1", false, "none");
if (!line || line.includes("<")) {
  throw new Error(`offline nb-NO pack failed: ${line}`);
}

const browser = await import(pathToFileURL(resolve(root, "packages/skald-lang/browser.js")).href);
const en = browser.skald("{A|B}", { seed: 1, case: "none" });
if (!en) throw new Error("browser entry produced empty output");

const packed = browser.skald("<firstname female>", {
  languagePack: nb,
  locale: "nb-NO",
  seed: 1,
  case: "none",
});
if (!packed || packed.includes("<")) {
  throw new Error(`browser entry nb-NO failed: ${packed}`);
}

console.log("browser fixture ok", en, line, packed);
