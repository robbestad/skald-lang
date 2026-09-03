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

export function createApi(Engine, defaultDictJson) {
  const defaultEngine = new Engine(defaultDictJson);

  function engineFor(options = {}) {
    const json = dictJson(options.dictionary);
    return json == null ? defaultEngine : new Engine(json);
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
    const raw = engineFor(options).explain(
      pattern,
      seedOf(options.seed),
      Boolean(options.nsfw),
      caseOf(options.case),
    );
    return JSON.parse(raw);
  }

  function compile(pattern, defaults = {}) {
    const inner = engineFor(defaults).compile(pattern);
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
        return JSON.parse(
          inner.explain(seedOf(o.seed), Boolean(o.nsfw), caseOf(o.case)),
        );
      },
    };
  }

  return { skald, compile, output, explain, Engine };
}
