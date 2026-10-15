<script setup lang="ts">
import LoadingFeedback from '../../../shared/LoadingFeedback.vue';
import { computed, onMounted, ref, watch } from 'vue';
import { Modal } from '@arco-design/web-vue';
import { all, api, can } from '../api';
import { useAction } from '../useAction';
import type { Pack } from '../../../shared/generated/Pack';
import type { PackInput } from '../../../shared/generated/PackInput';
import type { Release } from '../../../shared/generated/Release';
import type { DraftItemInput } from '../../../shared/generated/DraftItemInput';
import type { UpdatePlan } from '../../../shared/generated/UpdatePlan';
import type { ModRecord } from '../../../shared/generated/ModRecord';
import type { Artifact } from '../../../shared/generated/Artifact';
const { busy, error, success, run } = useAction();
const packs = ref<Pack[]>([]),
  selected = ref<Pack>(),
  releases = ref<Release[]>([]),
  release = ref<Release>(),
  baseline = ref<Release>();
const showPackForm = ref(false),
  packId = ref<string>(),
  packForm = ref<PackInput>({
    name: '',
    minecraft_version: '1.21.1',
    loader: 'neoforge',
  });
const showDraftForm = ref(false),
  version = ref(''),
  changelog = ref(''),
  base = ref('');
const items = ref<DraftItemInput[]>([]),
  mods = ref<ModRecord[]>([]),
  modId = ref(''),
  versions = ref<Artifact[]>([]),
  artifactId = ref(''),
  targetPath = ref(''),
  preview = ref<UpdatePlan>();
watch(
  [version, changelog, items],
  () => {
    preview.value = undefined;
  },
  { deep: true },
);
async function load() {
  packs.value = await all<Pack>('/admin/packs');
}
onMounted(() => run(load));
function editPack(pack?: Pack) {
  packId.value = pack?.id;
  packForm.value = pack
    ? {
        name: pack.name,
        minecraft_version: pack.minecraft_version,
        loader: pack.loader,
      }
    : { name: '', minecraft_version: '1.21.1', loader: 'neoforge' };
  showPackForm.value = true;
}
async function savePack() {
  await api(
    `/admin/packs${packId.value ? `/${packId.value}` : ''}`,
    packId.value ? 'PUT' : 'POST',
    packForm.value,
  );
  showPackForm.value = false;
  await load();
}
async function select(pack: Pack) {
  selected.value = pack;
  releases.value = await all<Release>(`/admin/packs/${pack.id}/releases`);
  release.value = undefined;
  preview.value = undefined;
  showDraftForm.value = false;
}
async function openRelease(row: Release) {
  if (!selected.value) return;
  release.value = await api<Release>(`/admin/packs/${selected.value.id}/releases/${row.id}`);
  version.value = release.value.version;
  changelog.value = release.value.changelog;
  items.value = release.value.items.map((item) => ({
    mod_id: item.mod_id,
    artifact_id: item.artifact_id,
    target_path: item.target.path,
  }));
  preview.value = undefined;
  showDraftForm.value = false;
  baseline.value = release.value.base_release_id
    ? await api<Release>(
        `/admin/packs/${selected.value.id}/releases/${release.value.base_release_id}`,
      )
    : undefined;
  if (can('mod:read')) mods.value = await all<ModRecord>('/admin/mods');
}
async function createDraft() {
  if (!selected.value) return;
  const row = await api<Release>(`/admin/packs/${selected.value.id}/drafts`, 'POST', {
    version: version.value,
    changelog: changelog.value,
    base_release_id: base.value || null,
  });
  await select(selected.value);
  await openRelease(row);
}
async function saveDraft() {
  if (!selected.value || !release.value) return;
  const row = await api<Release>(
    `/admin/packs/${selected.value.id}/drafts/${release.value.id}`,
    'PUT',
    {
      revision: release.value.revision,
      version: version.value,
      changelog: changelog.value,
      items: items.value,
    },
  );
  release.value = row;
  preview.value = undefined;
  releases.value = await all<Release>(`/admin/packs/${selected.value.id}/releases`);
}
async function loadArtifacts() {
  versions.value = modId.value ? await all<Artifact>(`/admin/artifacts?mod_id=${modId.value}`) : [];
  artifactId.value = '';
  targetPath.value = '';
}
function pickArtifact() {
  const artifact = versions.value.find((a) => a.id === artifactId.value);
  targetPath.value = artifact ? `mods/${artifact.filename}` : '';
}
function addItem() {
  if (!modId.value || !artifactId.value || !targetPath.value) return;
  const item = {
    mod_id: modId.value,
    artifact_id: artifactId.value,
    target_path: targetPath.value,
  };
  const index = items.value.findIndex((row) => row.mod_id === item.mod_id);
  if (index >= 0) items.value[index] = item;
  else items.value.push(item);
  preview.value = undefined;
}
async function previewDraft() {
  await saveDraft();
  if (!selected.value || !release.value) return;
  preview.value = await api<UpdatePlan>(
    `/admin/packs/${selected.value.id}/drafts/${release.value.id}/preview`,
    'POST',
    { revision: release.value.revision },
  );
}
function confirmPublish() {
  if (!preview.value || !release.value) return;
  const plan = preview.value;
  Modal.confirm({
    title: `发布 ${plan.target_version}？`,
    content: `本次新增 ${plan.operations.filter((o) => o.action === 'ADD').length} 项、替换 ${plan.operations.filter((o) => o.action === 'REPLACE').length} 项、删除 ${plan.operations.filter((o) => o.action === 'REMOVE').length} 项。正式快照发布后不可修改。`,
    okText: '确认发布',
    cancelText: '取消',
    onOk: () =>
      run(async () => {
        if (!selected.value || !release.value) return;
        const row = await api<Release>(
          `/admin/packs/${selected.value.id}/drafts/${release.value.id}/publish`,
          'POST',
          { revision: release.value.revision },
        );
        await load();
        const pack = packs.value.find((p) => p.id === selected.value?.id);
        if (pack) await select(pack);
        await openRelease(row);
      }, '版本已发布'),
  });
}
const history = computed(() => {
  if (!release.value || release.value.status !== 'published') return [];
  const previous = baseline.value?.items || [];
  const current = release.value.items;
  return [
    ...previous
      .filter((old) => !current.some((item) => item.mod_id === old.mod_id))
      .map((old) => ({ action: 'REMOVE', path: old.target.path })),
    ...current.flatMap((item) => {
      const old = previous.find((old) => old.mod_id === item.mod_id);
      return !old
        ? [{ action: 'ADD', path: item.target.path }]
        : old.target.path !== item.target.path || old.target.sha512 !== item.target.sha512
          ? [
              {
                action: 'REPLACE',
                path: `${old.target.path} → ${item.target.path}`,
              },
            ]
          : [];
    }),
  ];
});
</script>
<template>
  <div class="view-heading row">
    <div>
      <h1>整合包与发布</h1>
      <p class="muted">从完整清单创建草稿，预览最终差异，再发布不可变版本。</p>
    </div>
    <span class="spacer" />
    <button v-if="can('pack:write')" class="primary" @click="editPack()">创建整合包</button>
  </div>
  <LoadingFeedback :busy="busy" />
  <div v-if="error" class="error" role="alert">{{ error }}</div>
  <div v-if="success" class="success">{{ success }}</div>
  <section v-if="showPackForm" class="card section">
    <h2>{{ packId ? '编辑整合包' : '创建整合包' }}</h2>
    <form class="form-grid" @submit.prevent="run(savePack, '整合包已保存')">
      <label>
        名称
        <input v-model="packForm.name" required maxlength="200" />
      </label>
      <label>
        Minecraft 版本
        <input v-model="packForm.minecraft_version" required />
      </label>
      <label>
        Mod Loader
        <select v-model="packForm.loader">
          <option>neoforge</option>
          <option>forge</option>
          <option>fabric</option>
        </select>
      </label>
      <div class="row">
        <button class="primary" :disabled="busy">保存</button>
        <button type="button" @click="showPackForm = false">取消</button>
      </div>
    </form>
  </section>
  <div class="pack-grid">
    <section
      v-for="pack in packs"
      :key="pack.id"
      class="card pack-tile"
      :class="{ active: selected?.id === pack.id }"
    >
      <div class="row">
        <h2 :title="pack.name">{{ pack.name }}</h2>
        <span class="spacer" />
        <span class="pill">{{ pack.loader }}</span>
      </div>
      <p class="muted">Minecraft {{ pack.minecraft_version }}</p>
      <code>{{ pack.id }}</code>
      <div class="row">
        <button v-if="can('release:read')" :disabled="busy" @click="run(() => select(pack))">
          查看版本
        </button>
        <button v-if="can('pack:write')" @click="editPack(pack)">编辑</button>
      </div>
    </section>
    <div v-if="packs.length === 0" class="empty">还没有整合包。</div>
  </div>
  <section v-if="selected" class="card section">
    <div class="row">
      <h2>{{ selected.name }} · 版本</h2>
      <span class="spacer" />
      <button
        v-if="can('release:write')"
        @click="
          showDraftForm = true;
          release = undefined;
          version = '';
          changelog = '';
          base = '';
        "
      >
        创建草稿
      </button>
    </div>
    <form v-if="showDraftForm" class="form-grid" @submit.prevent="run(createDraft, '草稿已创建')">
      <label>
        展示版本号
        <input v-model="version" required placeholder="1.0.0" />
      </label>
      <label>
        从正式版本复制
        <select v-model="base">
          <option value="">当前正式版（首版为空清单）</option>
          <option
            v-for="row in releases.filter((r) => r.status === 'published')"
            :key="row.id"
            :value="row.id"
          >
            {{ row.version }}
          </option>
        </select>
      </label>
      <label class="full">
        更新说明
        <textarea v-model="changelog" rows="3" />
      </label>
      <button class="primary" :disabled="busy">创建</button>
    </form>
    <div v-if="releases.length === 0 && !showDraftForm" class="empty">尚无版本。</div>
    <div v-else class="table-scroll">
      <table>
        <thead>
          <tr>
            <th>版本号</th>
            <th>状态</th>
            <th>发布时间</th>
            <th>操作</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="row in releases" :key="row.id">
            <td>
              {{ row.version }}
              <span v-if="selected.current_release_id === row.id" class="pill">当前正式版</span>
            </td>
            <td>{{ row.status === 'draft' ? '草稿' : '正式版' }}</td>
            <td>
              {{ row.published_at ? new Date(row.published_at).toLocaleString() : '—' }}
            </td>
            <td>
              <button :disabled="busy" @click="run(() => openRelease(row))">
                {{ row.status === 'draft' ? '编辑 / 预览' : '查看快照' }}
              </button>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
  <section v-if="release" class="card section">
    <div class="row">
      <h2>
        {{ release.status === 'draft' ? '草稿编辑' : '正式快照' }} ·
        {{ release.version }}
      </h2>
      <span class="spacer" />
      <span class="pill">revision {{ release.revision }}</span>
    </div>
    <code class="muted">{{ release.id }}</code>
    <fieldset
      :disabled="busy"
      class="draft-editor"
      v-if="release.status === 'draft' && can('release:write')"
    >
      <div class="form-grid draft-meta">
        <label>
          版本号
          <input v-model="version" required />
        </label>
        <label>
          更新说明
          <textarea v-model="changelog" rows="2" />
        </label>
      </div>
      <div v-if="can('mod:read')" class="add-row">
        <select v-model="modId" aria-label="选择 Mod" @change="run(loadArtifacts)">
          <option value="">选择 Mod</option>
          <option v-for="mod in mods" :key="mod.id" :value="mod.id">
            {{ mod.name }} · {{ mod.modid }}
          </option>
        </select>
        <select v-model="artifactId" aria-label="选择文件版本" @change="pickArtifact">
          <option value="">选择文件版本</option>
          <option v-for="file in versions" :key="file.id" :value="file.id">
            {{ file.version }} · {{ file.filename }}
          </option>
        </select>
        <input v-model="targetPath" placeholder="mods/example.jar" aria-label="目标相对路径" />
        <button :disabled="!artifactId" @click="addItem">加入 / 替换</button>
      </div>
      <div class="table-scroll">
        <table>
          <thead>
            <tr>
              <th>Mod</th>
              <th>目标路径</th>
              <th>操作</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(item, index) in items" :key="item.mod_id">
              <td>
                {{
                  mods.find((m) => m.id === item.mod_id)?.name ||
                  release.items.find((i) => i.mod_id === item.mod_id)?.name_snapshot ||
                  item.mod_id
                }}
              </td>
              <td>
                <input v-model="item.target_path" aria-label="目标路径" />
              </td>
              <td>
                <button
                  @click="
                    items.splice(index, 1);
                    preview = undefined;
                  "
                >
                  移出清单
                </button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
      <p class="muted">
        {{ items.length }} 个文件。移出旧版条目会在差异中产生 REMOVE，玩家名单外文件不会被清理。
      </p>
      <div class="row">
        <button :disabled="busy" @click="run(saveDraft, '草稿已保存')">保存草稿</button>
        <button :disabled="busy" @click="run(previewDraft)">保存并预览差异</button>
        <button
          v-if="can('release:publish')"
          class="primary"
          :disabled="busy || !preview"
          @click="confirmPublish"
        >
          发布此版本
        </button>
      </div>
      <section v-if="preview" class="preview">
        <h3>本次差异 · {{ preview.operations.length }} 项</h3>
        <div v-for="operation in preview.operations" :key="operation.mod_id" class="change">
          <span class="pill">{{ operation.action }}</span>
          <code>
            {{ operation.old_path }}
            <span v-if="operation.old_path && operation.target">→</span>
            {{ operation.target?.path }}
          </code>
        </div>
      </section>
    </fieldset>
    <template v-else>
      <p class="changelog">{{ release.changelog || '暂无更新说明' }}</p>
      <h3>发布变更 · {{ history.length }} 项</h3>
      <div v-for="(change, index) in history" :key="index" class="change">
        <span class="pill">{{ change.action }}</span>
        <code>{{ change.path }}</code>
      </div>
      <h3 class="draft-meta">完整文件快照 · {{ release.items.length }} 项</h3>
      <div v-for="item in release.items" :key="item.mod_id" class="snapshot-row">
        <strong>{{ item.name_snapshot }}</strong>
        <span class="muted">{{ item.modid_snapshot }}</span>
        <code>{{ item.target.path }}</code>
      </div>
    </template>
  </section>
</template>
<style scoped>
.draft-editor {
  border: 0;
  padding: 0;
  min-width: 0;
}
.pack-tile h2 {
  min-width: 0;
  font-size: 17px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.pack-tile .pill {
  flex-shrink: 0;
}
.pack-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 18px;
  margin-bottom: 24px;
}
.pack-tile {
  padding: 23px;
  display: grid;
  gap: 8px;
}
.pack-tile.active {
  border-color: var(--accent);
}
.pack-tile code {
  font-size: 10px;
  color: var(--muted);
  overflow-wrap: anywhere;
}
.pack-tile .row:last-child {
  margin-top: 13px;
}
.draft-meta {
  margin-top: 25px;
}
.add-row {
  display: grid;
  grid-template-columns: 1fr 1fr 1fr auto;
  gap: 10px;
  margin: 22px 0;
}
.change {
  display: flex;
  gap: 12px;
  align-items: center;
  padding: 12px 0;
  border-bottom: 1px solid var(--line);
  overflow-wrap: anywhere;
}
.preview {
  margin-top: 28px;
}
.snapshot-row {
  display: grid;
  grid-template-columns: 1fr 1fr 2fr;
  gap: 12px;
  padding: 15px 0;
  border-bottom: 1px solid var(--line);
  font-size: 13px;
}
.changelog {
  white-space: pre-wrap;
  line-height: 1.9;
  margin-top: 24px;
}
</style>
