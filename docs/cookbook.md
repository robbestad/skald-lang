# Skald cookbook

Words come from a dictionary, not from a model. A model may write the **pattern**; it must not write the **sentence**.

`skald --prove --case none '<pattern>'` prints the sentence plus which bits were lexicon rows vs glue. If `density.warning` is set, the output is mostly template and will still read like the model’s prose.

## Brief → pattern → sentence

**Brief:** a male NPC finds an animal.

```
<firstname male :: hero> found [a] <noun-animal>.
```

```bash
npx skald-lang --prove --seed 42 --case none '<firstname male :: hero> found [a] <noun-animal>.'
```

**Brief:** the same person again, later.

```
<firstname male :: hero> walked into the <place> with <pron poss male> <noun-animal>. <::hero> did not knock.
```

**Brief:** three distinct pets, spoken as a list.

```
[let:pets; [collect:3; <noun-animal ::!p>]][join:pets; ,\s; and]
```

**Brief:** two rhyming nouns.

```
[rhyme:perfect]<noun ::~a> / <noun ::~a>
```

**Brief:** named slots, then a late pattern (the model writes `tpl`, Skald fills the map).

```
[let:row; [map: who; <firstname male>; what; <noun-animal>]]
[let:tpl; {[who] found [a] [what].}]
[tpl: row]
```

**Brief:** rewrite a filled sentence (pattern still owns the words).

```
[replace: <firstname male> found [a] <noun-animal>.; /found/; {met}]
```

**Brief:** a title channel in title case, body left alone.

```
[out:title]{[case:title]<adj> <noun>}[case:none]A <noun-animal> entered the <place>.
```

## NPC / flavor (high query density)

```
<firstname male> likes to <verb-transitive> <noun.plural> with <pron poss male> pet <noun-animal> on <timenoun dayofweek plural>.
```

```
[a] <adj> <noun-animal> <verb.ed> [a] <noun>.
```

```
{heads|tails} — the coin says {heads|tails}.
```

```
{(80)Usually|(20)Rarely}, [n:2;9] <noun-animal plural> appear in the <place>.
```

## Glue to avoid

These are mostly literal. `--prove` will warn. A model that emits them has already written the sentence:

```
In a world of endless possibility, the hero walked into the tavern.
```

Turn that into queries:

```
<firstname male :: hero> walked into the <place>.
```

Still some glue (`walked into the`). Tighter:

```
<firstname male :: hero> <verb.ed> the <place>.
```

## Receipt

`--prove` JSON fields:

- `picks` — dictionary rows (table, forms, classes)
- `parts` — `{ text, source: "dictionary"|"glue", table? }` in output order
- `notes` — rhyme miss and other runtime hints (e.g. no partner for a `::~` group)
- `density.glue_ratio` — glue characters / output characters
- `density.warning` — set when glue is ≥ 50% of a non-tiny string
