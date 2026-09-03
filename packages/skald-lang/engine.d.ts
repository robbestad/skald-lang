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
