# 3.0.1 measurements vs 2.2

Package version is 3.0.1. Snapshot from `scripts/bench-rc.mjs` on this
checkout. Baseline: `docs/benchmarks-2.2.json` (v2.2.0).
Not a gate. WASM gzip budget remains **500 KB**. Language packs are separate JSON.
JS heap and WASM linear memory are reported separately.

| Item | Value |
| --- | --- |
| npm `compile()` mean (50 runs, en-US) | 0.02 ms |
| npm `skald()` mean (200 runs, en-US) | 0.02 ms |
| npm compiled `.run()` mean (200 runs, en-US) | 0.01 ms |
| npm `explain()` mean (50 runs, en-US) | 0.06 ms |
| `Engine.fromLanguagePack` mean (20 loads, nb-NO) | 0.09 ms |
| npm `skald()` mean (50 runs, nb-NO pack) | 0.07 ms |
| npm `skald()` mean (50 runs, nn-NO pack) | 0.05 ms |
| JS heap delta after 50 `explain()` | -1734.8 KB |
| WASM linear memory | 3072.0 KB |
| native release binary (one pattern, wall) | 8.8 ms |
| `skald_wasm_bg.wasm` gzip | 393.9 KB (budget 500; 15.7 KB vs v2.2.0) |
| 2.2 npm `skald()` mean | 0.023 ms |
| 2.2 wasm gzip | 378.3 KB |
| `en-us.json` | 256.1 KB |
| `nb-NO.json` | 3.5 KB |
| `nb-NO.json` gzip | 0.9 KB |
| `nn-NO.json` | 3.5 KB |
| `nn-NO.json` gzip | 0.9 KB |

Language pack files are original curated cores, not ordbank dumps. Re-run:

```bash
node scripts/bench-rc.mjs
```
