# Editorial eval (not CI)

Run only with an explicit network/LLM budget. Structural tests remain authoritative.

Score each brief × draft × render:

1. Schema-pass on first model attempt
2. Repair success within two retries
3. Grammar and collocation
4. Causal links between beats
5. Referent clarity (same `::id` throughout)
6. Unwanted repetition
7. Contrast: open Mad Libs vs frame+Skald vs LLM-only prose

Do not gate CI on glue ratio. Record prompt version (`story-prompt-v1`) with the scores.
