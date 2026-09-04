/** Optional example StoryModel. Not imported by CI or the VM. */

export function createOpenAIModel({ apiKey, model = "gpt-4.1-mini" } = {}) {
  const key = apiKey ?? process.env.OPENAI_API_KEY;
  if (!key) {
    throw new Error("OPENAI_API_KEY is required for the example adapter");
  }
  return {
    async generate({ prompt }) {
      const res = await fetch("https://api.openai.com/v1/chat/completions", {
        method: "POST",
        headers: {
          authorization: `Bearer ${key}`,
          "content-type": "application/json",
        },
        body: JSON.stringify({
          model,
          response_format: { type: "json_object" },
          messages: [
            { role: "system", content: "Return only StoryDraft JSON." },
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
    },
  };
}
