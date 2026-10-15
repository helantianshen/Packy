<script setup lang="ts">
import LoadingFeedback from '../../../shared/LoadingFeedback.vue';
import { ref } from 'vue';
import { api, can } from '../api';
import { useAction } from '../useAction';
import type { ModRecord } from '../../../shared/generated/ModRecord';
import type { Artifact } from '../../../shared/generated/Artifact';
interface Hit {
  project_id: string;
  title: string;
  description: string;
  slug: string;
  downloads: number;
  icon_url?: string;
}
interface File {
  filename: string;
  url: string;
  size: number;
  primary: boolean;
  hashes: { sha512?: string };
}
interface Version {
  id: string;
  name: string;
  version_number: string;
  game_versions: string[];
  loaders: string[];
  files: File[];
}
const { busy, error, success, run } = useAction();
const query = ref(''),
  minecraft = ref(''),
  loader = ref(''),
  offset = ref(0),
  total = ref(0),
  hits = ref<Hit[]>([]),
  searched = ref(false);
const selected = ref<Hit>(),
  versions = ref<Version[]>([]),
  versionId = ref(''),
  fileIndex = ref(0),
  modid = ref(''),
  name = ref(''),
  created = ref<ModRecord>();
async function search() {
  const params = new URLSearchParams({
    query: query.value,
    offset: String(offset.value),
    limit: '20',
  });
  if (minecraft.value) params.set('minecraft_version', minecraft.value);
  if (loader.value) params.set('loader', loader.value);
  const result = await api<{ hits: Hit[]; total_hits: number }>(`/admin/modrinth/search?${params}`);
  hits.value = result.hits;
  total.value = result.total_hits;
  searched.value = true;
}
async function select(hit: Hit) {
  selected.value = hit;
  modid.value = hit.slug;
  name.value = hit.title;
  created.value = undefined;
  const params = new URLSearchParams();
  if (minecraft.value) params.set('minecraft_version', minecraft.value);
  if (loader.value) params.set('loader', loader.value);
  versions.value = await api<Version[]>(
    `/admin/modrinth/projects/${encodeURIComponent(hit.project_id)}/versions?${params}`,
  );
  versionId.value = versions.value[0]?.id || '';
  fileIndex.value = 0;
}
async function importFile() {
  const version = versions.value.find((v) => v.id === versionId.value),
    file = version?.files[fileIndex.value];
  if (!selected.value || !version || !file?.hashes.sha512)
    throw new Error('该文件缺少 SHA-512，不能录入');
  if (!created.value)
    created.value = await api<ModRecord>('/admin/mods', 'POST', {
      modid: modid.value,
      name: name.value,
      modrinth_project_id: selected.value.project_id,
      curseforge_project_id: null,
    });
  await api<Artifact>('/admin/artifacts', 'POST', {
    mod_id: created.value.id,
    version: version.version_number,
    filename: file.filename,
    size: file.size,
    sha512: file.hashes.sha512,
    hosted_file_id: null,
    sources: [{ type: 'modrinth', url: file.url, priority: 0 }],
  });
  selected.value = undefined;
}
</script>
<template>
  <div class="view-heading">
    <h1>发现 Mod</h1>
    <p class="muted">搜索 Modrinth，选择准确文件并录入目录。不会自动添加到正式整合包。</p>
  </div>
  <LoadingFeedback :busy="busy" />
  <div v-if="error" class="error" role="alert">{{ error }}</div>
  <div v-if="success" class="success">{{ success }}</div>
  <form
    class="search-form card section"
    @submit.prevent="
      run(async () => {
        offset = 0;
        await search();
      })
    "
  >
    <input v-model="query" placeholder="搜索 Mod 名称…" aria-label="搜索关键词" />
    <input v-model="minecraft" placeholder="Minecraft 版本（可选）" aria-label="Minecraft 版本" />
    <select v-model="loader" aria-label="加载器">
      <option value="">全部加载器</option>
      <option>neoforge</option>
      <option>forge</option>
      <option>fabric</option>
    </select>
    <button class="primary" :disabled="busy">
      {{ busy ? '正在请求…' : '搜索' }}
    </button>
  </form>
  <section v-if="selected" class="card section">
    <div class="row">
      <h2>录入 {{ selected.title }}</h2>
      <span class="spacer" />
      <button @click="selected = undefined">收起</button>
    </div>
    <p class="muted">
      平台 slug 不一定等于游戏 modid，请确认业务 modid 后保存。已有 Mod
      可在目录中为原记录手动录入文件。
    </p>
    <form
      class="form-grid"
      @submit.prevent="run(importFile, 'Mod 和文件已录入，可在整合包草稿中选用')"
    >
      <label>
        业务 modid
        <input v-model="modid" required :disabled="!!created" />
      </label>
      <label>
        显示名称
        <input v-model="name" required :disabled="!!created" />
      </label>
      <label>
        版本
        <select v-model="versionId" @change="fileIndex = 0">
          <option v-for="version in versions" :key="version.id" :value="version.id">
            {{ version.name }} · {{ version.game_versions.join(', ') }}
          </option>
        </select>
      </label>
      <label>
        文件
        <select v-model.number="fileIndex">
          <option
            v-for="(file, index) in versions.find((v) => v.id === versionId)?.files || []"
            :key="file.filename"
            :value="index"
          >
            {{ file.filename }} · {{ file.size }} 字节
          </option>
        </select>
      </label>
      <button v-if="can('mod:write')" class="primary" :disabled="busy || !versionId">
        录入文件
      </button>
    </form>
  </section>
  <div class="results">
    <article v-for="hit in hits" :key="hit.project_id" class="card result">
      <div class="row">
        <img v-if="hit.icon_url" :src="hit.icon_url" alt="" loading="lazy" />
        <div>
          <h2>{{ hit.title }}</h2>
          <small class="muted">{{ hit.slug }} · {{ hit.downloads.toLocaleString() }} 次下载</small>
        </div>
      </div>
      <p class="muted">{{ hit.description }}</p>
      <button :disabled="busy" @click="run(() => select(hit))">选择版本</button>
    </article>
  </div>
  <div v-if="searched && !hits.length" class="empty">
    没有匹配的项目，试试其他关键词或筛选条件。
  </div>
  <div v-if="searched" class="row pagination">
    <span class="muted">共 {{ total }} 个项目</span>
    <span class="spacer" />
    <button
      :disabled="busy || offset === 0"
      @click="
        run(async () => {
          offset -= 20;
          await search();
        })
      "
    >
      上一页
    </button>
    <button
      :disabled="busy || offset + 20 >= total"
      @click="
        run(async () => {
          offset += 20;
          await search();
        })
      "
    >
      下一页
    </button>
  </div>
</template>
<style scoped>
.search-form {
  display: grid;
  grid-template-columns: 2fr 1fr 1fr auto;
  gap: 12px;
}
.results {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(290px, 1fr));
  gap: 18px;
}
.result {
  padding: 22px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.result h2 {
  margin-bottom: 5px;
}
.result img {
  width: 44px;
  height: 44px;
  border-radius: 10px;
}
.result p {
  line-height: 1.7;
  font-size: 13px;
  flex: 1;
  margin: 0;
}
.result button {
  align-self: flex-start;
}
</style>
