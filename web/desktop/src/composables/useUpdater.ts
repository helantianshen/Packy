import { computed, onMounted, onUnmounted, ref, shallowRef } from 'vue';
import type { ClientSnapshot } from '../protocol';
const invoke = <T = unknown>(command: string, args?: Record<string, unknown>) =>
  window.packy!.invoke(command, args) as Promise<T>;

/** 窗口只消费核心状态，重载或隐藏不会创建新的更新任务 */
export function useUpdater() {
  const snapshot = shallowRef<ClientSnapshot>(),
    error = ref(''),
    notificationHint = ref(''),
    pending = ref(false);
  const desktop = !!window.packy;
  const initializing = ref(desktop);
  const pendingCommand = ref('');
  let unlisten: (() => void) | undefined;
  let disposed = false;
  let stopUnavailable: (() => void) | undefined;
  let initializationError = false;
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
  /** 技术详情由主进程记录，界面仅展示可采取行动的提示 */
  function message(cause: unknown) {
    const text = cause instanceof Error ? cause.message : String(cause);
    if (text.includes('(404)')) return '未找到整合包或发布记录，请联系维护者检查配置';
    if (/网络|连接|fetch|timeout|ECONN|ENOTFOUND/i.test(text))
      return '暂时无法连接更新服务，请检查网络后重试';
    if (/初始化|尚未就绪|断开/.test(text))
      return '更新核心尚未就绪，请重新打开客户端；若仍失败，请反馈日志';
    return '操作未完成，请重试；若仍失败，请将客户端日志反馈给维护者';
  }
  async function action(command: string, args?: Record<string, unknown>) {
    if (pending.value && command !== 'cancel_update') return;
    pending.value = true;
    pendingCommand.value = command;
    error.value = '';
    try {
      if (!desktop) throw new Error('请在 Packy 桌面应用中使用更新功能');
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
      stopUnavailable = window.packy!.onUnavailable((message) => {
        error.value = message;
        initializing.value = false;
      });
      // 先订阅再读快照，同一进程内按序号过滤过期状态
      const stop = window.packy!.onStatus((incoming) => {
        const current = snapshot.value;
        if (
          current &&
          current.status.instance_id === incoming.status.instance_id &&
          current.status.sequence > incoming.status.sequence
        )
          return;
        snapshot.value = incoming;
        if (initializationError) {
          error.value = '';
          initializationError = false;
        }
        initializing.value = false;
      });
      if (disposed) stop();
      else unlisten = stop;
      await refresh();
    } catch (cause) {
      if (!snapshot.value && !String(cause).includes('尚未就绪')) {
        initializationError = true;
        error.value = message(cause);
      }
    } finally {
      initializing.value = !snapshot.value && !error.value;
    }
  });
  onUnmounted(() => {
    disposed = true;
    unlisten?.();
    stopUnavailable?.();
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
