import { defineConfig } from 'vitest/config';

export default defineConfig({
  // Transform JSX with esbuild's automatic runtime so test/component files
  // don't need an explicit `import React`. This avoids coupling the test
  // pipeline to a specific @vitejs/plugin-react ↔ Vite version pairing
  // (plugin-react lags new Vite majors, which silently dropped the JSX
  // transform and produced "React is not defined").
  esbuild: {
    jsx: 'automatic',
    jsxImportSource: 'react',
  },
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/__tests__/setup.js'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'lcov'],
      include: ['src/components/**', 'src/hooks/**'],
    },
  },
});
