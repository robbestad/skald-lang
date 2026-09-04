declare module "*prompt.md?raw" {
  const text: string;
  export default text;
}

declare module "*story/palettes.mjs" {
  export const PALETTES: Record<
    string,
    {
      id: string;
      dictionary: { tables?: Record<string, unknown> };
      manifest?: { usage?: string };
    }
  >;
}

declare module "*story/runner.mjs" {
  export function renderStory(
    api: { explain: typeof import("skald-lang").explain },
    request: { seed?: number | string; paletteIds?: string[] },
    draft: unknown,
    palettes: { registry: unknown },
  ): {
    ok: boolean;
    artifact: {
      ok: boolean;
      text?: string;
      seed?: number | string;
      replayHash?: string;
      diagnostics?: { code: string; beatIndex: number | null; message: string }[];
      notes?: string[];
      cast?: Record<string, string>;
      picks?: { table: string; value: string; carrier?: string; emitted?: boolean }[];
      choices?: { kind: string; alternative: number; repeatIndex: number }[];
      parts?: { source: string; text: string }[];
      density?: { glue_ratio: number; queries: number };
      draft?: unknown;
      pattern?: string;
      paletteIds?: string[];
      telemetry?: { effectiveSeed?: number | string };
    };
  };
}
