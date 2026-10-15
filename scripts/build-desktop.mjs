import { parse } from 'yaml';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFileSync, mkdirSync, copyFileSync, writeFileSync } from 'node:fs';
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
  let platform = native;
  let checkOnly = false;
  const args = process.argv.slice(2);
  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--platform' && args[i + 1]) platform = args[++i];
    else if (args[i] === '--check') checkOnly = true;
    else throw new Error(`未知参数：${args[i]}`);
  }
  // 原生构建需要本机 SDK，不能把切换配置当成交叉编译
  if (!native || platform !== native)
    throw new Error(`当前系统只能构建 ${native || process.platform}，不能构建 ${platform}`);
  const target = run('rustc', ['-vV'], true).match(/^host: (.+)$/m)?.[1];
  if (!target) throw new Error('无法识别 Rust 本机目标');
  if (process.env.CARGO_BUILD_TARGET && process.env.CARGO_BUILD_TARGET !== target)
    throw new Error('CARGO_BUILD_TARGET 与本机目标不同，请在对应平台构建');
  const config = `config/client.${platform}.yaml`;
  const configBytes = readFileSync(resolve(root, config));
  parse(configBytes.toString());
  const name = `packy-desktop-${platform}-${target}`;
  const plan = { platform, target, config, output: `artifacts/desktop/${name}.tar.gz` };
  console.log(JSON.stringify(plan, null, 2));
  if (!checkOnly) {
    const npm = process.env.npm_execpath;
    if (!npm) throw new Error('请通过 npm run desktop:build 执行构建');
    const npmRun = (args) => run(process.execPath, [npm, ...args]);
    npmRun(['run', 'types']);
    npmRun([
      'run',
      'tauri',
      '--workspace',
      '@packy/desktop',
      '--',
      'build',
      '--no-bundle',
      '--target',
      target,
      '--',
      '--locked',
    ]);
    if (!configBytes.equals(readFileSync(resolve(root, config))))
      throw new Error('构建期间配置发生变化，请重新构建');
    const metadata = JSON.parse(
      run('cargo', ['metadata', '--no-deps', '--format-version', '1'], true),
    );
    const binary = `packy-desktop${platform === 'windows' ? '.exe' : ''}`;
    const source = resolve(metadata.target_directory, target, 'release', binary);
    const output = resolve(root, 'artifacts/desktop');
    const folder = resolve(output, name);
    mkdirSync(folder, { recursive: true });
    copyFileSync(source, resolve(folder, binary));
    const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');
    writeFileSync(
      resolve(folder, 'build-info.json'),
      JSON.stringify(
        {
          ...plan,
          config_sha256: sha256(configBytes),
          binary_sha256: sha256(readFileSync(source)),
        },
        null,
        2,
      ) + '\n',
    );
    // tar 保留 Linux/macOS 执行权限，归档中只有程序和构建摘要
    run('tar', ['-czf', resolve(root, plan.output), '-C', folder, binary, 'build-info.json']);
    console.log(`构建完成：${plan.output}`);
  }
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}
