# Story host

LLM writes beats. Skald fills names. This directory is the pipe — not a story VM.

```bash
# from repo root, after ./scripts/build-npm.sh
node examples/story/host.mjs examples/story/inn.json
node examples/story/host.mjs examples/story/grim-fairytale.json
```

- `story.schema.json` — seed, unique cast ids, non-empty beats
- `prompt.md` — give this to a model; lint notes mean revise the pattern
- `host.mjs` — validate, join beats with newlines, `explain({ story: true })`, exit 2 on story notes

Cast queries must appear in the beats as `<firstname female :: hero>` (and later `<::hero>`). The host does not generate names itself.
