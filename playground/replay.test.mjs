import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { runInNewContext } from "node:vm";
import { test } from "node:test";
import ts from "typescript";
import { explain } from "../packages/skald-lang/index.js";
import * as runner from "../examples/story/runner.mjs";
import { PALETTES } from "../examples/story/palettes.mjs";

// Exercise the real App handlers and generated JSX. Only the UI renderer and
// timers are replaced; story rendering, choice control, and replay use the VM.
const { outputText } = ts.transpileModule(readFileSync(new URL("./app.tsx", import.meta.url), "utf8"), {
  compilerOptions: {
    module: ts.ModuleKind.CommonJS,
    target: ts.ScriptTarget.ES2022,
    jsx: ts.JsxEmit.ReactJSX,
    jsxImportSource: "svenjs",
    esModuleInterop: true,
  },
});
const fixture = JSON.parse(readFileSync(new URL("../examples/story/stable-alternatives.json", import.meta.url), "utf8"));
const { request, draft } = runner.splitStoryDocument(fixture);
const saved = runner.renderStory({ explain }, request, draft, { registry: PALETTES }).artifact;
assert.equal(saved.ok, true);

function createApp() {
  const pending = new Map();
  let timerId = 0;
  const module = { exports: {} };
  const jsx = (type, props) => ({ type, props });
  const imports = {
    svenjs: { create: (config) => config },
    "svenjs/jsx-runtime": { jsx, jsxs: jsx, Fragment: "fragment" },
    "skald-lang": { explain },
    "../examples/story/runner.mjs": runner,
    "../examples/story/palettes.mjs": { PALETTES },
    "../examples/story/prompt.md?raw": "",
    "./svenjs-mark.svg?url": "",
  };
  runInNewContext(outputText, {
    exports: module.exports,
    module,
    // Imported runner functions share the application's JSON object realm in a browser.
    JSON,
    require(name) {
      assert.ok(Object.hasOwn(imports, name), `unexpected import ${name}`);
      return imports[name];
    },
    setTimeout(callback) { const id = ++timerId; pending.set(id, callback); return id; },
    clearTimeout(id) { pending.delete(id); },
  }, { filename: "app.tsx" });
  const app = {
    ...module.exports.App,
    setState(next) { this.state = typeof next === "function" ? next(this.state) : next; },
  };
  app.state = app.initialState();
  app.loadStory(JSON.stringify(fixture));
  return {
    app,
    flush() {
      const callbacks = [...pending.values()];
      pending.clear();
      callbacks.forEach((callback) => callback());
    },
    pending,
  };
}

function nodes(node) {
  if (Array.isArray(node)) return node.flatMap(nodes);
  if (!node || typeof node !== "object") return [];
  return [node, ...nodes(node.props?.children)];
}
function text(children) {
  if (Array.isArray(children)) return children.map(text).join("");
  return children && typeof children === "object" ? text(children.props?.children) : String(children ?? "");
}
function click(app, label) {
  const button = nodes(app.render()).find((node) => node.type === "button"
    && (node.props["aria-label"] === label || text(node.props.children) === label));
  assert.ok(button, `missing button ${label}`);
  assert.ok(!button.props.disabled, `disabled button ${label}`);
  button.props.onClick();
}
function paste(app, value) {
  const editor = nodes(app.render()).find((node) => node.type === "textarea" && node.props.id === "pattern");
  assert.ok(editor);
  const target = { value };
  editor.props.onInput({ target, currentTarget: target });
}

for (const field of ["text", "replayHash"]) {
  for (const action of ["live preview", "Run story", "before debounce"]) {
    test(`reject altered ${field} after ${action} without replacing the editor`, () => {
      const { app, flush, pending } = createApp();
      const original = JSON.stringify({ ...saved, [field]: "tampered" }, null, 2);
      paste(app, original);
      assert.equal(pending.size, 1);
      if (action === "live preview") flush();
      if (action === "Run story") click(app, "Run story");
      assert.notEqual(JSON.parse(app.state.receipt)[field], "tampered", "preview has regenerated saved fields");
      click(app, "Replay this artifact");
      assert.equal(app.state.storyJson, original);
      assert.equal(app.state.error, true);
      assert.ok(app.state.diagnostics.some((row) => row.code === "STORY_REPLAY_MISMATCH"));
      assert.doesNotMatch(app.state.status, /Replay verified/u);
      assert.equal(pending.size, 0);
      flush();
      assert.equal(app.state.storyJson, original);
    });
  }
}

test("valid pasted artifact keeps its original text, hash, and source", () => {
  const { app, flush } = createApp();
  const original = JSON.stringify(saved, null, 4);
  paste(app, original);
  flush();
  click(app, "Replay this artifact");
  assert.equal(app.state.error, false);
  assert.match(app.state.status, /Replay verified/u);
  assert.equal(app.state.storyJson, original);
  assert.equal(app.state.storyArtifact.replayHash, saved.replayHash);
});

test("incomplete saved artifacts cannot fall back to preview verification", () => {
  const { app, flush } = createApp();
  const incomplete = { ...saved };
  delete incomplete.ok;
  delete incomplete.text;
  delete incomplete.replayHash;
  const original = JSON.stringify(incomplete);
  paste(app, original);
  flush();
  click(app, "Replay this artifact");
  assert.equal(app.state.error, true);
  assert.equal(app.state.storyJson, original);
});

test("draft replay uses pending editor changes instead of a stale receipt", () => {
  const { app, pending } = createApp();
  const edited = structuredClone(fixture);
  edited.draft.beats.push("A new ending.");
  paste(app, JSON.stringify(edited));
  assert.equal(pending.size, 1);
  click(app, "Replay this artifact");
  assert.equal(app.state.error, false);
  assert.match(app.state.output, /A new ending\.$/u);
  assert.match(JSON.parse(app.state.storyJson).text, /A new ending\.$/u);
  assert.equal(pending.size, 0);
});

test("an explicit variation on a saved artifact creates a replayable new artifact", () => {
  const { app, flush } = createApp();
  paste(app, JSON.stringify(saved));
  flush();
  click(app, "Vary shirt");
  const varied = JSON.parse(app.state.storyJson);
  assert.notEqual(varied.text, saved.text);
  assert.notEqual(varied.replayHash, saved.replayHash);
  assert.equal(varied.replayHash, app.state.storyArtifact.replayHash);
  click(app, "Replay this artifact");
  assert.equal(app.state.error, false);
  assert.match(app.state.status, /Replay verified/u);
  assert.equal(app.state.storyArtifact.replayHash, varied.replayHash);
});
