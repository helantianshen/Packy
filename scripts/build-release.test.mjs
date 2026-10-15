import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const check = (...args) =>
  spawnSync(process.execPath, ['scripts/build-release.mjs', ...args], {
    cwd: root,
    encoding: 'utf8',
    env: { ...process.env, CARGO_BUILD_TARGET: '' },
  });

test('统一入口默认构建本机客户端和服务端', () => {
  const result = check('--check');
  assert.equal(result.status, 0, result.stderr);
  const plan = JSON.parse(result.stdout);
  assert.equal(plan.only, 'all');
  assert.equal(plan.server, `artifacts/server/packy-server-${plan.platform}-${plan.target}.tar.gz`);
});

test('服务端模式可独立执行，非法组件不能启动构建', () => {
  const server = check('--only', 'server', '--check');
  assert.equal(server.status, 0, server.stderr);
  assert.equal(JSON.parse(server.stdout).only, 'server');
  const invalid = check('--only', 'unknown', '--check');
  assert.equal(invalid.status, 1);
  assert.match(invalid.stderr, /--only 仅支持/);
});
