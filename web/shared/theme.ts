import { ref } from 'vue';

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
/** 桌面和后台共用颜色模式，切换不影响正在进行的业务任务 */
export function toggleTheme() {
  theme.value = theme.value === 'dark' ? 'light' : 'dark';
  applyTheme();
  try {
    localStorage.setItem('packy-theme', theme.value);
  } catch {
    /* 主题仍在当前会话生效 */
  }
}
applyTheme();
