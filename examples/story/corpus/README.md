# Story brief corpus

Versioned English briefs for editorial evaluation. **CI runs only structural tests**
(`examples/story/test.mjs`): schema, policy, goldens, multi-seed QA. Glue ratio is
observation, not a gate.

Human or LLM eval is `eval.mjs` and requires `--mock` (offline harness) or
`--approve-expensive` (live model, not part of ordinary builds).

| Id | Kind | Draft |
| --- | --- | --- |
| `inn` | scene | `../inn.json` |
| `grim` | fairytale | `../grim-fairytale.json` |
| `ledger` | document | `../ledger.json` |
| `letter` | letter | — |
| `testimony` | testimony | — |
| `register` | register | — |
| `banter` | dialogue | `../banter.json` |
| `heist` | humor | `../heist.json` |
| `notice` | notice | — |
| `fragments` | rhythm | — |
| `quiet` | low-deviation | — |
| `three` | ensemble | — |
| `inn-morning` | sequel | state from inn |
| `grim-return` | sequel | `../grim-return.json` |

Locale is en-US. Norwegian does not belong in this corpus.

```bash
node examples/story/corpus/eval.mjs --mock
node examples/story/host.mjs state saved-artifact.json
node examples/story/host.mjs loop corpus/briefs/inn-morning.md --mock --state inn-state.json
```
