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
  export type StoryChoiceState = {
    formatVersion: 1;
    groups: Record<string, {
      alternativeId: string;
      locked: boolean;
      rerollCount: number;
    }>;
  };
  export type StoryArtifact = {
    ok: boolean;
    text?: string;
    seed?: number | string;
    effectiveSeed?: number | string;
    replayHash?: string;
    diagnostics?: { code: string; beatIndex: number | null; message: string }[];
    notes?: string[];
    cast?: Record<string, string>;
    picks?: { table: string; value: string; carrier?: string; emitted?: boolean }[];
    choices?: {
      kind: string;
      alternative: number;
      repeatIndex: number;
      variationId?: string;
      alternativeId?: string;
      syncGroup?: string;
    }[];
    choiceState?: StoryChoiceState;
    parts?: { source: string; text: string }[];
    density?: { glue_ratio: number; queries: number };
    draft?: unknown;
    pattern?: string;
    paletteIds?: string[];
    variations?: {
      variationId?: string;
      syncGroup?: string;
      alternativeIds?: string[];
    }[];
    telemetry?: { effectiveSeed?: number | string };
  };
  export function splitStoryDocument(doc: unknown): {
    request: {
      seed?: number | string;
      paletteIds?: string[];
      variations?: unknown[];
      choiceState?: StoryChoiceState;
      [key: string]: unknown;
    };
    draft: unknown;
  };
  export function renderStory(
    api: { explain: typeof import("skald-lang").explain },
    request: { seed?: number | string; paletteIds?: string[] },
    draft: unknown,
    palettes: { registry: unknown },
  ): {
    ok: boolean;
    artifact: StoryArtifact;
  };
  export function setStoryChoiceLock(artifact: StoryArtifact, syncGroup: string, locked?: boolean): StoryChoiceState;
  export function rerollStoryChoice(artifact: StoryArtifact, syncGroup: string): StoryChoiceState;
}
