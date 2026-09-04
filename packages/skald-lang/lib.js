function seedOf(seed) {
  if (seed === undefined || seed === null) return undefined;
  return String(seed);
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

  return { skald, compile, output, explain, Engine };
}
