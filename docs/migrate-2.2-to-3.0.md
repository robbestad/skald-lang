# Migrating from Skald 2.2 to the 3.0 contracts

Package version is **3.0.1**. 3.0.0 stays published. 3.0.1 adds artifact
format 2, receipts, dictionary-hash parity, pack-as-run-recipe, preflight
(including empty candidate/regex sets), CLI `--locale`/`--pack`, and eval
`textHash`. Real browser fixtures, alternative-ID sync, and `llm-only`
samples remain open. This is not a rantjs migration;
see [migrate-from-rantjs.md](migrate-from-rantjs.md) for that.

## Seeds

Pass large integers as decimal strings. `9007199254740993` must not go through
`Number()`. Canonical form is a u64 decimal or `text:…`. The run profile is
`skald-pcg32-v1`. Leading zeros (`042`) and whitespace-padded decimals are errors.

## Locale and language packs

`en-US` remains the bundled default. `nb-NO` and `nn-NO` require a language pack
at compile time:

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

Calling `skald(pattern, { locale: "nb-NO" })` without a pack is
`missing language pack`, not a silent English run. `compile().run({ locale })`
is rejected: locale, `languagePack`, `dictionary`, and `merge` are compile-time
only. Norwegian packs do not support English `[a]`, verbal numbers, title case,
or rhyme. Authoritative sources are `locales/nb-NO.json` and `locales/nn-NO.json`;
npm copies are `skald-lang/nb-no.json` and `skald-lang/nn-no.json`.

## Portable artifacts

`.skald` stays raw pattern text. The sidecar path is the pattern path plus
`.json` (`inn.skald` → `inn.skald.json`). Format 2 locks locale, dependency
hashes, and the effective dictionary hash (`to_json` SHA-256). Format 1
imports without claiming locked replay. `run`/`verify` load `--pack`/`--dict`
from the sidecar when those flags are omitted, and resolve dependency paths
relative to the `.skald` file. `run --seed 42` writes
`<stem>.seed-42.receipt.json` and does not overwrite the default receipt;
`verify` replays the receipt seed. Native and npm CLIs: `manifest`, `inspect`,
`verify`, `run`. Artifact mode requires a `.skald` operand or `-f` so
`skald run away` stays a pattern.

## Story substitutions

Substitutions carry `variationId`, `syncGroup`, `origin`, and host-owned
`policy`. Identical `{a|b}` blocks still autosync by text. Distinct `syncGroup`
values stay independent. `[sync:]` is compiled after lint; draft beats must not
contain advanced tags.

## StoryState

Format version 2. Open threads are `{id, text}`. 2.2 string threads import.
Transitions are one atomic patch (`addFacts`, open, close, reopen) with
`patchId` and `baseStateHash`. Closing does not invent a fact. A failed story
run applies no patch. `loop file.json` is a StoryRequest envelope.

## Eval

Protocol `eval-1`. Machine scores and editorial 0/1/2 scores are separate.
The blind packet has no condition labels. `--report` is operator-only (variation
observation, omitted briefs). Hybrid score overlays lock to `textHash` of the
assessed text. There is still no real `llm-only` sample. Live eval stays behind
`--approve-expensive` and is not wired. Do not use an AI detector as a quality
gate. Do not invent `llm-only` by stripping Skald syntax.

## What does not change

- No `[plot]`, world model, or LLM in the VM.
- Full lexical coverage stays opt-in (`policy.fullLexicalCoverage`).
- WASM gzip budget is 500 KB. Language packs are separate JSON, not baked in.

## Release candidate

Verification only — this checklist does not tag or publish.

```bash
bash scripts/rc-verify.sh
node scripts/bench-rc.mjs
```

That run covers Rust fmt/clippy/tests, language-pack copies, WASM gzip budget,
npm/story/eval packet-vs-report split, empty-project package smoke, and
Playground `tsc --noEmit` plus Vite build. Measurements land in
[benchmarks-3.0-rc.md](benchmarks-3.0-rc.md).
