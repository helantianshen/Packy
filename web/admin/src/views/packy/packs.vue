<template>
  <div class="container"
    ><Breadcrumb :items="['Packy', '整合包与发布']" /><a-card
      class="general-card"
      title="整合包与发布"
    >
      <a-space wrap
        ><a-select
          v-model="selected"
          placeholder="选择整合包"
          :style="{ width: '260px' }"
          @change="loadReleases"
          ><a-option v-for="p in packs" :key="p.id" :value="p.id">{{
            p.name
          }}</a-option></a-select
        ><a-button type="primary" @click="editPack()">新建整合包</a-button
        ><a-button
          :disabled="!selected"
          @click="editPack(packs.find((p) => p.id === selected))"
          >编辑整合包</a-button
        ><a-button :disabled="!selected" @click="newDraft">创建草稿</a-button
        ><a-button @click="reload">刷新</a-button></a-space
      >
      <a-divider /><a-alert v-if="!packs.length"
        >尚无整合包，请先创建整合包，再添加 Mod 和发布快照</a-alert
      >
      <a-table
        row-key="id"
        :data="releases"
        :loading="loading"
        :scroll="{ x: 800 }"
        ><template #columns
          ><a-table-column title="版本" data-index="version" /><a-table-column
            title="状态"
            ><template #cell="{ record }"
              ><a-tag
                :color="record.status === 'published' ? 'green' : 'orange'"
                >{{ record.status === 'published' ? '已发布' : '草稿' }}</a-tag
              ></template
            ></a-table-column
          ><a-table-column
            title="发布日期"
            data-index="published_at"
          /><a-table-column title="发布 ID" data-index="id" /><a-table-column
            title="操作"
            ><template #cell="{ record }"
              ><a-button type="text" @click="openRelease(record)">{{
                record.status === 'draft' ? '编辑与发布' : '查看快照'
              }}</a-button></template
            ></a-table-column
          ></template
        ></a-table
      >
    </a-card>
    <a-modal
      v-model:visible="packVisible"
      title="整合包信息"
      :on-before-ok="savePack"
      ><a-form :model="packForm" layout="vertical"
        ><a-form-item label="名称" required
          ><a-input v-model="packForm.name" /></a-form-item
        ><a-form-item label="Minecraft 版本" required
          ><a-input
            v-model="packForm.minecraft_version"
            placeholder="1.21.1" /></a-form-item
        ><a-form-item label="加载器"
          ><a-select
            v-model="packForm.loader"
            :options="['neoforge', 'forge', 'fabric']" /></a-form-item></a-form
    ></a-modal>
    <a-drawer
      v-model:visible="draftVisible"
      :width="'min(100vw, 1000px)'"
      :footer="false"
      :title="draft?.status === 'draft' ? '编辑发布草稿' : '发布快照'"
    >
      <template v-if="draft"
        ><a-form
          :model="draft"
          layout="vertical"
          :disabled="draft.status !== 'draft'"
          ><a-form-item label="版本" required
            ><a-input v-model="draft.version" /></a-form-item
          ><a-form-item label="更新说明"
            ><a-textarea v-model="draft.changelog" /></a-form-item
        ></a-form>
        <a-space v-if="draft.status === 'draft'" wrap
          ><a-select
            v-model="addArtifact"
            allow-search
            placeholder="添加文件"
            :style="{ width: '360px' }"
            ><a-option v-for="a in artifacts" :key="a.id" :value="a.id"
              >{{ modName(a.mod_id) }} · {{ a.version }} ·
              {{ a.filename }}</a-option
            ></a-select
          ><a-button @click="addItem">加入快照</a-button></a-space
        >
        <a-table
          :data="items"
          row-key="mod_id"
          :pagination="false"
          :scroll="{ x: 650 }"
          ><template #columns
            ><a-table-column title="Mod"
              ><template #cell="{ record }">{{
                record.name_snapshot || modName(record.mod_id)
              }}</template></a-table-column
            ><a-table-column title="文件"
              ><template #cell="{ record }">{{
                artifacts.find((a) => a.id === record.artifact_id)?.filename ||
                record.artifact_id
              }}</template></a-table-column
            ><a-table-column title="安装路径"
              ><template #cell="{ record }"
                ><a-input
                  v-model="record.target_path"
                  :disabled="
                    draft.status !== 'draft'
                  " /></template></a-table-column
            ><a-table-column v-if="draft.status === 'draft'" title="操作"
              ><template #cell="{ record }"
                ><a-button
                  type="text"
                  status="danger"
                  @click="items = items.filter((i) => i !== record)"
                  >移除</a-button
                ></template
              ></a-table-column
            ></template
          ></a-table
        >
        <a-divider /><a-space v-if="draft.status === 'draft'"
          ><a-button type="primary" :loading="saving" @click="saveDraft"
            >保存草稿</a-button
          ><a-button :loading="saving" @click="previewDraft"
            >保存并预览差异</a-button
          ><a-popconfirm content="发布后快照不可修改，确认发布？" @ok="publish"
            ><a-button status="success" :loading="saving"
              >保存并发布</a-button
            ></a-popconfirm
          ></a-space
        >
        <a-table v-if="preview" :data="preview.operations" :pagination="false"
          ><template #columns
            ><a-table-column title="动作" data-index="action" /><a-table-column
              title="Mod"
              data-index="mod_id"
            /><a-table-column
              title="旧路径"
              data-index="old_path"
            /><a-table-column title="目标路径"
              ><template #cell="{ record }">{{
                record.target?.path
              }}</template></a-table-column
            ></template
          ></a-table
        >
      </template></a-drawer
    ></div
  >
</template>
<script setup lang="ts">
  import { ref, onMounted } from 'vue';
  import { Message } from '@arco-design/web-vue';
  import {
    api,
    all,
    type Pack,
    type Mod,
    type Artifact,
    type Release,
  } from '@/api/packy';
  const packs = ref<Pack[]>([]),
    mods = ref<Mod[]>([]),
    artifacts = ref<Artifact[]>([]),
    releases = ref<Release[]>([]),
    selected = ref(''),
    loading = ref(false),
    saving = ref(false),
    packVisible = ref(false),
    draftVisible = ref(false),
    editing = ref(''),
    addArtifact = ref('');
  const packForm = ref({
      name: '',
      minecraft_version: '1.21.1',
      loader: 'neoforge',
    }),
    draft = ref<Release>(),
    items = ref<
      {
        mod_id: string;
        artifact_id: string;
        name_snapshot?: string;
        target_path: string;
      }[]
    >([]),
    preview = ref<any>();
  const modName = (id: string) =>
    mods.value.find((m) => m.id === id)?.name || id;
  async function reload() {
    loading.value = true;
    try {
      [packs.value, mods.value, artifacts.value] = await Promise.all([
        all<Pack>('/packs'),
        all<Mod>('/mods'),
        all<Artifact>('/artifacts'),
      ]);
      if (!selected.value) selected.value = packs.value[0]?.id || '';
      await loadReleases();
    } finally {
      loading.value = false;
    }
  }
  async function loadReleases() {
    releases.value = selected.value
      ? await all<Release>(`/packs/${selected.value}/releases`)
      : [];
  }
  function editPack(p?: Pack) {
    editing.value = p?.id || '';
    packForm.value = {
      name: p?.name || '',
      minecraft_version: p?.minecraft_version || '1.21.1',
      loader: p?.loader || 'neoforge',
    };
    packVisible.value = true;
  }
  async function savePack() {
    try {
      const { data } = await api.request<Pack>({
        method: editing.value ? 'put' : 'post',
        url: '/packs' + (editing.value ? `/${editing.value}` : ''),
        data: packForm.value,
      });
      selected.value = data.id;
      await reload();
      return true;
    } catch {
      return false;
    }
  }
  async function newDraft() {
    const { data } = await api.post<Release>(
      `/packs/${selected.value}/drafts`,
      { version: `draft-${Date.now()}`, changelog: '' },
    );
    openRelease(data);
    await loadReleases();
  }
  async function openRelease(r: Release) {
    draft.value = JSON.parse(JSON.stringify(r));
    items.value = r.items.map((i) => ({
      mod_id: i.mod_id,
      artifact_id: i.artifact_id,
      target_path: i.target.path,
      name_snapshot: i.name_snapshot,
    }));
    preview.value = undefined;
    draftVisible.value = true;
    if (r.status === 'published')
      preview.value = (
        await api.get(`/packs/${r.pack_id}/releases/${r.id}/diff`)
      ).data;
  }
  function addItem() {
    const a = artifacts.value.find((f) => f.id === addArtifact.value);
    if (!a) return;
    if (items.value.some((i) => i.mod_id === a.mod_id)) {
      Message.warning('同一 Mod 只能出现一次，请先移除旧行');
      return;
    }
    items.value.push({
      mod_id: a.mod_id,
      artifact_id: a.id,
      target_path: `mods/${a.filename}`,
    });
  }
  async function saveDraft() {
    if (!draft.value) return;
    saving.value = true;
    try {
      const { data } = await api.put<Release>(
        `/packs/${selected.value}/drafts/${draft.value.id}`,
        {
          revision: draft.value.revision,
          version: draft.value.version,
          changelog: draft.value.changelog,
          items: items.value.map(({ mod_id, artifact_id, target_path }) => ({
            mod_id,
            artifact_id,
            target_path,
          })),
        },
      );
      draft.value = data;
      items.value = data.items.map((item) => ({
        mod_id: item.mod_id,
        artifact_id: item.artifact_id,
        name_snapshot: item.name_snapshot,
        target_path: item.target.path,
      }));
      preview.value = undefined;
      await loadReleases();
      Message.success('草稿已保存');
    } finally {
      saving.value = false;
    }
  }
  async function previewDraft() {
    await saveDraft();
    preview.value = (
      await api.post(
        `/packs/${selected.value}/drafts/${draft.value!.id}/preview`,
        { revision: draft.value!.revision },
      )
    ).data;
  }
  async function publish() {
    await saveDraft();
    draft.value = (
      await api.post(
        `/packs/${selected.value}/drafts/${draft.value!.id}/publish`,
        { revision: draft.value!.revision },
      )
    ).data;
    Message.success('发布成功');
    await reload();
  }
  onMounted(reload);
</script>
<style scoped lang="less">
  .container {
    padding: 0 20px 20px;
  }
  :deep(.arco-table) {
    margin-top: 16px;
  }
</style>
