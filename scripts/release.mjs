import { spawnSync } from 'node:child_process';
import { cp, mkdir, mkdtemp, readFile, rename } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const args = process.argv.slice(2);
if (args.some((arg) => arg !== '--dry-run')) throw new Error('仅支持 --dry-run 参数');
const dryRun = args.includes('--dry-run');
const expectedNode = (await readFile(path.join(root, '.node-version'), 'utf8')).trim();
if (process.versions.node !== expectedNode)
  throw new Error(`请使用 Node ${expectedNode}，当前为 ${process.versions.node}`);
const npmCLI = process.env.npm_execpath;
if (!npmCLI) throw new Error('请通过 release.sh、release.ps1 或 npm run release 调用');
const targets = ['linux', 'win32', 'darwin'].flatMap((platform) =>
  ['x64', 'arm64'].map((arch) => [platform, arch]),
);
function run(command, argv, env = {}) {
  console.log(`> ${[command, ...argv].map((value) => JSON.stringify(value)).join(' ')}`);
  if (dryRun) return;
  const result = spawnSync(command, argv, {
    cwd: root,
    stdio: 'inherit',
    env: { ...process.env, ...env },
    shell: false,
  });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`命令失败：${command}（退出码 ${result.status}）`);
}
function npm(argv) {
  run(process.execPath, [npmCLI, ...argv]);
}
async function copy(source, destination) {
  console.log(`复制 ${source} -> ${destination}`);
  if (!dryRun) await cp(source, destination, { recursive: true, verbatimSymlinks: true });
}

// 每次构建使用独立目录，只有全部成功才移除 incomplete 后缀
let output;
if (dryRun) {
  output = path.join(root, 'release', '<本次构建>.incomplete');
} else {
  run('go', ['version']);
  if (process.platform !== 'win32') run('wine', ['--version']);
  await mkdir(path.join(root, 'release'), { recursive: true });
  const temporary = await mkdtemp(path.join(root, 'release', 'build-'));
  output = temporary + '.incomplete';
  await rename(temporary, output);
}
try {
  npm(['ci']);
  npm(['--prefix', 'web/admin', 'ci']);
  npm(['--prefix', 'web/desktop', 'ci']);
  const serverDir = path.join(output, 'server', `${process.platform}-${process.arch}`);
  if (!dryRun) await mkdir(serverDir, { recursive: true });
  run(
    'go',
    [
      'build',
      '-trimpath',
      '-o',
      path.join(serverDir, process.platform === 'win32' ? 'packy-server.exe' : 'packy-server'),
      './cmd/server',
    ],
    {
      GOOS: process.platform === 'win32' ? 'windows' : process.platform,
      GOARCH: process.arch === 'x64' ? 'amd64' : process.arch,
      CGO_ENABLED: '1',
    },
  );
  await copy(
    path.join(root, 'config/server.yaml.example'),
    path.join(serverDir, 'config/server.yaml.example'),
  );
  await copy(path.join(root, 'sql/init.sql'), path.join(serverDir, 'sql/init.sql'));
  if (process.platform === 'linux') {
    await copy(path.join(root, 'scripts/server'), serverDir);
  }
  npm(['--prefix', 'web/admin', 'run', 'build']);
  await copy(path.join(root, 'web/admin/dist'), path.join(output, 'admin'));
  for (const [platform, arch] of targets) {
    npm(['run', 'desktop:build', '--', platform, arch]);
    await copy(
      path.join(root, 'artifacts/desktop', `Packy-${platform}-${arch}`),
      path.join(output, 'clients', `${platform}-${arch}`),
    );
  }
  const complete = output.replace(/\.incomplete$/, '');
  if (!dryRun) await rename(output, complete);
  console.log(`${dryRun ? '预览结束，未执行构建' : '全部构建完成'}：${complete}`);
} catch (error) {
  console.error(`构建未完成，已生成的文件保留在：${output}`);
  throw error;
}
