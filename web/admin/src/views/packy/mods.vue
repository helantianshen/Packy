<template>
  <div class="container"
    ><Breadcrumb :items="['Packy', 'Mod 与文件']" /><a-card
      class="general-card"
      title="Mod 与文件"
      ><a-space wrap
        ><a-input-search
          v-model="keyword"
          placeholder="搜索名称或 modid"
        /><a-button type="primary" @click="editMod()">新建 Mod</a-button
        ><a-button @click="reload">刷新</a-button></a-space
      ><a-table row-key="id" :data="filtered" :scroll="{ x: 700 }"
        ><template #columns
          ><a-table-column title="名称" data-index="name" /><a-table-column
            title="modid"
            data-index="modid"
          /><a-table-column
            title="Modrinth 项目"
            data-index="modrinth_project_id"
          /><a-table-column title="操作"
            ><template #cell="{ record }"
              ><a-space
                ><a-button type="text" @click="editMod(record)">编辑</a-button
                ><a-button type="text" @click="openFiles(record)"
                  >文件版本</a-button
                ></a-space
              ></template
            ></a-table-column
          ></template
        ></a-table
      ></a-card
    >
    <a-modal
      v-model:visible="modVisible"
      title="Mod 信息"
      :on-before-ok="saveMod"
      ><a-form :model="form" layout="vertical"
        ><a-form-item label="名称" required
          ><a-input v-model="form.name" /></a-form-item
        ><a-form-item label="modid" required
          ><a-input v-model="form.modid" /></a-form-item
        ><a-form-item label="Modrinth 项目 ID"
          ><a-input v-model="form.modrinth_project_id" /></a-form-item
        ><a-form-item label="CurseForge 项目 ID"
          ><a-input
            v-model="form.curseforge_project_id" /></a-form-item></a-form
    ></a-modal>
    <a-drawer
      v-model:visible="filesVisible"
      width="min(100vw, 1000px)"
      :footer="false"
      :title="`${current?.name} · 文件版本`"
      ><a-alert
        >文件版本不可变；修改文件或来源时创建新版本，再在发布草稿中选择</a-alert
      ><a-table :data="files" row-key="id"
        ><template #columns
          ><a-table-column title="版本" data-index="version" /><a-table-column
            title="文件"
            data-index="filename" /><a-table-column
            title="大小"
            data-index="size" /></template
      ></a-table>
      <a-divider>新增文件版本</a-divider
      ><a-form :model="artifact" layout="vertical"
        ><a-form-item label="上传托管 JAR（可选）"
          ><input
            type="file"
            accept=".jar"
            :disabled="saving"
            @change="upload" /></a-form-item
        ><a-form-item label="版本" required
          ><a-input v-model="artifact.version" /></a-form-item
        ><a-form-item label="文件名" required
          ><a-input
            v-model="artifact.filename"
            placeholder="example.jar" /></a-form-item
        ><a-form-item label="大小（字节）"
          ><a-input-number v-model="artifact.size" :min="0" /></a-form-item
        ><a-form-item label="SHA-512" required
          ><a-input v-model="artifact.sha512" /></a-form-item
        ><a-form-item label="下载来源"
          ><a-space direction="vertical" fill>
            <div
              v-for="(entry, idx) in artifact.sources"
              :key="idx"
              class="source-row"
            >
              <a-select
                v-model="entry.type"
                :style="{ width: '130px' }"
                :options="['github', 'modrinth', 'curseforge', 'server']"
              ></a-select>
              <a-input
                v-model="entry.url"
                placeholder="https://…"
                :style="{ width: '340px', maxWidth: '100%' }"
              ></a-input>
              <a-form-item
                :field="`sources.${idx}.priority`"
                label="优先级（数字越小越优先）"
                class="source-priority"
                ><a-input-number
                  v-model="entry.priority"
                  :min="0"
                  :style="{ width: '180px' }"
              /></a-form-item>
              <a-button @click="artifact.sources.splice(idx, 1)"
                >移除</a-button
              > </div
            ><a-button
              @click="
                artifact.sources.push({
                  type: 'github',
                  url: '',
                  priority: artifact.sources.length,
                })
              "
              >添加来源</a-button
            ></a-space
          ></a-form-item
        ><a-button type="primary" :loading="saving" @click="saveArtifact"
          >保存文件版本</a-button
        ></a-form
      ></a-drawer
    ></div
  >
</template>
<script setup lang="ts">
  import { ref, computed, onMounted } from 'vue';
  import { Message } from '@arco-design/web-vue';
  import { api, all, type Mod, type Artifact, type Source } from '@/api/packy';
  const mods = ref<Mod[]>([]),
    files = ref<Artifact[]>([]),
    keyword = ref(''),
    current = ref<Mod>(),
    editing = ref(''),
    modVisible = ref(false),
    filesVisible = ref(false),
    saving = ref(false);
  const form = ref({
    name: '',
    modid: '',
    modrinth_project_id: '',
    curseforge_project_id: '',
  });
  const emptyArtifact = () => ({
    version: '',
    filename: '',
    size: undefined as number | undefined,
    sha512: '',
    hosted_file_id: null as string | null,
    sources: [{ type: 'github', url: '', priority: 0 }] as Source[],
  });
  const artifact = ref(emptyArtifact());
  const filtered = computed(() =>
    mods.value.filter((m) => (m.name + ' ' + m.modid).includes(keyword.value)),
  );
  async function reload() {
    mods.value = await all<Mod>('/mods');
  }
  function editMod(m?: Mod) {
    editing.value = m?.id || '';
    form.value = {
      name: m?.name || '',
      modid: m?.modid || '',
      modrinth_project_id: m?.modrinth_project_id || '',
      curseforge_project_id: m?.curseforge_project_id || '',
    };
    modVisible.value = true;
  }
  async function saveMod() {
    try {
      await api.request({
        method: editing.value ? 'patch' : 'post',
        url: '/mods' + (editing.value ? `/${editing.value}` : ''),
        data: {
          ...form.value,
          modrinth_project_id: form.value.modrinth_project_id || null,
          curseforge_project_id: form.value.curseforge_project_id || null,
        },
      });
      await reload();
      return true;
    } catch {
      return false;
    }
  }
  async function openFiles(m: Mod) {
    current.value = m;
    files.value = (await all<Artifact>('/artifacts')).filter(
      (a) => a.mod_id === m.id,
    );
    artifact.value = emptyArtifact();
    filesVisible.value = true;
  }
  async function upload(e: Event) {
    const file = (e.target as HTMLInputElement).files?.[0];
    if (!file) return;
    saving.value = true;
    try {
      const body = new FormData();
      body.append('file', file);
      const { data } = await api.post('/files', body, { timeout: 0 });
      artifact.value = {
        ...artifact.value,
        filename: data.filename,
        size: data.size,
        sha512: data.sha512,
        hosted_file_id: data.id,
        sources: [{ type: 'server', url: data.url, priority: 0 }],
      };
    } finally {
      saving.value = false;
    }
  }
  async function saveArtifact() {
    saving.value = true;
    try {
      await api.post('/artifacts', {
        ...artifact.value,
        size: artifact.value.size ?? null,
        mod_id: current.value!.id,
      });
      Message.success('文件版本已保存');
      await openFiles(current.value!);
    } finally {
      saving.value = false;
    }
  }
  onMounted(reload);
</script>
<style scoped lang="less">
  .source-row {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: 8px;
  }
  .source-priority {
    width: 210px;
    margin-bottom: 0;
  }
  .container {
    padding: 0 20px 20px;
  }
  :deep(.arco-table) {
    margin-top: 16px;
  }
</style>
