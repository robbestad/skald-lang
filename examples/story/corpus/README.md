# Story brief corpus

Versioned English briefs for editorial evaluation. **CI runs only structural tests**
(`examples/story/test.mjs`): schema, policy, goldens, multi-seed QA. Glue ratio is
observation, not a gate.

Editorial eval is `eval.mjs --mock` (offline packet + inventory). Live generation is
not wired. See [eval.md](eval.md) for the `eval-1` protocol, rubric, and sample import.

| Id | Kind | Draft | Notes |
| --- | --- | --- | --- |
| `inn` | scene | `../inn.json` | imported mad-libs control |
| `grim` | fairytale | `../grim-fairytale.json` | |
| `ledger` | document | `../ledger.json` | |
| `letter` | letter | `../letter.json` | |
| `testimony` | testimony | `../testimony.json` | |
| `register` | register | `../register.json` | |
| `banter` | dialogue | `../banter.json` | |
| `heist` | humor | `../heist.json` | |
| `notice` | notice | `../notice.json` | |
| `fragments` | rhythm | `../fragments.json` | |
| `quiet` | low-deviation | `../quiet.json` | |
| `three` | ensemble | `../three.json` | |
| `inn-morning` | sequel | `../inn-morning.json` | `stateFrom: inn` |
| `grim-return` | sequel | `../grim-return.json` | `stateFrom: grim` |

Locale is en-US. Norwegian eval lives in `corpus/nb` and `corpus/nn`. Operator reports: `corpus/reports/`. `llm-only` is still missing; do not invent it.

`--mock` lists omitted briefs instead of skipping them silently. It does not invent
`llm-only` by stripping Skald syntax. Import frozen texts under `samples/`.

```bash
node examples/story/corpus/eval.mjs --mock
node examples/story/corpus/eval.mjs --mock --out packet.json --manifest key.json --report report.json
node examples/story/host.mjs state saved-artifact.json
node examples/story/host.mjs loop corpus/briefs/inn-morning.md --mock --state inn-state.json
```
