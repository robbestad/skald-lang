const TEXT_SEED_PREFIX = "text:";
const CANONICAL_U64 = /^(0|[1-9]\d*)$/;
const U64_MAX = 0xffff_ffff_ffff_ffffn;
export const RUN_PROFILE = "skald-pcg32-v1";

function looksNumeric(value) {
  return /^[+-]?(?:\d+\.?\d*|\.\d+)(?:[eE][+-]?\d+)?$/.test(value);
}

function parseU64Decimal(value, label = "integer seed") {
  if (!CANONICAL_U64.test(value)) {
    throw new Error(`invalid u64 seed ${JSON.stringify(value)}`);
  }
  let n;
  try {
    n = BigInt(value);
  } catch {
    throw new Error(`${label} ${JSON.stringify(value)} does not fit in u64`);
  }
  if (n > U64_MAX) {
    throw new Error(`${label} ${JSON.stringify(value)} does not fit in u64`);
  }
  return value;
}

export function canonicalSeed(seed) {
  if (seed === undefined || seed === null) return undefined;
  if (typeof seed === "bigint") {
    if (seed < 0n || seed > U64_MAX) {
      throw new Error("integer seed does not fit in u64");
    }
    return seed.toString();
  }
  if (typeof seed === "number") {
    if (!Number.isSafeInteger(seed) || seed < 0) {
      throw new Error(
        "numeric seed must be a safe non-negative integer; pass a decimal string for values above Number.MAX_SAFE_INTEGER",
      );
    }
    return String(seed);
  }
  if (typeof seed === "object") {
    if (seed.type === "u64") {
      return parseU64Decimal(String(seed.value ?? ""));
    }
    if (seed.type === "text") {
      const value = seed.value;
      if (typeof value !== "string" || !value) {
        throw new Error("text seed must be a non-empty string");
      }
      return `${TEXT_SEED_PREFIX}${value}`;
    }
    throw new Error("seed object must have type \"u64\" or \"text\"");
  }
  if (typeof seed === "string") {
    const trimmed = seed.trim();
    if (trimmed !== seed && (CANONICAL_U64.test(trimmed) || looksNumeric(trimmed))) {
      throw new Error(
        `invalid integer seed ${JSON.stringify(seed)}; surrounding whitespace is not part of a u64 decimal`,
      );
    }
    if (seed.startsWith(TEXT_SEED_PREFIX) || CANONICAL_U64.test(seed) || !looksNumeric(seed)) {
      if (seed === "" || seed === TEXT_SEED_PREFIX) {
        throw new Error("seed must not be empty");
      }
      if (CANONICAL_U64.test(seed)) parseU64Decimal(seed);
      return seed;
    }
    throw new Error(
      `invalid integer seed ${JSON.stringify(seed)}; use a canonical u64 decimal or a non-numeric text seed`,
    );
  }
  throw new Error("seed must be a number, string, bigint, or { type, value }");
}

function seedOf(seed) {
  return canonicalSeed(seed);
}

function caseOf(mode) {
  if (mode === undefined || mode === null) return undefined;
  return String(mode);
}

function dictJson(dictionary) {
  if (dictionary == null) return null;
  return typeof dictionary === "string" ? dictionary : JSON.stringify(dictionary);
}

function mergeTables(baseJson, extraJson) {
  const base = JSON.parse(baseJson);
  const extra = JSON.parse(extraJson);
  base.tables = { ...base.tables, ...(extra.tables ?? extra) };
  return JSON.stringify(base);
}

function budgetOf(options = {}) {
  const b = options.budget ?? {};
  return {
    maxSteps: b.max_steps ?? b.maxSteps ?? undefined,
    maxOutput: b.max_output ?? b.maxOutput ?? undefined,
    maxDepth: b.max_depth ?? b.maxDepth ?? undefined,
  };
}

function callFull(engine, method, pattern, options = {}) {
  const b = budgetOf(options);
  const fn = engine[method] ?? engine[method.replace("Full", "_full")];
  return fn.call(
    engine,
    pattern,
    seedOf(options.seed),
    Boolean(options.nsfw),
    caseOf(options.case),
    Boolean(options.story),
    b.maxSteps,
    b.maxOutput,
    b.maxDepth,
  );
}

function callCompiledFull(inner, method, options = {}) {
  const b = budgetOf(options);
  const fn = inner[method] ?? inner[method.replace("Full", "_full")];
  return fn.call(
    inner,
    seedOf(options.seed),
    Boolean(options.nsfw),
    caseOf(options.case),
    Boolean(options.story),
    b.maxSteps,
    b.maxOutput,
    b.maxDepth,
  );
}

export function createApi(Engine, defaultDictJson) {
  const defaultEngine = new Engine(defaultDictJson);
  const cache = new Map();

  function engineFor(options = {}) {
    const json = dictJson(options.dictionary);
    if (json == null) return defaultEngine;
    const merge = options.merge !== false;
    const key = `${merge ? "m" : "r"}:${json}`;
    let engine = cache.get(key);
    if (engine) return engine;
    if (merge && typeof defaultEngine.overlay === "function") {
      engine = defaultEngine.overlay(json);
    } else {
      engine = new Engine(merge ? mergeTables(defaultDictJson, json) : json);
    }
    cache.set(key, engine);
    return engine;
  }

  function skald(pattern, options = {}) {
    return callFull(engineFor(options), "runFull", pattern, options);
  }

  function output(pattern, options = {}) {
    return JSON.parse(
      callFull(engineFor(options), "outputFull", pattern, options),
    );
  }

  function explain(pattern, options = {}) {
    return JSON.parse(
      callFull(engineFor(options), "explainFull", pattern, options),
    );
  }

  function compile(pattern, defaults = {}) {
    const engine = engineFor(defaults);
    const inner = engine.compile(pattern);
    return {
      run(options = {}) {
        const o = { ...defaults, ...options };
        delete o.dictionary;
        delete o.merge;
        return callCompiledFull(inner, "runFull", o);
      },
      output(options = {}) {
        const o = { ...defaults, ...options };
        delete o.dictionary;
        delete o.merge;
        return JSON.parse(callCompiledFull(inner, "outputFull", o));
      },
      explain(options = {}) {
        const o = { ...defaults, ...options };
        delete o.dictionary;
        delete o.merge;
        return JSON.parse(callCompiledFull(inner, "explainFull", o));
      },
    };
  }

  return { skald, compile, output, explain, Engine, canonicalSeed, RUN_PROFILE };
}
