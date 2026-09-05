import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { initSync, Engine } from "./pkg/skald_wasm.js";
import { createApi } from "./lib.js";

const wasmPath = fileURLToPath(new URL("./pkg/skald_wasm_bg.wasm", import.meta.url));
const dictPath = fileURLToPath(new URL("./en-us.json", import.meta.url));

initSync({ module: readFileSync(wasmPath) });

const { skald, compile, output, explain, preflight, canonicalSeed, RUN_PROFILE } = createApi(
  Engine,
  readFileSync(dictPath, "utf8"),
);

export { skald, compile, output, explain, preflight, Engine, canonicalSeed, RUN_PROFILE };
