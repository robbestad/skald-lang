import { create } from "svenjs";
import { explain } from "skald-lang";
import svenjsMark from "./svenjs-mark.svg?url";

const EXAMPLES = [
  "<firstname male> likes to <verb-transitive> <noun.plural> with <pron poss male> pet <noun-animal> on <timenoun dayofweek plural>.",
  "<firstname male :: hero> walked into the <place> with <pron poss male> <noun-animal>. <::hero> did not knock.",
  "[let:pets; [collect:3; <noun-animal ::!p>]][join:pets; ,\\s; and]",
  "[rhyme:perfect]<noun ::~a> / <noun ::~a>",
  "[out:title]{<adj> <noun>}[out:body]{A <noun-animal> entered the <place>.}",
];

type DemoState = {
  pattern: string;
  seed: string;
  output: string;
  picks: string;
  status: string;
  error: boolean;
};

function parseSeed(value: string): number | string | undefined {
  const trimmed = value.trim();
  if (!trimmed) return undefined;
  if (/^\d+$/.test(trimmed)) return Number(trimmed);
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

function evaluate(
  pattern: string,
  seed: string,
): Pick<DemoState, "output" | "picks" | "status" | "error"> {
  if (!pattern.trim()) {
    return {
      output: "",
      picks: "",
      status: "Write a pattern first.",
      error: false,
    };
  }
  try {
    const result = explain(pattern, {
      seed: parseSeed(seed),
      case: "none",
    });
    const summary = result.picks
      .map((p) =>
        p.carrier ? `${p.table} (${p.carrier})=${p.value}` : `${p.table}=${p.value}`,
      )
      .join(" · ");
    return {
      output: formatOutput(result.text, result.channels),
      picks: summary,
      status: "",
      error: false,
    };
  } catch (err) {
    return {
      output: "",
      picks: "",
      status: err instanceof Error ? err.message : String(err),
      error: true,
    };
  }
}

export const App = create<Record<string, never>, DemoState>({
  initialState() {
    const pattern = EXAMPLES[0] ?? "";
    const seed = "42";
    return { pattern, seed, ...evaluate(pattern, seed) };
  },
  run() {
    this.setState({
      ...this.state,
      ...evaluate(this.state.pattern, this.state.seed),
    });
  },
  loadExample(pattern: string) {
    this.setState({
      ...this.state,
      pattern,
      ...evaluate(pattern, this.state.seed),
    });
  },
  reseed() {
    const seed = String(Math.floor(Math.random() * 1_000_000_000));
    this.setState({
      ...this.state,
      seed,
      ...evaluate(this.state.pattern, seed),
    });
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
    const { pattern, seed, output, picks, status, error } = this.state;

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
          <label htmlFor="pattern">Pattern</label>
          <textarea
            id="pattern"
            spellcheck={false}
            value={pattern}
            onInput={(e: InputEvent) =>
              this.setState({
                ...this.state,
                pattern: (e.target as HTMLTextAreaElement).value,
              })
            }
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
                onInput={(e: InputEvent) =>
                  this.setState({
                    ...this.state,
                    seed: (e.target as HTMLInputElement).value,
                  })
                }
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
          </div>
          <p className="hint">⌘/Ctrl + Enter runs the pattern.</p>
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
          {picks ? <p className="picks">{picks}</p> : null}
          {status ? (
            <p
              className={error ? "status error" : "status"}
              role="status"
              aria-live="polite"
            >
              {status}
            </p>
          ) : null}
        </section>

        <section>
          <h2>Examples</h2>
          <div className="chips">
            {EXAMPLES.map((example) => (
              <button
                key={example}
                type="button"
                className={example === pattern ? "active" : ""}
                onClick={() => this.loadExample(example)}
              >
                {example}
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
        </section>

        <footer>
          <p>
            Skald 1.0 — sister to{" "}
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
