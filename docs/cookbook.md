# Skald cookbook

Words come from a dictionary, not from a model. A model may write the **pattern**; it must not write the **sentence**.

`skald --prove --case none '<pattern>'` prints the sentence plus which bits were lexicon rows vs glue.

`--prove` `density.warning` (≥ 50% glue) means two different things:

- **NPC / flavor:** the model already wrote the line. Tighten queries.
- **Story:** expected. You wrote the sentence *frame*; Skald filled names and a few closed choices. 70–80% glue is correct. 80% queries is *Chip ate her*.

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

## Stories (frame vs slot)

The dictionary is a bag of words, not a world model. `<verb.ed>` × `<noun>` will not make a scene. You write the predicate, the causality, and the time. Skald fills *referents*: names, carriers, and tiny `{a|b|c}` blocks where **every** alternative is grammatical in that frame.

**Do not** in a story beat:

- `<verb.ed>` or `<verb-transitive>` plus any noun (`Chip ate her`, `nailed a fishy bass`)
- `<adj>` on a person or a job (`a derogatory hobbit`, `a juicy waiter`)
- `<place>`, `<noun-container>`, `<noun-liquid>`, `<noun-surface>` as if they meant “the inn / a cup / ale / the table” (those classes include closet, toilet, bleach, ceiling)
- `<verb-walk>` as “went” — the class also has *joust* and *stampede*

**Do:**

- `<firstname female :: hero> … <::hero> … <pron poss female>`
- `{walked|came|limped} into the inn`
- `{ale|stew|bread}`, `{said|muttered|whispered}`
- a role as a tiny block: `the {knight|ranger|traveler}`

### Beats (same cast)

Arrival, order, reply, outside, leaving. Share `::hero` / `::other`.

```
<firstname female :: hero> the {knight|ranger|traveler} and <firstname male :: other> the {liar|thief|priest} {walked|came} to the inn.
<::hero> sat by the {fire|window|door}. <::other> {ordered|asked for} {ale|stew|bread}.
The {innkeeper|boy} brought {a cup|a bowl|a plate} and {left|waited}.
<::other> {said|muttered}, looking at <pron acc female>.
<::hero> {did not answer|drank|stood}.
Outside, the {road|yard} was {dark|quiet|wet}.
{Then|At last} <::hero> {paid|rose|took her pack}. <::other> {smiled|did not follow|watched}.
```

```bash
npx skald-lang --seed 11 --case none --story -f inn.skald
node examples/story/host.mjs examples/story/inn.json
```

Seed 11:

```
Crystal the traveler and Elliot the liar came to the inn.
Crystal sat by the window. Elliot ordered ale.
The boy brought a plate and left.
Elliot muttered, looking at her.
Crystal did not answer.
Outside, the road was dark.
At last Crystal paid. Elliot watched.
```

### Don’t (same brief, open tables)

```
<firstname female :: hero>, [a] <adj> <noun-job>, <verb.ed> toward the <place>.
<firstname male :: other> <verb-transitive ed> [a] <adj> <noun-animal>.
<::other> <verb.ed> <pron acc female>.
```

That is how you get *Rebecca, the groggy knight, fiddled toward the mountain inn* and *Chip ate her*.

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

## Glue to avoid (flavor)

These are mostly literal. `--prove` will warn. A model that emits them as *flavor* has already written the sentence:

```
In a world of endless possibility, the hero walked into the tavern.
```

Turn the *referents* into queries, keep the verb if it has to collocate:

```
<firstname male :: hero> walked into the <place>.
```

Do **not** “tighten” that to `<::hero> <verb.ed> the <place>` unless you want Mad Libs. For a one-line NPC blurb, more queries are fine (`likes to <verb-transitive> …`). For a story beat, the verb stays glue or a three-word `{block}`.

## Receipt

`--prove` JSON fields:

- `picks` — dictionary rows (table, forms, classes)
- `parts` — `{ text, source: "dictionary"|"glue", table? }` in output order
- `notes` — rhyme miss and other runtime hints (e.g. no partner for a `::~` group)
- `density.glue_ratio` — glue characters / output characters
- `density.warning` — set when glue is ≥ 50% of a non-tiny string
