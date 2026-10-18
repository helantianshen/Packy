async (page) => {
  const errors = [];
  page.on('pageerror', (e) => errors.push(e.message));
  await page.addInitScript(() => {
    const id = '11111111-1111-4111-8111-111111111111';
    const state = {
      can_launch: false,
      config: {
        pack_name: '蒹葭服务器整合包',
        pack_id: id,
        server_url: 'https://example.invalid',
        game_dir: '.minecraft',
        launcher_path: 'HMCL.jar',
      },
      preferences: { background_enabled: true },
      state: { installed_release_id: id },
      status: {
        task_id: '',
        instance_id: 'test',
        sequence: 1,
        phase: 'available',
        completed_files: 0,
        total_files: 2,
        downloaded_bytes: 0,
        total_bytes: 134217728,
        bytes_per_second: 0,
      },
      plan: {
        pack_id: id,
        pack_name: '蒹葭服务器整合包',
        target_release_id: '22222222-2222-4222-8222-222222222222',
        target_version: '1.5.0',
        changelog: '优化性能与光影兼容性\n本次更新调整渲染组件，并移除旧版兼容补丁。',
        operations: [
          {
            action: 'ADD',
            mod_id: 'a',
            target: {
              path: 'mods/sodium-0.6.jar',
              size: 67108864,
              sha512: 'a'.repeat(128),
              sources: [],
            },
          },
          {
            action: 'REPLACE',
            mod_id: 'b',
            old_path: 'mods/iris-1.6.jar',
            target: {
              path: 'mods/iris-1.7.jar',
              size: 67108864,
              sha512: 'b'.repeat(128),
              sources: [],
            },
          },
          { action: 'REMOVE', mod_id: 'c', old_path: 'mods/legacy-fix.jar' },
        ],
      },
    };
    state.download_defaults = {
      max_concurrent_downloads: 4,
      check_timeout_seconds: 30,
      download_timeout_seconds: 600,
      max_attempts: 3,
      speed_limit_kib: 0,
      proxies: { modrinth: '', curseforge: '', github: 'http://127.0.0.1:7890', server: '' },
    };
    state.preferences.download = JSON.parse(sessionStorage.getItem('download-test') || '{}');
    state.download = {
      ...state.download_defaults,
      ...state.preferences.download,
      proxies: { ...state.download_defaults.proxies, ...state.preferences.download.proxies },
    };
    let listener;
    window.__state = state;
    window.__calls = [];
    window.__emit = (phase, patch = {}) => {
      Object.assign(state.status, { phase, sequence: state.status.sequence + 1, ...patch });
      listener?.(structuredClone(state));
    };
    window.packy = {
      branding: async () => ({}),
      onUnavailable: () => () => {},
      onStatus: (fn) => {
        listener = fn;
        return () => {};
      },
      invoke: async (command, args) => {
        window.__calls.push({ command, args });
        if (command === 'start_update')
          window.__emit('downloading', {
            task_id: 'task-1',
            downloaded_bytes: 83256934,
            bytes_per_second: 8598323,
          });
        if (command === 'cancel_update') window.__emit('cancelled');
        if (command === 'retry_update') window.__emit('downloading', { error: undefined });
        if (command === 'set_background_enabled')
          state.preferences.background_enabled = args.enabled;
        if (command === 'set_download_settings') {
          if (window.__failSave) throw new Error('fixture save failure');
          state.preferences.download = args;
          sessionStorage.setItem('download-test', JSON.stringify(args));
          state.download = {
            ...state.download_defaults,
            ...args,
            proxies: { ...state.download_defaults.proxies, ...args.proxies },
          };
          listener?.(structuredClone(state));
        }
        return structuredClone(state);
      },
    };
    localStorage.setItem('packy-theme', 'light');
  });
  await page.setViewportSize({ width: 1060, height: 760 });
  await page.goto('http://127.0.0.1:5179');
  await page.getByRole('button', { name: '配置与说明', exact: true }).click();
  await page.getByLabel('最大并发下载数').fill('7');
  await page.getByLabel('GitHub 代理', { exact: true }).fill('');
  await page.getByLabel('总下载限速（KiB/s）').fill('1024');
  await page.getByRole('button', { name: '保存下载设置', exact: true }).click();
  await page.getByRole('status').filter({ hasText: '已保存' }).waitFor();
  const saved = await page.evaluate(() => JSON.parse(sessionStorage.getItem('download-test')));
  if (
    saved.max_concurrent_downloads !== 7 ||
    saved.proxies.github !== '' ||
    saved.speed_limit_kib !== 1024 ||
    saved.max_attempts !== undefined
  )
    throw Error('override persistence');
  await page.reload();
  await page.getByRole('button', { name: '配置与说明', exact: true }).click();
  if ((await page.getByLabel('最大并发下载数').inputValue()) !== '7') throw Error('reload');
  await page.getByLabel('Modrinth 代理', { exact: true }).fill('http://user:secret@localhost:7890');
  await page.getByRole('button', { name: '保存下载设置', exact: true }).click();
  await page.getByRole('alert').filter({ hasText: '无账号密码' }).waitFor();
  await page.getByRole('button', { name: '取消修改', exact: true }).click();
  await page.getByLabel('最大尝试次数').fill('5');
  await page.evaluate(() => (window.__failSave = true));
  await page.getByRole('button', { name: '保存下载设置', exact: true }).click();
  await page.getByRole('alert').filter({ hasText: '保存失败' }).waitFor();
  await page.evaluate(() => (window.__failSave = false));
  await page.getByRole('button', { name: '全部恢复默认', exact: true }).click();
  await page.getByRole('button', { name: '保存下载设置', exact: true }).click();
  await page.getByRole('status').filter({ hasText: '已保存' }).waitFor();
  if ((await page.getByLabel('最大并发下载数').inputValue()) !== '4') throw Error('defaults');
  await page.waitForTimeout(300);
  await page.screenshot({
    path: '.impeccable/review/client-b/settings-download-light.png',
    fullPage: true,
  });
  await page.getByRole('button', { name: '切换深色主题' }).click();
  await page.waitForTimeout(350);
  await page.screenshot({
    path: '.impeccable/review/client-b/settings-download-dark.png',
    fullPage: true,
  });
  await page.setViewportSize({ width: 390, height: 844 });
  await page.waitForTimeout(100);
  if (await page.evaluate(() => document.documentElement.scrollWidth > innerWidth))
    throw Error('overflow');
  await page.screenshot({
    path: '.impeccable/review/client-b/settings-download-mobile.png',
    fullPage: true,
  });
  if (errors.length) throw Error(errors.join('\n'));
};
