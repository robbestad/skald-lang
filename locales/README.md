# Language packs

`nb-NO.json` and `nn-NO.json` are curated Bokmål and Nynorsk cores for Skald 3.0
work. They are original example data (ISC), not dumps of Norsk ordbank. Load them
as language packs; they are not baked into the WASM binary.

```js
import { skald } from "skald-lang";
import nb from "skald-lang/nb-no.json" with { type: "json" };
import nn from "skald-lang/nn-no.json" with { type: "json" };

skald("<firstname female> åpnet <noun n definite>.", {
  languagePack: nb,
  locale: "nb-NO",
  seed: 1,
  case: "none",
});
skald("<firstname female> opna <noun n definite>.", {
  languagePack: nn,
  locale: "nn-NO",
  seed: 1,
  case: "none",
});
```

See [docs/migrate-2.2-to-3.0.md](../docs/migrate-2.2-to-3.0.md) for the 2.2 → 3.0
locale contract. Capabilities: no English `[a]`, no verbal numbers, no English title case, no rhyme.
Nouns carry gender as classes (`m` / `f` / `n`) and four forms. Pronouns are
declared rows, not inferred from names. The two packs are not interchangeable:
Bokmål `hun`/`katter` vs Nynorsk `ho`/`kattar`.
