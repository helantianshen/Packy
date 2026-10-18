import { logError } from './logging';
import {
  app,
  BrowserWindow,
  ipcMain,
  Tray,
  Menu,
  Notification,
  dialog,
  nativeImage,
} from 'electron';
import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { readBrandFile, brandDataURL } from './brand-files';
import {
  createMessageConnection,
  StreamMessageReader,
  StreamMessageWriter,
  type MessageConnection,
} from 'vscode-jsonrpc/node';
let win: BrowserWindow | undefined,
  tray: Tray | undefined,
  child: ChildProcessWithoutNullStreams | undefined,
  rpc: MessageConnection | undefined;
let quitting = false,
  exiting = false,
  ready = false,
  polling = false,
  initialError = '',
  timer: NodeJS.Timeout | undefined;
type Snapshot = {
  preferences: { background_enabled: boolean };
  status: {
    phase: string;
    task_id: string;
    sequence: number;
    error?: { code: string; message: string; request_id?: string };
  };
  config: { pack_name: string };
};
let snapshot: Snapshot | undefined;
let lastStatusError = '';
const branding: Record<string, string> = {};
async function brandImage(name: string) {
  const bytes = await readBrandFile(packRoot(), name);
  if (!bytes || !brandDataURL(bytes, name)) return undefined;
  const image = nativeImage.createFromBuffer(bytes);
  const size = image.getSize();
  if (
    image.isEmpty() ||
    size.width > 8192 ||
    size.height > 8192 ||
    size.width * size.height > 32000000
  )
    return undefined;
  return image;
}
const allowed = new Set([
  'get_status',
  'check_update',
  'start_update',
  'retry_update',
  'cancel_update',
  'set_background_enabled',
  'set_download_settings',
  'launch_game',
]);
function critical() {
  return snapshot?.status.phase === 'installing' || snapshot?.status.phase === 'saving_state';
}
function resource(name: string) {
  return app.isPackaged
    ? path.join(process.resourcesPath, name)
    : path.join(app.getAppPath(), name);
}
function packRoot() {
  if (!app.isPackaged) {
    const root = process.env.PACKY_DEV_ROOT;
    if (!root || !path.isAbsolute(root)) throw new Error('开发模式需要绝对路径 PACKY_DEV_ROOT');
    return root;
  }
  // macOS 从应用包外定位整合包，其余平台从便携分发目录外定位
  return process.platform === 'darwin'
    ? path.resolve(process.execPath, '../../../..')
    : path.dirname(path.dirname(process.execPath));
}
async function refresh() {
  if (!ready || !rpc || polling) return;
  polling = true;
  try {
    const previous = snapshot;
    snapshot = await rpc.sendRequest<Snapshot>('get_status');
    const key = snapshot.status.error ? JSON.stringify(snapshot.status.error) : '';
    if (key && key !== lastStatusError) logError('更新任务失败', key);
    lastStatusError = key;
    if (!win?.isDestroyed()) win?.webContents.send('packy:status', snapshot);
    if (
      snapshot.status.phase === 'completed' &&
      (previous?.status.phase !== 'completed' ||
        previous?.status.task_id !== snapshot.status.task_id) &&
      Notification.isSupported()
    ) {
      const notice = new Notification({
        title: 'Packy 更新完成',
        body: '整合包版本已保存，可以打开启动器',
      });
      notice.on('click', () => win?.show());
      notice.show();
    }
  } finally {
    polling = false;
  }
}
async function quit() {
  if (quitting || exiting) return;
  exiting = true;
  try {
    if (ready && rpc) {
      await refresh();
      if (critical()) throw new Error('正在安装或保存版本，请稍后退出');
      await rpc.sendRequest('shutdown');
      child?.stdin.end();
      await new Promise<void>((resolve) => {
        if (!child || child.exitCode !== null) {
          resolve();
          return;
        }
        child.once('exit', () => resolve());
      });
    }
    quitting = true;
    if (timer) clearInterval(timer);
    tray?.destroy();
    app.quit();
  } catch (e) {
    await dialog.showMessageBox(win!, { type: 'info', message: String(e) });
  } finally {
    exiting = false;
  }
}
async function initialize() {
  const root = packRoot();
  child = spawn(
    resource(process.platform === 'win32' ? 'sidecar/packy-client.exe' : 'sidecar/packy-client'),
    [],
    { cwd: root, stdio: 'pipe', windowsHide: true, shell: false },
  );
  child.stderr.on('data', (data) => logError('Sidecar stderr', data.toString()));
  const hello = new Promise<void>((resolve, reject) => {
    const timeout = setTimeout(() => reject(new Error('更新核心启动超时')), 15000);
    child!.once('error', reject);
    rpc = createMessageConnection(
      new StreamMessageReader(child!.stdout),
      new StreamMessageWriter(child!.stdin),
    );
    rpc.onNotification('hello', (message: { protocol_version: number }) => {
      clearTimeout(timeout);
      if (message.protocol_version !== 1) reject(new Error('更新核心协议不匹配'));
      else resolve();
    });
    rpc.onClose(() => {
      ready = false;
      initialError = '更新核心已断开，请重新打开应用恢复任务';
      if (!quitting && !win?.isDestroyed())
        win?.webContents.send('packy:unavailable', initialError);
    });
    rpc.listen();
  });
  child.on('exit', (code, signal) => {
    if (!quitting && !exiting) logError('更新核心退出', `code=${code}, signal=${signal}`);
    ready = false;
  });
  await hello;
  snapshot = await rpc!.sendRequest<Snapshot>('initialize', {
    pack_root: root,
    protocol_version: 1,
  });
  ready = true;
  timer = setInterval(() => {
    void refresh().catch((e) => {
      initialError = String(e);
    });
  }, 250);
}
// 每个整合包使用独立 Electron 会话，第二个窗口请求唤醒已有实例
try {
  app.setPath(
    'userData',
    path.join(
      app.getPath('userData'),
      createHash('sha256').update(packRoot()).digest('hex').slice(0, 24),
    ),
  );
} catch {
  /* 缺少开发根目录时由初始化流程报告可读错误 */
}
const ownsWindow = app.requestSingleInstanceLock();
if (!ownsWindow) {
  quitting = true;
  app.quit();
}
app.on('second-instance', () => {
  win?.show();
  win?.focus();
});
app.on('before-quit', (event) => {
  if (!quitting) {
    event.preventDefault();
    void quit();
  }
});
app.on('window-all-closed', () => {});
app
  .whenReady()
  .then(async () => {
    if (!ownsWindow) return;
    app.setAppUserModelId('Packy');
    for (const [key, name] of Object.entries({
      logo: 'logo.png',
      light: 'background-light.webp',
      dark: 'background-dark.webp',
    })) {
      const bytes = await readBrandFile(packRoot(), name).catch(() => undefined);
      const url = bytes && brandDataURL(bytes, name);
      if (url) branding[key] = url;
    }
    win = new BrowserWindow({
      width: 1060,
      height: 760,
      minWidth: 720,
      minHeight: 560,
      show: false,
      autoHideMenuBar: true,
      icon: branding.logo
        ? nativeImage.createFromDataURL(branding.logo)
        : resource('assets/icon.png'),
      webPreferences: {
        preload: path.join(__dirname, 'preload.js'),
        contextIsolation: true,
        nodeIntegration: false,
        sandbox: true,
      },
    });
    win.webContents.on('console-message', (details) => {
      if (details.level === 'error' || details.level === 'warning')
        logError('界面控制台', details.message);
    });
    win.webContents.on('render-process-gone', (_event, details) =>
      logError('界面进程退出', details.reason),
    );
    win.webContents.setWindowOpenHandler(() => ({ action: 'deny' }));
    win.webContents.on('will-navigate', (event) => event.preventDefault());
    win.webContents.session.setPermissionRequestHandler((_contents, _permission, callback) =>
      callback(false),
    );
    win.webContents.session.webRequest.onHeadersReceived((details, callback) =>
      callback({
        responseHeaders: {
          ...details.responseHeaders,
          'Content-Security-Policy': [
            "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self' data:; connect-src 'none'",
          ],
        },
      }),
    );
    ipcMain.handle('packy:branding', (event) => {
      if (event.sender !== win?.webContents || event.senderFrame !== win.webContents.mainFrame)
        throw new Error('不允许的来源');
      return branding;
    });
    ipcMain.handle(
      'packy:command',
      async (event, command: string, args?: Record<string, unknown>) => {
        if (
          event.sender !== win?.webContents ||
          event.senderFrame !== win.webContents.mainFrame ||
          !allowed.has(command)
        )
          throw new Error('不允许的命令');
        try {
          if (!ready || !rpc) throw new Error(initialError || '更新核心尚未就绪');
          const value = await rpc.sendRequest(command, args);
          await refresh();
          return value;
        } catch (error) {
          logError(`客户端命令失败: ${command}`, error);
          throw error;
        }
      },
    );
    win.on('close', (event) => {
      if (quitting) return;
      event.preventDefault();
      if (snapshot?.preferences.background_enabled && tray) win?.hide();
      else void quit();
    });
    win.once('ready-to-show', () => win?.show());
    await win.loadFile(path.join(app.getAppPath(), 'dist/index.html'));
    try {
      const trayName = process.platform === 'darwin' ? 'trayTemplate.png' : 'tray.png';
      const custom = await brandImage(trayName);
      const icon = custom || nativeImage.createFromPath(resource('assets/icon.png'));
      const density = await brandImage(trayName.replace('.png', '@2x.png'));
      if (density) icon.addRepresentation({ scaleFactor: 2, buffer: density.toPNG() });
      if (process.platform === 'darwin' && custom) icon.setTemplateImage(true);
      tray = new Tray(
        icon.resize({
          width: process.platform === 'darwin' ? 18 : 24,
          height: process.platform === 'darwin' ? 18 : 24,
        }),
      );
      tray.setToolTip('Packy');
      tray.setContextMenu(
        Menu.buildFromTemplate([
          { label: '打开 Packy', click: () => win?.show() },
          { label: '退出', click: () => void quit() },
        ]),
      );
      tray.on('click', () => win?.show());
    } catch (e) {
      logError('托盘不可用', e);
    }
    try {
      await initialize();
      await refresh();
    } catch (e) {
      initialError = '更新核心初始化失败，请检查整合包目录，详细原因见客户端日志';
      logError('更新核心初始化失败', e);
      win.webContents.send('packy:unavailable', initialError);
      await dialog.showMessageBox(win, {
        type: 'error',
        message: '更新核心初始化失败',
        detail: initialError,
      });
      child?.stdin.end();
    }
  })
  .catch((error) => {
    logError('客户端启动失败', error);
    dialog.showErrorBox('Packy 启动失败', '请检查客户端日志中的详细原因');
    quitting = true;
    app.quit();
  });
app.on('activate', () => win?.show());
