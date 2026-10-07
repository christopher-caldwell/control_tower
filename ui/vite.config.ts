import { fileURLToPath } from 'node:url'

import react from '@vitejs/plugin-react'
import { defineConfig } from 'vitest/config'

import { workspaceBackend } from './dev/workspace_backend.ts'

export default defineConfig({
  plugins: [react(), workspaceBackend()],
  resolve: { alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) } },
  base: '/',
  test: {
    environment: 'jsdom',
    setupFiles: './test/setup.ts',
    css: true,
    include: ['src/**/*.test.{ts,tsx}'],
  },
})
