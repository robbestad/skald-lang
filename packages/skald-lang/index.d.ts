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
}

export interface Options {
  seed?: number | string;
  nsfw?: boolean;
  case?: CaseMode;
  dictionary?: Dictionary | string;
}

export interface Output {
  text: string;
  channels: Record<string, string>;
  picks: QueryPick[];
}

export interface Compiled {
  run(options?: Options): string;
  output(options?: Options): Output;
  explain(options?: Options): Output;
}

export function skald(pattern: string, options?: Options): string;
export function output(pattern: string, options?: Options): Output;
export function explain(pattern: string, options?: Options): Output;
export function compile(pattern: string, defaults?: Options): Compiled;

export class Engine {
  constructor(dictJson: string);
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
