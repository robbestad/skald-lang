import { defineConfig } from "vite";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = dirname(fileURLToPath(import.meta.url));
const pkg = join(root, "..", "packages", "skald-lang");
const examples = join(root, "..", "examples");
const docs = join(root, "..", "docs");

export default defineConfig({
  root,
  base: "./",
  esbuild: {
    jsx: "automatic",
    jsxImportSource: "svenjs",
  },
  resolve: {
    alias: {
      "skald-lang": join(pkg, "browser.js"),
    },
  },
  optimizeDeps: {
    exclude: ["svenjs", "skald-lang"],
  },
  assetsInclude: ["**/*.wasm"],
  server: {
    port: 5174,
    fs: {
      allow: [root, pkg, examples, docs],
    },
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
  },
});
