# Editorial eval (not CI)

Structural tests remain authoritative. Do not gate CI on glue ratio. Do not use an
AI-detector score as quality.

## Blind packet

`node examples/story/corpus/eval.mjs --mock --out packet.json --manifest key.json`

- `packet.json`: shuffled samples labeled only `id` / `briefId` / `text`
- `key.json`: answer key mapping `id` → `{ condition, scores }` (`hybrid` or `llm-only`)

Human samples are included only when a real human text is supplied; briefs without a draft are omitted.

Score each sample on: schema, repair, grammar, causality, referents, repetition, form,
ending. Record `promptVersion`, `skaldVersion`, and seed with the sheet.

Live runs require `--approve-expensive` plus an explicit provider budget
(`--max-model-calls`, `--max-cost-usd`). They are not wired as a default CI job.

Contrast conditions:

1. **hybrid** — full Skald story pipe
2. **llm-only** — prose without Skald substitutions
3. **human** — optional committed sample

Do not treat story output as if no word came from a model.
