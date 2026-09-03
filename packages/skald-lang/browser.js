import init, { Engine } from "./pkg/skald_wasm.js";
import { createApi } from "./lib.js";

await init();
const dict = await fetch(new URL("./en-us.json", import.meta.url)).then((r) => {
  if (!r.ok) throw new Error(`skald-lang: failed to load dictionary (${r.status})`);
  return r.text();
});

const { skald, compile, output, explain } = createApi(Engine, dict);

export { skald, compile, output, explain, Engine };
