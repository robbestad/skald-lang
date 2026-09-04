import innDict from "../../docs/beats/data/inn.json" with { type: "json" };

export const PALETTES = {
  inn: {
    id: "inn",
    dictionary: innDict,
    manifest: {
      usage:
        "Closed inn drinks/food. Use <inn_drink>, never <noun-liquid> or <place>.",
    },
  },
  forest: {
    id: "forest",
    dictionary: { tables: {} },
    manifest: {
      usage:
        "Copy closed blocks: the trees {bent closer|shut out the moon|began to whisper}; {no wind|no birds|no path}.",
    },
  },
  road: {
    id: "road",
    dictionary: { tables: {} },
    manifest: {
      usage:
        "Copy closed blocks: {walked|followed|turned back}; the {pass|trail|bell-road}.",
    },
  },
};
