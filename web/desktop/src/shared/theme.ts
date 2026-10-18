import { nextTick, ref } from 'vue';

/** 主题在本地保存，访问存储失败时仍允许正常使用界面 */
function savedTheme(): 'light' | 'dark' {
  try {
    const saved = localStorage.getItem('packy-theme');
    if (saved === 'light' || saved === 'dark') return saved;
  } catch {
    /* 隐私模式下采用系统主题 */
  }
  return matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}
export const theme = ref(savedTheme());
export function applyTheme() {
  document.documentElement.dataset.theme = theme.value;
  if (theme.value === 'dark') document.body.setAttribute('arco-theme', 'dark');
  else document.body.removeAttribute('arco-theme');
}
let requestedTheme = theme.value;
let themeTransition: ViewTransition | undefined;
/** 主题偏好只影响表现，不改变更新任务 */
export function setTheme(value: 'light' | 'dark') {
  if (value === requestedTheme) return;
  requestedTheme = value;
  themeTransition?.skipTransition();
  const update = async () => {
    theme.value = requestedTheme;
    applyTheme();
    await nextTick();
  };
  if (document.startViewTransition && !matchMedia('(prefers-reduced-motion: reduce)').matches) {
    themeTransition = document.startViewTransition(update);
    // 快速切换会主动跳过旧动画，其 ready Promise 会拒绝
    void themeTransition.ready.catch(() => {});
  } else {
    void update();
  }
  try {
    localStorage.setItem('packy-theme', requestedTheme);
  } catch {
    /* 存储不可用时保持会话内主题 */
  }
}
export function toggleTheme() {
  setTheme(requestedTheme === 'dark' ? 'light' : 'dark');
}
applyTheme();
