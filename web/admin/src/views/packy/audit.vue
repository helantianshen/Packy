<template>
  <div class="container"
    ><Breadcrumb :items="['Packy', '审计日志']" /><a-card
      class="general-card"
      title="审计日志"
      ><a-space class="table-toolbar" wrap
        ><a-input
          v-model="filters.action"
          placeholder="操作，例如 release.publish"
        /><a-input v-model="filters.pack_id" placeholder="整合包 ID" /><a-select
          v-model="filters.result"
          allow-clear
          placeholder="结果"
          :style="{ width: '140px' }"
          :options="['success', 'failed']"
        /><a-button
          type="primary"
          @click="
            page = 1;
            load();
          "
          >查询</a-button
        ></a-space
      ><a-table
        row-key="id"
        :data="logs"
        :loading="loading"
        :scroll="{ x: 1000 }"
        :pagination="{ current: page, total, pageSize: 20 }"
        @page-change="
          (p) => {
            page = p;
            load();
          }
        "
        ><template #columns
          ><a-table-column title="时间" :width="190"
            ><template #cell="{ record }">{{
              new Date(record.occurred_at).toLocaleString()
            }}</template></a-table-column
          ><a-table-column
            title="管理员"
            data-index="actor_name"
          /><a-table-column title="操作" data-index="action" /><a-table-column
            title="结果"
            data-index="result"
          /><a-table-column
            title="资源"
            data-index="resource_id"
          /><a-table-column title="详情"
            ><template #cell="{ record }"
              ><a-button type="text" @click="detail = record"
                >查看变更</a-button
              ></template
            ></a-table-column
          ></template
        ></a-table
      ></a-card
    ><a-drawer
      :visible="!!detail"
      width="min(100vw, 800px)"
      title="审计详情"
      :footer="false"
      @cancel="detail = undefined"
      ><a-descriptions v-if="detail" :column="1" bordered
        ><a-descriptions-item label="请求 ID">{{
          detail.request_id
        }}</a-descriptions-item
        ><a-descriptions-item label="资源 ID">{{
          detail.resource_id
        }}</a-descriptions-item></a-descriptions
      ><a-table
        v-if="detail"
        :data="
          Object.entries(detail.changes || {}).map(([field, change]) => ({
            field,
            change,
          }))
        "
        :pagination="false"
        ><template #columns
          ><a-table-column title="字段" data-index="field" /><a-table-column
            title="前后变化"
            ><template #cell="{ record }">
              <pre>{{ JSON.stringify(record.change, null, 2) }}</pre>
            </template></a-table-column
          ></template
        ></a-table
      ></a-drawer
    ></div
  >
</template>
<script setup lang="ts">
  import { ref, reactive, onMounted } from 'vue';
  import { api, type Audit } from '@/api/packy';
  const logs = ref<Audit[]>([]),
    page = ref(1),
    total = ref(0),
    loading = ref(false),
    detail = ref<Audit>();
  const filters = reactive({ action: '', pack_id: '', result: '' });
  async function load() {
    loading.value = true;
    try {
      const { data } = await api.get('/audit-logs', {
        params: { ...filters, page: page.value, page_size: 20 },
      });
      logs.value = data.items;
      total.value = data.total;
    } finally {
      loading.value = false;
    }
  }
  onMounted(load);
</script>
<style scoped>
  .table-toolbar {
    margin-bottom: 16px;
  }
  .container {
    padding: 0 20px 20px;
  }
  pre {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
</style>
