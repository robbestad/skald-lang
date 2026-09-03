# Skald

**Write a pattern. Get a sentence.** A dictionary-native generative language — seeded, explainable, and the same VM in native code, the CLI, and WASM.

Sister project to [rantjs](../rantjs). rantjs stays the JavaScript dialect. Skald is the next language: Patterns (`<noun>`, `{a|b}`, `[rep:3]`) with a small set of primitives. Dictionary queries return **entries**, not strings.

```bash
npm install skald-lang
npx skald-lang --seed 42 '<firstname male> found [a] <noun-animal>.'
```

```js
import { skald, compile, explain, output } from "skald-lang";

skald("<firstname male> found [a] <noun-animal>.", { seed: 42 });
compile(pattern).run({ seed: 1 });
explain("<firstname male :: hero> and <::hero>", { seed: 11, case: "none" });
// { text, channels, picks }
```

```rust
use skald::{skald, Options, Seed};

let line = skald(
    "[case:sentence][numfmt:verbal][rs:10;.\\s]{[rn].}",
    &Options {
        seed: Some(Seed::Int(1)),
        ..Options::default()
    },
)?;
```

```
[let:pets; [collect:3; <noun-animal ::!p>]]
[join:pets; ,\s; and]

[fn:greet; name]{{Hi|Hello}, [name]!}
[greet: Ada]

[out:title]{[case:title]<adj> <noun>}
[out:body]{A <noun-animal> entered the <place>.}

<firstname ~ /^[AEIOU]/>
<noun-animal !~ /cat|dog/>

[rhyme:perfect]
<noun ::~a> / <noun ::~a>
```

## Why Skald

- **One function.** `skald(pattern)` returns a string. Compile when you will run it more than once.
- **Entries, not strings.** `<firstname male :: hero>` binds the row. `<::hero plural>` is the same person in another form.
- **Repeatable.** Same pattern plus the same seed is the same sentence (PCG32 — not portable from rantjs).
- **Explainable.** `explain()` lists the dictionary rows that were chosen, with table, classes, and source span.
- **A small language.** Stdlib is ≤ 25 tags. Lists, functions, and channels are a handful of primitives, not a zoo.
- **The same VM everywhere.** Native crate, CLI, and `skald-lang` on npm (WASM + English beside the binary, not baked into it).
- **Budgeted.** 100k steps, 1 MB output, 64 call depth. Overflow is an error, not a hang.
- **NSFW is a flag.** Entries tagged `nsfw` stay out unless you pass `{ nsfw: true }` or query that class.

Use it for NPC chatter, item flavor, test fixtures, prompt variation, worldbuilding, and any place a hardcoded string would go stale.

Coming from rantjs 3? See [docs/migrate-from-rantjs.md](docs/migrate-from-rantjs.md).

## Give it to an LLM

Paste the block below into ChatGPT, Codex, Claude, or another coding assistant, then replace the bracketed brief.

```text
Build [DESCRIBE THE FEATURE] using skald-lang, a procedural text generator (Rust VM, npm package skald-lang).

Install: npm install skald-lang
Import: import { skald, compile, explain, output } from "skald-lang";
Engine only (no English): import { Engine } from "skald-lang/engine";

API:
- skald(pattern, options?: { seed?: number | string, nsfw?: boolean, case?: CaseMode, dictionary?: Dictionary }): string
- compile(pattern, defaults?) -> { run(options?), output(options?), explain(options?) }
- output(pattern, options?) -> { text, channels, picks }
- explain(pattern, options?) -> { text, channels, picks }  // picks are dictionary rows
- CaseMode: none|default|first|word|title|upper|lower|sentence
- CLI: npx skald-lang --seed 42 '<pattern>'

Rules:
- Do not concatenate random words yourself. Write a Skald pattern and call skald() or compile().
- Pass a seed when output must be repeatable (tests, saved content, screenshots).
- Do not invent dictionary tables. Use only the tables listed below.
- Filters and inflections go inside the query. Separators may be space, dash, or dot: <noun animal plural>, <noun-animal plural>, <verb.ed> are the same idea.
- Unknown tags throw (often with "Did you mean"). Unknown queries stay in the output as <raw>; they do not print "undefined".
- Queries return entries. Carriers bind the row: <::hero plural> is the same entry, other form.
- Do not use Rant 3 query builders, replacers, subroutines, or $[sub]. Use [let]/[fn]/[out]/[join] instead.
- this is NOT React. There is no component API.

Pattern dialect:
- Query: <table filter inflection>
- Match carrier: <firstname male :: hero> … <::hero>
  Unique (no repeats in one run): <noun-animal ::!pet>
  Rhyme group: [rhyme:perfect] <noun ::~a> / <noun ::~a>   (also ::&id)
- Regex on the selected form: <firstname ~ /^[AEIOU]/>   <noun-animal !~ /cat|dog/>
- Block (pick one): {heads|tails}     Weighted: {(80)common|(20)rare}
- Repeat next block: [rep:3]{x}       Join: [sep:\s] or [sep:\n] or [rs:3;.]
- Article: [a] next word becomes "a" or "an"
- Case for the finished string: [case:none|first|word|title|upper|lower|sentence]
- Integer: [n:min;max]     Format: [numfmt:verbal|roman|hex]
- Branch: [if:name]{then}{else}   (carrier or [let] binding)
- Chance the next block runs: [chance:50]{maybe}
- Lock two blocks to the same pick: [x:name;locked]{A|B}[x:name;locked]{A|B}
  Other sync: locked|deck|cdeck|forward|reverse|no-repeat|ping|pong
- Bind / lists: [let:name; value]  [collect:n; body]  [join:list; sep; and]  [len:x]  [pick:list]
- Function: [fn:greet; name]{Hi [name]} then [greet: Ada]
- Channel: [out:title]{…}   result.channels.title  (not in main text)
- Random A–Z letter: \C
- Nested braces are allowed.

Dictionary tables (en-US):
abstract, activity, adj, adv, alien, amount, color, conj, country, em, emo,
face, faced, firstname (alias: name), greet, noun, place, prefix, prepos,
preposition (alias: with), pron (alias: pro), quality, rel, say, sconj,
sound, substance, surname, timeadv, timenoun, title, unit, verb, verbimg,
vocal, x, yn

Useful filters / inflections:
- firstname: male, female
- noun: plural (alias pl), animal, tool, surface, furniture, body, liquid,
  insect, clothes, plant, person, ball, fruit, container, job, weapon, food,
  vehicle, shape
- verb: transitive, intransitive, ed, ing, s
- pron: poss, male, female, acc, nom, self
- yn: yes, no
- timenoun: dayofweek, month, holiday, plural
- rel: male, female
- adj, adv, place, color, greet, title, country work as <table> with optional class filters

Rhyme needs pronunciation on the first hit (`| pron` in .dic). Missing phones is a runtime error, not a silent miss. Modes: perfect, slant (alias slant-rhyme), alliteration.

Custom dictionary shape if you must add words:
{
  tables: {
    pet: {
      name: "pet",
      subs: ["default", "plural"],
      entries: [{ forms: ["capybara", "capybaras"], classes: ["animal"], phones: ["k\"{p-i-bArr-V", "k\"{p-i-bArr-Vz"] }],
    },
  },
}

Return working code. Prefer one or two rich patterns over many tiny ones.
```

## Patterns

**Queries** pull a random dictionary entry:

```
<firstname male>
<noun-animal plural>
<verb.ed>
<pron poss male>
<yn yes>
```

**Blocks** choose an alternative. A block with no repeater runs once. `|` splits options. Nested braces work. Prefix an option with `(weight)` to bias it.

```
{heads|tails}
{Example text}
[rep:3][sep:\s]{click|clack}
{(80)common|(20)rare}
```

**Tags**

| Tag | Effect |
| --- | --- |
| `[case:none\|default\|first\|word\|title\|upper\|lower\|sentence]` | Casing for the finished string |
| `[rep:n]` / `[r]` | Repeat the next block `n` times |
| `[sep:\s\|\n\|literal]` | Join those repetitions |
| `[rs:n;sep]` | Repeat and join in one tag |
| `[n:min;max]` | Random integer in range |
| `[numfmt:verbal\|roman\|hex]` | How `[n]` / `[rn]` print |
| `[a]` | Insert *a* or *an* before the next word |
| `[if:name]{then}{else}` | Branch on whether carrier or let `name` is set |
| `[chance:p]{…}` | Run the next block with probability `p` (0–100) |
| `[x:name;locked\|deck\|cdeck\|forward\|reverse\|no-repeat\|ping\|pong]` | Synchronize later blocks |
| `[protect:…]` | Run a block without leaking outer `[rep]`/`[sep]` |
| `[let:name; value]` | Bind a value (string, number, list, entry) |
| `[collect:n; body]` | Build a list |
| `[join:list; sep; and]` | Join a list; two separators is an Oxford list |
| `[len:x]` / `[pick:list]` | Length / pick one |
| `[fn:name; params]{body}` | Define a function; call with `[name: args]` |
| `[out:name]{…}` | Write to a named channel (not main) |
| `[rhyme:perfect\|slant\|alliteration]` | Rhyme mode for `::~id` groups |

**Carriers** remember a result so a character stays the same person:

```
<firstname male :: hero> saw <::hero> in the <place>.
```

**Escapes:** `\C` is a random A–Z letter.

Entries tagged `nsfw` are omitted unless the query asks (`<noun nsfw>`) or you pass `{ nsfw: true }`.

Unknown tags throw. Unknown query tables are left in the output as `<raw>`.

## API

```ts
import { skald, compile, explain, output } from "skald-lang";

skald(pattern);
skald(pattern, { seed: 42, case: "none" });

const line = compile(pattern);
line.run({ seed: 1 });
line.run({ seed: 2 });

output("[out:title]{Hi}body", { case: "none" });
// { text: "body", channels: { main: "body", title: "Hi" }, picks: [] }

explain("<firstname male :: hero>", { seed: 11, case: "none" });
// { text, channels, picks: [{ table, value, forms, classes, form, args, carrier, span }] }
```

Custom dictionary: pass `{ dictionary }` as the JSON object or a JSON string. The WASM engine is `new Engine(dictJson)` from `skald-lang/engine`.

## CLI

```bash
npx skald-lang '<pattern>'
npx skald-lang --seed 7 --case none -f story.skald
npx skald-lang --explain --seed 11 --case none '<firstname male :: hero>'
cargo run -p skald -- --seed 42 '<firstname male> found [a] <noun-animal>.'
```

## Playground

SvenJS 3.2.1 UI. From the repo root:

```bash
./scripts/playground.sh
# open http://127.0.0.1:5174/
```

Or `npm run playground` from `packages/skald-lang`.

## Development

```bash
cargo test -p skald
./scripts/build-npm.sh
node packages/skald-lang/test.mjs    # native == wasm goldens
```

Dictionary sources live in `vocab/` (Rantionary plus a few custom tables). `skald-export-dict` writes `packages/skald-lang/en-us.json`. The wasm core does not embed English.

## License

ISC
