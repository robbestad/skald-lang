# Story host

LLM writes beats. Skald fills names. This directory is the pipe — not a story VM.

```bash
# from repo root, after ./scripts/build-npm.sh
node examples/story/host.mjs check examples/story/inn.json
node examples/story/host.mjs render examples/story/inn.json
node examples/story/host.mjs render examples/story/inn.json --json
node examples/story/host.mjs loop --brief "Two travelers reach an inn." --mock
node examples/story/test.mjs
```

One pipe: **brief → check → render → receipt**. `loop --mock` runs the repair loop offline. `render --json` is the StoryArtifact (seed, cast, picks, choices, diagnostics, replay hash). Pattern glue is model-written; dictionary picks are Skald; `{a|b|c}` words are pattern-written but Skald-chosen.

In the programmatic `StoryRequest`, user-authored premise and form constraints belong
in `narrativeBrief`. This is untrusted model input and is deliberately separate from
the host-owned numeric `seed`. The older `brief` spelling remains an input alias.

Native overlay (trusted file path, not from a draft):

```bash
cargo run -p skald -- --seed 11 --case none --dict docs/beats/data/inn.json \
  '[case:none]<firstname female> ordered <inn_drink>.'
```

- `runner.mjs` — validate, analyze, prelude, palettes, render, artifact, mock repair loop
- `palettes.mjs` — host registry (`inn`, `forest`, `road`); drafts use `paletteIds`, never paths
- `story.schema.json` — `schemaVersion`, unique cast ids, simple `cast.query`, beats
- `prompt.md` — give this to a model; diagnostics mean revise the **draft**, not the sentence
- `host.mjs` — Node CLI over the runner (`check` / `render`)

Cast is host prelude: `[out:cast_hero]{<firstname female :: hero>}`. Beats only recall `<::hero>`.
