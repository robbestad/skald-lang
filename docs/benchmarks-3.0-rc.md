# 3.0-rc measurements

Package version remains 2.2.0. Snapshot from `scripts/bench-rc.mjs` on this
checkout — not a gate, and not a claim that 3.0 is faster than 2.2. There is no
stored 2.2 gold file; the 2.2 contract that still applies is the **400 KB**
gzipped WASM budget. Language packs are separate JSON and are not in that budget.

| Item | Value |
| --- | --- |
| npm `skald()` mean (200 runs, en-US) | 0.02 ms |
| npm compiled `.run()` mean (200 runs, en-US) | 0.01 ms |
| npm `explain()` mean (50 runs, en-US) | 0.04 ms |
| `Engine.fromLanguagePack` mean (20 loads, nb-NO) | 0.09 ms |
| npm `skald()` mean (50 runs, nb-NO pack) | 0.05 ms |
| npm `skald()` mean (50 runs, nn-NO pack) | 0.04 ms |
| heap delta after 50 `explain()` | 262.7 KB |
| native release binary (one pattern, wall) | 6.6 ms |
| `skald_wasm_bg.wasm` gzip | 390.3 KB (budget 400) |
| `en-us.json` | 256.1 KB |
| `nb-NO.json` | 3.5 KB |
| `nb-NO.json` gzip | 0.9 KB |
| `nn-NO.json` | 3.5 KB |
| `nn-NO.json` gzip | 0.9 KB |

Language pack files are original curated cores, not ordbank dumps. Re-run:

```bash
node scripts/bench-rc.mjs
```
