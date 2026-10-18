const { _electron: electron } = require(process.env.PACKY_PLAYWRIGHT_MODULE || 'playwright-core');
const fs = require('node:fs/promises');
const path = require('node:path');
const os = require('node:os');
const assert = require('node:assert/strict');
(async () => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), 'Packy 下载设置 '));
  const repo = path.resolve(__dirname, '../../..');
  let app;
  try {
    await fs.mkdir(path.join(root, '.minecraft/mods'), { recursive: true });
    await fs.mkdir(path.join(root, '.updater'), { recursive: true });
    await fs.writeFile(
      path.join(root, '.updater/state.json'),
      JSON.stringify({ installed_release_id: '11111111-1111-4111-8111-111111111111' }),
    );
    for (let attempt = 0; attempt < 2; attempt++) {
      app = await electron.launch({
        executablePath: path.join(repo, 'web/desktop/node_modules/electron/dist/electron'),
        args: [path.join(repo, 'web/desktop')],
        env: {
          ...process.env,
          PACKY_DEV_ROOT: root,
          XDG_CONFIG_HOME: path.join(root, 'electron-config'),
        },
      });
      const page = await app.firstWindow();
      await page.getByRole('button', { name: '配置与说明', exact: true }).click();
      await page.getByLabel('最大并发下载数').waitFor();
      if (attempt === 0) {
        await page.getByLabel('最大并发下载数').fill('6');
        await page.getByLabel('总下载限速（KiB/s）').fill('256');
        await page.getByLabel('GitHub 代理', { exact: true }).fill('http://127.0.0.1:7890');
        await page.getByRole('button', { name: '保存下载设置', exact: true }).click();
        await page.getByRole('status').filter({ hasText: '已保存' }).waitFor();
        const snapshot = await page.evaluate(() => window.packy.invoke('get_status'));
        assert.equal(snapshot.download.max_concurrent_downloads, 6);
        assert.equal(snapshot.download.speed_limit_kib, 256);
        const preferences = JSON.parse(
          await fs.readFile(path.join(root, '.updater/preferences.json'), 'utf8'),
        );
        assert.equal(preferences.download.proxies.github, 'http://127.0.0.1:7890');
      } else {
        assert.equal(await page.getByLabel('最大并发下载数').inputValue(), '6');
        await page.getByRole('button', { name: '全部恢复默认', exact: true }).click();
        await page.getByRole('button', { name: '保存下载设置', exact: true }).click();
        await page.getByRole('status').filter({ hasText: '已保存' }).waitFor();
        const snapshot = await page.evaluate(() => window.packy.invoke('get_status'));
        assert.equal(snapshot.download.max_concurrent_downloads, 4);
        assert.equal(snapshot.download.proxies.github, '');
      }
      await app.close();
      app = undefined;
    }
    console.log(
      'PASS: native Electron -> RPC -> Sidecar settings save, file persistence, app restart and defaults',
    );
  } finally {
    if (app) await app.close();
    await fs.rm(root, { recursive: true, force: true });
  }
})().catch((e) => {
  console.error(e);
  process.exitCode = 1;
});
