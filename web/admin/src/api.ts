import { ref, shallowRef } from 'vue';
import type { Account } from '../../shared/generated/Account';
import type { LoginResponse } from '../../shared/generated/LoginResponse';
import type { Page } from '../../shared/generated/Page';

/** Token 仅驻留内存，刷新页面后重新登录 */
export const account = shallowRef<Account>();
const token = ref('');
export function logoutLocal() {
  token.value = '';
  account.value = undefined;
}
/** 所有接口共享错误处理，401 清理会话，服务端始终执行实际授权 */
export async function api<T>(path: string, method = 'GET', body?: unknown): Promise<T> {
  const headers: Record<string, string> = {};
  if (token.value) headers.Authorization = `Bearer ${token.value}`;
  if (body !== undefined && !(body instanceof FormData))
    headers['Content-Type'] = 'application/json';
  let response: Response;
  try {
    response = await fetch(`/api/v1${path}`, {
      method,
      headers,
      body: body === undefined ? undefined : body instanceof FormData ? body : JSON.stringify(body),
    });
  } catch {
    throw new Error('无法连接服务端，请检查网络或反向代理配置');
  }
  if (response.status === 204) return undefined as T;
  const data: unknown = await response.json().catch(() => null);
  if (!response.ok) {
    if (response.status === 401) logoutLocal();
    const error = data as { message?: string; request_id?: string } | null;
    throw new Error(
      `${error?.message || `请求失败（HTTP ${response.status}）`}${error?.request_id ? ` · ${error.request_id}` : ''}`,
    );
  }
  return data as T;
}
export async function login(username: string, password: string) {
  const result = await api<LoginResponse>('/auth/login', 'POST', {
    username,
    password,
  });
  token.value = result.access_token;
  account.value = await api<Account>('/auth/me');
}
export async function logout() {
  try {
    await api('/auth/logout', 'POST', {});
  } finally {
    logoutLocal();
  }
}
/** 只用于小型引用选择列表，服务端每页仍限制 100 条 */
export async function all<T>(path: string): Promise<T[]> {
  const items: T[] = [];
  for (let page = 1; ; page++) {
    const result = await api<Page<T>>(
      `${path}${path.includes('?') ? '&' : '?'}page=${page}&page_size=100`,
    );
    items.push(...result.items);
    if (items.length >= result.total || result.items.length === 0) return items;
  }
}
export function can(permission: string, system = false) {
  return (
    !!account.value?.permissions.includes(permission) &&
    (!system || account.value.roles.includes('super_admin'))
  );
}
