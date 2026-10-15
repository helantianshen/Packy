import { computed, onMounted, onUnmounted, ref, shallowRef } from 'vue';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { isPermissionGranted, requestPermission } from '@tauri-apps/plugin-notification';
import type { ClientSnapshot } from '../../../../web/shared/generated/ClientSnapshot';
import type { TaskStatus } from '../../../../web/shared/generated/TaskStatus';

/** 窗口只消费核心状态，重载或隐藏不会创建新的更新任务 */
export function useUpdater() {
  const snapshot = shallowRef<ClientSnapshot>(),
    error = ref(''),
    notificationHint = ref(''),
    pending = ref(false);
  const desktop = isTauri();
  const initializing = ref(desktop);
  const pendingCommand = ref('');
  let unlisten: UnlistenFn | undefined;
  let disposed = false;
  const status = computed(() => snapshot.value?.status);
  const active = computed(
    () =>
      !!status.value &&
      ['checking', 'downloading', 'verifying', 'installing', 'saving_state'].includes(
        status.value.phase,
      ),
  );
  async function refresh() {
    snapshot.value = await invoke<ClientSnapshot>('get_status');
  }
  /** Tauri 业务错误直接保留安全消息，其他异常统一转换为可读文本 */
  function message(cause: unknown) {
    return typeof cause === 'object' && cause !== null && 'message' in cause
      ? String(cause.message)
      : String(cause);
  }
  async function action(command: string, args?: Record<string, unknown>) {
    if (pending.value) return;
    pending.value = true;
    pendingCommand.value = command;
    error.value = '';
    try {
      if (!desktop) throw new Error('请在 Packy 桌面应用中使用更新功能');
      if (command === 'start_update') {
        try {
          if (!(await isPermissionGranted()) && (await requestPermission()) !== 'granted')
            notificationHint.value = '系统通知未获授权，仍可从托盘打开窗口查看结果。';
        } catch {
          notificationHint.value = '系统通知暂不可用，仍可从托盘打开窗口查看结果。';
        }
      }
      await invoke(command, args);
      await refresh();
    } catch (cause) {
      error.value = message(cause);
      if (desktop) await refresh().catch(() => undefined);
    } finally {
      pending.value = false;
      pendingCommand.value = '';
    }
  }
  onMounted(async () => {
    if (!desktop) {
      error.value = '浏览器用于界面预览，实际更新请运行 Packy 桌面应用。';
      return;
    }
    try {
      // 先订阅再读快照，避免初始化期间漏掉终态；落后的事件由序号过滤
      const stop = await listen<TaskStatus>('packy:update-status', (event) => {
        const current = snapshot.value;
        if (!current) return;
        const previous = current.status,
          incoming = event.payload;
        if (previous.task_id === incoming.task_id && previous.sequence >= incoming.sequence) return;
        snapshot.value = { ...current, status: incoming };
        if (['completed', 'failed', 'cancelled'].includes(incoming.phase))
          void refresh().catch((cause) => {
            error.value = message(cause);
          });
      });
      if (disposed) stop();
      else unlisten = stop;
      await refresh();
    } catch (cause) {
      error.value = message(cause);
    } finally {
      initializing.value = false;
    }
  });
  onUnmounted(() => {
    disposed = true;
    unlisten?.();
  });
  return {
    snapshot,
    status,
    error,
    notificationHint,
    pending,
    pendingCommand,
    initializing,
    active,
    desktop,
    action,
  };
}
