import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  envPrefix: ['VITE_', 'TAURI_ENV_*'],
  server: { port: 1420, strictPort: true },
  build: { outDir: 'dist', target: 'es2022' },
  test: { include: ['tests/**/*.test.ts'], environment: 'node' },
});
