import { mkdtemp, writeFile, mkdir, readFile, rm } from 'node:fs/promises';
import { spawn, spawnSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createServer } from 'node:net';
import assert from 'node:assert/strict';
const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
if (process.platform === 'win32') {
  console.log('SKIP: POSIX SIGTERM test');
  process.exit(0);
}
const dir = await mkdtemp(path.join(tmpdir(), 'packy-smoke-'));
const suffix = process.platform === 'win32' ? '.exe' : '';
let server;
function run(command, args, options = {}) {
  const result = spawnSync(command, args, { cwd: repo, encoding: 'utf8', ...options });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`${command}: ${result.stderr}`);
  return result.stdout;
}
async function until(fn) {
  for (let i = 0; i < 100; i++) {
    try {
      const result = await fn();
      if (result) return result;
    } catch {}
    await new Promise((r) => setTimeout(r, 100));
  }
  throw new Error('等待超时');
}
async function stop(child) {
  if (!child || child.exitCode !== null) return;
  child.kill('SIGTERM');
  await new Promise((r) => child.once('exit', r));
}
try {
  const probe = createServer();
  await new Promise((r) => probe.listen(0, '127.0.0.1', r));
  const port = probe.address().port;
  await new Promise((r) => probe.close(r));
  const origin = `http://127.0.0.1:${port}`;
  const configPath = path.join(dir, 'server.yaml');
  await writeFile(
    configPath,
    JSON.stringify({
      listen: `127.0.0.1:${port}`,
      database: path.join(dir, 'db.sqlite'),
      files_dir: path.join(dir, 'files'),
      public_url: origin,
      jwt_secret: 'temporary-smoke-only-not-a-production-secret',
      modrinth_user_agent: 'Packy-test/0.2',
    }),
  );
  const env = { ...process.env, PACKY_CONFIG: configPath };
  const serverBin = path.join(dir, `server${suffix}`);
  run('go', ['build', '-o', serverBin, './cmd/server']);
  // 测试夹具显式准备结构和执行初始化 SQL，服务正常启动不代做初始化
  const absent = spawnSync(serverBin, [], { env, encoding: 'utf8' });
  assert.notEqual(absent.status, 0);
  run(serverBin, ['migrate'], { env });
  const hash = run(serverBin, ['password-hash'], { input: 'smoke-test-password\n' }).trim();
  const sql = (await readFile(path.join(repo, 'sql/init.sql'), 'utf8')).replaceAll(
    'REPLACE_WITH_BCRYPT_HASH',
    hash,
  );
  run('sqlite3', [path.join(dir, 'db.sqlite')], { input: sql });
  server = spawn(serverBin, [], { env, stdio: ['ignore', 'ignore', 'inherit'] });
  await until(async () => (await fetch(origin + '/healthz')).ok);
  const oldFrontend = await fetch(origin + '/admin/');
  assert.equal(oldFrontend.status, 404);
  assert.match(oldFrontend.headers.get('content-type'), /application\/json/);
  let token = '';
  async function api(method, url, data) {
    const headers = {};
    if (token) headers.Authorization = `Bearer ${token}`;
    let body;
    if (data instanceof FormData) body = data;
    else if (data !== undefined) {
      headers['Content-Type'] = 'application/json';
      body = JSON.stringify(data);
    }
    const response = await fetch(origin + '/api/v1' + url, { method, headers, body });
    if (!response.ok) throw new Error(`${url} ${response.status}: ${await response.text()}`);
    return response.status === 204 ? null : response.json();
  }
  token = (await api('POST', '/auth/login', { username: 'admin', password: 'smoke-test-password' }))
    .access_token;

  const { request } = await import('node:http');
  const exited = new Promise((resolve) => server.once('exit', resolve));
  const responsePromise = new Promise((resolve, reject) => {
    const req = request(
      origin + '/api/v1/admin/files',
      {
        method: 'POST',
        headers: {
          Authorization: `Bearer ${token}`,
          'Content-Type': 'multipart/form-data; boundary=ReviewBoundary',
        },
      },
      (res) => {
        res.resume();
        res.on('end', () => resolve(res.statusCode));
        res.on('error', reject);
      },
    );
    req.on('error', reject);
    req.setTimeout(12000, () => req.destroy(new Error('upload timeout')));
    req.write(
      '--ReviewBoundary\r\nContent-Disposition: form-data; name="file"; filename="slow.jar"\r\nContent-Type: application/java-archive\r\n\r\n',
    );
    req.write(Buffer.alloc(64 * 1024, 65));
    setTimeout(() => {
      server.kill('SIGTERM');
      setTimeout(() => req.end('\r\n--ReviewBoundary--\r\n'), 300);
    }, 200);
  });
  assert.equal(await responsePromise, 201);
  await exited;
  console.log('PASS: SIGTERM drains the in-flight upload before exiting');
} finally {
  await stop(server);
  await rm(dir, { recursive: true, force: true });
}
