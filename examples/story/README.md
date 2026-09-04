# Story host

LLM writes beats. Skald fills names. This directory is the pipe — not a story VM.

```bash
# from repo root, after ./scripts/build-npm.sh
node examples/story/host.mjs check examples/story/inn.json
node examples/story/host.mjs render examples/story/inn.json
node examples/story/host.mjs render examples/story/inn.json --json
node examples/story/host.mjs loop --brief "Two travelers reach an inn." --deviation 35 --expansion 50 --theme "dry humor" --mock
node examples/story/test.mjs
```

One pipe: **brief → check → render → receipt**. `loop --mock` runs the repair loop offline. `render --json` is the StoryArtifact (seed, cast, picks, choices, diagnostics, replay hash). Pattern glue is model-written; dictionary picks are Skald; `{a|b|c}` words are pattern-written but Skald-chosen.

In the programmatic `StoryRequest`, user-authored premise and form constraints belong
in `narrativeBrief`. The runner forwards that exact field to every model attempt and
records it in the artifact. It is untrusted model input and is deliberately separate
from the host-owned numeric `seed`. The older request spelling `brief` remains an
input alias, but model adapters receive only the canonical `narrativeBrief` name.

The brief is creatively binding even though it is not operationally trusted. The
model must realize its causal plot, viewpoint, form, rhythm, fixed facts, and ending
inside the beat frames. In particular, a requested artifact form (work papers,
letters, testimony, and so on) must become the surface form of the beats rather than
being summarized by ordinary narration. These are model-contract requirements; the
deterministic validator checks structure and Skald safety, not literary fidelity.
Adapters may additionally implement `review({ narrativeBrief, draft, prompt })`. The
runner then applies a structured editorial gate before rendering and feeds stable
creative diagnostics back through the same bounded repair loop. The OpenAI example
adapter enables this gate; offline adapters that omit `review` retain deterministic
structural-only behavior.

Adapters may also implement `plan(...)`. The resulting StoryIntent separates immutable
anchors from a small number of new causal or relational developments and translates
theme into `use`/`avoid` constraints. Review responses identify minimal replacement
ranges. The runner rejects cast changes, beat-count changes, and edits outside those
ranges, keeping accepted material byte-for-byte stable. Idea-level repetition is a
separate blocking diagnostic when added beats keep demonstrating one trait instead of
creating causal or relational movement.
Hard dimensions (identity, form, causality, ending, canonical facts) must pass fully;
softer qualities pass on an aggregate threshold instead of requiring artificial
perfection in every category.

The OpenAI example defaults both drafting and editorial review to `gpt-4.1` because
complex form-bound briefs require the drafting model to act on semantic diagnostics.
Callers can still pass `model: "gpt-4.1-mini"` when cost matters more than literary
fidelity, independently of `reviewModel`.

`cast` may be empty. Proper names and named nonhuman identities supplied by the brief
are canonical literals, not generation slots; only unnamed roles that need generated
personal names belong in `cast`. The narrative reviewer scores identity preservation
separately and rejects renamed characters, invented names, and omitted supplied titles.

`StoryRequest.deviation` and `StoryRequest.expansion` are independent 0-100 creative
controls. Deviation governs permission to move beyond the brief's supplied event
sequence. Expansion is a proportional degree of meaningful new development, not a
fixed word target: 0 keeps roughly the brief's scale, while 100 permits development up
to the configured maximum. Word counting provides only a hard safety ceiling—at least
600 words for a short seed, normally up to 4x the brief, capped at 2,000 words by
default. The semantic reviewer judges whether development is proportionate. Defaults
are deviation 35 and expansion 50.

`StoryRequest.theme` is free text such as `dry humor`, `serious`, or a fuller tonal and
thematic instruction. It is delimited as untrusted creative input, locked through
repairs, scored independently by the reviewer, and recorded in replay alongside both
scalar controls.
The default structural ceiling is 48 beats so expanded stories can reach their word
target without forcing paragraph-sized beats; the document-size limit still applies.

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
