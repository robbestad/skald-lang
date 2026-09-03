import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { initSync, Engine } from "./pkg/skald_wasm.js";

const wasmPath = fileURLToPath(new URL("./pkg/skald_wasm_bg.wasm", import.meta.url));
initSync({ module: readFileSync(wasmPath) });

export { Engine };
