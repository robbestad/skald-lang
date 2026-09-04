/** Optional example StoryModel. Not imported by CI or the VM. */

export function createOpenAIModel({
  apiKey,
  model = "gpt-4.1",
  reviewModel = "gpt-4.1",
} = {}) {
  const key = apiKey ?? process.env.OPENAI_API_KEY;
  if (!key) {
    throw new Error("OPENAI_API_KEY is required for the example adapter");
  }
  async function requestJson(selectedModel, system, prompt) {
      const res = await fetch("https://api.openai.com/v1/chat/completions", {
        method: "POST",
        headers: {
          authorization: `Bearer ${key}`,
          "content-type": "application/json",
        },
        body: JSON.stringify({
          model: selectedModel,
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
      const text = body.choices?.[0]?.message?.content ?? "{}";
      return JSON.parse(text);
  }
  return {
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
