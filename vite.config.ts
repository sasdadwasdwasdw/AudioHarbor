import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import tailwind from '@tailwindcss/vite';
export default defineConfig({ plugins: [react(), tailwind()], clearScreen: false, server: { port: 1420, strictPort: true, watch: { ignored: ['**/src-tauri/**', '**/.cache/**', '**/.playwright-mcp/**'] } }, envPrefix: ['VITE_', 'TAURI_ENV_'], build: { target: 'chrome105' }, test: { include: ['src/**/*.test.ts'] } });
