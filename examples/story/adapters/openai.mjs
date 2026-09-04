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
