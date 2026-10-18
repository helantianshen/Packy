<template>
  <div class="container"
    ><Breadcrumb :items="['Packy', 'Modrinth 导入']" /><a-card
      class="general-card"
      title="Modrinth 导入"
      ><a-space class="table-toolbar" wrap
        ><a-input-search
          v-model="query"
          placeholder="搜索 Modrinth 项目"
          @search="search"
        /><a-button :loading="loading" @click="search">搜索</a-button></a-space
      ><a-table :data="projects" row-key="project_id"
        ><template #columns
          ><a-table-column title="项目" data-index="title" /><a-table-column
            title="说明"
            data-index="description"
          /><a-table-column title="操作"
            ><template #cell="{ record }"
              ><a-button type="text" @click="versionsFor(record)"
                >选择版本</a-button
              ></template
            ></a-table-column
          ></template
        ></a-table
      ></a-card
    ><a-drawer
      v-model:visible="visible"
      width="min(100vw, 900px)"
      title="选择文件导入"
      :footer="false"
      ><a-alert
        >导入到全局 Mod 文件库；完成后在整合包发布草稿中选择此文件</a-alert
      ><a-form :model="form" layout="vertical"
        ><a-form-item label="modid（游戏内标识，请核实）"
          ><a-input v-model="form.modid" /></a-form-item
        ><a-form-item label="名称"><a-input v-model="form.name" /></a-form-item
        ><a-form-item label="版本"
          ><a-select v-model="versionID" allow-search
            ><a-option v-for="v in versions" :key="v.id" :value="v.id"
              >{{ v.version_number }} · {{ v.game_versions.join(', ') }} ·
              {{ v.loaders.join(', ') }}</a-option
            ></a-select
          ></a-form-item
        ></a-form
      ><a-table :data="version?.files || []" :pagination="false"
        ><template #columns
          ><a-table-column title="文件" data-index="filename" /><a-table-column
            title="大小"
            data-index="size"
          /><a-table-column title="操作"
            ><template #cell="{ record }"
              ><a-button
                type="primary"
                :loading="loading"
                @click="importFile(record)"
                >导入</a-button
              ></template
            ></a-table-column
          ></template
        ></a-table
      ></a-drawer
    ></div
  >
</template>
<script setup lang="ts">
  import { ref, computed } from 'vue';
  import { Message } from '@arco-design/web-vue';
  import { api, all, type Mod } from '@/api/packy';
  const query = ref(''),
    projects = ref<any[]>([]),
    versions = ref<any[]>([]),
    versionID = ref(''),
    project = ref<any>(),
    visible = ref(false),
    loading = ref(false),
    form = ref({ modid: '', name: '' });
  const version = computed(() =>
    versions.value.find((v) => v.id === versionID.value),
  );
  async function search() {
    loading.value = true;
    try {
      projects.value = (
        await api.get('/modrinth/search', {
          params: {
            query: query.value,
            limit: 50,
            facets: JSON.stringify([['project_type:mod']]),
          },
        })
      ).data.hits;
    } finally {
      loading.value = false;
    }
  }
  let selection = 0;
  async function versionsFor(p: any) {
    if (loading.value) return;
    const request = ++selection;
    visible.value = false;
    const available = (
      await api.get(`/modrinth/projects/${p.project_id}/versions`)
    ).data;
    if (request !== selection) return;
    project.value = p;
    form.value = { modid: p.slug.replaceAll('-', '_'), name: p.title };
    versions.value = available;
    versionID.value = versions.value[0]?.id || '';
    visible.value = true;
  }
  async function importFile(f: any) {
    if (loading.value || !version.value?.files.includes(f)) return;
    const selectedProject = project.value.project_id;
    const selectedForm = { ...form.value };
    const selectedVersion = version.value.version_number;
    const file = JSON.parse(JSON.stringify(f));
    loading.value = true;
    try {
      const mods = await all<Mod>('/mods');
      let mod = mods.find((m) => m.modrinth_project_id === selectedProject);
      if (!mod)
        mod = (
          await api.post<Mod>('/mods', {
            ...selectedForm,
            modrinth_project_id: selectedProject,
            curseforge_project_id: null,
          })
        ).data;
      await api.post('/artifacts', {
        mod_id: mod.id,
        version: selectedVersion,
        filename: file.filename,
        size: file.size,
        sha512: file.hashes.sha512,
        hosted_file_id: null,
        sources: [{ type: 'modrinth', url: file.url, priority: 0 }],
      });
      Message.success('导入成功');
      visible.value = false;
    } finally {
      loading.value = false;
    }
  }
</script>
<style scoped>
  .table-toolbar {
    margin-bottom: 16px;
  }
  .container {
    padding: 0 20px 20px;
  }
</style>
