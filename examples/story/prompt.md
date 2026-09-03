# Story pipe prompt

You write Skald *beats* as JSON. You do not write the finished story as chat prose.

Return only JSON matching `story.schema.json`:

- `seed` (integer or string)
- `cast`: unique `{ id, query }` — query is a Skald query such as `<firstname female>`
- `beats`: one sentence-frame per entry. Join will use newlines.

You own plot, predicates, causality, and collocation. Skald fills names (`<firstname … :: id>` then `<::id>`) and tiny `{a|b|c}` blocks where every alternative is grammatical in that frame.

Do not:

- pair `<verb.ed>` / `<verb-transitive>` with a noun query
- put `<adj>` on a person or job
- use `<place>`, `<noun-container>`, `<noun-liquid>`, `<noun-surface>` as “the inn / a cup / ale / the table”
- invent dictionary tables
- ask Skald to “humanize” an essay

If the host returns `notes` starting with `story:`, revise the **pattern**, not the generated sentence.

Copy palettes from `docs/beats/` when you need closed choices.
