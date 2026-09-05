# Migrating from Skald 2.2 to the 3.0 contracts

Package version is **3.0.0**. That tag is a partial PLANv3 delivery: seeds,
language-pack loader, nb/nn cores, and StoryState 2 are in the package.
Complete replay verification, preflight, CLI locale, and a finished editorial
eval are 3.0.1 work. This is not a rantjs migration;
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
`.json` (`inn.skald` → `inn.skald.json`). In 3.0.0 the sidecar holds
`formatVersion`, `runtimeVersion`, `runProfile`, and SHA-256 of the raw UTF-8
pattern. It does **not** lock language packs, overlay order, or an effective
dictionary hash; `locale` is recorded as `en-US`. `verify` checks the pattern
hash only. Native and npm CLIs: `manifest`, `inspect`, `verify`, `run`.
Artifact mode requires a `.skald` operand or `-f` so `skald run away` stays a
pattern.

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
observation, omitted briefs). 3.0.0 did not complete the release eval: most
briefs are omitted, hybrids are unscored, and there is no `llm-only` sample.
Live eval stays behind `--approve-expensive` and is not wired. Do not use an
AI detector as a quality gate. Do not invent `llm-only` by stripping Skald
syntax.

## What does not change

- No `[plot]`, world model, or LLM in the VM.
- Full lexical coverage stays opt-in (`policy.fullLexicalCoverage`).
- WASM gzip budget remains 400 KB. Language packs are separate JSON, not baked in.

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
