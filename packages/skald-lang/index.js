import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { initSync, Engine } from "./pkg/skald_wasm.js";
import { createApi } from "./lib.js";

const wasmPath = fileURLToPath(new URL("./pkg/skald_wasm_bg.wasm", import.meta.url));
const dictPath = fileURLToPath(new URL("./en-us.json", import.meta.url));

initSync({ module: readFileSync(wasmPath) });

const { skald, compile, output, explain, canonicalSeed } = createApi(
  Engine,
  readFileSync(dictPath, "utf8"),
);

export { skald, compile, output, explain, Engine, canonicalSeed };
export const RUN_PROFILE = "skald-pcg32-v1";
