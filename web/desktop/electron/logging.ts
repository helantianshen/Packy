import { app } from 'electron';
import log from 'electron-log/main';
import path from 'node:path';

export const logPath = path.join(app.getPath('appData'), 'Packy', 'logs', 'client.log');
log.transports.file.resolvePathFn = () => logPath;
log.transports.file.maxSize = 5 * 1024 * 1024;

/** 日志不保留 URL 查询参数、用户凭据或认证头 */
export function safeLogText(value: unknown): string {
  const text = value instanceof Error ? value.stack || value.message : String(value);
  return text
    .replace(/https?:\/\/[^\s"'<>]+/gi, (raw) => {
      try {
        const url = new URL(raw);
        url.username = '';
        url.password = '';
        url.search = '';
        url.hash = '';
        return url.toString();
      } catch {
        return '[URL]';
      }
    })
    .replace(/(Bearer\s+)\S+/gi, '$1[已隐藏]')
    .replace(/((?:password|token|secret|authorization)\s*[=:]\s*)[^\s,;]+/gi, '$1[已隐藏]')
    .slice(0, 16384);
}
export function logError(context: string, error: unknown) {
  log.error(context, safeLogText(error));
}
process.on('uncaughtExceptionMonitor', (error) => logError('未捕获异常', error));
process.on('unhandledRejection', (error) => logError('未处理的异步异常', error));
log.info('客户端启动', { version: app.getVersion(), platform: process.platform });
