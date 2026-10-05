import { defineConfig } from '@playwright/test'

export default defineConfig({
  testDir: './browser_tests',
  fullyParallel: false,
  reporter: 'list',
  use: {
    baseURL: 'http://127.0.0.1:4178',
    browserName: 'chromium',
    viewport: { width: 1280, height: 820 },
  },
  webServer: {
    command: 'pnpm dev --host 127.0.0.1 --port 4178 --strictPort',
    url: 'http://127.0.0.1:4178',
    reuseExistingServer: !process.env.CI,
  },
})
