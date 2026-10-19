import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vitest/config';

// Plain unit tests of the UI's logic (no browser, no TestBed).
export default defineConfig({
  resolve: {
    // The same mapping as tsconfig.json's `paths`, which vitest doesn't read.
    alias: {
      'miniframework-ng': fileURLToPath(
        new URL('../../microcontroller/miniframework/web/projects/miniframework-ng/src/index.ts', import.meta.url),
      ),
    },
  },
  test: {
    include: ['src/**/*.test.ts'],
    environment: 'node',
  },
});
