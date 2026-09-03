# Migrating from rantjs 3 to Skald

rantjs 3 stays the JavaScript dialect. Skald is a new language with the same pattern shape (`<>` `{}` `[]`) and a small stdlib. Seeds are **not** portable: Skald uses PCG32, rantjs does not.

Install: `npm install skald-lang`

```js
// rantjs
import { rant, compile, explain } from "rantjs";
rant(pattern, { seed: 42 });

// Skald
import { skald, compile, explain, output } from "skald-lang";
skald(pattern, { seed: 42, case: "none" });
```

## What maps

| rantjs 3 | Skald |
| --- | --- |
| `rant(pattern)` | `skald(pattern)` |
| `compile(pattern).run()` | `compile(pattern).run()` |
| `explain(pattern)` → `{ text, picks }` | `explain(pattern)` → `{ text, channels, picks }` |
| `{ seed, nsfw, dictionary }` | same, plus `{ case }` |
| `import … from "rantjs/engine"` | `import { Engine } from "skald-lang/engine"` |
| `npx rantjs` | `npx skald-lang` |
| `<table filter inflection>` | same (`-` `.` space) |
| `<::hero>` match carrier | same; the binding is the **entry**, so `<::hero plural>` is the other form |
| `::!id` unique | same |
| `{a\|b}`, `(weight)`, `[rep]`, `[sep]`, `[rs]`, `[a]`, `[case]`, `[n]`, `[numfmt]`, `[if]`, `[chance]`, `[x]`, `[protect]`, `\C` | same |
| custom `{ tables: { … } }` | same JSON shape; Skald also stores `phones` when present |

## What Skald adds

- `[let]`, `[collect]`, `[join]`, `[len]`, `[pick]`
- `[fn:name; params]{body}` then `[name: args]`
- `[out:name]{…}` named channels; `output()` / `--channels`
- Regex queries: `<firstname ~ /^[AEIOU]/>`, `<noun-animal !~ /cat\|dog/>`
- `[replace: input; /pat/; body]` with `[m]` / `[m1]` per match
- `[map: k; v; …]` named bag; read with `[name: key]`; a Pattern `[tpl: row]` spreads the keys. No `get`/`set`/`keys` family.
- Rhyme: `[rhyme:perfect\|slant\|alliteration\|weak\|syllabic]` and `::~id` (alias `::&id`). Needs `| pron` on the first hit. A miss after that stays `<raw>` and `explain().notes` names the group.
- `Options.budget` for step / output / depth caps (defaults unchanged)
- Named `[out]` channels keep the case that was active when they were written; main uses the final `[case]`
- `explain()` picks include `forms`, `classes`, `form`, `span`
- Unknown tags suggest a neighbor: `[cae]` → `Did you mean [case]?`

## Out of scope

rantjs 3 already dropped Rant 3’s channels, subroutines, query builders, replacers, and targets. Skald does **not** restore those as a tag zoo. The full list lives in the README section **Out of scope**; in short:

| Not Skald | Instead |
| --- | --- |
| `[qname]`, `[qcf]`, `[qsub]` | `<noun-animal ::!p>` |
| `` [`regex`: …] `` | `[replace: input; /pat/; body]` |
| `$[sub]`, `[after]` | `[fn]` |
| `[chan]`, public/private/internal | `[out:name]` |
| targets, flags, `[vs]`, `[while]`, `pipe` | `[x]`, `[if]`, `[rep]`, host loops |
| `ladd` / `lmap` / … | `[collect]` `[join]` `[pick]` `[len]` |
| object get/set/keys zoo | `[map]` + `[name: key]` |
| extra Rant 3 rhyme *tags* | `[rhyme:weak]` / `[rhyme:syllabic]` as phone-keys, or no |
| `.rantpkg`, Rant 4 VM | this package |
| Paste an essay into Skald to “de-AI” it | Pattern in, sentence out. A model may write the pattern. |

Seeds are not portable from rantjs or Rant.

## Default case

rantjs applies default (first-letter) casing unless you write `[case:none]`. Skald does the same unless you pass `{ case: "none" }` or `[case:none]`.
