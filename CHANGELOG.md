# Changelog

## Unreleased (3.0.1)

3.0.0 is a partial PLANv3 delivery. Remaining contracts (no tag until an explicit yes):

- Artifact format 2: locale, language-pack hashes, overlay order, effective dictionary hash, receipt verification
- Preflight: unknown table/form/capability fails `verify` and artifact `run`
- Locale on native CLI (`--pack` / `--locale`) and npm `skald-lang/artifact` export
- Completed editorial eval (14 en-US briefs, nb/nn sets, stored report). Do not invent `llm-only`
- Explicit sync without text autosync for new substitutions
- Browser fixtures and a stored 2.2 benchmark baseline

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
