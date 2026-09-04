#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PKG="$ROOT/packages/skald-lang"

cargo run -p skald --bin skald-export-dict --quiet -- "$PKG/en-us.json"
cp "$ROOT/locales/nb-NO.json" "$PKG/nb-no.json"
wasm-pack build "$ROOT/crates/skald-wasm" --release --target web --out-dir "$PKG/pkg"
rm -f "$PKG/pkg/package.json" "$PKG/pkg/.gitignore" "$PKG/pkg/README.md"

WASM="$PKG/pkg/skald_wasm_bg.wasm"
if command -v gzip >/dev/null; then
  BYTES=$(wc -c < "$WASM" | tr -d ' ')
  GZIP=$(gzip -c "$WASM" | wc -c | tr -d ' ')
  echo "wasm $BYTES bytes ($GZIP gzip)"
  if [ "$GZIP" -ge 400000 ]; then
    echo "wasm gzip $GZIP exceeds 400000 byte budget" >&2
    exit 1
  fi
fi
