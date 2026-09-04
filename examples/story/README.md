# Story host

A model writes the story and Skald parametrizes it. This directory is the model pipe — not a story VM.

```bash
# from repo root, after ./scripts/build-npm.sh
node examples/story/host.mjs check examples/story/inn.json
node examples/story/host.mjs render examples/story/inn.json
node examples/story/host.mjs render examples/story/inn.json --json
node examples/story/host.mjs replay saved-artifact.json
node examples/story/host.mjs pattern examples/story/inn.json --skald my-story.skald
node examples/story/host.mjs loop --brief "Two travelers reach an inn." --deviation 35 --expansion 50 --theme "dry humor" --mock --palette inn
node examples/story/host.mjs state saved-artifact.json
node examples/story/host.mjs loop examples/story/corpus/briefs/grim-return.md --mock --state grim-state.json
node examples/story/corpus/eval.mjs --mock
OPENAI_API_KEY=... node examples/story/host.mjs loop story-brief.md \
  --provider openai --model <provider-model-id> --reasoning low \
  --deviation 60 --expansion 60 --theme "dry humor" \
  --writing-style "close third; varied rhythm; humor through physical timing" \
  --max-model-calls 12 --max-cost-usd 0.25 --artifact mr-egg.json
node examples/story/host.mjs loop story-brief.md \
  --provider ollama --model qwen3:14b --reasoning low \
  --max-model-calls 12 --artifact local-story.json
node examples/story/test.mjs
```

One pipe: **brief → check → render → receipt**. `loop --mock` runs the repair loop offline. `render --json` is the StoryArtifact (seed, cast, picks, choices, diagnostics, replay hash). Pattern glue is model-written; dictionary picks are Skald; `{a|b|c}` words are pattern-written but Skald-chosen.

Save the complete StoryArtifact from the first model-loop run. `replay` reads its
embedded draft, seed, palette ids, and policy and renders it locally through Skald; it
does not construct or call a StoryModel. The same Skald/dictionary/palette versions give
the same output. Keep the artifact as the durable replay input rather than only saving
the rendered prose.

Skald does not add a textual watermark, zero-width marker, or hidden provenance payload.
Model-free patterns can run fully locally. For model-written patterns the artifact
records declared origin and the runtime choices Skald made; it does not remove model
origin or existing watermarks. Provenance (`promptVersion`, `skaldVersion`, hashes,
model-call telemetry, draft, and pattern) is auditable metadata, not a watermark.
No detector can establish from prose alone that arbitrary model-assisted text is or is
not AI-generated, so provenance claims should be based on the saved artifact.

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

The full adapter pipeline is
`plan → design → compose → manuscript review → segment → skaldize → review`.
`compose` writes a coherent manuscript without Skald or beat boundaries. `segment`
preserves that prose in literal sentence frames. `skaldize` proposes exact, indexed
`literal → pattern` substitutions instead of returning rewritten prose; the host applies
them mechanically. Default variation is selective: freeze plot-bearing verbs, motif
words, facts, and character voice; parametrize names, interchangeable details, and
curated micro-actions. Identical `{a|b|c}` blocks are synchronized by the host. Closed
`{original|alternative}` blocks are preferred when an open dictionary query would damage
grammar or collocation. Full lexical coverage is opt-in via
`policy.fullLexicalCoverage` or `loop --full-lexical-coverage`. Legacy `generate`
adapters remain supported.

The pre-segmentation manuscript gate requires consequential change, distinct paragraph
functions, dramatized rather than explained themes, and an ending prepared by concrete
earlier material. Failed diagnostics must quote exact manuscript evidence. Exact titles,
proper names, and mandatory formulae identified as `requiredLiterals` in StoryIntent are
also enforced mechanically before segmentation.

`STORY_WRITERLY_ASIDE` rejects aphoristic narrator commentary, epigrammatic reversals,
and ornamental metaphor templates that pause action to announce wit or significance,
including “X was merely Y wearing Z” and “X was not the same as Y”. The compose and
review contracts require concrete action, dialogue, timing, or consequence instead.

Review chooses local or global revision scope. Local repair can change only listed beat
ranges: it must not call `compose`, must keep cast and beat count fixed, and frozen beats
stay byte-identical. Illegal edits are `STORY_REVISION_DRIFT`. Structural defects in arc,
ordering, causality, motif work, or ending setup return to whole-manuscript composition
before segmentation and Skald substitution run again.

`check`, `pattern`, and `render` apply the same `paletteIds` allowlist. `loop --palette <id>`
repeats to fill `request.paletteIds`. Without Skald substitutions, rendered
`artifact.text` equals the manuscript, including blank lines and indentation.

StoryDraft JSON is only `schemaVersion`, `cast`, and `beats`. Host files are envelopes:
seed, palettes, policy, creative controls, and a nested `draft`. `check` validates the
envelope once; draft analysis validates the nested draft.

## Ways to run

There are four distinct routes:

1. **Manual, no model:** author StoryDraft JSON, then use `check`, `pattern`, `render`,
   or `replay`. No provider configuration or network is involved.
2. **Local model loop:** use `--provider ollama`, an installed model id, and optionally
   `--base-url` or `--context-size`. Story generation and review stay on the Ollama
   host. The default context is 16384 tokens; small contexts can truncate structured
   review or Skald-transform JSON.
3. **Remote model loop:** use `--provider openai` with `OPENAI_API_KEY` and a public
   model id. This route reports estimated token cost for known models.
4. **Interactive model tool:** ask a coding/chat model already working in the project
   to author StoryDraft JSON, then run the same model-free `check`, `pattern`, and
   `render` commands. This uses the current tool as the writing step without requiring
   the Story Runner to make an API or Ollama request. An envelope may record
   `provider`, `model`, and `reasoning` as provenance without making them executable.

Model-loop `loop` runs require explicit `--provider`, `--model`, and `--reasoning`.
The adapter receives the model id verbatim, so use an id exposed by that provider; a
friendly model name configured inside Codex is not automatically a public API model id.
`--review-model` may override the review model. `--mock`, `render`, and `replay` never
contact a provider.

The model loop is optional. Authors can write StoryDraft JSON themselves, validate it with
`check`, export its compiled pattern with `pattern --skald name.skald`, and run the
file directly:

```bash
skald --seed 11 --case none -f name.skald
```

`--artifact name.json` writes the complete receipt and a sibling `name.skald`.
The sibling contains the executable pattern, including the cast prelude, so it can be
edited and rerun without another model call. `--skald another.skald` overrides its path.

For OpenAI runs, `--max-model-calls` stops before a request beyond the limit, while
`--max-cost-usd` stops before the next request after measured estimated cost reaches
the limit. Artifact telemetry records requests, input, cached-input, output, and
reasoning tokens plus estimated USD. Token counts remain exact provider-reported usage;
cost is marked unknown for model ids without a price entry. These limits do not affect
manual authoring, rendering, or replay.
An explicit cost limit is rejected when either selected model lacks a known price,
rather than pretending that an unenforceable zero-dollar estimate is a real cap.

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
to the configured scale reference. Stories may exceed that reference by default; the
semantic reviewer judges whether development is proportionate. A caller that needs an
operational word cap can opt in with `policy.enforceExpansion: true`. Defaults are
deviation 35 and expansion 50.

`StoryRequest.theme` is free text such as `dry humor`, `serious`, or a fuller tonal and
thematic instruction. It is delimited as untrusted creative input, locked through
repairs, scored independently by the reviewer, and recorded in replay alongside both
scalar controls.
`StoryRequest.writingStyle` is a separate free-text surface-form control for viewpoint,
narrative distance, syntax, diction, rhythm, interiority, and comic or serious timing.
It is locked through repairs and stored in replay. `theme` says what the treatment is;
`writingStyle` says how the prose behaves.
The default structural ceiling is 128 beats so complete manuscripts can be segmented
without truncation; expanded stories can reach their word
target without forcing paragraph-sized beats; the document-size limit still applies.

Native overlay (trusted file path, not from a draft):

```bash
cargo run -p skald -- --seed 11 --case none --dict docs/beats/data/inn.json \
  '[case:none]<firstname female> ordered <inn_drink>.'
```

- `runner.mjs` — validate, analyze, prelude, palettes, render, artifact, mock repair loop
- `palettes.mjs` — host registry (`inn`, `forest`, `road`); drafts use `paletteIds`, never paths
- `story-draft.schema.json` — `schemaVersion`, unique cast ids, simple `cast.query`, beats
- `story.schema.json` — host envelope (seed, paletteIds, policy, creative controls, nested draft, optional storyState)
- `story-state.schema.json` — host-side continuation note (identities, facts, motifs); not a VM world model
- `corpus/` — 14 en-US briefs, multi-seed QA, `eval-1` harness (`eval.mjs --mock`; live generation unwired)
- `prompt.md` — give this to a model; diagnostics mean revise the **draft**, not the sentence
- `host.mjs` — Node CLI over the runner (`check` / `render`)

Cast is host prelude: `[out:cast_hero]{<firstname female :: hero>}`. Beats only recall `<::hero>`.
