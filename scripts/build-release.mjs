import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFileSync, mkdirSync, copyFileSync, cpSync, rmSync, writeFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
function run(command, args, capture = false) {
  const result = spawnSync(command, args, {
    cwd: root,
    encoding: 'utf8',
    stdio: capture ? 'pipe' : 'inherit',
  });
  if (result.error) throw result.error;
  if (result.status !== 0)
    throw new Error(`${command} 执行失败（${result.status}）${result.stderr || ''}`);
  return result.stdout;
}

try {
  const native = { win32: 'windows', linux: 'linux', darwin: 'macos' }[process.platform];
  let platform = native,
    only = 'all',
    check = false;
  const args = process.argv.slice(2);
  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--platform' && args[i + 1]) platform = args[++i];
    else if (args[i] === '--only' && args[i + 1]) only = args[++i];
    else if (args[i] === '--check') check = true;
    else throw new Error(`未知参数：${args[i]}`);
  }
  if (!['all', 'desktop', 'server'].includes(only))
    throw new Error('--only 仅支持 all、desktop、server');
  if (!native || platform !== native)
    throw new Error('本机只构建当前系统，三端客户端和服务端全量构建请运行 Packy builds CI');
  const target = run('rustc', ['-vV'], true).match(/^host: (.+)$/m)?.[1];
  if (!target) throw new Error('无法识别 Rust 本机目标');
  if (process.env.CARGO_BUILD_TARGET && process.env.CARGO_BUILD_TARGET !== target)
    throw new Error('CARGO_BUILD_TARGET 与本机目标不符');
  const name = `packy-server-${platform}-${target}`;
  const plan = { only, platform, target, server: `artifacts/server/${name}.tar.gz` };
  console.log(JSON.stringify(plan, null, 2));
  if (!check) {
    const npm = process.env.npm_execpath;
    if (!npm) throw new Error('请通过 npm run release:build 执行');
    const npmRun = (args) => run(process.execPath, [npm, ...args]);
    if (only !== 'server')
      run(process.execPath, ['scripts/build-desktop.mjs', '--platform', platform]);
    if (only !== 'desktop') {
      if (only === 'server') npmRun(['run', 'types']);
      npmRun(['run', 'build', '--workspace', '@packy/admin']);
      run('cargo', ['build', '--locked', '--release', '-p', 'packy-server', '--target', target]);
      const metadata = JSON.parse(
        run('cargo', ['metadata', '--no-deps', '--format-version', '1'], true),
      );
      const binary = `packy-server${platform === 'windows' ? '.exe' : ''}`;
      const source = resolve(metadata.target_directory, target, 'release', binary);
      const folder = resolve(root, 'artifacts/server', name);
      mkdirSync(folder, { recursive: true });
      copyFileSync(source, resolve(folder, binary));
      // 后台目录只用于本次打包，清除历史 chunk 后复制完整静态产物
      rmSync(resolve(folder, 'admin'), { recursive: true, force: true });
      cpSync(resolve(root, 'web/admin/dist'), resolve(folder, 'admin'), { recursive: true });
      mkdirSync(resolve(folder, 'config'), { recursive: true });
      copyFileSync(
        resolve(root, 'config/server.yaml.example'),
        resolve(folder, 'config/server.yaml.example'),
      );
      for (const template of ['server.yaml.example', 'server.env.example'])
        rmSync(resolve(folder, template), { force: true });
      copyFileSync(resolve(root, 'examples/server-README.txt'), resolve(folder, 'README.txt'));
      const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');
      writeFileSync(
        resolve(folder, 'build-info.json'),
        JSON.stringify(
          {
            platform,
            target,
            binary_sha256: sha256(readFileSync(source)),
            admin_index_sha256: sha256(readFileSync(resolve(folder, 'admin/index.html'))),
          },
          null,
          2,
        ) + '\n',
      );
      run('tar', [
        '-czf',
        resolve(root, plan.server),
        '-C',
        folder,
        binary,
        'admin',
        'config/server.yaml.example',
        'README.txt',
        'build-info.json',
      ]);
      console.log(`服务端构建完成：${plan.server}`);
    }
  }
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}
