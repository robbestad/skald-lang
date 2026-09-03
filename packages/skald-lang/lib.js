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

function applyStory(engine, pattern, parsed, options) {
  if (!options.story) return parsed;
  const extra = JSON.parse(engine.story_lint(pattern));
  const notes = [...(parsed.notes ?? []), ...extra];
  return { ...parsed, notes };
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
    engine = new Engine(merge ? mergeTables(defaultDictJson, json) : json);
    cache.set(key, engine);
    return engine;
  }

  function skald(pattern, options = {}) {
    return engineFor(options).run(
      pattern,
      seedOf(options.seed),
      Boolean(options.nsfw),
      caseOf(options.case),
    );
  }

  function output(pattern, options = {}) {
    const raw = engineFor(options).run_output(
      pattern,
      seedOf(options.seed),
      Boolean(options.nsfw),
      caseOf(options.case),
    );
    return JSON.parse(raw);
  }

  function explain(pattern, options = {}) {
    const engine = engineFor(options);
    const raw = engine.explain(
      pattern,
      seedOf(options.seed),
      Boolean(options.nsfw),
      caseOf(options.case),
    );
    return applyStory(engine, pattern, JSON.parse(raw), options);
  }

  function compile(pattern, defaults = {}) {
    const engine = engineFor(defaults);
    const inner = engine.compile(pattern);
    return {
      run(options = {}) {
        const o = { ...defaults, ...options };
        return inner.run(seedOf(o.seed), Boolean(o.nsfw), caseOf(o.case));
      },
      output(options = {}) {
        const o = { ...defaults, ...options };
        return JSON.parse(
          inner.run_output(seedOf(o.seed), Boolean(o.nsfw), caseOf(o.case)),
        );
      },
      explain(options = {}) {
        const o = { ...defaults, ...options };
        const parsed = JSON.parse(
          inner.explain(seedOf(o.seed), Boolean(o.nsfw), caseOf(o.case)),
        );
        return applyStory(engine, pattern, parsed, o);
      },
    };
  }

  return { skald, compile, output, explain, Engine };
}
