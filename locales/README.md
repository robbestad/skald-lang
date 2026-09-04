# Language packs

`nb-NO.json` is a curated Bokmål core for Skald 3.0 work. It is original example
data (ISC), not a dump of Norsk ordbank. Load it as a language pack; it is not
baked into the WASM binary.

```js
import { skald } from "skald-lang";
import nb from "skald-lang/nb-no.json" with { type: "json" };

skald("<firstname female> åpnet <noun n definite>.", {
  languagePack: nb,
  locale: "nb-NO",
  seed: 1,
  case: "none",
});
```

Capabilities: no English `[a]`, no verbal numbers, no English title case, no rhyme.
Nouns carry gender as classes (`m` / `f` / `n`) and four forms. Pronouns are
declared rows, not inferred from names. `nn-NO` is not installed yet.
