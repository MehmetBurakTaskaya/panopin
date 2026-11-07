import react from '@vitejs/plugin-react';
import { defineConfig } from 'vitest/config';

// Tauri loads the dev server from a fixed port and owns the terminal output.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    host: '127.0.0.1',
    port: 1420,
    strictPort: true,
    watch: { ignored: ['**/src-tauri/**'] },
  },
  envPrefix: ['VITE_', 'TAURI_ENV_'],
  build: {
    // WebView2 (Chromium) on Windows, WKWebView on macOS 13+.
    target: ['chrome120', 'safari16'],
    outDir: 'dist',
    // One stylesheet for every window, linked from the page itself, so a window's look never
    // depends on code loaded later (D-086). CI checks it with scripts/check-bundle.mjs.
    cssCodeSplit: false,
  },
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.{ts,tsx}'],
    setupFiles: ['src/test/setup.ts'],
  },
});
