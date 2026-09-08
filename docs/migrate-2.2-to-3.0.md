# Migrating from Skald 2.2 to the 3.0 contracts

Package version is **3.0.3**. 3.0.0–3.0.2 stay published. 3.0.3 locks verify
to the stored recipe, reads browser smoke from `#out`, and keeps pack overlay
forms and Story replay hashes honest. The unreleased Story Runner adds opt-in
alternative-ID sync, saved choice locks, and targeted rerolls; `llm-only` samples
remain open. This is not a rantjs migration;
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

skald("<firstname female> åpnet {døren|vinduet}.", {
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
`policy`. Legacy blocks without `variationId` still autosync by text. New substitutions
need an explicit `syncGroup` to stay aligned. Distinct groups stay independent.
`[sync:]` is compiled after lint; draft beats must not contain advanced tags.

The unreleased host accepts optional `alternativeIds` aligned with each member's
single flat literal choice block. Every member of an identified group needs the same
ID set; the host compiles them in ID order. Reordering an alternative with its ID
preserves the selected identity. Old variations without this field retain positional
semantics and replay hashes. Adding IDs can change old seeded results, so create a
new artifact when opting in. Selected IDs and original byte spans appear on artifact
choices. See the [Story host example and limits](../examples/story/README.md).

The optional envelope field `choiceState` uses its own `formatVersion: 1` and maps
each saved `syncGroup` to `{alternativeId, locked, rerollCount}` under `groups`.
The host's `lock`, `unlock`, and `reroll` commands, or the corresponding Playground
controls, create it from a successful artifact. Saved selections persist on render;
unlocking enables reroll without changing the current selection. Reroll picks a
different ID in that group while preserving cast and other choices for the same
draft, seed, and language data.

Save a new full StoryArtifact when opting in. Its replay hash includes the selected
IDs, lock flags, and counters; its executable `.skald` also preserves the saved
decisions. A missing group, removed selected alternative, invalid state version, or
attempt to reroll a locked group produces `STORY_CHOICE_CONFLICT`. Old artifacts
without `choiceState` retain their original replay contract. No core RNG or language
syntax changes are required.

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
