import { spawnSync } from 'node:child_process';
import { mkdir, cp, writeFile, rm } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
function run(command, args) {
  const result = spawnSync(command, args, { cwd: root, stdio: 'inherit' });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`${command} 失败`);
}
const out = path.join(root, 'artifacts', `server-${process.platform}-${process.arch}`);
// 移除旧打包流程遗留的后台产物，API 包不携带前端资源
await rm(path.join(out, 'web', 'admin'), { recursive: true, force: true });
await rm(path.join(out, 'LICENSE-arco'), { force: true });
await mkdir(path.join(out, 'config'), { recursive: true });
await mkdir(path.join(out, 'sql'), { recursive: true });
run('go', [
  'build',
  '-trimpath',
  '-o',
  path.join(out, process.platform === 'win32' ? 'packy-server.exe' : 'packy-server'),
  './cmd/server',
]);
await cp(
  path.join(root, 'config/server.yaml.example'),
  path.join(out, 'config/server.yaml.example'),
);
await cp(path.join(root, 'sql'), path.join(out, 'sql'), { recursive: true });
if (process.platform === 'linux') {
  await cp(path.join(root, 'scripts/server'), out, { recursive: true });
}
await cp(path.join(root, 'README.md'), path.join(out, 'README.md'));
await writeFile(
  path.join(out, 'build-info.json'),
  JSON.stringify(
    {
      platform: process.platform,
      arch: process.arch,
      stack: 'Go + Gin + GORM (API only)',
      built_at: new Date().toISOString(),
    },
    null,
    2,
  ),
);
console.log(out);
