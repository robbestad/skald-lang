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

const ENGINE_CACHE_LIMIT = 16;

function looksLikeLanguagePack(json) {
  try {
    const obj = JSON.parse(json);
    return Boolean(obj && typeof obj === "object" && obj.formatVersion != null);
  } catch {
    return false;
  }
}

export function createApi(Engine, defaultDictJson) {
  const defaultEngine = new Engine(defaultDictJson);
  const cache = new Map();

  function putCache(key, engine) {
    if (cache.size >= ENGINE_CACHE_LIMIT) {
      const oldest = cache.keys().next().value;
      cache.delete(oldest);
    }
    cache.set(key, engine);
    return engine;
  }

  function engineFor(options = {}) {
    const locale = options.locale;
    const json = dictJson(options.languagePack ?? options.dictionary);
    if (locale && locale !== "en-US" && (json == null || !looksLikeLanguagePack(json))) {
      throw new Error(`missing language pack for ${locale}`);
    }
    if (json == null) return defaultEngine;
    if (looksLikeLanguagePack(json)) {
      const key = `p:${json}`;
      let engine = cache.get(key);
      if (!engine) {
        if (typeof Engine.fromLanguagePack !== "function") {
          throw new Error("language packs require Engine.fromLanguagePack");
        }
        engine = Engine.fromLanguagePack(json);
        putCache(key, engine);
      }
      const packLocale = typeof engine.locale === "function" ? engine.locale() : undefined;
      if (locale && packLocale && packLocale !== locale) {
        throw new Error(`language pack locale ${packLocale} does not match ${locale}`);
      }
      return engine;
    }
    const merge = options.merge !== false;
    const key = `${merge ? "m" : "r"}:${json}`;
    let engine = cache.get(key);
    if (engine) return engine;
    if (merge && typeof defaultEngine.overlay === "function") {
      engine = defaultEngine.overlay(json);
    } else {
      engine = new Engine(merge ? mergeTables(defaultDictJson, json) : json);
    }
    return putCache(key, engine);
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
    const runOpts = (options = {}) => {
      if (
        options.locale != null
        || options.languagePack != null
        || options.dictionary != null
        || Object.prototype.hasOwnProperty.call(options, "merge")
      ) {
        throw new Error("locale, languagePack, dictionary, and merge are compile-time only");
      }
      const o = { ...defaults, ...options };
      delete o.dictionary;
      delete o.merge;
      delete o.locale;
      delete o.languagePack;
      return o;
    };
    return {
      run(options = {}) {
        return callCompiledFull(inner, "runFull", runOpts(options));
      },
      output(options = {}) {
        return JSON.parse(callCompiledFull(inner, "outputFull", runOpts(options)));
      },
      explain(options = {}) {
        return JSON.parse(callCompiledFull(inner, "explainFull", runOpts(options)));
      },
    };
  }

  return { skald, compile, output, explain, Engine, canonicalSeed, RUN_PROFILE };
}
