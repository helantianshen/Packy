import { contextBridge, ipcRenderer } from 'electron';
// 仅暴露固定业务命令，不向页面提供 Node 或任意 IPC 能力
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
contextBridge.exposeInMainWorld('packy', {
  branding: () => ipcRenderer.invoke('packy:branding'),
  onUnavailable(listener: (message: string) => void) {
    const handler = (_event: unknown, value: string) => listener(value);
    ipcRenderer.on('packy:unavailable', handler);
    return () => ipcRenderer.removeListener('packy:unavailable', handler);
  },
  invoke(command: string, args?: Record<string, unknown>) {
    if (!allowed.has(command)) return Promise.reject(new Error('未知命令'));
    return ipcRenderer.invoke('packy:command', command, args);
  },
  onStatus(listener: (snapshot: unknown) => void) {
    const handler = (_event: unknown, value: unknown) => listener(value);
    ipcRenderer.on('packy:status', handler);
    return () => ipcRenderer.removeListener('packy:status', handler);
  },
});
