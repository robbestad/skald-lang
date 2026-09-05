# Editorial eval (not CI)

Structural tests remain authoritative. Do not gate CI on glue ratio. Do not use an
AI-detector score as quality.

Protocol version: `eval-1`. Package version stays 2.2.0 until an explicit 3.0 tag.

## Three questions

1. **Is the run correct?** Machine only. Schema, repair, unresolved queries, empty
   referents, locked literals, replay. CI owns this. Do not search output for `<`;
   use runtime diagnostics (`STORY_UNRESOLVED`).
2. **How much variation actually occurred?** Observation, never a gate. Unique
   outputs, collision rate, theoretical closed-group product (after sync), and
   seen alternatives per `variationId`. Small distance is often the goal.
3. **Is the prose editorially sound?** Human or frozen-sample scoring on the rubric
   below. Missing scores are `null`. Mock never invents them.

## Rubric (0 / 1 / 2)

Score only editorial dimensions. 0 = clear break, 1 = mixed or weak, 2 = met.

| Dimension | 0 | 1 | 2 |
| --- | --- | --- | --- |
| grammar | Broken agreement, impossible collocation, or open Mad Libs residue | Awkward but parseable | Grammatical in the requested locale and collocation |
| causality | Beats do not cause or recontextualize each other | Loose sequence, one unearned jump | Each beat follows from prior pressure, choice, or evidence |
| referents | Names or pronouns drift or collide | One unclear callback | Stable identities; locked literals intact |
| repetition | Same dramatic function restated | Motif repeats without new work | Recurring detail changes meaning or cost |
| form | Brief's container ignored (letter summarized, not written) | Mixed container | Surface form matches the brief |
| ending | Stopped, or a speech that was not prepared | Ending present but unearned | Prepared by earlier concrete material |
| voice | Viewpoint or `writingStyle` abandoned | Uneven | Viewpoint, rhythm, and distance hold |

Schema and repair are **not** in this table. They live on the manifest as `machine`.

## Conditions

| Condition | Meaning |
| --- | --- |
| `hybrid` | Full Skald story pipe (frame + substitutions) |
| `llm-only` | Actual model prose **without** Skald substitution. Not a regex-stripped draft. |
| `human` | Committed human text for that brief. Omit the row if none exists. |
| `mad-libs` | Negative control: open queries / collapsed dictionary line |

`--mock` renders committed drafts as `hybrid` and loads `corpus/samples/*.json`.
It does **not** synthesize `llm-only` by stripping `{a\|b}` and `<query>`.
Briefs without a draft or imported sample are listed in the report as omitted.
Sequels record `stateFrom`; they are not silently dropped.

## Blind packet

```bash
node examples/story/corpus/eval.mjs --mock --out packet.json --manifest key.json --report report.json
```

- `packet.json`: shuffled samples with `id`, `briefId`, `kind`, `stateFrom`, `brief`,
  `text`. No `condition`, no scores, no answer key.
- `key.json`: `id` → `{ condition, source, locale, machine, editorial, generation, notes }`.
  `generation` holds provider/model/reasoning/budget/token/cost when an imported
  sample recorded them. Frozen `editorial` 0/1/2 scores may also live here.
  The packet never includes this.
- `report.json`: inventory, omitted briefs (with reasons), missing conditions
  such as real `llm-only`, import errors, frozen editorial counts, and variation
  observations. This file is **not** the blind packet and must not go to raters.

Live generation requires a later PR. `--approve-expensive` still exits 2 and says
the live path is unwired. Frozen imported samples can be scored fully offline.

Release eval (3.0-rc) freezes sample list, seeds, rubric, and scoring rule first.
It covers all 14 en-US briefs plus nb/nn sets once those exist. The bar is no
unresolved systematic regression on causality, form, ending, and voice — a
documented editorial judgement, not a statistical proof.

Offline RC checklist: `bash scripts/rc-verify.sh`. Do not tag or publish from
that script. See [docs/migrate-2.2-to-3.0.md](../../../docs/migrate-2.2-to-3.0.md).
