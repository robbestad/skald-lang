export type CaseMode =
  | "none"
  | "default"
  | "first"
  | "word"
  | "title"
  | "upper"
  | "lower"
  | "sentence";

export interface Dictionary {
  tables: Record<
    string,
    {
      name: string;
      subs: string[];
      entries: { forms: string[]; classes: string[]; phones?: string[] }[];
    }
  >;
}

export interface QueryPick {
  table: string;
  value: string;
  forms: string[];
  classes: string[];
  form: number;
  args: string[];
  carrier?: string;
  span: { start: number; end: number };
  channel: string | null;
  emitted: boolean;
}

export interface Choice {
  kind: string;
  span: { start: number; end: number };
  alternative: number;
  repeatIndex: number;
  channel: string | null;
  emitted: boolean;
}

export interface Diagnostic {
  code: string;
  severity: string;
  beatIndex: number | null;
  span: { start: number; end: number } | null;
  message: string;
  hint?: string;
}

export interface OutputPart {
  text: string;
  source: "dictionary" | "glue";
  table?: string;
}

export interface Density {
  glue_ratio: number;
  queries: number;
  warning?: string;
}

export interface Budget {
  max_steps?: number;
  maxSteps?: number;
  max_output?: number;
  maxOutput?: number;
  max_depth?: number;
  maxDepth?: number;
}

export type Seed =
  | number
  | string
  | bigint
  | { type: "u64"; value: string }
  | { type: "text"; value: string };

export interface Options {
  seed?: Seed;
  nsfw?: boolean;
  case?: CaseMode;
  locale?: "en-US" | "nb-NO" | "nn-NO" | string;
  languagePack?: object | string;
  dictionary?: Dictionary | string;
  /** Merge `dictionary` over bundled English (default true when dictionary is set). */
  merge?: boolean;
  /** Add story-lint notes and diagnostics to explain() and output(). */
  story?: boolean;
  budget?: Budget;
}

/** Per-run options for compile().run. Dictionary/locale/pack are compile-time only. */
export type RunOptions = Omit<Options, "dictionary" | "merge" | "locale" | "languagePack">;

export interface Output {
  text: string;
  channels: Record<string, string>;
  picks: QueryPick[];
  parts: OutputPart[];
  partsByChannel?: Record<string, OutputPart[]>;
  density?: Density;
  notes?: string[];
  choices?: Choice[];
  diagnostics?: Diagnostic[];
  unresolved?: {
    kind: string;
    raw: string;
    table: string;
    carrier?: string;
    span: { start: number; end: number };
  }[];
}

export interface Compiled {
  run(options?: RunOptions): string;
  output(options?: RunOptions): Output;
  explain(options?: RunOptions): Output;
}

export function skald(pattern: string, options?: Options): string;
export function output(pattern: string, options?: Options): Output;
export function explain(pattern: string, options?: Options): Output;
export function compile(pattern: string, defaults?: Options): Compiled;

export class Engine {
  constructor(dictJson: string);
  static fromLanguagePack(json: string): Engine;
  locale(): string | undefined;
  run(
    pattern: string,
    seed?: string | null,
    nsfw?: boolean,
    caseMode?: string | null,
  ): string;
  run_output(
    pattern: string,
    seed?: string | null,
    nsfw?: boolean,
    caseMode?: string | null,
  ): string;
  explain(
    pattern: string,
    seed?: string | null,
    nsfw?: boolean,
    caseMode?: string | null,
  ): string;
  story_lint(pattern: string): string;
  compile(pattern: string): {
    run(seed?: string | null, nsfw?: boolean, caseMode?: string | null): string;
    run_output(
      seed?: string | null,
      nsfw?: boolean,
      caseMode?: string | null,
    ): string;
    explain(
      seed?: string | null,
      nsfw?: boolean,
      caseMode?: string | null,
    ): string;
  };
}

export const RUN_PROFILE: "skald-pcg32-v1";
export function canonicalSeed(seed?: Seed | null): string | undefined;
