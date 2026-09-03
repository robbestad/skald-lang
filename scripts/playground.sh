#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/playground"
if [ ! -d node_modules ]; then
  npm install
fi
echo "Playground: http://127.0.0.1:5174/"
exec npm run dev
