/** Offline StoryModel for CI. First call can return a bad draft; later calls return good. */

export function createMockModel({ bad, good }) {
  let calls = 0;
  return {
    async generate({ diagnostics }) {
      calls += 1;
      if (calls === 1 && bad) return structuredClone(bad);
      if (diagnostics?.length && good) return structuredClone(good);
      return structuredClone(good ?? bad);
    },
    get calls() {
      return calls;
    },
  };
}
