/** Environment-neutral Story Runner. No fs, no process, no fetch. */

export const SCHEMA_VERSION = 1;
export const DEFAULT_MAX_REPAIRS = 2;
export const MAX_DOCUMENT_CHARS = 20_000;
export const MAX_BEATS = 24;
export const MAX_BLOCK_ALTS = 6;
export const MAX_BLOCK_WORDS = 8;
export const MAX_BLOCK_NEST = 2;

const CARRIER_ID = /^[A-Za-z][A-Za-z0-9_]{0,31}$/;
const QUERY_RE = /<([^<>]*)>/g;
const BLOCK_RE = /\{([^{}]*)\}/g;
const ADVANCED_TAG_RE =
  /\[(replace|map|fn|collect|join|rep|rs|protect|rhyme|out|let|x)\b/i;

const OPEN_VERB = new Set(["verb", "say", "verbimg"]);
const OPEN_ADJ = new Set(["adj"]);
const OPEN_PLACE = new Set(["place"]);
const OPEN_NOUN = new Set(["noun"]);
const SAFE_TABLES = new Set(["firstname", "name", "pron", "pro", "surname", "title"]);

export function diagnostic(code, message, extra = {}) {
  return {
    code,
    severity: extra.severity ?? "error",
    beatIndex: extra.beatIndex ?? null,
    span: extra.span ?? null,
    message,
    hint: extra.hint,
  };
}

function walkUnknownKeys(obj, allowed) {
  return Object.keys(obj).filter((k) => !allowed.has(k));
}

export function validateStoryDraft(draft, policy = {}) {
  const diagnostics = [];
  if (draft == null || typeof draft !== "object" || Array.isArray(draft)) {
    return {
      ok: false,
      diagnostics: [diagnostic("STORY_SCHEMA", "StoryDraft must be an object")],
    };
  }
  const unknown = walkUnknownKeys(
    draft,
    new Set(["schemaVersion", "cast", "beats"]),
  );
  if (unknown.length) {
    diagnostics.push(
      diagnostic(
        "STORY_SCHEMA",
        `unknown fields: ${unknown.join(", ")}`,
      ),
    );
  }
  if (draft.schemaVersion !== SCHEMA_VERSION) {
    diagnostics.push(
      diagnostic(
        "STORY_SCHEMA",
        `schemaVersion must be ${SCHEMA_VERSION}`,
      ),
    );
  }
  if ("seed" in draft) {
    diagnostics.push(
      diagnostic("STORY_SCHEMA", "seed is a host control; omit it from StoryDraft"),
    );
  }
  const raw = JSON.stringify(draft);
  if (raw.length > (policy.maxDocumentChars ?? MAX_DOCUMENT_CHARS)) {
    diagnostics.push(diagnostic("STORY_SIZE", "StoryDraft exceeds size limit"));
  }
  if (!Array.isArray(draft.cast) || draft.cast.length === 0) {
    diagnostics.push(diagnostic("STORY_CAST", "cast must be a non-empty array"));
  } else {
    const seen = new Set();
    draft.cast.forEach((row, i) => {
      if (!row || typeof row !== "object") {
        diagnostics.push(diagnostic("STORY_CAST", `cast[${i}] is not an object`));
        return;
      }
      const extra = walkUnknownKeys(row, new Set(["id", "query"]));
      if (extra.length) {
        diagnostics.push(
          diagnostic("STORY_CAST", `cast[${i}] unknown fields: ${extra.join(", ")}`),
        );
      }
      if (!CARRIER_ID.test(row.id ?? "")) {
        diagnostics.push(
          diagnostic("STORY_CAST", `cast[${i}].id is not a safe carrier id`),
        );
      }
      if (seen.has(row.id)) {
        diagnostics.push(diagnostic("STORY_CAST", `duplicate cast id: ${row.id}`));
      }
      seen.add(row.id);
      const q = parseSimpleQuery(row.query);
      if (!q.ok) {
        diagnostics.push(diagnostic("STORY_CAST", `cast[${i}].query: ${q.error}`));
      }
    });
  }
  if (!Array.isArray(draft.beats) || draft.beats.length === 0) {
    diagnostics.push(diagnostic("STORY_BEATS", "beats must be a non-empty array"));
  } else if (draft.beats.length > (policy.maxBeats ?? MAX_BEATS)) {
    diagnostics.push(diagnostic("STORY_SIZE", "too many beats"));
  } else {
    draft.beats.forEach((beat, i) => {
      if (typeof beat !== "string" || beat.trim().length === 0) {
        diagnostics.push(
          diagnostic("STORY_BEATS", `beats[${i}] must be a non-empty string`, {
            beatIndex: i,
          }),
        );
      }
    });
  }
  return { ok: diagnostics.length === 0, diagnostics };
}

export function parseSimpleQuery(raw) {
  if (typeof raw !== "string") return { ok: false, error: "query must be a string" };
  const trimmed = raw.trim();
  if (!trimmed.startsWith("<") || !trimmed.endsWith(">")) {
    return { ok: false, error: "query must be a single <table …> form" };
  }
  const inner = trimmed.slice(1, -1);
  if (inner.includes("<") || inner.includes("[") || inner.includes("{")) {
    return { ok: false, error: "query cannot contain tags or blocks" };
  }
  if (inner.includes("::")) {
    return { ok: false, error: "query cannot include a carrier" };
  }
  const table = inner.trim().split(/[\s.\-]+/)[0];
  if (!table) return { ok: false, error: "query needs a table name" };
  return { ok: true, table, inner, raw: trimmed };
}

function tableOf(inner) {
  const table = inner.trim().split(/[\s.\-]+/)[0] ?? "";
  if (table === "name") return "firstname";
  if (table === "pro") return "pron";
  return table;
}

function argsOf(inner) {
  return inner
    .trim()
    .split(/[\s.\-]+/)
    .slice(1)
    .filter(Boolean);
}

export function analyzeStoryDraft(draft, policy = {}) {
  const schema = validateStoryDraft(draft, policy);
  const diagnostics = [...schema.diagnostics];
  if (!schema.ok && diagnostics.some((d) => d.code === "STORY_SCHEMA")) {
    return { ok: false, diagnostics };
  }
  const allowed = new Set(policy.allowedTables ?? []);
  const advanced = policy.allowAdvancedTags === true;
  const maxAlts = policy.maxBlockAlts ?? MAX_BLOCK_ALTS;
  const maxWords = policy.maxBlockWords ?? MAX_BLOCK_WORDS;
  const castIds = new Set((draft.cast ?? []).map((c) => c.id));
  const castGenders = new Set();
  for (const row of draft.cast ?? []) {
    const q = parseSimpleQuery(row.query);
    if (!q.ok) continue;
    for (const a of argsOf(q.inner)) {
      if (a === "male" || a === "female") castGenders.add(a);
    }
  }

  (draft.beats ?? []).forEach((beat, beatIndex) => {
    if (typeof beat !== "string") return;
    if (!advanced && ADVANCED_TAG_RE.test(beat)) {
      diagnostics.push(
        diagnostic(
          "STORY_ADVANCED_TAG",
          "advanced tags are off in story beats",
          { beatIndex, hint: "Keep tags in the host prelude" },
        ),
      );
    }
    let nest = 0;
    for (const ch of beat) {
      if (ch === "{") nest += 1;
      if (ch === "}") nest = Math.max(0, nest - 1);
      if (nest > (policy.maxBlockNest ?? MAX_BLOCK_NEST)) {
        diagnostics.push(
          diagnostic("STORY_BLOCK", "block nesting exceeds the story limit", {
            beatIndex,
          }),
        );
        break;
      }
    }
    for (const m of beat.matchAll(BLOCK_RE)) {
      const alts = m[1].split("|");
      if (alts.length > maxAlts) {
        diagnostics.push(
          diagnostic("STORY_BLOCK", `block has ${alts.length} alternatives (max ${maxAlts})`, {
            beatIndex,
            span: { start: m.index, end: m.index + m[0].length },
          }),
        );
      }
      for (const alt of alts) {
        const words = alt.trim().split(/\s+/).filter(Boolean);
        if (words.length > maxWords) {
          diagnostics.push(
            diagnostic("STORY_BLOCK", "block alternative is too long", {
              beatIndex,
              span: { start: m.index, end: m.index + m[0].length },
            }),
          );
        }
      }
    }
    for (const m of beat.matchAll(QUERY_RE)) {
      const inner = m[1];
      const span = { start: m.index, end: m.index + m[0].length };
      const carrierMatch = inner.match(/::\s*([!~&=]*)\s*([A-Za-z][A-Za-z0-9_]*)/);
      const table = tableOf(inner.replace(/::.*$/, ""));
      const args = argsOf(inner.replace(/::.*$/, ""));
      if (inner.trim().startsWith("::")) {
        const id = inner.replace(/^::\s*[!~&=]*/, "").trim().split(/\s+/)[0];
        if (!castIds.has(id)) {
          diagnostics.push(
            diagnostic("STORY_CARRIER", `unknown carrier '${id}'`, {
              beatIndex,
              span,
            }),
          );
        }
        continue;
      }
      if (table === "pron" || table === "pro") {
        const g = args.find((a) => a === "male" || a === "female");
        if (g && castGenders.size && !castGenders.has(g)) {
          diagnostics.push(
            diagnostic(
              "STORY_CAST",
              `pronoun gender '${g}' matches no cast query`,
              { beatIndex, span },
            ),
          );
        }
      }
      if (carrierMatch) {
        diagnostics.push(
          diagnostic(
            "STORY_CARRIER",
            "beats may not define match carriers; declare them in cast",
            { beatIndex, span },
          ),
        );
      }
      if (OPEN_VERB.has(table)) {
        diagnostics.push(
          diagnostic("STORY_OPEN_VERB", "Open verb query in a story frame", {
            beatIndex,
            span,
            hint: "Write the predicate or use a small closed block",
          }),
        );
      }
      if (OPEN_ADJ.has(table)) {
        diagnostics.push(
          diagnostic("STORY_OPEN_ADJ", "Open adjective query in a story frame", {
            beatIndex,
            span,
          }),
        );
      }
      if (OPEN_PLACE.has(table)) {
        diagnostics.push(
          diagnostic("STORY_OPEN_PLACE", "Open place query in a story frame", {
            beatIndex,
            span,
            hint: "Write the setting or use a scene palette",
          }),
        );
      }
      if (OPEN_NOUN.has(table) && !allowed.has(`noun-${args[0] ?? ""}`) && !allowed.has(table)) {
        const cls = args.find((a) =>
          ["container", "liquid", "surface", "job", "person"].includes(a),
        );
        if (cls === "container") {
          diagnostics.push(
            diagnostic("STORY_OPEN_CONTAINER", "Open noun-container query in a story frame", {
              beatIndex,
              span,
            }),
          );
        } else if (cls === "liquid") {
          diagnostics.push(
            diagnostic("STORY_OPEN_LIQUID", "Open noun-liquid query in a story frame", {
              beatIndex,
              span,
            }),
          );
        } else if (cls === "surface") {
          diagnostics.push(
            diagnostic("STORY_OPEN_SURFACE", "Open noun-surface query in a story frame", {
              beatIndex,
              span,
            }),
          );
        } else if (policy.strictNoun !== false) {
          diagnostics.push(
            diagnostic("STORY_OPEN_NOUN", "Open noun query in a story frame", {
              beatIndex,
              span,
            }),
          );
        }
      }
      if (
        table &&
        !SAFE_TABLES.has(table) &&
        !OPEN_VERB.has(table) &&
        !OPEN_ADJ.has(table) &&
        !OPEN_PLACE.has(table) &&
        !OPEN_NOUN.has(table) &&
        !allowed.has(table)
      ) {
        diagnostics.push(
          diagnostic("STORY_TABLE", `query table '${table}' is not allowed`, {
            beatIndex,
            span,
          }),
        );
      }
    }
  });

  return { ok: diagnostics.length === 0, diagnostics };
}

export function buildCastPrelude(cast) {
  return (cast ?? [])
    .map((row) => {
      const q = parseSimpleQuery(row.query);
      if (!q.ok) return "";
      return `[out:cast_${row.id}]{<${q.inner} :: ${row.id}>}`;
    })
    .join("");
}

export function buildStoryPattern(draft, _cast, _palettes) {
  const prelude = buildCastPrelude(draft.cast);
  const beats = draft.beats ?? [];
  const sourceMap = { preludeEnd: prelude.length, beats: [] };
  let offset = prelude.length;
  const chunks = [];
  beats.forEach((beat, i) => {
    if (i > 0) {
      chunks.push("\n");
      offset += 1;
    }
    const start = offset;
    chunks.push(beat);
    offset += beat.length;
    sourceMap.beats.push({ index: i, start, end: offset });
  });
  return { pattern: `${prelude}${chunks.join("")}`, prelude, sourceMap };
}

export function mapPatternSpan(sourceMap, span) {
  const start = span?.start ?? 0;
  const end = span?.end ?? start;
  if (start < (sourceMap.preludeEnd ?? 0)) {
    return { beatIndex: null, span: { start, end } };
  }
  for (const beat of sourceMap.beats ?? []) {
    if (start >= beat.start && start < beat.end) {
      return {
        beatIndex: beat.index,
        span: { start: start - beat.start, end: Math.max(end - beat.start, start - beat.start) },
      };
    }
  }
  return { beatIndex: null, span: { start, end } };
}

export function ensureSeed(request) {
  if (request.seed !== undefined && request.seed !== null && request.seed !== "") {
    return request;
  }
  return { ...request, seed: Math.floor(Math.random() * 1_000_000_000) };
}

export const PROMPT_VERSION = "story-prompt-v1";

export function buildModelPrompt({
  prompt,
  brief,
  schemaVersion = SCHEMA_VERSION,
  paletteManifest = [],
  diagnostics = [],
  failingDraft = null,
  castRequirements = null,
}) {
  const palettes = (paletteManifest ?? [])
    .map((p) => `- ${p.id}: tables ${ (p.tables ?? []).join(", ") || "(none)" }. ${p.usage ?? ""}`)
    .join("\n");
  const diag = diagnostics.length
    ? JSON.stringify(diagnostics, null, 2)
    : "(none)";
  const failed = failingDraft ? JSON.stringify(failingDraft, null, 2) : "(none)";
  const cast = castRequirements
    ? JSON.stringify(castRequirements, null, 2)
    : "(host will accept any valid cast)";
  return `${prompt}

## This run
schemaVersion: ${schemaVersion}
brief:
${brief ?? ""}

cast requirements:
${cast}

available palettes:
${palettes || "(none — use firstname/pron and closed {a|b|c} blocks)"}

previous draft:
${failed}

diagnostics to fix:
${diag}

Return only StoryDraft JSON. Do not choose a seed. Do not include file paths.
`;
}

export function mergePalettes(registry, paletteIds, policy = {}) {
  const ids = paletteIds ?? [];
  const tables = {};
  const manifests = [];
  const seen = new Set();
  const core = new Set(["firstname", "noun", "place", "verb", "adj", "pron"]);
  for (const id of ids) {
    const entry = registry[id];
    if (!entry) {
      return {
        ok: false,
        diagnostics: [diagnostic("STORY_PALETTE", `unknown palette id '${id}'`)],
      };
    }
    const dict = entry.dictionary?.tables ?? entry.tables ?? {};
    for (const name of Object.keys(dict)) {
      if (core.has(name) && policy.allowCoreOverride !== true) {
        return {
          ok: false,
          diagnostics: [
            diagnostic(
              "STORY_PALETTE",
              `palette '${id}' overrides core table '${name}'`,
            ),
          ],
        };
      }
      if (seen.has(name) && policy.allowPaletteCollision !== true) {
        return {
          ok: false,
          diagnostics: [
            diagnostic(
              "STORY_PALETTE",
              `palette table '${name}' collides`,
            ),
          ],
        };
      }
      seen.add(name);
      tables[name] = dict[name];
    }
    manifests.push({
      id,
      tables: Object.keys(dict),
      usage: entry.manifest?.usage ?? entry.usage ?? "",
    });
  }
  return {
    ok: true,
    dictionary: { tables },
    allowedTables: [...seen],
    manifests,
    diagnostics: [],
  };
}

function hashString(s) {
  let h = 2166136261;
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return (h >>> 0).toString(16).padStart(8, "0");
}

export function createStoryArtifact(request, draft, result, extra = {}) {
  const diagnostics = [
    ...(extra.diagnostics ?? []),
    ...(result.diagnostics ?? []),
  ];
  const notes = result.notes ?? [];
  const ok = diagnostics.every((d) => d.severity !== "error");
  const replay = {
    schemaVersion: SCHEMA_VERSION,
    seed: request.seed,
    skaldVersion: extra.skaldVersion ?? "2.0.0",
    promptVersion: extra.promptVersion ?? "story-prompt-v1",
    paletteIds: request.paletteIds ?? [],
    paletteHash: extra.paletteHash ?? "",
    draft,
    pattern: extra.pattern,
    text: result.text,
    cast: extra.resolvedCast ?? {},
    picks: result.picks ?? [],
    choices: result.choices ?? [],
    diagnostics,
  };
  return {
    ok,
    ...replay,
    notes,
    channels: result.channels ?? {},
    parts: result.parts ?? [],
    partsByChannel: result.partsByChannel ?? {},
    density: result.density,
    replayHash: hashString(JSON.stringify(replay)),
    telemetry: extra.telemetry ?? null,
  };
}

export function resolveCast(result, draft) {
  const out = {};
  for (const row of draft.cast ?? []) {
    const channel = result.channels?.[`cast_${row.id}`];
    const pick = (result.picks ?? []).find(
      (p) => p.carrier === row.id && p.emitted,
    );
    out[row.id] = channel || pick?.value || "";
  }
  return out;
}

function uniqueCastNames(resolved) {
  const values = Object.values(resolved).filter(Boolean);
  const seen = new Set();
  for (const v of values) {
    const key = v.toLowerCase();
    if (seen.has(key)) return false;
    seen.add(key);
  }
  return true;
}

function runtimeDiagnostics(result, sourceMap) {
  const extra = [];
  for (const u of result.unresolved ?? []) {
    const mapped = mapPatternSpan(sourceMap, u.span);
    const code = u.kind === "unbound" ? "STORY_CARRIER" : "STORY_UNRESOLVED";
    extra.push(
      diagnostic(
        code,
        u.kind === "unbound"
          ? `unbound carrier '${u.carrier ?? ""}'`
          : `unresolved query <${u.raw}>`,
        { beatIndex: mapped.beatIndex, span: mapped.span },
      ),
    );
  }
  return extra;
}

export function renderStory(api, request, draft, palettes) {
  request = ensureSeed(request);
  const merged = mergePalettes(palettes.registry ?? palettes, request.paletteIds ?? [], request.policy);
  if (!merged.ok) {
    return {
      ok: false,
      artifact: createStoryArtifact(request, draft, { text: "", diagnostics: merged.diagnostics }, {
        diagnostics: merged.diagnostics,
      }),
    };
  }
  const policy = {
    ...(request.policy ?? {}),
    allowedTables: [
      ...((request.policy && request.policy.allowedTables) ?? []),
      ...merged.allowedTables,
    ],
  };
  const analysis = analyzeStoryDraft(draft, policy);
  if (!analysis.ok) {
    return {
      ok: false,
      artifact: createStoryArtifact(request, draft, { text: "", diagnostics: analysis.diagnostics }, {
        diagnostics: analysis.diagnostics,
        paletteHash: hashString(JSON.stringify(merged.dictionary)),
      }),
    };
  }
  const built = buildStoryPattern(draft);
  const pattern = built.pattern;
  const seed = request.seed;
  const options = {
    seed,
    case: "none",
    story: true,
  };
  if (Object.keys(merged.dictionary.tables).length > 0) {
    options.dictionary = merged.dictionary;
    options.merge = request.merge !== false;
  }
  const explain = api.explain;
  let result;
  let effectiveSeed = seed;
  try {
    result = explain(pattern, options);
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    const code = /budget/i.test(message) ? "STORY_BUDGET" : "STORY_RUNTIME";
    return {
      ok: false,
      artifact: createStoryArtifact(
        request,
        draft,
        { text: "", diagnostics: [diagnostic(code, message)] },
        {
          pattern,
          diagnostics: [diagnostic(code, message)],
          paletteHash: hashString(JSON.stringify(merged.dictionary)),
        },
      ),
    };
  }
  let resolved = resolveCast(result, draft);
  let retries = 0;
  const maxNameRetry = request.policy?.castNameRetries ?? 3;
  while (!uniqueCastNames(resolved) && retries < maxNameRetry) {
    retries += 1;
    effectiveSeed =
      typeof seed === "number" ? seed + retries : `${seed}:${retries}`;
    result = explain(pattern, { ...options, seed: effectiveSeed });
    resolved = resolveCast(result, draft);
  }
  const extraDiag = runtimeDiagnostics(result, built.sourceMap);
  if (!uniqueCastNames(resolved)) {
    extraDiag.push(
      diagnostic("STORY_CAST_NAME", "generated cast names collided after retry"),
    );
  }
  for (const [id, name] of Object.entries(resolved)) {
    if (!name) {
      extraDiag.push(diagnostic("STORY_CARRIER", `empty referent for '${id}'`));
    }
  }
  const artifact = createStoryArtifact(request, draft, result, {
    pattern,
    resolvedCast: resolved,
    diagnostics: extraDiag,
    paletteHash: hashString(JSON.stringify(merged.dictionary)),
    telemetry: { castNameRetries: retries, effectiveSeed },
  });
  return { ok: artifact.ok, artifact };
}

export async function runStoryLoop(api, request, model, palettes, extra = {}) {
  request = ensureSeed(request);
  const locked = {
    seed: request.seed,
    paletteIds: [...(request.paletteIds ?? [])],
    brief: request.brief,
    policy: request.policy,
    castRequirements: request.castRequirements,
  };
  const maxRepairs = request.policy?.maxRepairs ?? DEFAULT_MAX_REPAIRS;
  const telemetry = { modelCalls: 0, diagnostics: [] };
  const palette = mergePalettes(palettes.registry ?? palettes, locked.paletteIds, locked.policy);
  const paletteManifest = palette.ok ? palette.manifests : [];
  const prompt = extra.prompt ?? "";
  const genArgs = (diagnostics, failingDraft) => ({
    brief: locked.brief,
    schemaVersion: SCHEMA_VERSION,
    schema: extra.schema ?? null,
    castRequirements: locked.castRequirements,
    paletteManifest,
    diagnostics,
    failingDraft,
    policy: locked.policy,
    prompt: buildModelPrompt({
      prompt,
      brief: locked.brief,
      schemaVersion: SCHEMA_VERSION,
      paletteManifest,
      diagnostics,
      failingDraft,
      castRequirements: locked.castRequirements,
    }),
  });
  let draft = await model.generate(genArgs([], null));
  telemetry.modelCalls += 1;
  for (let i = 0; i <= maxRepairs; i++) {
    const analysis = analyzeStoryDraft(draft, {
      ...(locked.policy ?? {}),
      allowedTables: palette.ok ? palette.allowedTables : [],
    });
    if (analysis.ok) {
      const rendered = renderStory(
        api,
        { ...request, seed: locked.seed, paletteIds: locked.paletteIds, brief: locked.brief },
        draft,
        palettes,
      );
      rendered.artifact.telemetry = {
        ...rendered.artifact.telemetry,
        ...telemetry,
        repairAttempts: i,
      };
      return rendered;
    }
    telemetry.diagnostics = analysis.diagnostics;
    if (i === maxRepairs) {
      return {
        ok: false,
        artifact: createStoryArtifact(
          { ...request, seed: locked.seed, paletteIds: locked.paletteIds },
          draft,
          { text: "", diagnostics: analysis.diagnostics },
          { diagnostics: analysis.diagnostics, telemetry: { ...telemetry, repairAttempts: i } },
        ),
      };
    }
    draft = await model.generate(genArgs(analysis.diagnostics, draft));
    telemetry.modelCalls += 1;
  }
  return {
    ok: false,
    artifact: createStoryArtifact(
      { ...request, seed: locked.seed, paletteIds: locked.paletteIds },
      draft,
      { text: "" },
      { telemetry },
    ),
  };
}
