import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const native = { win32: 'windows', linux: 'linux', darwin: 'macos' }[process.platform];
const check = (...args) =>
  spawnSync(process.execPath, ['scripts/build-desktop.mjs', ...args], {
    cwd: root,
    encoding: 'utf8',
    env: { ...process.env, CARGO_BUILD_TARGET: '' },
  });

test('本机构建选择对应配置，并按系统和目标隔离产物', () => {
  const result = check('--platform', native, '--check');
  assert.equal(result.status, 0, result.stderr);
  const plan = JSON.parse(result.stdout);
  assert.equal(plan.config, `config/client.${native}.yaml`);
  assert.equal(plan.output, `artifacts/desktop/packy-desktop-${native}-${plan.target}.tar.gz`);
  assert.ok(plan.target);
});

test('切换平台参数不能冒充交叉编译', () => {
  const other = native === 'windows' ? 'linux' : 'windows';
  const result = check('--platform', other, '--check');
  assert.equal(result.status, 1);
  assert.match(result.stderr, /当前系统只能构建/);
});

test('未知参数在构建前失败', () => {
  const result = check('--unknown');
  assert.equal(result.status, 1);
  assert.match(result.stderr, /未知参数/);
});
