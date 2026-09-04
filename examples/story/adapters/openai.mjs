/** Optional example StoryModel. Not imported by CI or the VM. */

export function createOpenAIModel({
  apiKey,
  model = "gpt-4.1",
  reviewModel = "gpt-4.1",
  reasoningEffort,
  maxModelCalls = Infinity,
  maxCostUsd = Infinity,
} = {}) {
  const key = apiKey ?? process.env.OPENAI_API_KEY;
  if (!key) {
    throw new Error("OPENAI_API_KEY is required for the example adapter");
  }
  const totals = {
    requests: 0,
    inputTokens: 0,
    cachedInputTokens: 0,
    outputTokens: 0,
    reasoningTokens: 0,
    estimatedCostUsd: 0,
  };
  const prices = {
    "gpt-4.1": { input: 2, cachedInput: 0.5, output: 8 },
    "gpt-4.1-mini": { input: 0.4, cachedInput: 0.1, output: 1.6 },
    "gpt-4.1-nano": { input: 0.1, cachedInput: 0.025, output: 0.4 },
  };
  const priceFor = (selectedModel) => prices[selectedModel] ?? null;
  if (
    Number.isFinite(maxCostUsd) &&
    (priceFor(model) === null || priceFor(reviewModel) === null)
  ) {
    throw new Error(
      "STORY_MODEL_BUDGET: --max-cost-usd requires known prices for both model ids",
    );
  }
  async function requestJson(selectedModel, system, prompt) {
      if (totals.requests >= maxModelCalls) {
        throw new Error(`STORY_MODEL_BUDGET: model-call limit ${maxModelCalls} reached`);
      }
      if (totals.estimatedCostUsd >= maxCostUsd) {
        throw new Error(`STORY_MODEL_BUDGET: estimated cost limit $${maxCostUsd} reached`);
      }
      const res = await fetch("https://api.openai.com/v1/chat/completions", {
        method: "POST",
        headers: {
          authorization: `Bearer ${key}`,
          "content-type": "application/json",
        },
        body: JSON.stringify({
          model: selectedModel,
          ...(reasoningEffort ? { reasoning_effort: reasoningEffort } : {}),
          response_format: { type: "json_object" },
          messages: [
            { role: "system", content: system },
            { role: "user", content: prompt },
          ],
        }),
      });
      if (!res.ok) {
        throw new Error(`openai adapter: ${res.status} ${await res.text()}`);
      }
      const body = await res.json();
      const usage = body.usage ?? {};
      const inputTokens = usage.prompt_tokens ?? 0;
      const cachedInputTokens = usage.prompt_tokens_details?.cached_tokens ?? 0;
      const outputTokens = usage.completion_tokens ?? 0;
      const reasoningTokens = usage.completion_tokens_details?.reasoning_tokens ?? 0;
      const price = priceFor(selectedModel);
      totals.requests += 1;
      totals.inputTokens += inputTokens;
      totals.cachedInputTokens += cachedInputTokens;
      totals.outputTokens += outputTokens;
      totals.reasoningTokens += reasoningTokens;
      if (price) {
        const uncachedInputTokens = Math.max(0, inputTokens - cachedInputTokens);
        totals.estimatedCostUsd +=
          (uncachedInputTokens * price.input +
            cachedInputTokens * price.cachedInput +
            outputTokens * price.output) /
          1_000_000;
      }
      const text = body.choices?.[0]?.message?.content ?? "{}";
      return JSON.parse(text);
  }
  return {
    getUsage() {
      return {
        ...totals,
        estimatedCostUsd: Number(totals.estimatedCostUsd.toFixed(6)),
        costKnown: priceFor(model) !== null && priceFor(reviewModel) !== null,
      };
    },
    async plan({ prompt }) {
      return requestJson(
        model,
        "Return only StoryIntent JSON. Do not write the story yet.",
        prompt,
      );
    },
    async design({ prompt }) {
      return requestJson(model, "Return only StoryDesign JSON. Do not write prose yet.", prompt);
    },
    async compose({ prompt }) {
      return requestJson(model, "Return only manuscript JSON with a text field.", prompt);
    },
    async reviewManuscript({ prompt }) {
      return requestJson(
        reviewModel,
        "Return only adversarial manuscript review JSON. Quote exact evidence; do not rewrite.",
        prompt,
      );
    },
    async segment({ prompt }) {
      return requestJson(model, "Return only literal StoryDraft JSON. Do not rewrite the manuscript.", prompt);
    },
    async skaldize({ prompt }) {
      return requestJson(
        model,
        "Return only Skald transform JSON. Propose substitutions without rewriting prose.",
        prompt,
      );
    },
    async reviewSkaldization({ prompt }) {
      return requestJson(
        reviewModel,
        "Return only Skald lexical coverage review JSON. Do not rewrite prose.",
        prompt,
      );
    },
    async revise({ prompt }) {
      return requestJson(model, "Return only the locally repaired StoryDraft JSON.", prompt);
    },
    async generate({ prompt }) {
      return requestJson(model, "Return only StoryDraft JSON.", prompt);
    },
    async review({ prompt }) {
      return requestJson(
        reviewModel,
        "Return only narrative review JSON. Do not rewrite the story.",
        prompt,
      );
    },
  };
}
