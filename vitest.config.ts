import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    include: ["src/**/*.test.ts"],
    environment: "node",
    reporters: "default",
    coverage: {
      provider: "v8",
      reporter: ["text", "html"],
    },
  },
});
