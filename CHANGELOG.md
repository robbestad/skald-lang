# Changelog

## Unreleased

- Locked `verify` rejects recipe-changing `--case`/`--nsfw`/`--story`/`--pron` the same way as `run`. Pronunciation sidecars are stored in the artifact recipe and reloaded on replay.
- Browser smoke reads `#out` only, so `SMOKE:OK` in the script source cannot pass the test, and a forced failure is required to be red.
- Pack overlays cannot replace a multi-form table with a lemma-only row. Strict artifact runs reject unknown recall forms even without pack capabilities. Preflight skips `[fn]` bodies and does not treat plural-regex candidate sets as static empties.
- Story replay restores `storyIntent`/`storyDesign`/`manuscript` and the incoming story state. Implicit close/open patch ids include the current state hash and applied patch history so a later close is not treated as a retry.
- Frozen `docs/benchmarks-3.0.1.json` is not overwritten. Live snapshots go to `docs/benchmarks-latest.json` with commit, platform, and Node metadata.

## 3.0.2

PLANv3dot2 A–E and S. 3.0.0 and 3.0.1 stay published. Remaining: sync on stable alternative IDs, `llm-only` samples.

- Language packs bind locale, capabilities, and dictionary together. Missing capability keys follow the pack locale (nb/nn never get silent English defaults). Table `subs` enforce form length even without top-level `forms`.
- npm `--pack` applies `--dict` overlays the same way as native; `languagePack` + `dictionary` no longer drops the overlay.
- Strict pack/artifact runs reject unknown recall/plural forms and unresolved queries on the taken path. Preflight does not treat rhyme/unique as match-binds, does not leak block-alternative bindings, and uses the actual NSFW flag for candidate sets.
- Artifact receipts are format 2: main text plus named channels, a chosen seed when none was given, and `dictOnly`. Format 1 receipts still verify the recipe; presentation JSON is not treated as a failed replay. Verify checks receipt format/run profile and does not call a missing receipt a completed replay. Locked `run` rejects options that change the recipe, not repeated matching `--pack`/`--dict`/`--case first`.
- Shared `Engine` types for top-level and `skald-lang/engine`. `skald-lang/artifact` ships Node-only declarations. Installed-tarball smoke typechecks a consumer and cold-starts WASM in a real browser. `engines.node` `>=20` is exercised.
- Story Runner artifacts keep locale (and language-pack id) so an nb save does not replay as English. Patches bind `patchId` to a payload hash: identical retry is a no-op, different operations conflict. Eval variation notes say closed-block combinations are counted, not the full dictionary/cast space.
- `explain`/`output` picks keep language-pack `entryId` when the bound row has one. Norwegian teaching fixtures use closed `{døren|vinduet}` / `{døra|vindauget}` instead of a broad definite noun. Benchmarks record compile time separately from compiled `.run()` and report WASM memory distinct from JS heap.

## 3.0.1

Artifact replay contract, empty-candidate preflight, eval textHash, 500 KB WASM budget. 3.0.0 stays published. Not complete PLANv3: real browser fixture, sync on stable alternative IDs, compile-time/memory vs 2.2, and `llm-only` samples remain open.

- Preflight: unknown table/form/unbound carrier/capability fails artifact `verify`/`run` and language-pack `skald()`; legacy English `skald()` still emits `<query>`
- Explicit sync: new substitutions with `variationId` do not text-autosync; the same `syncGroup` may wrap parallel form sets with equal alternative counts
- Artifact format 2: locale, dependency hashes, dictionary hash, receipts. Format 1 imports without claiming locked replay. Story `replayHash` is SHA-256.
- Native/npm `--locale` / `--pack`; npm export `skald-lang/artifact`
- Editorial eval: drafts for all 14 en-US briefs, frozen hybrid scores, nb/nn mini-sets, stored reports. `llm-only` is still missing and is not invented.
- Browser entry loads `en-us.json` as a module (no dictionary fetch). Offline fixture + 2.2 baseline in `docs/benchmarks-2.2.json`.
- Receipts store the effective run seed. `run --seed 42` writes `<stem>.seed-42.receipt.json` and does not overwrite the default receipt. `verify` replays the receipt seed.
- npm hashes the effective dictionary with the same canonical JSON as native `to_json`. Native and npm artifacts share `dictionaryHash`.
- `run`/`verify` load language packs from manifest dependencies, resolved relative to the `.skald` file.
- Preflight rejects empty candidate combinations (`PREFLIGHT_EMPTY_CANDIDATES`), including after regex filters.
- WASM gzip budget is 500 KB.
- Editorial overlays require `textHash` of the assessed hybrid; a changed draft does not inherit old scores. `llm-only` is still missing.

## 3.0.0

Partial PLANv3: language packs, StoryState 2, eval-1 protocol, nb-NO and nn-NO cores. Not a complete replay or editorial-eval release.

- Eval protocol `eval-1`: editorial rubric, corpus inventory, sample import, honest mock scores
- Mock eval no longer treats stripped drafts as `llm-only` or `<` in output as grammar
- Omitted briefs and sequel `stateFrom` are reported instead of skipped silently
- Canonical seed protocol (`skald-pcg32-v1`): u64 decimal or text; CLI no longer rounds large integers; unsafe JS numbers are rejected
- Story artifacts record `runProfile`, `effectiveSeed`, and `castNameRetries` in the replay payload
- Draft and envelope schema versions remain `1`; StoryState is format version `2`
- Language pack formatVersion 1: strict loader, capabilities, entry ids; missing locale pack is `STORY_MISSING_LANGUAGE_PACK`
- `crates/skald/vocab` is the authoritative English source; repo-root `vocab/` must match
- Portable `.skald` sidecar (`<file>.skald.json`): `manifest`, `inspect`, `verify`, `run` on native and npm CLIs
- Story substitutions carry `variationId`, `syncGroup`, `origin`, and `policy`; explicit sync groups skip text autosync; overlapping literals are rejected
- StoryState 2: stable thread ids, `stateHash`, atomic patches (`addFacts` / open / close / reopen), no silent truncation; 2.2 string threads import; failed runs do not apply a patch
- Curated `nb-NO` core pack (noun gender/forms, declared pronouns, no English `[a]`/title-case/verbal numbers); `kaffe` palette; Bokmål abbreviation segmentation; 100-seed fixtures outside Story Runner
- Curated `nn-NO` core pack with the same contract (`ho`/`hennar`, `kattar`/`kattane`, `gjekk`/`opna`); `kaffi` palette; Nynorsk abbreviation segmentation; 100-seed fixtures
- Eval report (separate from the blind packet): variation observation, pairwise manuscript/variant diffs, frozen imported editorial scores; live eval still unwired
- `loop <request.json>` reads a StoryRequest envelope; `state`/`loop` take `--patch` and repeatable `--closed-thread`
- Migration guide for 2.2 → 3.0 contracts (`docs/migrate-2.2-to-3.0.md`)
- RC verification (`scripts/rc-verify.sh`) and size/timing snapshot (`docs/benchmarks-3.0-rc.md`)
- Package smoke installs the npm tarball in an empty project and exercises `skald-lang`, `skald-lang/engine`, language packs, CLI, and artifact commands
- Playground `tsc --noEmit` in CI (Vite build is not a TypeScript check)

## 2.2.0

en-US corpus, host-side StoryState, multi-seed QA, and a blind eval harness.

- 14 English briefs covering scene, document form, dialogue, humor, and sequels
- `extractStoryState` / `applyStoryState`; `host.mjs state` and `loop --state`
- Multi-seed structural QA over committed drafts
- `corpus/eval.mjs --mock` writes a blind packet; live eval is `--approve-expensive`

## 2.1.0

Story runner, overlay CLI, structured diagnostics, provenance, and selective variation.

- Native/npm CLI `--dict` / `--dict-only`; `--story` exit 2 for argument, `-f`, and stdin; `:story` in both REPLs
- WASM `runFull` / `explainFull` / `outputFull` plus `Engine.overlay`; compile-time dictionary; merge defaults to overlay
- `explain`/`output`: `choices`, `partsByChannel`, `picks[].channel`/`emitted`, `unresolved` with source spans
- Story host: schema, cast prelude, palette ids, mock repair loop, seed matrix
- Playground: Pattern and Story JSON modes, palettes, receipt
- CI for Rust, npm/wasm, story tests, playground, package smoke
- `check` / `pattern` / `render` share palette allowlists from `paletteIds`; `loop --palette <id>`
- Staged local revision edits listed beat ranges without a new `compose`; illegal edits are `STORY_REVISION_DRIFT`
- Beat slices reconstruct manuscript whitespace; without Skald substitutions, `artifact.text === manuscript.text`
- StoryDraft schema (`schemaVersion`, `cast`, `beats`) is separate from the host envelope
- Story diagnostics are deduplicated by code, beat, and span
- Playground chip for the inline-cast inn pattern no longer claims to be Story JSON mode
- Selective Skald variation is the default; `policy.fullLexicalCoverage` / `--full-lexical-coverage` restores the old lexical-coverage gate
- Repeated identical `{a|b|c}` blocks are synchronized with `[sync:choiceN;locked]` in the compiled pattern

Do not treat story output as if no word came from a model. Glue and `{a|b|c}` alternatives are pattern-written; Skald chooses the alternative and fills dictionary slots.

## 2.0.0

Story lint, dictionary overlay, partner-aware rhyme.
