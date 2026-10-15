<script setup lang="ts">
import LoadingFeedback from '../../../shared/LoadingFeedback.vue';
import { onMounted, ref } from 'vue';
import { all, api, can } from '../api';
import { useAction } from '../useAction';
import type { ModRecord } from '../../../shared/generated/ModRecord';
import type { ModInput } from '../../../shared/generated/ModInput';
import type { Page } from '../../../shared/generated/Page';
import type { Artifact } from '../../../shared/generated/Artifact';
import type { ArtifactInput } from '../../../shared/generated/ArtifactInput';
import type { HostedFile } from '../../../shared/generated/HostedFile';
const { busy, error, success, run } = useAction();
const keyword = ref(''),
  page = ref(1),
  total = ref(0),
  mods = ref<ModRecord[]>([]),
  editing = ref<string>(),
  showForm = ref(false);
const blank = (): ModInput => ({
  modid: '',
  name: '',
  modrinth_project_id: null,
  curseforge_project_id: null,
});
const form = ref(blank());
const selected = ref<ModRecord>(),
  artifacts = ref<Artifact[]>([]),
  showArtifact = ref(false);
const blankArtifact = (): ArtifactInput => ({
  mod_id: '',
  version: '',
  filename: '',
  size: null,
  sha512: '',
  hosted_file_id: null,
  sources: [{ type: 'modrinth', url: '', priority: 0 }],
});
const fileForm = ref(blankArtifact());
async function load() {
  const data = await api<Page<ModRecord>>(
    `/admin/mods?page=${page.value}&page_size=20&keyword=${encodeURIComponent(keyword.value)}`,
  );
  mods.value = data.items;
  total.value = data.total;
}
onMounted(() => run(load));
function edit(mod?: ModRecord) {
  editing.value = mod?.id;
  form.value = mod
    ? {
        modid: mod.modid,
        name: mod.name,
        modrinth_project_id: mod.modrinth_project_id,
        curseforge_project_id: mod.curseforge_project_id,
      }
    : blank();
  showForm.value = true;
}
async function save() {
  await api(
    `/admin/mods${editing.value ? `/${editing.value}` : ''}`,
    editing.value ? 'PATCH' : 'POST',
    form.value,
  );
  showForm.value = false;
  await load();
}
async function select(mod: ModRecord) {
  selected.value = mod;
  artifacts.value = await all<Artifact>(`/admin/artifacts?mod_id=${mod.id}`);
  showArtifact.value = false;
}
function newArtifact() {
  fileForm.value = blankArtifact();
  fileForm.value.mod_id = selected.value?.id || '';
  showArtifact.value = true;
}
async function upload(event: Event) {
  const file = (event.target as HTMLInputElement).files?.[0];
  if (!file) return;
  await run(async () => {
    const body = new FormData();
    body.set('file', file);
    const result = await api<HostedFile>('/admin/files', 'POST', body);
    fileForm.value.filename = result.filename;
    fileForm.value.size = result.size;
    fileForm.value.sha512 = result.sha512;
    fileForm.value.hosted_file_id = result.id;
    fileForm.value.sources = [{ type: 'server', url: result.url, priority: 0 }];
  }, '托管文件上传完成，填写版本后保存');
}
async function saveArtifact() {
  await api('/admin/artifacts', 'POST', fileForm.value);
  if (selected.value) await select(selected.value);
}
</script>
<template>
  <div class="view-heading row">
    <div>
      <h1>Mod 目录</h1>
      <p class="muted">业务 modid 唯一，名称可以重复。编辑原记录可保留历史关联。</p>
    </div>
    <span class="spacer" />
    <button v-if="can('mod:write')" class="primary" @click="edit()">添加 Mod</button>
  </div>
  <LoadingFeedback :busy="busy" />
  <div v-if="error" class="error" role="alert">{{ error }}</div>
  <div v-if="success" class="success">{{ success }}</div>
  <form
    class="row search"
    @submit.prevent="
      run(async () => {
        page = 1;
        await load();
      })
    "
  >
    <input v-model="keyword" placeholder="搜索名称或 modid" aria-label="搜索 Mod" />
    <button :disabled="busy">搜索</button>
  </form>
  <section v-if="showForm" class="card section">
    <h2>{{ editing ? '编辑 Mod' : '添加 Mod' }}</h2>
    <form class="form-grid" @submit.prevent="run(save, 'Mod 已保存')">
      <label>
        业务 modid
        <input v-model="form.modid" required maxlength="128" />
      </label>
      <label>
        显示名称
        <input v-model="form.name" required maxlength="200" />
      </label>
      <label>
        Modrinth Project ID
        <input
          :value="form.modrinth_project_id || ''"
          @input="form.modrinth_project_id = ($event.target as HTMLInputElement).value || null"
        />
      </label>
      <label>
        CurseForge Project ID
        <input
          :value="form.curseforge_project_id || ''"
          @input="form.curseforge_project_id = ($event.target as HTMLInputElement).value || null"
        />
      </label>
      <div class="row">
        <button class="primary" :disabled="busy">保存</button>
        <button type="button" @click="showForm = false">取消</button>
      </div>
    </form>
  </section>
  <section class="card section">
    <div v-if="mods.length === 0" class="empty">暂无 Mod，可手动添加或从 Modrinth 录入。</div>
    <div v-else class="table-scroll">
      <table>
        <thead>
          <tr>
            <th>名称</th>
            <th>modid</th>
            <th>操作</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="mod in mods" :key="mod.id">
            <td>{{ mod.name }}</td>
            <td>
              <code>{{ mod.modid }}</code>
            </td>
            <td class="row">
              <button :disabled="busy" @click="run(() => select(mod))">文件版本</button>
              <button v-if="can('mod:write')" @click="edit(mod)">编辑</button>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
    <div class="row pagination">
      <span class="muted">共 {{ total }} 条</span>
      <span class="spacer" />
      <button
        :disabled="busy || page === 1"
        @click="
          run(async () => {
            page--;
            await load();
          })
        "
      >
        上一页
      </button>
      <span>{{ page }}</span>
      <button
        :disabled="busy || page * 20 >= total"
        @click="
          run(async () => {
            page++;
            await load();
          })
        "
      >
        下一页
      </button>
    </div>
  </section>
  <section v-if="selected" class="card section">
    <div class="row">
      <h2>{{ selected.name }} · 文件版本</h2>
      <span class="spacer" />
      <button v-if="can('mod:write')" @click="newArtifact">录入文件</button>
    </div>
    <form v-if="showArtifact" @submit.prevent="run(saveArtifact, '文件版本已录入')">
      <div class="form-grid">
        <label>
          Mod 版本
          <input v-model="fileForm.version" required />
        </label>
        <label>
          文件名
          <input v-model="fileForm.filename" required placeholder="example.jar" />
        </label>
        <label>
          字节数（未知可留空）
          <input
            type="number"
            min="0"
            :value="fileForm.size ?? ''"
            @input="
              fileForm.size =
                ($event.target as HTMLInputElement).value === ''
                  ? null
                  : Number(($event.target as HTMLInputElement).value)
            "
          />
        </label>
        <label v-if="can('file:write')">
          上传为托管来源
          <input type="file" accept=".jar" :disabled="busy" @change="upload" />
        </label>
        <label class="full">
          SHA-512
          <textarea v-model="fileForm.sha512" required rows="2" minlength="128" maxlength="128" />
        </label>
      </div>
      <div v-for="(source, index) in fileForm.sources" :key="index" class="source-row">
        <select v-model="source.type" aria-label="来源类型">
          <option v-for="type in ['modrinth', 'curseforge', 'github', 'server']" :key="type">
            {{ type }}
          </option>
        </select>
        <input
          v-model="source.url"
          type="url"
          required
          placeholder="HTTP / HTTPS 直链"
          aria-label="下载直链"
        />
        <input v-model.number="source.priority" type="number" min="0" aria-label="来源优先级" />
        <button
          type="button"
          :disabled="fileForm.sources.length === 1"
          @click="fileForm.sources.splice(index, 1)"
        >
          移除
        </button>
      </div>
      <div class="row">
        <button
          type="button"
          @click="
            fileForm.sources.push({
              type: 'github',
              url: '',
              priority: fileForm.sources.length,
            })
          "
        >
          添加备用来源
        </button>
        <span class="spacer" />
        <button class="primary" :disabled="busy">保存文件版本</button>
        <button type="button" @click="showArtifact = false">取消</button>
      </div>
    </form>
    <div v-if="artifacts.length === 0 && !showArtifact" class="empty">
      此 Mod 尚未录入文件版本。
    </div>
    <div v-for="artifact in artifacts" :key="artifact.id" class="file-row">
      <div>
        <strong>{{ artifact.version }}</strong>
        <p class="muted">
          {{ artifact.filename }} · {{ artifact.size ?? '未知' }} 字节 ·
          {{ artifact.sources.length }} 个来源
        </p>
      </div>
      <code class="hash">{{ artifact.sha512 }}</code>
    </div>
  </section>
</template>
<style scoped>
.source-row {
  display: grid;
  grid-template-columns: 125px 1fr 85px auto;
  gap: 10px;
  margin: 12px 0;
}
.file-row {
  padding: 18px 0;
  border-bottom: 1px solid var(--line);
}
.file-row p {
  font-size: 12px;
  margin: 8px 0;
}
.hash {
  font-size: 10px;
  overflow-wrap: anywhere;
  color: var(--muted);
}
.search input {
  max-width: 450px;
}
</style>
