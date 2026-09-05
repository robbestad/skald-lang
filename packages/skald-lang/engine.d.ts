export class Engine {
  constructor(dictJson: string);
  static fromLanguagePack(json: string): Engine;
  locale(): string | undefined;
  preflight(pattern: string): void;
  overlay(extraJson: string): Engine;
  run(
    pattern: string,
    seed?: string | null,
    nsfw?: boolean,
    caseMode?: string | null,
  ): string;
  runFull(
    pattern: string,
    seed?: string | null,
    nsfw?: boolean,
    caseMode?: string | null,
    story?: boolean,
    maxSteps?: number | null,
    maxOutput?: number | null,
    maxDepth?: number | null,
  ): string;
  run_output(
    pattern: string,
    seed?: string | null,
    nsfw?: boolean,
    caseMode?: string | null,
  ): string;
  outputFull(
    pattern: string,
    seed?: string | null,
    nsfw?: boolean,
    caseMode?: string | null,
    story?: boolean,
    maxSteps?: number | null,
    maxOutput?: number | null,
    maxDepth?: number | null,
  ): string;
  explain(
    pattern: string,
    seed?: string | null,
    nsfw?: boolean,
    caseMode?: string | null,
  ): string;
  explainFull(
    pattern: string,
    seed?: string | null,
    nsfw?: boolean,
    caseMode?: string | null,
    story?: boolean,
    maxSteps?: number | null,
    maxOutput?: number | null,
    maxDepth?: number | null,
  ): string;
  story_lint(pattern: string): string;
  compile(pattern: string): {
    run(seed?: string | null, nsfw?: boolean, caseMode?: string | null): string;
    runFull(
      seed?: string | null,
      nsfw?: boolean,
      caseMode?: string | null,
      story?: boolean,
      maxSteps?: number | null,
      maxOutput?: number | null,
      maxDepth?: number | null,
    ): string;
    run_output(
      seed?: string | null,
      nsfw?: boolean,
      caseMode?: string | null,
    ): string;
    outputFull(
      seed?: string | null,
      nsfw?: boolean,
      caseMode?: string | null,
      story?: boolean,
      maxSteps?: number | null,
      maxOutput?: number | null,
      maxDepth?: number | null,
    ): string;
    explain(
      seed?: string | null,
      nsfw?: boolean,
      caseMode?: string | null,
    ): string;
    explainFull(
      seed?: string | null,
      nsfw?: boolean,
      caseMode?: string | null,
      story?: boolean,
      maxSteps?: number | null,
      maxOutput?: number | null,
      maxDepth?: number | null,
    ): string;
  };
}
