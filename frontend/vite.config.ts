import { rmSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

const projectDir = dirname(fileURLToPath(import.meta.url));
const generatedAssetsDir = resolve(projectDir, "../web/assets");

export default defineConfig({
  plugins: [
    {
      name: "clean-generated-web-assets",
      apply: "build",
      buildStart() {
        rmSync(generatedAssetsDir, { recursive: true, force: true });
      },
    },
    react(),
    tailwindcss(),
  ],
  build: {
    outDir: "../web",
    emptyOutDir: false,
    sourcemap: false,
    manifest: "asset-manifest.json",
  },
});
