import { spawnSync } from 'node:child_process';
import { mkdir, cp, mkdtemp, rm } from 'node:fs/promises';
import path from 'node:path';
import { tmpdir } from 'node:os';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const platform = process.argv[2] || process.platform;
const arch = process.argv[3] || process.arch;
if (!['linux', 'win32', 'darwin'].includes(platform) || !['x64', 'arm64'].includes(arch))
  throw new Error('支持 linux/win32/darwin 与 x64/arm64');
const require = createRequire(path.join(root, 'web/desktop/package.json'));
const { packager } = await import(require.resolve('@electron/packager'));
function run(command, args, env = {}) {
  const result = spawnSync(command, args, {
    cwd: root,
    stdio: 'inherit',
    env: { ...process.env, ...env },
    shell: false,
  });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`${command} 构建失败`);
}
const npmCLI =
  process.env.npm_execpath ||
  path.resolve(
    path.dirname(process.execPath),
    process.platform === 'win32'
      ? 'node_modules/npm/bin/npm-cli.js'
      : '../lib/node_modules/npm/bin/npm-cli.js',
  );
run(process.execPath, [npmCLI, '--prefix', 'web/desktop', 'run', 'build']);
const temporary = await mkdtemp(path.join(tmpdir(), 'packy-package-'));
try {
  const sidecar = path.join(temporary, 'sidecar');
  await mkdir(sidecar, { recursive: true });
  run(
    'go',
    [
      'build',
      '-trimpath',
      '-o',
      path.join(sidecar, platform === 'win32' ? 'packy-client.exe' : 'packy-client'),
      './cmd/client',
    ],
    {
      CGO_ENABLED: '0',
      GOOS: platform === 'win32' ? 'windows' : platform,
      GOARCH: arch === 'x64' ? 'amd64' : 'arm64',
    },
  );
  const app = path.join(temporary, 'app');
  await mkdir(app, { recursive: true });
  // 打包输入只包含当前构建与资源，依赖由 Packager 保留生产部分
  await cp(path.join(root, 'web/desktop'), app, {
    recursive: true,
    filter: (src) => !['sidecar', '.vite'].includes(path.basename(src)),
  });
  const result = await packager({
    dir: app,
    name: 'Packy',
    platform,
    arch,
    out: path.join(root, 'artifacts', 'desktop'),
    overwrite: true,
    asar: true,
    prune: true,
    extraResource: [sidecar, path.join(root, 'web/desktop/assets')],
    icon: platform === 'win32' ? path.join(root, 'web/desktop/assets/icon.ico') : undefined,
    ignore: [/^\/src/, /^\/electron/, /^\/tests/, /^\/tsconfig/, /^\/vite.config/],
    appBundleId: 'top.hikaritish.packy',
  });
  console.log(result.join('\n'));
} finally {
  await rm(temporary, { recursive: true, force: true });
}
