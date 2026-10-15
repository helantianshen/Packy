<script setup lang="ts">
import LoadingFeedback from '../../../shared/LoadingFeedback.vue';
import { onMounted, ref } from 'vue';
import { api, can } from '../api';
import { useAction } from '../useAction';
import type { SourceDictionaryItem } from '../../../shared/generated/SourceDictionaryItem';
const { busy, error, success, run } = useAction();
const sources = ref<SourceDictionaryItem[]>([]);
async function load() {
  sources.value = await api('/admin/system/download-source-types');
}
onMounted(() => run(load));
async function save(source: SourceDictionaryItem) {
  await api(`/admin/system/download-source-types/${source.code}`, 'PUT', {
    name: source.name,
    enabled: source.enabled,
    sort_order: source.sort_order,
  });
  await load();
}
</script>
<template>
  <div class="view-heading">
    <h1>下载源</h1>
    <p class="muted">四类来源共用文件校验。字典调整影响后续计划，已取得的任务继续使用原记录。</p>
  </div>
  <LoadingFeedback :busy="busy" />
  <div v-if="error" class="error" role="alert">{{ error }}</div>
  <div v-if="success" class="success">{{ success }}</div>
  <section class="card section">
    <div class="table-scroll">
      <table>
        <thead>
          <tr>
            <th>类型</th>
            <th>显示名称</th>
            <th>默认排序</th>
            <th>状态</th>
            <th>操作</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="source in sources" :key="source.code">
            <td>
              <code>{{ source.code }}</code>
            </td>
            <td>
              <input v-model="source.name" aria-label="来源名称" maxlength="80" />
            </td>
            <td>
              <input
                type="number"
                min="0"
                v-model.number="source.sort_order"
                aria-label="来源排序"
              />
            </td>
            <td>
              <label class="inline">
                <input type="checkbox" v-model="source.enabled" />
                启用
              </label>
            </td>
            <td>
              <button
                v-if="can('source:write')"
                :disabled="busy"
                @click="run(() => save(source), '来源已保存')"
              >
                保存
              </button>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
</template>
