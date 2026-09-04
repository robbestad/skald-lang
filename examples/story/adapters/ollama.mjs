/** Local Ollama StoryModel adapter. */

export function createOllamaModel({
  model,
  reviewModel = model,
  reasoningEffort,
  baseUrl = "http://127.0.0.1:11434",
  maxModelCalls = Infinity,
  contextSize = 16_384,
} = {}) {
  if (!model) throw new Error("Ollama model id is required");
  const totals = {
    requests: 0,
    inputTokens: 0,
    cachedInputTokens: 0,
    outputTokens: 0,
    reasoningTokens: 0,
    estimatedCostUsd: 0,
    durationMs: 0,
    costKnown: true,
  };

  async function requestJson(selectedModel, system, prompt) {
    let activePrompt = prompt;
    for (let attempt = 0; attempt < 2; attempt += 1) {
      if (totals.requests >= maxModelCalls) {
        throw new Error(`STORY_MODEL_BUDGET: model-call limit ${maxModelCalls} reached`);
      }
      const started = performance.now();
      const response = await fetch(`${baseUrl.replace(/\/$/, "")}/api/chat`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          model: selectedModel,
          stream: false,
          format: "json",
          messages: [
            { role: "system", content: system },
            { role: "user", content: activePrompt },
          ],
          think: Boolean(reasoningEffort && reasoningEffort !== "none"),
          options: { num_ctx: contextSize },
        }),
      });
      if (!response.ok) {
        throw new Error(`ollama adapter: ${response.status} ${await response.text()}`);
      }
      const body = await response.json();
      const content = body.message?.content ?? "{}";
      totals.requests += 1;
      totals.inputTokens += body.prompt_eval_count ?? 0;
      totals.outputTokens += body.eval_count ?? 0;
      totals.durationMs += Math.round(performance.now() - started);
      try {
        return JSON.parse(content);
      } catch (error) {
        if (attempt === 1) throw error;
        activePrompt = `${prompt}\n\nYour previous response was invalid JSON:\n${content}\n\nParse error: ${error.message}\nReturn the same requested data as valid JSON only.`;
      }
    }
  }

  return {
    getUsage() {
      return { ...totals };
    },
    plan: ({ prompt }) => requestJson(model, "Return only StoryIntent JSON. Do not write the story yet.", prompt),
    design: ({ prompt }) => requestJson(model, "Return only StoryDesign JSON. Do not write prose yet.", prompt),
    compose: ({ prompt }) => requestJson(model, "Return only manuscript JSON with a text field.", prompt),
    reviewManuscript: ({ prompt }) => requestJson(reviewModel, "Return only adversarial manuscript review JSON. Quote exact evidence; do not rewrite.", prompt),
    segment: ({ prompt }) => requestJson(model, "Return only literal StoryDraft JSON. Do not rewrite the manuscript.", prompt),
    skaldize: ({ prompt }) => requestJson(model, "Return only Skald transform JSON. Propose substitutions without rewriting prose.", prompt),
    reviewSkaldization: ({ prompt }) => requestJson(reviewModel, "Return only Skald lexical coverage review JSON. Do not rewrite prose.", prompt),
    revise: ({ prompt }) => requestJson(model, "Return only the locally repaired StoryDraft JSON.", prompt),
    generate: ({ prompt }) => requestJson(model, "Return only StoryDraft JSON.", prompt),
    review: ({ prompt }) => requestJson(reviewModel, "Return only narrative review JSON. Do not rewrite the story.", prompt),
  };
}
