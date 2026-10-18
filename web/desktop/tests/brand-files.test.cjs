const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const os = require('node:os');
const path = require('node:path');
const { readBrandFile } = require('../dist-electron/brand-files.js');
test('品牌读取限定文件名、大小和普通文件，不跟随链接', async () => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), 'packy-brand-'));
  const directory = path.join(root, '.updater/branding');
  try {
    await fs.mkdir(directory, { recursive: true });
    await fs.writeFile(path.join(directory, 'logo.png'), 'test-image');
    assert.equal((await readBrandFile(root, 'logo.png')).toString(), 'test-image');
    assert.equal(await readBrandFile(root, '../state.json'), undefined);
    assert.equal(await readBrandFile(root, 'missing.png'), undefined);
    await fs.writeFile(
      path.join(directory, 'background-light.webp'),
      Buffer.alloc(8 * 1024 * 1024 + 1),
    );
    assert.equal(await readBrandFile(root, 'background-light.webp'), undefined);
    await fs.mkdir(path.join(directory, 'background-dark.webp'));
    assert.equal(await readBrandFile(root, 'background-dark.webp'), undefined);
    if (process.platform !== 'win32') {
      await fs.symlink(path.join(directory, 'logo.png'), path.join(directory, 'tray.png'));
      assert.equal(await readBrandFile(root, 'tray.png'), undefined);
      await fs.rename(directory, directory + '-real');
      await fs.symlink(directory + '-real', directory);
      assert.equal(await readBrandFile(root, 'logo.png'), undefined);
    }
  } finally {
    await fs.rm(root, { recursive: true, force: true });
  }
});
test('只接受有效 PNG/WebP 头和有限尺寸，不把 SVG 交给渲染器', async () => {
  const { brandDataURL } = require('../dist-electron/brand-files.js');
  const png = await fs.readFile(path.join(__dirname, '../assets/icon.png'));
  const webp = await fs.readFile(path.join(__dirname, '../public/branding/background-light.webp'));
  assert.ok(brandDataURL(png, 'logo.png').startsWith('data:image/png'));
  assert.ok(brandDataURL(webp, 'background-light.webp').startsWith('data:image/webp'));
  assert.equal(brandDataURL(Buffer.from('<svg/>'), 'logo.png'), undefined);
  assert.equal(brandDataURL(Buffer.from('corrupted'), 'background-dark.webp'), undefined);
  assert.equal(brandDataURL(png, 'background-light.webp'), undefined);
  const huge = Buffer.from(png);
  huge.writeUInt32BE(9000, 16);
  assert.equal(brandDataURL(huge, 'logo.png'), undefined);
});
