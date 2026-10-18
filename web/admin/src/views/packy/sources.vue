<template>
  <div class="container"
    ><Breadcrumb :items="['Packy', '下载来源']" /><a-card
      class="general-card"
      title="下载来源"
      ><a-alert class="table-toolbar"
        >禁用来源会影响后续生成的更新计划，已有客户端任务保留原计划</a-alert
      ><a-table :data="sources" row-key="code" :pagination="false"
        ><template #columns
          ><a-table-column title="代码" data-index="code" /><a-table-column
            title="名称"
            ><template #cell="{ record }"
              ><a-input v-model="record.name" /></template></a-table-column
          ><a-table-column title="启用"
            ><template #cell="{ record }"
              ><a-switch v-model="record.enabled" /></template></a-table-column
          ><a-table-column title="排序"
            ><template #cell="{ record }"
              ><a-input-number
                v-model="record.sort_order"
                :min="0" /></template></a-table-column
          ><a-table-column title="操作"
            ><template #cell="{ record }"
              ><a-button type="text" @click="save(record)"
                >保存</a-button
              ></template
            ></a-table-column
          ></template
        ></a-table
      ></a-card
    ></div
  >
</template>
<script setup lang="ts">
  import { ref, onMounted } from 'vue';
  import { Message } from '@arco-design/web-vue';
  import { api, type SourceType } from '@/api/packy';
  const sources = ref<SourceType[]>([]);
  async function load() {
    sources.value = (await api.get('/system/download-source-types')).data;
  }
  async function save(s: SourceType) {
    await api.put(`/system/download-source-types/${s.code}`, {
      name: s.name,
      enabled: s.enabled,
      sort_order: s.sort_order,
    });
    Message.success('已保存');
    await load();
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
</style>
