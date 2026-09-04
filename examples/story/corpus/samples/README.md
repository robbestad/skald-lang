# Imported eval samples

JSON files here are frozen texts for the blind harness. They are not generated
by `--mock`.

```json
{
  "briefId": "inn",
  "condition": "llm-only",
  "text": "…",
  "locale": "en-US",
  "notes": "optional",
  "generation": {
    "provider": "openai",
    "model": "gpt-5",
    "reasoning": "low",
    "maxModelCalls": 12,
    "maxCostUsd": 0.25,
    "promptTokens": 1000,
    "completionTokens": 400,
    "costUsd": 0.02
  }
}
```

`condition` is one of `hybrid`, `llm-only`, `human`, `mad-libs`. `llm-only` must
be actual model prose without Skald substitution. Do not import a stripped draft
under that name.

`locale` must match the corpus (`en-US` today). Omit it to inherit the corpus
locale. A mismatch is an import error, not a mixed-language packet.

`generation` is optional structured provenance for `llm-only` / frozen `hybrid`.
It is stored on the manifest, never on the blind packet. Unknown generation
fields are rejected. Free-form `notes` are also manifest-only.

`editorial` is optional frozen 0/1/2 scores (or omitted/null). Mock does not
invent scores for rendered hybrids.

Human rows belong here only when a real human text exists. Sequels may use a
sample even when the brief has no committed draft; set `briefId` to the sequel
id (`inn-morning`, `grim-return`).
