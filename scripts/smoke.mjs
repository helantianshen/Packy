import { mkdtemp, writeFile, mkdir, readFile, rm } from 'node:fs/promises';
import { spawn, spawnSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createServer } from 'node:net';
import { createRequire } from 'node:module';
import assert from 'node:assert/strict';
const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const require = createRequire(path.join(repo, 'web/desktop/package.json'));
const {
  createMessageConnection,
  StreamMessageReader,
  StreamMessageWriter,
} = require('vscode-jsonrpc/node');
const dir = await mkdtemp(path.join(tmpdir(), 'packy-smoke-'));
const suffix = process.platform === 'win32' ? '.exe' : '';
let server, client, rpc;
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
  const pack = await api('POST', '/admin/packs', {
    name: '隔离整合包',
    minecraft_version: '1.21.1',
    loader: 'neoforge',
  });
  let base = await api('POST', `/admin/packs/${pack.id}/drafts`, { version: '1', changelog: '' });
  base = await api('POST', `/admin/packs/${pack.id}/drafts/${base.id}/publish`, {
    revision: base.revision,
  });
  const mod = await api('POST', '/admin/mods', {
    modid: 'smoke',
    name: '测试模组',
    modrinth_project_id: null,
    curseforge_project_id: null,
  });
  const content = Buffer.from('isolated jar fixture');
  const form = new FormData();
  form.append('file', new Blob([content]), 'smoke.jar');
  const hosted = await api('POST', '/admin/files', form);
  const range = await fetch(hosted.url, { headers: { Range: 'bytes=1-3' } });
  assert.equal(range.status, 206);
  assert.equal(await range.text(), content.subarray(1, 4).toString());
  const artifact = await api('POST', '/admin/artifacts', {
    mod_id: mod.id,
    version: '1',
    filename: hosted.filename,
    size: hosted.size,
    sha512: hosted.sha512,
    hosted_file_id: hosted.id,
    sources: [{ type: 'server', url: hosted.url, priority: 0 }],
  });
  let draft = await api('POST', `/admin/packs/${pack.id}/drafts`, {
    version: '2',
    changelog: 'smoke',
  });
  draft = await api('PUT', `/admin/packs/${pack.id}/drafts/${draft.id}`, {
    revision: draft.revision,
    version: '2',
    changelog: 'smoke',
    items: [{ mod_id: mod.id, artifact_id: artifact.id, target_path: 'mods/中文 空格.jar' }],
  });
  draft = await api('POST', `/admin/packs/${pack.id}/drafts/${draft.id}/publish`, {
    revision: draft.revision,
  });
  const root = path.join(dir, '整合包 空格');
  await mkdir(path.join(root, '.updater'), { recursive: true });
  await mkdir(path.join(root, 'mods'));
  await writeFile(
    path.join(root, '.updater/state.json'),
    JSON.stringify({ installed_release_id: base.id }),
  );
  await writeFile(path.join(root, 'mods/player.jar'), 'sentinel');
  const configOverlay = path.join(dir, 'client.yaml');
  await writeFile(
    configOverlay,
    JSON.stringify({
      pack_id: pack.id,
      pack_name: pack.name,
      server_url: origin,
      game_dir: '.',
      launcher_path: 'launcher.sh',
    }),
  );
  const platform =
    process.platform === 'win32' ? 'windows' : process.platform === 'darwin' ? 'macos' : 'linux';
  const overlay = path.join(dir, 'overlay.json');
  await writeFile(
    overlay,
    JSON.stringify({
      Replace: { [path.join(repo, `config/client.${platform}.yaml`)]: configOverlay },
    }),
  );
  const clientBin = path.join(dir, `client${suffix}`);
  run('go', ['build', '-overlay', overlay, '-o', clientBin, './cmd/client']);
  client = spawn(clientBin, [], { stdio: 'pipe' });
  client.stderr.pipe(process.stderr);
  rpc = createMessageConnection(
    new StreamMessageReader(client.stdout),
    new StreamMessageWriter(client.stdin),
  );
  const hello = new Promise((resolve) => rpc.onNotification('hello', resolve));
  rpc.listen();
  assert.equal((await hello).protocol_version, 1);
  await rpc.sendRequest('initialize', { pack_root: root, protocol_version: 1 });
  const settings = await rpc.sendRequest('set_download_settings', {
    max_concurrent_downloads: 2,
    max_attempts: 2,
    speed_limit_kib: 64,
    proxies: { modrinth: '', curseforge: '', github: '', server: '' },
  });
  assert.equal(settings.download.max_concurrent_downloads, 2);
  assert.equal(settings.download.check_timeout_seconds, 30);
  const prefs = JSON.parse(await readFile(path.join(root, '.updater/preferences.json'), 'utf8'));
  assert.equal(prefs.download.speed_limit_kib, 64);
  await rpc.sendRequest('check_update');
  await rpc.sendRequest('start_update');
  await until(async () => {
    const s = await rpc.sendRequest('get_status');
    if (s.status.phase === 'failed') throw new Error(s.status.error.message);
    return s.status.phase === 'completed';
  });
  const snapshot = await rpc.sendRequest('get_status');
  assert.equal(snapshot.state.installed_release_id, draft.id);
  assert.equal(snapshot.can_launch, true);
  assert.deepEqual(await readFile(path.join(root, 'mods/[测试模组]smoke.jar')), content);
  assert.equal(await readFile(path.join(root, 'mods/player.jar'), 'utf8'), 'sentinel');
  await api('PATCH', `/admin/mods/${mod.id}`, {
    modid: mod.modid,
    name: '重命名:模组',
    modrinth_project_id: null,
    curseforge_project_id: null,
  });
  let renamed = await api('POST', `/admin/packs/${pack.id}/drafts`, { version: '3' });
  renamed = await api('PUT', `/admin/packs/${pack.id}/drafts/${renamed.id}`, {
    revision: renamed.revision,
    version: '3',
    items: [{ mod_id: mod.id, artifact_id: artifact.id, target_path: draft.items[0].target.path }],
  });
  renamed = await api('POST', `/admin/packs/${pack.id}/drafts/${renamed.id}/publish`, {
    revision: renamed.revision,
  });
  await rpc.sendRequest('check_update');
  await rpc.sendRequest('start_update');
  await until(async () => {
    const s = await rpc.sendRequest('get_status');
    return s.status.phase === 'completed' && s.state.installed_release_id === renamed.id;
  });
  assert.deepEqual(await readFile(path.join(root, 'mods/[重命名_模组]smoke.jar')), content);
  await assert.rejects(readFile(path.join(root, 'mods/[测试模组]smoke.jar')), { code: 'ENOENT' });
  assert.equal(await readFile(path.join(root, 'mods/player.jar'), 'utf8'), 'sentinel');
  await rpc.sendRequest('shutdown');
  client.stdin.end();
  await new Promise((r) => client.once('exit', r));
  const audit = await api('GET', '/admin/audit-logs?page_size=100');
  assert.ok(audit.items.some((x) => x.action === 'release.publish' && x.result === 'success'));
  assert.ok(audit.items.some((x) => x.action === 'auth.login' && x.actor_id === 1));
  await api('POST', '/auth/logout');
  assert.equal(
    (await fetch(origin + '/api/v1/auth/me', { headers: { Authorization: `Bearer ${token}` } }))
      .status,
    401,
  );
  console.log(
    'PASS: manual SQL initialization, authentication, publish, upload/range, stdio update, formatted filename, rename replacement, state, sentinel, audit, logout',
  );
} finally {
  rpc?.dispose();
  await stop(client);
  await stop(server);
  await rm(dir, { recursive: true, force: true });
}
