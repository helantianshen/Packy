import { ref } from 'vue';

/** 提交统一串行化，错误留在当前页面，不把失败显示为成功 */
export function useAction() {
  const busy = ref(false),
    error = ref(''),
    success = ref('');
  async function run(action: () => Promise<void>, message = '') {
    if (busy.value) return;
    busy.value = true;
    error.value = '';
    success.value = '';
    try {
      await action();
      success.value = message;
    } catch (cause) {
      error.value = cause instanceof Error ? cause.message : String(cause);
    } finally {
      busy.value = false;
    }
  }
  return { busy, error, success, run };
}
