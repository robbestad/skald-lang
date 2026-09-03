# skald-lang

WASM build of [Skald](https://github.com/robbestad/skald-lang). Same engine as the Rust CLI; English dictionary loaded beside the wasm (not baked into it).

```bash
npm install skald-lang
npx skald-lang --seed 42 '<firstname male> found [a] <noun-animal>.'
```

```js
import { skald, compile, explain, output } from "skald-lang";

skald("<firstname male> found [a] <noun-animal>.", { seed: 42, case: "none" });

const line = compile("<firstname male>").run({ seed: 1, case: "none" });
const { text, channels } = output("[out:title]{Hi}body", { case: "none" });
const { picks } = explain("<firstname male :: hero>", { seed: 11, case: "none" });
```

Interpreter without English:

```js
import { Engine } from "skald-lang/engine";
const engine = new Engine(JSON.stringify({ tables: {} }));
```

Coming from rantjs 3: see the [migration notes](../../docs/migrate-from-rantjs.md).

Build from the repo root: `scripts/build-npm.sh`.
