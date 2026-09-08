import { create } from "svenjs";
import { explain } from "skald-lang";
import { PALETTES } from "../examples/story/palettes.mjs";
import {
  renderStory,
  rerollStoryChoice,
  setStoryChoiceLock,
  splitStoryDocument,
} from "../examples/story/runner.mjs";
import type { StoryArtifact } from "../examples/story/runner.mjs";
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

const CHOICE_DEMO = JSON.stringify({
  schemaVersion: 1,
  seed: 42,
  paletteIds: [],
  draft: {
    schemaVersion: 1,
    cast: [{ id: "hero", query: "<firstname female>" }],
    beats: [
      "<::hero> wore a {red|blue} shirt and carried a {letter|map}.",
      "Later, <::hero> still wore the {blue|red} shirt. Outside, the sky was {clear|cloudy}.",
    ],
  },
  variations: [
    {
      variationId: "shirt-first", beatIndex: 0, literal: "red", pattern: "{red|blue}",
      syncGroup: "shirt", alternativeIds: ["red", "blue"],
    },
    {
      variationId: "shirt-later", beatIndex: 1, literal: "red", pattern: "{blue|red}",
      syncGroup: "shirt", alternativeIds: ["blue", "red"],
    },
    {
      variationId: "keepsake", beatIndex: 0, literal: "letter", pattern: "{letter|map}",
      syncGroup: "keepsake", alternativeIds: ["letter", "map"],
    },
  ],
}, null, 2);

function identifiedChoices(artifact: StoryArtifact | null) {
  const groups = new Map<string, { alternativeId: string; locked: boolean; canVary: boolean }>();
  for (const choice of artifact?.choices ?? []) {
    if (!choice.syncGroup || !choice.alternativeId) continue;
    const saved = artifact?.choiceState?.groups[choice.syncGroup];
    const variation = artifact?.variations?.find((row) => row.syncGroup === choice.syncGroup);
    groups.set(choice.syncGroup, {
      alternativeId: saved?.alternativeId ?? choice.alternativeId,
      locked: saved?.locked ?? false,
      canVary: (variation?.alternativeIds?.length ?? 0) > 1,
    });
  }
  return [...groups].map(([syncGroup, choice]) => ({ syncGroup, ...choice }));
}

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
  storyArtifact: StoryArtifact | null;
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
  | "storyArtifact"
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
    storyArtifact: null,
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
    const { request: savedRequest, draft } = splitStoryDocument(doc);
    const enteredSeed = parseSeed(seed);
    const request = {
      ...savedRequest,
      // Keep the saved seed type: cast retry seeds distinguish numbers and strings.
      seed: enteredSeed == null || enteredSeed === String(savedRequest.seed)
        ? savedRequest.seed ?? enteredSeed ?? 11
        : enteredSeed,
      paletteIds: paletteId ? [paletteId] : (savedRequest.paletteIds ?? []),
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
    const receipt = JSON.stringify(artifact, null, 2);
    const partsLine = (artifact.parts ?? [])
      .map((p: { source: string; text: string }) => `${p.source}:${JSON.stringify(p.text)}`)
      .join(" · ");
    const choicesLine = (artifact.choices ?? [])
      .map(
        (c) => c.alternativeId != null
          ? `${c.variationId} → ${c.alternativeId} ×${c.repeatIndex}`
          : `${c.kind} alt ${c.alternative} ×${c.repeatIndex}`,
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
      storyArtifact: artifact,
      receipt,
      status: artifact.ok ? "" : (artifact.diagnostics ?? []).some((d) => d.code === "STORY_CHOICE_CONFLICT")
        ? "Resolve the saved choice conflict in the JSON, then run again."
        : "Story policy failed. Revise the draft.",
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
    clearTimeout(debounceTimer);
    const next =
      this.state.mode === "story"
        ? evaluateStory(this.state.storyJson, this.state.seed, this.state.paletteId)
        : evaluate(this.state.pattern, this.state.seed, this.state.storyLint);
    this.setState({ ...this.state, ...next });
  },
  setMode(mode: "pattern" | "story") {
    clearTimeout(debounceTimer);
    const next =
      mode === "story"
        ? evaluateStory(this.state.storyJson, this.state.seed, this.state.paletteId)
        : evaluate(this.state.pattern, this.state.seed, this.state.storyLint);
    this.setState({ ...this.state, mode, ...next });
  },
  loadExample(pattern: string) {
    clearTimeout(debounceTimer);
    const storyLint = pattern === STORY_INN;
    this.setState({
      ...this.state,
      mode: "pattern",
      pattern,
      storyLint,
      ...evaluate(pattern, this.state.seed, storyLint),
    });
  },
  loadStory(storyJson: string) {
    clearTimeout(debounceTimer);
    const { request } = splitStoryDocument(JSON.parse(storyJson));
    const seed = String(request.seed ?? 42);
    this.setState({
      ...this.state,
      mode: "story",
      storyJson,
      seed,
      paletteId: "",
      ...evaluateStory(storyJson, seed, ""),
    });
  },
  replayReceipt() {
    clearTimeout(debounceTimer);
    try {
      const saved: StoryArtifact = JSON.parse(this.state.receipt);
      const seed = String(saved.seed ?? "");
      const next = evaluateStory(this.state.receipt, seed, "");
      const replayed = next.storyArtifact;
      if (!saved.ok || !replayed?.ok || !saved.replayHash
        || replayed.text !== saved.text || replayed.replayHash !== saved.replayHash) {
        throw new Error("The rendered text or replay hash does not match the saved artifact. The editor has been preserved.");
      }
      this.setState({
        ...this.state,
        mode: "story",
        storyJson: this.state.receipt,
        seed,
        paletteId: "",
        ...next,
        status: "Replay verified: text and replay hash match the saved artifact.",
      });
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      this.setState({
        ...this.state,
        diagnostics: [
          ...this.state.diagnostics.filter((d: DemoState["diagnostics"][number]) => d.code !== "STORY_REPLAY_MISMATCH"),
          { code: "STORY_REPLAY_MISMATCH", beatIndex: null, message },
        ],
        status: "Could not verify this replay. The editor has been preserved.",
        error: true,
      });
    }
  },
  changeChoice(syncGroup: string, action: "lock" | "unlock" | "vary") {
    clearTimeout(debounceTimer);
    const current = evaluateStory(this.state.storyJson, this.state.seed, this.state.paletteId);
    const artifact = current.storyArtifact;
    if (!artifact?.ok) {
      this.setState({ ...this.state, ...current });
      return;
    }
    try {
      const choiceState = action === "vary"
        ? rerollStoryChoice(artifact, syncGroup)
        : setStoryChoiceLock(artifact, syncGroup, action === "lock");
      const seed = String(artifact.seed ?? this.state.seed);
      const storyJson = JSON.stringify({
        ...JSON.parse(this.state.storyJson),
        seed: artifact.seed,
        paletteIds: artifact.paletteIds ?? [],
        choiceState,
      }, null, 2);
      const next = evaluateStory(storyJson, seed, "");
      this.setState({
        ...this.state,
        storyJson,
        seed,
        paletteId: "",
        ...next,
        status: next.error ? next.status : action === "vary"
          ? `Varied ${syncGroup}. Other choices are preserved.`
          : `${syncGroup} ${action === "lock" ? "locked" : "unlocked"}.`,
      });
    } catch (err) {
      this.setState({
        ...this.state,
        ...current,
        status: err instanceof Error ? err.message : String(err),
        error: true,
      });
    }
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
        status: "Copied complete story artifact for replay.",
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
      storyArtifact,
      receipt,
      status,
      error,
    } = this.state;
    const choiceGroups = identifiedChoices(storyArtifact);

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
              {mode === "story" ? "Run story" : "Run pattern"}
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
                    <option value="">from JSON</option>
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
                  Copy artifact
                </button>
                <button type="button" className="ghost" onClick={() => this.loadStory(CHOICE_DEMO)}>
                  Try lock &amp; vary
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
          {mode === "story" && choiceGroups.length ? (
            <section className="choice-controls" aria-labelledby="choice-controls-title">
              <h2 id="choice-controls-title">Story choices</h2>
              <p className="choice-help">
                Lock details to keep them. Vary one detail across its occurrences;
                names and other choices stay the same. Changes are saved in the JSON.
              </p>
              <ul className="choice-list">
                {choiceGroups.map((choice) => (
                  <li className="choice-row" key={choice.syncGroup}>
                    <div className="choice-detail">
                      <strong>{choice.syncGroup}</strong>
                      <span>Selected: <code>{choice.alternativeId}</code></span>
                    </div>
                    <div className="choice-actions">
                      <button
                        type="button"
                        className={choice.locked ? "" : "ghost"}
                        aria-label={`${choice.locked ? "Unlock" : "Lock"} ${choice.syncGroup}`}
                        aria-pressed={choice.locked}
                        disabled={!storyArtifact?.ok}
                        onClick={() => this.changeChoice(choice.syncGroup, choice.locked ? "unlock" : "lock")}
                      >
                        {choice.locked ? "Unlock" : "Lock"}
                      </button>
                      <button
                        type="button"
                        className="ghost"
                        aria-label={`Vary ${choice.syncGroup}`}
                        disabled={choice.locked || !choice.canVary || !storyArtifact?.ok}
                        title={choice.locked ? "Unlock this detail to vary it." : !choice.canVary ? "This detail has only one alternative." : "Choose a different alternative."}
                        onClick={() => this.changeChoice(choice.syncGroup, "vary")}
                      >
                        Vary this
                      </button>
                    </div>
                  </li>
                ))}
              </ul>
            </section>
          ) : null}
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
              <label htmlFor="receipt">Complete artifact · replay JSON</label>
              <textarea id="receipt" readOnly spellcheck={false} value={receipt} />
              <button type="button" className="ghost" disabled={!storyArtifact?.ok} onClick={() => this.replayReceipt()}>
                Replay this artifact
              </button>
            </>
          ) : null}
        </section>

        <section>
          <h2>Examples</h2>
          <div className="chips">
            <button type="button" onClick={() => this.loadStory(CHOICE_DEMO)}>
              <span className="chip-kicker">Lock &amp; vary a detail · Story JSON</span>
              <span className="chip-pattern">Keep a character’s name, lock a keepsake, and vary the same shirt in two sentences.</span>
            </button>
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
