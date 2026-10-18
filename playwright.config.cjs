const { defineConfig } = require('@playwright/test');
module.exports = defineConfig({
  testDir: './web/admin/tests',
  testMatch: '**/*.spec.cjs',
  workers: 1,
  timeout: 30000,
  use: { channel: 'chromium', baseURL: 'http://127.0.0.1:5187', trace: 'retain-on-failure' },
  webServer: {
    command: 'npm --prefix web/admin run dev -- --host 127.0.0.1 --port 5187 --strictPort',
    url: 'http://127.0.0.1:5187',
    reuseExistingServer: false,
    timeout: 60000,
  },
});
