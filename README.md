# Skald

**Write a pattern. Get a sentence whose words did not come from a model.**

Skald is a dictionary engine, not a language model. It never samples a transformer, so there is no SynthID / statistical watermark on the *words*. A model may write the pattern; Skald fills the slots from a lexicon. Sister to [rantjs](../rantjs). Queries return **entries**, not strings.

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

[replace: hello world; /world/; {earth}]

[let:row; [map: who; <firstname male>; what; <noun-animal>]]
[let:tpl; {[who] found [a] [what].}]
[tpl: row]
```

## Why Skald

- **Not a model.** `explain` / `--prove` shows which spans were lexicon vs glue.
- **One function.** `skald(pattern)` returns a string. Compile when you will run it more than once.
- **Entries, not strings.** `<firstname male :: hero>` binds the row. `<::hero plural>` is the same person in another form.
- **Repeatable.** Same pattern plus the same seed is the same sentence (PCG32 — not portable from rantjs).
- **Explainable.** `explain()` lists the dictionary rows that were chosen, with table, classes, and source span.
- **A small language.** Stdlib is ≤ ~27 tags (`replace` and `map` are the extras). Lists, functions, and channels are a handful of primitives, not a zoo.
- **The same VM everywhere.** Native crate, CLI, and `skald-lang` on npm (WASM + English beside the binary, not baked into it).
- **Budgeted.** 100k steps, 1 MB output, 64 call depth — override via `Options.budget`. Overflow is an error, not a hang.
- **NSFW is a flag.** Entries tagged `nsfw` stay out unless you pass `{ nsfw: true }` or query that class.

Use it for NPC chatter, item flavor, test fixtures, prompt variation, worldbuilding, and any place a hardcoded string would go stale.

Coming from rantjs 3? See [docs/migrate-from-rantjs.md](docs/migrate-from-rantjs.md). Pattern recipes (including brief → pattern → sentence): [docs/cookbook.md](docs/cookbook.md). Stories: [examples/story/prompt.md](examples/story/prompt.md) is the canonical model card; `node examples/story/host.mjs check|render|loop` is the pipe. Glue and `{a|b|c}` are pattern-written; Skald fills dictionary slots and chooses the alternative.

## Out of scope

Skald is a generator. New capability has to compose from the stdlib tags, or wait for **one** new stdlib name — not a family of tags.

**Never (not 1.0, not later as a zoo):**

| Leave it out | Use instead |
| --- | --- |
| Query builders (`[qname]`, `[qcf]`, `[qsub]`, …) | Write the query: `<noun-animal ::!p>` |
| Replacer mini-language (`` [`regex`: …] ``) | `[replace: input; /pat/; body]` |
| Subroutines / `$[sub]` / `[after]` | `[fn:name; params]{body}` then `[name: args]` |
| Channels as visibility (`public`/`private`/`internal`, `[chan]`) | `[out:name]{…}` and `output().channels` |
| Targets, flags, `[vs]` | `[x]`, `[if]`, `[let]` |
| Unbounded `[while]`, `pipe` / piping | `[rep]`, `[collect]`, host loops |
| List mutation (`ladd`, `laddn`, `lmap`, …) | `[collect]` + `[join]` + `[pick]` + `[len]` |
| Arithmetic / object / variable zoos (`get`/`set`/`keys`/…) | `[n]`, `[let]`, `[map]` + `[name: key]`, host language |
| Emoji, accent, and other garnish tags | Dictionary entries or host strings |
| Bytecode VM, Turing-complete “full language” | This crate / `skald-lang` |
| A component API | `skald()` returns a string |
| Invented dictionary tables | The en-US list below, or a `{ tables }` you pass in |
| Watermark stripper / «paste an essay, get human text» | Collaboration is *pattern in, sentence out*. Glue stays glue. |

Rhyme modes beyond phone-keys on the same `| pron` data are out — not eight new tags. Seeds are not portable from rantjs.

A model that writes long literal prose *and names it flavor* has already written the sentence. `--prove` warns when output is ≥ 50% glue: for an NPC line, rewrite denser; for a **story**, that warning is the frame doing its job. Stories that are 80% queries read like *Chip ate her*. See [docs/cookbook.md](docs/cookbook.md) **Stories**.

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
- output(pattern, options?) -> { text, channels, picks, notes }
- explain(pattern, options?) -> { text, channels, picks, parts, density, notes }
- CaseMode: none|default|first|word|title|upper|lower|sentence
- CLI: npx skald-lang --seed 42 '<pattern>'   (no args: REPL, or stdin if piped)
- Story lint: npx skald-lang --story --case none '<pattern>'  (JSON; exit 2 if story notes)
- Overlay: skald(pattern, { dictionary: sceneJson }) merges over English; { merge: false } replaces

Rules:
- Do not concatenate random words yourself. Write a Skald pattern and call skald() or compile().
- Pass a seed when output must be repeatable (tests, saved content, screenshots).
- Do not invent dictionary tables. Use only the tables listed below.
- Filters and inflections go inside the query. Separators may be space, dash, or dot: <noun animal plural>, <noun-animal plural>, <verb.ed> are the same idea.
- Unknown tags throw (often with "Did you mean"). Unknown queries stay in the output as <raw>; they do not print "undefined".
- Queries return entries. Carriers bind the row: <::hero plural> is the same entry, other form.
- Never write the finished sentence as chat prose. Write a Skald pattern and run it.
- Flavor / NPC one-liners: keep glue short. Emit <firstname male> found [a] <noun-animal>. — not "Armani found a hedgehog."
- Stories: you write the sentence *frame* (predicate, causality, time). Skald fills *referents* (names, carriers, tiny {a|b|c} where every alternative fits the frame). 70–80% glue is correct. --prove density.warning is expected.
- Story beats must NOT use: <verb.ed> or <verb-transitive> plus a noun; <adj> on a person or job; <place> / <noun-container> / <noun-liquid> / <noun-surface> as stand-ins for "inn / cup / ale / table" (classes are too wide: closet, toilet, bleach, ceiling); <verb-walk> as "went" (includes joust, stampede).
- Story beats MAY use: <firstname female :: hero> … <::hero> … <pron poss female>; {walked|came|limped} into the inn; {ale|stew|bread}; {said|muttered}; the {knight|ranger|traveler}.
- this is NOT React. There is no component API.

Out of scope (do not emit these; they are not Skald):
- Query builders: [qname], [qcf], [qsub], and friends. Write <noun-animal ::!p> instead.
- Replacer mini-language: [`regex`: …]. Use [replace: input; /pat/; body].
- Subroutines: $[sub], [after]. Use [fn] / [name: args].
- Channels: [chan], public/private/internal. Use [out:name] and output().channels.
- Targets, flags, [vs], [while], pipe.
- List mutation: ladd, laddn, lmap. Use [collect], [join], [pick], [len].
- Arithmetic/object/variable zoos (no get/set/keys family). Use [map] + [name: key]. Emoji/accent tags.
- Invented dictionary tables. Only the tables listed below, or a { tables } object the caller passes.
- Do not take an essay and "run it through Skald". That is not a feature. Pattern in, sentence out.

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
  A [let] value that is a block `{…}` is a Pattern (late). [name] runs it.
- Map: [let:row; [map: who; Ada; what; hedgehog]] then [row: who]
  Overlay: [map: row; title; Sir]   Pattern: [tpl: row] spreads the keys
- Function: [fn:greet; name]{Hi [name]} then [greet: Ada]
- Channel: [out:name]{…}   result.channels.title  (not in main text). [case] on a named channel freezes at write; main uses the final mode.
- Replace: [replace: input; /pat/; body]   binds [m] (full match) and [m1]… per group
- Random A–Z letter: \C
- Nested braces are allowed.
- Story frame (names + tiny blocks; verbs stay glue or {walked|came}):
  <firstname female :: hero> the {knight|ranger|traveler} {walked|came} to the inn.

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

Rhyme needs pronunciation on the first hit (`| pron` in .dic). Missing phones is a runtime error, not a silent miss. A miss after the first hit stays `<raw>` and `explain().notes` says which group had no partner. Modes: perfect, slant (alias slant-rhyme), alliteration, weak, syllabic — extra keys on the same phones, not extra tags. Native `--pron file` loads a sidecar (`word X-SAMPA` per line) for rows with no `| pron`.

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

If the brief is a story, return JSON beats for examples/story/host.mjs (or one pattern that shares carriers across 4–8 beats). Do not slot every verb. Prefer the inn-story shape in docs/cookbook.md over open <verb.ed> chains. Run with --story and fix any story: notes.

If the brief is flavor / NPC / test fixtures, prefer high query density.

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
| `[case:none\|default\|first\|word\|title\|upper\|lower\|sentence]` | Casing. Named channels snapshot it at write; main uses the last mode |
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
| `[let:name; value]` | Bind a value (string, number, list, entry, or a `{block}` Pattern) |
| `[collect:n; body]` | Build a list |
| `[join:list; sep; and]` | Join a list; two separators is an Oxford list |
| `[len:x]` / `[pick:list]` | Length / pick one |
| `[fn:name; params]{body}` | Define a function; call with `[name: args]` |
| `[out:name]{…}` | Write to a named channel (not main) |
| `[replace: input; /pat/; body]` | Regex rewrite; `[m]` / `[m1]`… are the match |
| `[map: k; v; …]` | Named bag. Overlay with `[map: bag; k; v]`. Read `[bag: k]`. `[len]`/`[pick]`/`[join]` use values |
| `[rhyme:perfect\|slant\|alliteration\|weak\|syllabic]` | Rhyme mode for `::~id` groups |

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
npx skald-lang --prove --seed 42 --case none '<firstname male> found [a] <noun-animal>.'
cargo run -p skald -- --seed 42 '<firstname male> found [a] <noun-animal>.'
cargo run -p skald -- --seed 11 --case none --dict docs/beats/data/inn.json \
  '[case:none]<firstname female> ordered <inn_drink>.'
skald                 # REPL (tty) or read stdin if piped
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
cargo test --workspace
./scripts/build-npm.sh
node packages/skald-lang/test.mjs    # native == wasm goldens
node examples/story/test.mjs
npm run build --prefix playground
```

Unicode property classes in regex (`\p{L}`) are the opt-in Cargo feature `unicode-regex` (off in wasm so gzip stays under 400 KB).

Dictionary sources live in `vocab/`. `skald-export-dict` writes `packages/skald-lang/en-us.json`. The wasm core does not embed English.

## License

ISC
