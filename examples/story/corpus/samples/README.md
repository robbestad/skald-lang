# Imported eval samples

JSON files here are frozen texts for the blind harness. They are not generated
by `--mock`.

```json
{
  "briefId": "inn",
  "condition": "human",
  "text": "…",
  "locale": "en-US",
  "notes": "optional"
}
```

`condition` is one of `hybrid`, `llm-only`, `human`, `mad-libs`. `llm-only` must
be actual model prose without Skald substitution. Do not import a stripped draft
under that name.

Human rows belong here only when a real human text exists. Sequels may use a
sample even when the brief has no committed draft; set `briefId` to the sequel
id (`inn-morning`, `grim-return`).
