import { create } from "svenjs";
import { explain } from "skald-lang";
import { PALETTES } from "../examples/story/palettes.mjs";
import { renderStory } from "../examples/story/runner.mjs";
import promptDoc from "../examples/story/prompt.md?raw";
import svenjsMark from "./svenjs-mark.svg?url";

const STORY_INN = `<firstname female :: hero> the {knight|ranger|traveler} and <firstname male :: other> the {liar|thief|priest} {walked|came} to the inn.
<::hero> sat by the {fire|window|door}. <::other> {ordered|asked for} {ale|stew|bread}.
The {innkeeper|boy} brought {a cup|a bowl|a plate} and {left|waited}.
<::other> {said|muttered}, looking at <pron acc female>.
<::hero> {did not answer|drank|stood}.
Outside, the {road|yard} was {dark|quiet|wet}.
{Then|At last} <::hero> {paid|rose|took her pack}. <::other> {smiled|did not follow|watched}.`;

const EXAMPLES: { title: string; pattern: string }[] = [
  {
    title: "NPC line",
    pattern:
      "<firstname male> likes to <verb-transitive> <noun.plural> with <pron poss male> pet <noun-animal> on <timenoun dayofweek plural>.",
  },
  {
    title: "Same person twice",
    pattern:
      "<firstname male :: hero> walked into the <place> with <pron poss male> <noun-animal>. <::hero> did not knock.",
  },
  {
    title: "Oxford list",
    pattern: "[let:pets; [collect:3; <noun-animal ::!p>]][join:pets; ,\\s; and]",
  },
  {
    title: "Rhyme",
    pattern: "[rhyme:perfect]<noun ::~a> / <noun ::~a>",
  },
  {
    title: "Title + body",
    pattern:
      "[out:title]{[case:title]<adj> <noun>}[case:none]A <noun-animal> entered the <place>.",
  },
  {
    title: "Replace",
    pattern: "[replace: hello world; /world/; {earth}]",
  },
  {
    title: "Map + pattern",
    pattern:
      "[let:row; [map: who; <firstname male>; what; <noun-animal>]][let:tpl; {[who] found [a] [what].}][tpl: row]",
  },
  {
    title: "Standalone story pattern — inline cast",
    pattern: STORY_INN,
  },
];

const STORY_JSON = `{
  "schemaVersion": 1,
  "cast": [
    { "id": "hero", "query": "<firstname female>" },
    { "id": "other", "query": "<firstname male>" }
  ],
  "beats": [
    "<::hero> the {knight|ranger|traveler} and <::other> the {liar|thief|priest} {walked|came} to the inn.",
    "<::hero> sat by the {fire|window|door}. <::other> {ordered|asked for} {ale|stew|bread}."
  ]
}`;

type DemoState = {
  mode: "pattern" | "story";
  pattern: string;
  storyJson: string;
  paletteId: string;
  seed: string;
  output: string;
  picks: string;
  density: string;
  storyLint: boolean;
  storyNotes: string[];
  diagnostics: { code: string; beatIndex: number | null; message: string }[];
  castLine: string;
  partsLine: string;
  choicesLine: string;
  receipt: string;
  status: string;
  error: boolean;
};

let debounceTimer: ReturnType<typeof setTimeout> | undefined;

function parseSeed(value: string): number | string | undefined {
  const trimmed = value.trim();
  if (!trimmed) return undefined;
  return trimmed;
}

function formatOutput(text: string, channels: Record<string, string>): string {
  const named = Object.entries(channels)
    .filter(([k, v]) => k !== "main" && v)
    .sort(([a], [b]) => a.localeCompare(b));
  const lines: string[] = [];
  if (text) lines.push(text);
  for (const [k, v] of named) lines.push(`[${k}] ${v}`);
  return lines.join("\n");
}

function emptyEval(): Pick<
  DemoState,
  | "output"
  | "picks"
  | "density"
  | "storyNotes"
  | "diagnostics"
  | "castLine"
  | "partsLine"
  | "choicesLine"
  | "receipt"
  | "status"
  | "error"
> {
  return {
    output: "",
    picks: "",
    density: "",
    storyNotes: [],
    diagnostics: [],
    castLine: "",
    partsLine: "",
    choicesLine: "",
    receipt: "",
    status: "",
    error: false,
  };
}

function evaluate(
  pattern: string,
  seed: string,
  storyLint: boolean,
): ReturnType<typeof emptyEval> {
  if (!pattern.trim()) {
    return { ...emptyEval(), status: "Write a pattern first." };
  }
  try {
    const result = explain(pattern, {
      seed: parseSeed(seed),
      case: "none",
      story: storyLint,
    });
    const summary = result.picks
      .map((p) =>
        p.carrier ? `${p.table} (${p.carrier})=${p.value}` : `${p.table}=${p.value}`,
      )
      .join(" · ");
    const glue = result.density
      ? result.density.warning
        ? `${Math.round(result.density.glue_ratio * 100)}% glue · ${result.density.queries} dictionary rows — expected for a story frame; rewrite if this was an NPC line`
        : `${Math.round(result.density.glue_ratio * 100)}% glue · ${result.density.queries} dictionary rows`
      : "";
    const storyNotes = (result.notes ?? []).filter((n) =>
      String(n).startsWith("story:"),
    );
    const diagnostics = (result.diagnostics ?? []).map((d) => ({
      code: d.code,
      beatIndex: d.beatIndex,
      message: d.message,
    }));
    return {
      ...emptyEval(),
      output: formatOutput(result.text, result.channels),
      picks: summary,
      density: glue,
      storyNotes,
      diagnostics,
      partsLine: (result.parts ?? [])
        .map((p) => `${p.source}:${JSON.stringify(p.text)}`)
        .join(" · "),
      choicesLine: (result.choices ?? [])
        .map((c) => `${c.kind} alt ${c.alternative} ×${c.repeatIndex}`)
        .join(" · "),
      status: "",
      error: false,
    };
  } catch (err) {
    return {
      ...emptyEval(),
      status: err instanceof Error ? err.message : String(err),
      error: true,
    };
  }
}

function evaluateStory(
  storyJson: string,
  seed: string,
  paletteId: string,
): ReturnType<typeof emptyEval> {
  if (!storyJson.trim()) {
    return { ...emptyEval(), status: "Write a StoryDraft JSON first." };
  }
  try {
    const doc = JSON.parse(storyJson);
    const draft = {
      schemaVersion: doc.schemaVersion ?? 1,
      cast: doc.cast,
      beats: doc.beats,
    };
    const request = {
      seed: parseSeed(seed) ?? doc.seed ?? 11,
      paletteIds: paletteId ? [paletteId] : (doc.paletteIds ?? []),
    };
    const { artifact } = renderStory({ explain }, request, draft, {
      registry: PALETTES,
    });
    const diagnostics = (artifact.diagnostics ?? []).map((d: {
      code: string;
      beatIndex: number | null;
      message: string;
    }) => ({
      code: d.code,
      beatIndex: d.beatIndex,
      message: d.message,
    }));
    const cast = artifact.cast ?? {};
    const castLine = Object.entries(cast)
      .map(([k, v]) => `${k}=${v}`)
      .join(" · ");
    const receipt = JSON.stringify(
      {
        seed: artifact.seed,
        effectiveSeed: artifact.telemetry?.effectiveSeed,
        replayHash: artifact.replayHash,
        paletteIds: artifact.paletteIds,
        pattern: artifact.pattern,
        text: artifact.text,
        cast: artifact.cast,
        picks: artifact.picks,
        choices: artifact.choices,
        parts: artifact.parts,
        diagnostics: artifact.diagnostics,
        draft: artifact.draft,
      },
      null,
      2,
    );
    const partsLine = (artifact.parts ?? [])
      .map((p: { source: string; text: string }) => `${p.source}:${JSON.stringify(p.text)}`)
      .join(" · ");
    const choicesLine = (artifact.choices ?? [])
      .map(
        (c: { kind: string; alternative: number; repeatIndex: number }) =>
          `${c.kind} alt ${c.alternative} ×${c.repeatIndex}`,
      )
      .join(" · ");
    return {
      ...emptyEval(),
      output: artifact.text ?? "",
      picks: (artifact.picks ?? [])
        .filter((p: { emitted?: boolean }) => p.emitted !== false)
        .map((p: { table: string; value: string; carrier?: string }) =>
          p.carrier ? `${p.table} (${p.carrier})=${p.value}` : `${p.table}=${p.value}`,
        )
        .join(" · "),
      density: artifact.density
        ? `${Math.round(artifact.density.glue_ratio * 100)}% glue · ${artifact.density.queries} dictionary rows`
        : "",
      storyNotes: (artifact.notes ?? []).filter((n: string) =>
        String(n).startsWith("story:"),
      ),
      diagnostics,
      castLine,
      partsLine,
      choicesLine,
      receipt,
      status: artifact.ok ? "" : "Story policy failed. Revise the draft.",
      error: !artifact.ok,
    };
  } catch (err) {
    return {
      ...emptyEval(),
      status: err instanceof Error ? err.message : String(err),
      error: true,
    };
  }
}

export const App = create<Record<string, never>, DemoState>({
  initialState() {
    const pattern = EXAMPLES[0]?.pattern ?? "";
    const seed = "42";
    const storyLint = false;
    return {
      mode: "pattern" as const,
      pattern,
      storyJson: STORY_JSON,
      paletteId: "",
      seed,
      storyLint,
      ...evaluate(pattern, seed, storyLint),
    };
  },
  run() {
    const next =
      this.state.mode === "story"
        ? evaluateStory(this.state.storyJson, this.state.seed, this.state.paletteId)
        : evaluate(this.state.pattern, this.state.seed, this.state.storyLint);
    this.setState({ ...this.state, ...next });
  },
  setMode(mode: "pattern" | "story") {
    const next =
      mode === "story"
        ? evaluateStory(this.state.storyJson, this.state.seed, this.state.paletteId)
        : evaluate(this.state.pattern, this.state.seed, this.state.storyLint);
    this.setState({ ...this.state, mode, ...next });
  },
  loadExample(pattern: string) {
    const storyLint = pattern === STORY_INN;
    this.setState({
      ...this.state,
      mode: "pattern",
      pattern,
      storyLint,
      ...evaluate(pattern, this.state.seed, storyLint),
    });
  },
  reseed() {
    const seed = String(Math.floor(Math.random() * 1_000_000_000));
    const next =
      this.state.mode === "story"
        ? evaluateStory(this.state.storyJson, seed, this.state.paletteId)
        : evaluate(this.state.pattern, seed, this.state.storyLint);
    this.setState({ ...this.state, seed, ...next });
  },
  toggleStoryLint() {
    const storyLint = !this.state.storyLint;
    this.setState({
      ...this.state,
      storyLint,
      ...evaluate(this.state.pattern, this.state.seed, storyLint),
    });
  },
  async copyReceipt() {
    const text = this.state.receipt;
    if (!text) return;
    try {
      await navigator.clipboard.writeText(text);
      this.setState((s: DemoState) => ({
        ...s,
        status: "Copied repair payload.",
        error: false,
      }));
    } catch {
      this.setState((s: DemoState) => ({
        ...s,
        status: "Select the receipt and copy it.",
        error: false,
      }));
    }
  },
  async copy() {
    const text = this.state.output;
    if (!text) return;
    try {
      await navigator.clipboard.writeText(text);
      this.setState((s: DemoState) => ({
        ...s,
        status: "Copied.",
        error: false,
      }));
    } catch {
      const el = this._output as HTMLTextAreaElement | undefined;
      el?.select();
      this.setState((s: DemoState) => ({
        ...s,
        status: "Select the output and copy it.",
        error: false,
      }));
    }
  },
  render() {
    const {
      mode,
      pattern,
      storyJson,
      paletteId,
      seed,
      output,
      picks,
      density,
      storyLint,
      storyNotes,
      diagnostics,
      castLine,
      partsLine,
      choicesLine,
      receipt,
      status,
      error,
    } = this.state;

    return (
      <div className="page">
        <header className="mast">
          <div>
            <p className="eyebrow">Write a pattern, get a sentence</p>
            <h1>Skald</h1>
          </div>
          <nav>
            <a href="https://github.com/robbestad/skald-lang">GitHub</a>
            <a href="https://www.npmjs.com/package/skald-lang">npm</a>
          </nav>
        </header>

        <section className="stage">
          <div className="toolbar">
            <button
              type="button"
              className={mode === "pattern" ? "" : "ghost"}
              onClick={() => this.setMode("pattern")}
            >
              Pattern
            </button>
            <button
              type="button"
              className={mode === "story" ? "" : "ghost"}
              onClick={() => this.setMode("story")}
            >
              Story JSON
            </button>
          </div>
          <label htmlFor="pattern">{mode === "story" ? "StoryDraft JSON" : "Pattern"}</label>
          <textarea
            id="pattern"
            spellcheck={false}
            value={mode === "story" ? storyJson : pattern}
            onInput={(e: InputEvent) => {
              const next = (e.target as HTMLTextAreaElement).value;
              if (mode === "story") {
                this.setState({ ...this.state, storyJson: next });
              } else {
                this.setState({ ...this.state, pattern: next });
              }
              clearTimeout(debounceTimer);
              debounceTimer = setTimeout(() => this.run(), 280);
            }}
            onKeyDown={(e: KeyboardEvent) => {
              if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
                e.preventDefault();
                this.run();
              }
            }}
          />
          <div className="toolbar">
            <button type="button" onClick={() => this.run()}>
              Run pattern
            </button>
            <label className="seed">
              Seed
              <input
                id="seed"
                type="text"
                inputMode="numeric"
                placeholder="optional"
                value={seed}
                onInput={(e: InputEvent) => {
                  const next = (e.target as HTMLInputElement).value;
                  this.setState({ ...this.state, seed: next });
                  clearTimeout(debounceTimer);
                  debounceTimer = setTimeout(() => this.run(), 280);
                }}
                onKeyDown={(e: KeyboardEvent) => {
                  if (e.key === "Enter") {
                    e.preventDefault();
                    this.run();
                  }
                }}
              />
            </label>
            <button type="button" className="ghost" onClick={() => this.reseed()}>
              New seed
            </button>
            <button type="button" className="ghost" onClick={() => this.copy()}>
              Copy output
            </button>
            {mode === "pattern" ? (
              <label className="seed">
                <input
                  type="checkbox"
                  checked={storyLint}
                  onChange={() => this.toggleStoryLint()}
                />
                Story lint
              </label>
            ) : (
              <>
                <label className="seed">
                  Palette
                  <select
                    value={paletteId}
                    onChange={(e: Event) => {
                      const next = (e.target as HTMLSelectElement).value;
                      this.setState({
                        ...this.state,
                        paletteId: next,
                        ...evaluateStory(this.state.storyJson, this.state.seed, next),
                      });
                    }}
                  >
                    <option value="">none</option>
                    {Object.keys(PALETTES).map((id) => (
                      <option value={id} key={id}>
                        {id}
                      </option>
                    ))}
                  </select>
                </label>
                <button
                  type="button"
                  className="ghost"
                  disabled={!receipt}
                  onClick={() => this.copyReceipt()}
                >
                  Copy repair payload
                </button>
              </>
            )}
          </div>
          <p className="hint">Live as you type. ⌘/Ctrl + Enter runs now.</p>
          <label htmlFor="output">Sentence</label>
          <textarea
            id="output"
            readOnly
            spellcheck={false}
            value={output}
            ref={(el: HTMLTextAreaElement | null) => {
              this._output = el;
            }}
          />
          {castLine ? <p className="picks">cast {castLine}</p> : null}
          {picks ? <p className="picks">{picks}</p> : null}
          {partsLine ? <p className="density">lineage {partsLine}</p> : null}
          {choicesLine ? <p className="density">choices {choicesLine}</p> : null}
          {density ? <p className="density">{density}</p> : null}
          {diagnostics.length ? (
            <ul className="story-notes" role="status">
              {diagnostics.map((d, i) => (
                <li key={`${d.code}-${i}`}>
                  {d.code}
                  {d.beatIndex != null ? ` · beat ${d.beatIndex + 1}` : ""}: {d.message}
                </li>
              ))}
            </ul>
          ) : null}
          {storyNotes.length ? (
            <ul className="story-notes" role="status">
              {storyNotes.map((note) => (
                <li key={note}>{note}</li>
              ))}
            </ul>
          ) : null}
          {status ? (
            <p
              className={error ? "status error" : "status"}
              role="status"
              aria-live="polite"
            >
              {status}
            </p>
          ) : null}
          {mode === "story" && receipt ? (
            <>
              <label htmlFor="receipt">Receipt</label>
              <textarea id="receipt" readOnly spellcheck={false} value={receipt} />
            </>
          ) : null}
        </section>

        <section>
          <h2>Examples</h2>
          <div className="chips">
            {EXAMPLES.map((example) => (
              <button
                key={example.title}
                type="button"
                className={example.pattern === pattern ? "active" : ""}
                onClick={() => this.loadExample(example.pattern)}
              >
                <span className="chip-kicker">{example.title}</span>
                <span className="chip-pattern">{example.pattern}</span>
              </button>
            ))}
          </div>
        </section>

        <section className="grid">
          <article>
            <h2>Queries</h2>
            <p>
              Angle brackets pull a dictionary <em>entry</em>, not just a string.
              Filters and inflections can be written with a space, dash, or dot.
            </p>
            <pre>{`<firstname male>
<noun-animal plural>
<verb.ed>
<::hero plural>`}</pre>
          </article>
          <article>
            <h2>Lists &amp; functions</h2>
            <p>
              <code>[collect]</code> and <code>[join]</code> make an Oxford list.
              <code>[fn]</code> is the only user-function form.
              <code>[map]</code> is a named bag; <code>[row: who]</code> reads a key.
              <code>[replace]</code> rewrites with a regex; <code>[m]</code> is the match.
            </p>
            <pre>{`[let:pets; [collect:3; <noun-animal ::!p>]]
[join:pets; ,\\s; and]
[fn:greet; name]{Hi [name]}`}</pre>
          </article>
          <article>
            <h2>Rhyme &amp; channels</h2>
            <p>
              <code>::~id</code> is a rhyme group. <code>[out:name]</code> writes a
              named field instead of main text.
            </p>
            <pre>{`[rhyme:perfect]<noun ::~a> / <noun ::~a>
[out:title]{<adj> <noun>}`}</pre>
          </article>
          <article>
            <h2>Stories</h2>
            <p>
              You write the frame. Skald fills names and chooses among tiny{" "}
              <code>{"{a|b|c}"}</code> blocks. Glue is pattern-written; dictionary
              picks are Skald. High glue on a story is expected. Canonical model
              card: <code>examples/story/prompt.md</code>.
            </p>
            <pre>{promptDoc.split("\n").slice(0, 8).join("\n")}</pre>
          </article>
        </section>

        <footer>
          <p>
            Skald 2.0 — sister to{" "}
            <a href="https://github.com/robbestad/Rantjs">rantjs</a>. Dictionary
            compiled from Rantionary. Same VM in native, CLI, and WASM.
          </p>
          <a
            className="svenjs-credit"
            href="https://svenjs.xyz/"
            rel="noopener noreferrer"
          >
            <img
              className="svenjs-mark"
              src={svenjsMark}
              width="36"
              height="36"
              alt="SvenJS"
            />
            <span className="svenjs-credit-copy">
              <span className="svenjs-credit-kicker">UI built with</span>
              <span className="svenjs-credit-name">SvenJS 3.2.1</span>
            </span>
          </a>
        </footer>
      </div>
    );
  },
});
