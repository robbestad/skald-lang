# Story brief corpus

Versioned briefs for redaksjonell vurdering. **CI kjører bare strukturelle tester** (`examples/story/test.mjs`): schema, policy, goldens, seed-matrise. Glue-ratio er observasjon, ikke port.

Menneskelig eller LLM-eval er beskrevet i `eval.md` og krever `--approve-expensive` / nettverk. Den inngår ikke i vanlige builds.

| Brief | Draft | Merknad |
| --- | --- | --- |
| `briefs/inn.md` | `../inn.json` | To personer, kro |
| `briefs/grim.md` | `../grim-fairytale.json` | Lenger, samme cast-regel |

Mål som eval.md lister (ikke CI): schema-pass første forsøk, reparasjon innen retry, grammatikk, kollokasjon, kausalitet, referentklarhet, repetisjon, og forskjell Mad Libs vs frame+Skald vs LLM-only.
