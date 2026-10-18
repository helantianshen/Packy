const { _electron: electron } = require(process.env.PACKY_PLAYWRIGHT_MODULE || 'playwright-core');
const fs = require('node:fs/promises'),
  os = require('node:os'),
  path = require('node:path'),
  assert = require('node:assert/strict');
(async () => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), 'Packy 中文 空格-'));
  const repo = path.resolve(__dirname, '../../..');
  const output = repo + '/.impeccable/review/client-b/';
  await fs.mkdir(output, { recursive: true });
  await fs.mkdir(root + '/.minecraft/mods', { recursive: true });
  await fs.mkdir(root + '/.updater/branding', { recursive: true });
  await fs.writeFile(
    root + '/.updater/state.json',
    JSON.stringify({ installed_release_id: '11111111-1111-4111-8111-111111111111' }),
  );
  await fs.copyFile(repo + '/web/desktop/assets/icon.png', root + '/.updater/branding/logo.png');
  await fs.copyFile(
    repo + '/web/desktop/public/branding/background-light.webp',
    root + '/.updater/branding/background-light.webp',
  );
  await fs.writeFile(root + '/.updater/branding/background-dark.webp', 'corrupted');
  const app = await electron.launch({
    executablePath: repo + '/web/desktop/node_modules/electron/dist/electron',
    args: [repo + '/web/desktop'],
    env: {
      ...process.env,
      PACKY_DEV_ROOT: root,
      XDG_CONFIG_HOME: path.join(root, 'electron-config'),
    },
    timeout: 30000,
  });
  try {
    const page = await app.firstWindow();
    const errors = [];
    page.on('pageerror', (e) => errors.push(e.message));
    await page.waitForFunction(
      async () => {
        try {
          return !!(await window.packy.invoke('get_status')).status;
        } catch {
          return false;
        }
      },
      null,
      { timeout: 20000 },
    );
    await page.waitForFunction(
      () => !document.body.textContent.includes('更新核心尚未就绪，请重新打开客户端'),
    );
    const brands = await page.evaluate(() => window.packy.branding());
    assert.ok(brands.logo?.startsWith('data:image/png'));
    assert.ok(brands.light?.startsWith('data:image/webp'));
    assert.equal(brands.dark, undefined);
    await page.getByRole('button', { name: '配置与说明', exact: true }).click();
    await page.screenshot({ path: output + 'native-settings.png', fullPage: true });
    const scaling = [];
    for (const zoom of [1, 1.25, 1.5, 2]) {
      await app.evaluate(
        ({ BrowserWindow }, z) => BrowserWindow.getAllWindows()[0].webContents.setZoomFactor(z),
        zoom,
      );
      await page.waitForTimeout(150);
      const size = await page.evaluate(() => ({
        width: innerWidth,
        scroll: document.documentElement.scrollWidth,
      }));
      assert.ok(size.scroll <= size.width, JSON.stringify({ zoom, ...size }));
      scaling.push({ zoom, ...size });
    }
    const capture = await app.evaluate(async ({ BrowserWindow }) =>
      (await BrowserWindow.getAllWindows()[0].capturePage()).toPNG().toString('base64'),
    );
    await fs.writeFile(output + 'native-zoom200.png', Buffer.from(capture, 'base64'));
    await app.evaluate(({ BrowserWindow }) =>
      BrowserWindow.getAllWindows()[0].webContents.setZoomFactor(1),
    );
    await page.evaluate(() => window.packy.invoke('set_background_enabled', { enabled: true }));
    await app.evaluate(({ BrowserWindow }) => BrowserWindow.getAllWindows()[0].close());
    assert.ok(
      await app.evaluate(({ BrowserWindow }) => !BrowserWindow.getAllWindows()[0].isVisible()),
    );
    await app.evaluate(({ BrowserWindow }) => BrowserWindow.getAllWindows()[0].show());
    await page.evaluate(() => window.packy.invoke('set_background_enabled', { enabled: false }));
    assert.equal(errors.length, 0, errors.join('\n'));
    console.log(
      JSON.stringify({
        passed: true,
        branding: 'logo/light override; corrupt dark falls back',
        scaling,
        tray: 'close hides window',
      }),
    );
  } finally {
    await app.close();
    await fs.rm(root, { recursive: true, force: true });
  }
})().catch((e) => {
  console.error(e);
  process.exitCode = 1;
});
