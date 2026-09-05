#!/usr/bin/env bash
# Install produced packages in an empty project. Direct file:// import is not enough.
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
if [ ! -f "$ROOT/packages/skald-lang/pkg/skald_wasm_bg.wasm" ]; then
  ./scripts/build-npm.sh
fi
(
  cd "$ROOT/packages/skald-lang"
  npm pack --ignore-scripts --pack-destination "$STAGE"
)
TARBALL=$(ls "$STAGE"/skald-lang-*.tgz | head -1)
test -f "$TARBALL"
tar -tzf "$TARBALL" > "$STAGE/tarball.list"
grep -q 'package/nb-no.json' "$STAGE/tarball.list"
grep -q 'package/nn-no.json' "$STAGE/tarball.list"
grep -q 'package/engine.js' "$STAGE/tarball.list"

echo "== empty project install =="
mkdir -p "$STAGE/app"
cat > "$STAGE/app/package.json" <<'EOF'
{
  "name": "skald-smoke-app",
  "private": true,
  "type": "module"
}
EOF
(
  cd "$STAGE/app"
  npm install "$TARBALL"
)

cat > "$STAGE/app/smoke.mjs" <<'EOF'
import { compile, skald } from "skald-lang";
import { Engine } from "skald-lang/engine";
import nb from "skald-lang/nb-no.json" with { type: "json" };
import nn from "skald-lang/nn-no.json" with { type: "json" };

const en = skald("{A|B}", { seed: 1, case: "none" });
if (!en) throw new Error("empty skald output");
const compiled = compile("{A|B}", { case: "none" }).run({ seed: 1 });
if (compiled !== en) throw new Error(`compile/run mismatch ${compiled} vs ${en}`);

let missing = false;
try {
  skald("Ada", { locale: "nb-NO", case: "none" });
} catch (err) {
  missing = String(err).includes("missing language pack");
}
if (!missing) throw new Error("nb-NO without pack should fail");

const nbLine = skald("<firstname female>", {
  languagePack: nb,
  locale: "nb-NO",
  seed: 1,
  case: "none",
});
if (!nbLine || /<firstname/.test(nbLine)) throw new Error(`nb-NO unresolved ${nbLine}`);

const nnLine = skald("<firstname female>", {
  languagePack: nn,
  locale: "nn-NO",
  seed: 1,
  case: "none",
});
if (!nnLine || /<firstname/.test(nnLine)) throw new Error(`nn-NO unresolved ${nnLine}`);

const engine = Engine.fromLanguagePack(JSON.stringify(nb));
const engineLine = engine.run("<firstname female>", "1", false, "none");
if (!engineLine) throw new Error("engine.fromLanguagePack empty");

console.log("npm install smoke", en, nbLine, nnLine, engineLine);
EOF

(
  cd "$STAGE/app"
  node smoke.mjs
  printf '%s\n' '{A|B}' > sample.skald
  node_modules/.bin/skald-lang --seed 1 --case none -f sample.skald
  node_modules/.bin/skald-lang --seed 1 --case none manifest sample.skald
  test -f sample.skald.json
  node_modules/.bin/skald-lang inspect sample.skald
  node_modules/.bin/skald-lang verify sample.skald
  node_modules/.bin/skald-lang --seed 1 --case none run sample.skald
)

echo "smoke packages ok"
