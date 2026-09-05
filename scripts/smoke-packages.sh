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
./scripts/build-npm.sh
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
grep -q 'package/artifact.mjs' "$STAGE/tarball.list"
grep -q 'package/artifact.d.ts' "$STAGE/tarball.list"
grep -q 'package/engine.d.ts' "$STAGE/tarball.list"
grep -q 'package/index.d.ts' "$STAGE/tarball.list"

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
import { patternHash } from "skald-lang/artifact";
import nb from "skald-lang/nb-no.json" with { type: "json" };
import nn from "skald-lang/nn-no.json" with { type: "json" };

if (!patternHash("hello").startsWith("sha256:")) throw new Error("artifact export missing");

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

cat > "$STAGE/app/consumer.ts" <<'EOF'
import { compile, skald, Engine as TopEngine } from "skald-lang";
import { Engine } from "skald-lang/engine";
import { manifestForPattern, patternHash, RECEIPT_FORMAT_VERSION } from "skald-lang/artifact";
import type { Options, RunOptions } from "skald-lang";
import nb from "skald-lang/nb-no.json" with { type: "json" };

const compileOpts: Options = { case: "none", languagePack: nb, locale: "nb-NO" };
const runOpts: RunOptions = { seed: 1, case: "none" };
const hash: string = patternHash("{A|B}");
if (!hash.startsWith("sha256:")) throw new Error("typed patternHash");
const manifest = manifestForPattern("{A|B}", { seed: 1, caseMode: "none" });
if (manifest.formatVersion < 1) throw new Error("typed manifest");
if (RECEIPT_FORMAT_VERSION < 1) throw new Error("typed receipt version");
const line: string = skald("{A|B}", { seed: 1, case: "none" });
const compiled: string = compile("{A|B}", { case: "none" }).run(runOpts);
if (compiled !== line) throw new Error("typed compile mismatch");
const packed: string = compile("<firstname female>", compileOpts).run(runOpts);
if (!packed || packed.includes("<")) throw new Error("typed pack run");
const engine: Engine = Engine.fromLanguagePack(JSON.stringify(nb));
const top: TopEngine = TopEngine.fromLanguagePack(JSON.stringify(nb));
engine.preflight("<firstname female>");
top.overlay("{}");
EOF

(
  cd "$STAGE/app"
  node smoke.mjs
  npm install --no-save --no-fund --no-audit typescript@5.9.2
  npx tsc --strict --noEmit --module nodenext --moduleResolution nodenext --target es2022 --resolveJsonModule consumer.ts
  printf '%s\n' '{A|B}' > sample.skald
  node_modules/.bin/skald-lang --seed 1 --case none -f sample.skald
  node_modules/.bin/skald-lang --seed 1 --case none manifest sample.skald
  test -f sample.skald.json
  node_modules/.bin/skald-lang inspect sample.skald
  node_modules/.bin/skald-lang verify sample.skald
  node_modules/.bin/skald-lang --seed 1 --case none run sample.skald
  node_modules/.bin/skald-lang --seed 1 --case none --locale nb-NO --pack node_modules/skald-lang/nb-no.json '<firstname female>'
)

echo "== browser smoke (cold WASM) =="
node "$ROOT/scripts/browser-smoke.mjs" "$STAGE/app"

NODE20=""
if [ -x /opt/homebrew/opt/node@20/bin/node ]; then
  NODE20=/opt/homebrew/opt/node@20/bin/node
elif command -v node-20 >/dev/null 2>&1; then
  NODE20="$(command -v node-20)"
fi
if [ -n "${SMOKE_NODE20:-}" ]; then
  NODE20="$SMOKE_NODE20"
fi
if [ -n "$NODE20" ]; then
  echo "== engines field: Node 20 =="
  "$NODE20" --version
  ( cd "$STAGE/app" && "$NODE20" smoke.mjs )
fi

echo "smoke packages ok"
