import axios from 'axios';
export interface Source {
  type: string;
  url: string;
  priority: number;
}
export interface Pack {
  id: string;
  name: string;
  minecraft_version: string;
  loader: string;
  current_release_id: string | null;
}
export interface Mod {
  id: string;
  modid: string;
  name: string;
  modrinth_project_id: string | null;
  curseforge_project_id: string | null;
}
export interface Artifact {
  id: string;
  mod_id: string;
  version: string;
  filename: string;
  size: number | null;
  sha512: string;
  sources: Source[];
  hosted_file_id: string | null;
}
export interface Item {
  mod_id: string;
  artifact_id: string;
  modid_snapshot: string;
  name_snapshot: string;
  target: {
    path: string;
    sha512: string;
    size: number | null;
    sources: Source[];
  };
}
export interface Release {
  id: string;
  pack_id: string;
  version: string;
  changelog: string;
  revision: number;
  status: string;
  items: Item[];
  published_at: string | null;
}
export interface SourceType {
  code: string;
  name: string;
  enabled: boolean;
  sort_order: number;
}
export interface Audit {
  id: string;
  occurred_at: string;
  actor_name: string;
  action: string;
  result: string;
  resource_id: string;
  request_id: string;
  changes: Record<string, unknown>;
}
export interface Page<T> {
  items: T[];
  total: number;
  page: number;
  page_size: number;
}
export const api = axios.create({ baseURL: '/api/v1/admin' });
// 业务实例共享认证与错误处理拦截器
import { getToken, clearToken } from '@/utils/auth';
import { Message } from '@arco-design/web-vue';
api.interceptors.request.use((c) => {
  const token = getToken();
  if (token) c.headers.Authorization = `Bearer ${token}`;
  return c;
});
api.interceptors.response.use(
  (r) => r,
  (e) => {
    const message = e.response?.data?.message || '请求失败';
    Message.error(message);
    if (e.response?.status === 401) {
      clearToken();
      location.assign(`${import.meta.env.BASE_URL}login`);
    }
    return Promise.reject(new Error(message));
  },
);
export async function all<T>(path: string): Promise<T[]> {
  const out: T[] = [];
  for (let page = 1; ; page++) {
    const { data } = await api.get<Page<T>>(path, {
      params: { page, page_size: 100 },
    });
    out.push(...data.items);
    if (out.length >= data.total) return out;
  }
}
