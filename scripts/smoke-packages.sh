#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
STAGE="$ROOT/target/smoke"
rm -rf "$STAGE"
mkdir -p "$STAGE"

echo "== cargo package =="
cargo package -p skald --allow-dirty
VER=$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)
CRATE="$ROOT/target/package/skald-$VER"
test -d "$CRATE"
( cd "$CRATE" && cargo test --lib )

echo "== npm pack =="
./scripts/build-npm.sh
(
  cd "$ROOT/packages/skald-lang"
  TARBALL=$(npm pack --ignore-scripts | tail -1)
  mkdir -p "$STAGE/npm"
  tar -xzf "$TARBALL" -C "$STAGE/npm"
  rm -f "$TARBALL"
)
node --input-type=module -e "
import { skald } from 'file://$STAGE/npm/package/index.js';
const s = skald('{A|B}', { seed: 1, case: 'none' });
if (!s) throw new Error('empty skald output');
console.log('npm smoke', s);
"

echo "smoke packages ok"
