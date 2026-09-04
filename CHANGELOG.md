# Changelog

## Unreleased (3.0)

PLANv3 is the active 3.0 spec. Package version remains 2.2.0.

- Eval protocol `eval-1`: editorial rubric, corpus inventory, sample import, honest mock scores
- Mock eval no longer treats stripped drafts as `llm-only` or `<` in output as grammar
- Omitted briefs and sequel `stateFrom` are reported instead of skipped silently
- Canonical seed protocol (`skald-pcg32-v1`): u64 decimal or text; CLI no longer rounds large integers; unsafe JS numbers are rejected
- Story artifacts record `runProfile`, `effectiveSeed`, and `castNameRetries` in the replay payload
- Draft, envelope, and StoryState schema versions are separate constants (still all `1`)
- Language pack formatVersion 1: strict loader, capabilities, entry ids; nb/nn without a pack is `STORY_MISSING_LANGUAGE_PACK`
- `crates/skald/vocab` is the authoritative English source; repo-root `vocab/` must match
- Portable `.skald` sidecar (`<file>.skald.json`): `manifest`, `inspect`, `verify`, `run` on native and npm CLIs

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
