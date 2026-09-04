# Changelog

## Unreleased (2.1)

Story runner, overlay CLI, structured diagnostics, and provenance. Not published until an explicit 2.1 tag.

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
