#!/usr/bin/env node
/** Cold-start the installed npm package in a real browser. No prior Node WASM init. */
import { createServer } from "node:http";
import { existsSync, readFileSync } from "node:fs";
import { spawn } from "node:child_process";
import { extname, join, resolve } from "node:path";

const appDir = resolve(process.argv[2] ?? ".");
const pkg = join(appDir, "node_modules", "skald-lang");
if (!existsSync(join(pkg, "browser.js"))) {
  throw new Error(`skald-lang browser entry missing under ${pkg}`);
}

const MIME = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".wasm": "application/wasm",
  ".ts": "text/plain; charset=utf-8",
  ".map": "application/json; charset=utf-8",
};

function page(kind) {
  const script = kind === "engine"
    ? `const { Engine } = await import("skald-lang/engine");
  const nb = (await import("skald-lang/nb-no.json", { with: { type: "json" } })).default;
  const engine = Engine.fromLanguagePack(JSON.stringify(nb));
  const engineLine = engine.run("<firstname female>", "1", false, "none");
  if (!engineLine) throw new Error("engine-only empty");
  out.textContent = "SMOKE:OK " + engineLine;`
    : `const { skald } = await import("skald-lang");
  const nb = (await import("skald-lang/nb-no.json", { with: { type: "json" } })).default;
  const nn = (await import("skald-lang/nn-no.json", { with: { type: "json" } })).default;
  const en = skald("{A|B}", { seed: 1, case: "none" });
  if (!en) throw new Error("empty top-level browser run");
  const nbLine = skald("<firstname female>", { languagePack: nb, locale: "nb-NO", seed: 1, case: "none" });
  if (!nbLine || nbLine.includes("<")) throw new Error("nb-NO unresolved " + nbLine);
  const nnLine = skald("<firstname female>", { languagePack: nn, locale: "nn-NO", seed: 1, case: "none" });
  if (!nnLine || nnLine.includes("<")) throw new Error("nn-NO unresolved " + nnLine);
  out.textContent = "SMOKE:OK " + [en, nbLine, nnLine].join(" | ");`;
  return `<!doctype html>
<meta charset="utf-8">
<title>skald-lang browser smoke ${kind}</title>
<body>
<pre id="out">pending</pre>
<script type="importmap">
{
  "imports": {
    "skald-lang": "./node_modules/skald-lang/browser.js",
    "skald-lang/engine": "./node_modules/skald-lang/engine-browser.js",
    "skald-lang/nb-no.json": "./node_modules/skald-lang/nb-no.json",
    "skald-lang/nn-no.json": "./node_modules/skald-lang/nn-no.json"
  }
}
</script>
<script type="module">
const out = document.getElementById("out");
try {
  ${script}
} catch (err) {
  out.textContent = "SMOKE:FAIL " + (err && err.stack ? err.stack : String(err));
}
</script>
</body>
`;
}

function chromeBin() {
  const candidates = [
    process.env.CHROME,
    process.env.GOOGLE_CHROME,
    "google-chrome",
    "google-chrome-stable",
    "chromium",
    "chromium-browser",
    "/usr/bin/google-chrome",
    "/usr/bin/google-chrome-stable",
    "/usr/bin/chromium-browser",
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    "/Applications/Chromium.app/Contents/MacOS/Chromium",
  ].filter(Boolean);
  for (const bin of candidates) {
    if (bin.includes("/") && existsSync(bin)) return bin;
  }
  return candidates.find((bin) => !bin.includes("/")) ?? null;
}

function dumpDom(chrome, url) {
  const args = [
    "--headless=new",
    "--disable-gpu",
    "--no-first-run",
    "--virtual-time-budget=8000",
    "--dump-dom",
    url,
  ];
  return new Promise((resolveDump, reject) => {
    const child = spawn(chrome, args, { stdio: ["ignore", "pipe", "pipe"] });
    let stdout = "";
    let stderr = "";
    const timer = setTimeout(() => {
      child.kill("SIGKILL");
      reject(new Error(`chrome timed out: ${stderr.slice(-800) || stdout.slice(-200)}`));
    }, 20000);
    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdout.on("data", (chunk) => { stdout += chunk; });
    child.stderr.on("data", (chunk) => { stderr += chunk; });
    child.on("error", (err) => {
      clearTimeout(timer);
      reject(err);
    });
    child.on("close", (code) => {
      clearTimeout(timer);
      if (code !== 0 && !stdout.includes("SMOKE:")) {
        reject(new Error(`chrome exit ${code}: ${stderr || stdout}`));
        return;
      }
      resolveDump(stdout);
    });
  });
}

const server = createServer((req, res) => {
  const url = new URL(req.url ?? "/", "http://127.0.0.1");
  if (process.env.SKALD_SMOKE_LOG) process.stderr.write(`${req.method} ${url.pathname}\n`);
  if (url.pathname === "/" || url.pathname === "/index.html") {
    res.writeHead(200, { "content-type": MIME[".html"], "cache-control": "no-store" });
    res.end(page("top"));
    return;
  }
  if (url.pathname === "/engine.html") {
    res.writeHead(200, { "content-type": MIME[".html"], "cache-control": "no-store" });
    res.end(page("engine"));
    return;
  }
  const rel = decodeURIComponent(url.pathname.replace(/^\//, ""));
  if (rel.includes("..")) {
    res.writeHead(400);
    res.end("bad path");
    return;
  }
  const file = join(appDir, rel);
  if (!existsSync(file)) {
    res.writeHead(404);
    res.end("missing");
    return;
  }
  const type = MIME[extname(file)] ?? "application/octet-stream";
  res.writeHead(200, { "content-type": type, "cache-control": "no-store" });
  res.end(readFileSync(file));
});

await new Promise((ok) => server.listen(0, "127.0.0.1", ok));
const { port } = server.address();
const chrome = chromeBin();
if (!chrome) {
  server.close();
  throw new Error("no Chrome/Chromium binary for browser smoke");
}

try {
  for (const path of ["/index.html", "/engine.html"]) {
    const url = `http://127.0.0.1:${port}${path}`;
    const dom = await dumpDom(chrome, url);
    if (!dom.includes("SMOKE:OK")) {
      const marker = dom.match(/SMOKE:[^<]*/)?.[0] ?? dom.slice(-500);
      throw new Error(`browser smoke failed ${path}: ${marker}`);
    }
    const ok = dom.match(/SMOKE:OK[^<]*/)?.[0] ?? "SMOKE:OK";
    console.log("browser smoke ok", path, ok);
  }
} finally {
  server.close();
}
