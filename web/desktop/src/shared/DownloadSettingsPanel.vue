<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import type { ClientSnapshot, DownloadOverrides, DownloadSettings } from '../protocol';
const props = defineProps<{ snapshot: ClientSnapshot }>();
const overrides = ref<DownloadOverrides>({});
const dirty = ref(false),
  saving = ref(false),
  message = ref(''),
  failed = ref(false);
const fields = [
  {
    key: 'max_concurrent_downloads',
    label: '最大并发下载数',
    min: 1,
    max: 16,
    hint: '同时下载的文件数量，1–16',
  },
  {
    key: 'speed_limit_kib',
    label: '总下载限速（KiB/s）',
    min: 0,
    max: 1048576,
    hint: '0 表示不限速；1024 KiB/s = 1 MiB/s',
  },
  { key: 'check_timeout_seconds', label: '检查更新超时（秒）', min: 5, max: 300, hint: '5–300 秒' },
  {
    key: 'download_timeout_seconds',
    label: '单次下载超时（秒）',
    min: 30,
    max: 3600,
    hint: '30–3600 秒；低速下载时可适当增加',
  },
  {
    key: 'max_attempts',
    label: '最大尝试次数',
    min: 1,
    max: 10,
    hint: '包含首次下载，失败后按原计划重试，1–10 次',
  },
] as const;
const sources = {
  modrinth: 'Modrinth',
  curseforge: 'CurseForge',
  github: 'GitHub',
  server: '服务端',
};
const effective = computed<DownloadSettings>(() => ({
  ...props.snapshot.download_defaults,
  ...overrides.value,
  proxies: { ...props.snapshot.download_defaults.proxies, ...overrides.value.proxies },
}));
function reload() {
  overrides.value = structuredClone(props.snapshot.preferences.download || {});
  dirty.value = false;
}
watch(
  () => props.snapshot.preferences.download,
  () => {
    if (!dirty.value && !saving.value) reload();
  },
  { immediate: true },
);
function change(key: (typeof fields)[number]['key'], event: Event) {
  overrides.value[key] = Number((event.target as HTMLInputElement).value);
  dirty.value = true;
  message.value = '';
}
function proxy(source: string, event: Event) {
  overrides.value.proxies = {
    ...overrides.value.proxies,
    [source]: (event.target as HTMLInputElement).value.trim(),
  };
  dirty.value = true;
  message.value = '';
}
function reset(key?: (typeof fields)[number]['key'], source?: string) {
  if (key) delete overrides.value[key];
  else if (source) delete overrides.value.proxies?.[source];
  else overrides.value = {};
  dirty.value = true;
  message.value = '';
}
async function save() {
  failed.value = false;
  message.value = '';
  for (const f of fields) {
    const value = effective.value[f.key];
    if (!Number.isInteger(value) || value < f.min || value > f.max) {
      failed.value = true;
      message.value = `${f.label}超出允许范围`;
      return;
    }
  }
  for (const value of Object.values(effective.value.proxies)) {
    if (!value) continue;
    try {
      const u = new URL(value);
      if (
        !['http:', 'https:'].includes(u.protocol) ||
        !u.hostname ||
        u.username ||
        u.password ||
        u.search ||
        u.hash ||
        (u.pathname !== '/' && u.pathname !== '')
      )
        throw new Error();
    } catch {
      failed.value = true;
      message.value = '代理需为无账号密码的 HTTP/HTTPS 地址，留空直连';
      return;
    }
  }
  saving.value = true;
  try {
    await window.packy!.invoke(
      'set_download_settings',
      JSON.parse(JSON.stringify(overrides.value)),
    );
    dirty.value = false;
    message.value = '已保存，下次检查或下载（含失败重试）生效';
  } catch {
    failed.value = true;
    message.value = '保存失败，设置未生效，请重试';
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <form class="panel download-settings" @submit.prevent="save">
    <h2>下载设置</h2>
    <p class="secondary">默认值由整合包提供，你的调整仅保存在本机。运行中的任务不受影响。</p>
    <fieldset :disabled="saving">
      <div class="download-field-grid">
        <div v-for="field in fields" :key="field.key" class="download-field">
          <label :for="`download-${field.key}`">{{ field.label }}</label>
          <input
            :id="`download-${field.key}`"
            type="number"
            required
            step="1"
            :min="field.min"
            :max="field.max"
            :value="effective[field.key]"
            @input="change(field.key, $event)"
          />
          <small class="secondary">{{ field.hint }}</small>
          <div class="download-inheritance">
            <small class="secondary">
              {{ overrides[field.key] === undefined ? '使用默认' : '已自定义' }} · 默认
              {{ snapshot.download_defaults[field.key] }}
            </small>
            <button
              v-if="overrides[field.key] !== undefined"
              type="button"
              @click="reset(field.key)"
            >
              恢复默认
            </button>
          </div>
        </div>
      </div>
      <div class="setting-divider" />
      <h3>下载源代理</h3>
      <p class="secondary">
        按渠道同时作用于 HTTP/HTTPS，留空强制直连，不使用系统环境代理。服务端代理也用于检查更新。
      </p>
      <div class="download-field-grid">
        <div v-for="(label, source) in sources" :key="source" class="download-field">
          <label :for="`proxy-${source}`">{{ label }} 代理</label>
          <input
            :id="`proxy-${source}`"
            type="text"
            :value="effective.proxies[source] || ''"
            placeholder="留空直连，例如 http://127.0.0.1:7890"
            autocomplete="off"
            spellcheck="false"
            @input="proxy(source, $event)"
          />
          <div class="download-inheritance">
            <small class="secondary">
              {{ overrides.proxies?.[source] === undefined ? '使用默认' : '已自定义' }} ·
              {{ effective.proxies[source] ? '代理连接' : '直连' }}
            </small>
            <button
              v-if="overrides.proxies?.[source] !== undefined"
              type="button"
              @click="reset(undefined, source)"
            >
              恢复默认
            </button>
          </div>
        </div>
      </div>
      <p class="secondary">
        仅支持无账号密码的 HTTP/HTTPS 代理地址。限速为所有文件合计，允许短暂缓冲突发。
      </p>
      <div class="download-actions">
        <button class="primary" type="submit" :disabled="!dirty || saving">
          {{ saving ? '保存中…' : '保存下载设置' }}
        </button>
        <button
          type="button"
          :disabled="!dirty"
          @click="
            reload();
            message = '';
          "
        >
          取消修改
        </button>
        <button type="button" @click="reset()">全部恢复默认</button>
      </div>
    </fieldset>
    <p
      v-if="message"
      :role="failed ? 'alert' : 'status'"
      :class="failed ? 'download-error' : 'secondary'"
    >
      {{ message }}
    </p>
  </form>
</template>

<style scoped>
.download-settings {
  grid-column: 1 / -1;
  padding: 24px;
}
.download-settings > p,
.download-settings h3 + p {
  margin: 8px 0 20px;
}
@media (max-width: 760px) {
  .download-settings {
    padding: 16px;
  }
}
.download-settings fieldset {
  border: 0;
  padding: 0;
  margin: 20px 0 0;
  min-width: 0;
}
.download-field-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 22px 24px;
}
.download-field {
  display: flex;
  flex-direction: column;
  gap: 8px;
  min-width: 0;
}
.download-field input {
  width: 100%;
  min-width: 0;
  min-height: 40px;
  padding: 9px 12px;
  border: 1px solid var(--line);
  border-radius: 6px;
  color: var(--text);
  background: var(--surface);
  font: inherit;
  transition: border-color 140ms;
}
.download-field input:focus-visible {
  outline: 2px solid var(--focus);
  outline-offset: 2px;
}
.download-inheritance {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  min-height: 30px;
}
.download-inheritance button {
  padding: 3px 8px;
  min-height: 28px;
  font-size: 12px;
}
.download-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  margin-top: 20px;
}
.download-error {
  color: var(--danger);
}
@media (max-width: 760px) {
  .download-field-grid {
    grid-template-columns: 1fr;
  }
}
@media (prefers-reduced-motion: reduce) {
  .download-field input {
    transition: none;
  }
}
</style>
