import react from '@vitejs/plugin-react'
import { defineConfig } from 'vitest/config'

export default defineConfig({
  plugins: [react()],
  resolve: { alias: { '@': new URL('./src', import.meta.url).pathname } },
  base: '/',
  test: {
    environment: 'jsdom',
    setupFiles: './src/test_setup.ts',
    css: true,
    include: ['src/**/*.test.{ts,tsx}'],
  },
})
