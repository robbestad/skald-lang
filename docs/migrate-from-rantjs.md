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
- Rhyme: `[rhyme:perfect\|slant\|alliteration]` and `::~id` (alias `::&id`). Needs `| pron` on the first hit.
- `explain()` picks include `forms`, `classes`, `form`, `span`
- Unknown tags suggest a neighbor: `[cae]` → `Did you mean [case]?`

## What does not come back

rantjs 3 already dropped Rant 3’s channels, subroutines, query builders, replacers, and targets. Skald does **not** restore those as a tag zoo. Use `[out]`, `[fn]`, `[let]`, and JavaScript/Rust around the pattern.

`$[sub]`, `[chan]`, `[vs]`, `[qname]` / `[qcf]`, and Rant 3’s extra rhyme modes are not Skald.

## Default case

rantjs applies default (first-letter) casing unless you write `[case:none]`. Skald does the same unless you pass `{ case: "none" }` or `[case:none]`.
