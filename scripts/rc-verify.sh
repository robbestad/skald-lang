#!/usr/bin/env bash
# Offline 3.0 release-candidate checks. Does not tag or publish.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
TMP="$(mktemp -d "${TMPDIR:-/tmp}/skald-rc.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT

echo "== rust fmt/clippy/test =="
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --offline

echo "== language pack copies =="
cmp locales/nb-NO.json packages/skald-lang/nb-no.json
cmp locales/nn-NO.json packages/skald-lang/nn-no.json

echo "== wasm / npm build =="
./scripts/build-npm.sh

echo "== npm / story / eval =="
node packages/skald-lang/test.mjs
node examples/story/test.mjs
node examples/story/corpus/eval.mjs --mock \
  --out "$TMP/packet.json" \
  --manifest "$TMP/manifest.json" \
  --report "$TMP/report.json"
node --input-type=module -e "
import { readFileSync } from 'node:fs';
const packet = JSON.parse(readFileSync(process.argv[1], 'utf8'));
const report = JSON.parse(readFileSync(process.argv[2], 'utf8'));
if ('condition' in (packet.samples?.[0] ?? {})) throw new Error('packet leaked condition');
if ('variation' in packet) throw new Error('packet leaked variation');
if (!report.variation) throw new Error('report missing variation');
if (!String(report.notes ?? '').includes('not the blind packet')) throw new Error('report notes');
console.log('eval packet/report split ok');
" "$TMP/packet.json" "$TMP/report.json"

echo "== package smoke =="
bash scripts/smoke-packages.sh

echo "== playground typecheck + build =="
if [ ! -d playground/node_modules ]; then
  echo "playground/node_modules missing; run npm ci --prefix playground" >&2
  exit 1
fi
npm run typecheck --prefix playground
npm run build --prefix playground

echo "rc-verify ok (no tag, no publish)"
