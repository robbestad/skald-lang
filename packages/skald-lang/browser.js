import init, { Engine } from "./pkg/skald_wasm.js";
import { createApi } from "./lib.js";
import en from "./en-us.json" with { type: "json" };

await init();
const dict = JSON.stringify(en);

const { skald, compile, output, explain, preflight, dictionaryJson, canonicalSeed, RUN_PROFILE } = createApi(Engine, dict);

export { skald, compile, output, explain, preflight, dictionaryJson, Engine, canonicalSeed, RUN_PROFILE };
