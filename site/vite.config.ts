import { defineConfig } from "vitest/config";

export default defineConfig({
  build: { target: "es2022", assetsInlineLimit: 0 },
  test: { include: ["test/**/*.test.ts"] },
});
