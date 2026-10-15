<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { AnimatePresence, motion } from 'motion-v';
import {
  Box,
  Sun,
  Moon,
  ArrowUpRight,
  ShieldCheck,
  Download,
  Check,
  RefreshCw,
  Gamepad2,
  Info,
  Layers,
  WifiOff,
  CheckCircle2,
  Clock,
  TriangleAlert,
} from '@lucide/vue';
import { theme, toggleTheme } from '../../../web/shared/theme';
import LoadingFeedback from '../../../web/shared/LoadingFeedback.vue';
import { useUpdater } from './composables/useUpdater';
const {
  snapshot,
  status,
  error,
  notificationHint,
  pending,
  pendingCommand,
  initializing,
  active,
  desktop,
  action,
} = useUpdater();
const page = ref<'home' | 'details'>('home'),
  confirm = ref<HTMLDialogElement>();
// 页面切换不改变透明度和位置，Motion 完成退场后才挂载目标页面
const pageMotion = {
  initial: false as const,
  exit: { opacity: 1, transition: { duration: 0 } },
};
const packName = computed(() => snapshot.value?.config.pack_name || '蒹葭服务器整合包');
const backgroundEnabled = computed(() => snapshot.value?.preferences.background_enabled ?? true);
const plan = computed(() => snapshot.value?.plan);
const phases: Record<string, string> = {
  idle: '准备就绪',
  checking: '正在检查更新',
  available: '发现可用更新',
  ready: '已经是最新版本',
  downloading: '正在下载文件',
  verifying: '正在校验文件',
  installing: '正在安装更新',
  saving_state: '正在保存版本',
  completed: '更新完成',
  failed: '需要处理',
  cancelled: '更新已取消',
};
const loadingLabel = computed(() => {
  if (initializing.value) return '正在读取整合包配置…';
  if (pendingCommand.value === 'set_background_enabled') return '正在保存后台运行设置…';
  if (pendingCommand.value === 'check_update') return '正在获取更新清单…';
  if (pendingCommand.value === 'start_update') return '正在准备更新任务…';
  if (pendingCommand.value === 'cancel_update') return '正在安全停止下载…';
  if (pendingCommand.value === 'launch_game') return '正在打开启动器…';
  return '正在读取任务状态…';
});
const checking = computed(
  () => pendingCommand.value === 'check_update' || status.value?.phase === 'checking',
);
const updating = computed(() => active.value && status.value?.phase !== 'checking');
const showChecking = ref(false);
const settledPhase = ref('idle');
// 快速检查只锁定操作，持续超过 150ms 才显示加载反馈，避免提示一闪而过
watch(checking, (busy, _, cleanup) => {
  showChecking.value = false;
  if (!busy) return;
  const timer = setTimeout(() => (showChecking.value = true), 150);
  cleanup(() => clearTimeout(timer));
});
watch(
  () => status.value?.phase,
  (phase) => {
    if (phase && phase !== 'checking') settledPhase.value = phase;
  },
  { immediate: true, flush: 'sync' },
);
const feedbackBusy = computed(() =>
  checking.value ? showChecking.value : pending.value || initializing.value,
);
const title = computed(
  () =>
    phases[
      checking.value && !showChecking.value ? settledPhase.value : status.value?.phase || 'idle'
    ] || '准备就绪',
);
const changeCounts = computed(() => ({
  add: plan.value?.operations.filter((o) => o.action === 'ADD').length || 0,
  replace: plan.value?.operations.filter((o) => o.action === 'REPLACE').length || 0,
  remove: plan.value?.operations.filter((o) => o.action === 'REMOVE').length || 0,
}));
const progress = computed(() =>
  status.value?.phase === 'completed'
    ? 100
    : status.value?.total_bytes
      ? Math.min(100, Math.round((status.value.downloaded_bytes / status.value.total_bytes) * 100))
      : 0,
);
const retryable = computed(
  () => !!status.value?.task_id && ['failed', 'cancelled'].includes(status.value.phase),
);
const canLaunch = computed(() => !!snapshot.value && !updating.value && !retryable.value);
const primaryLabel = computed(() => {
  if (checking.value) return '检查更新';
  if (retryable.value) return '重试原任务';
  if (status.value?.phase === 'available') return '查看并安装';
  return updating.value ? title.value : '检查更新';
});
function primaryAction() {
  if (retryable.value) void action('retry_update', { task_id: status.value?.task_id });
  else if (status.value?.phase === 'available') confirm.value?.showModal();
  else void action('check_update');
}
function bytes(value: number | null | undefined) {
  if (value == null) return '未知';
  if (value < 1024) return `${value} B`;
  const units = ['KiB', 'MiB', 'GiB'];
  let amount = value / 1024,
    index = 0;
  while (amount >= 1024 && index < units.length - 1) {
    amount /= 1024;
    index++;
  }
  return `${amount.toFixed(1)} ${units[index]}`;
}
async function start() {
  confirm.value?.close();
  await action('start_update');
}
</script>
<template>
  <div class="shell">
    <aside>
      <div class="brand">
        <span class="brand-icon"><Box :size="23" /></span>
        Packy
      </div>
      <div class="eyebrow">你的游戏空间</div>
      <nav>
        <button :class="{ selected: page === 'home' }" @click="page = 'home'">
          <Layers :size="17" />
          整合包更新
        </button>
        <button :class="{ selected: page === 'details' }" @click="page = 'details'">
          <Info :size="17" />
          配置与说明
        </button>
      </nav>
      <div class="sidebar-bottom muted">
        <div class="row">
          <span class="status-dot" :class="{ 'is-disabled': !backgroundEnabled }" />
          {{ backgroundEnabled ? '系统托盘 · 后台常驻' : '关闭窗口时安全退出' }}
        </div>
        <p>让每次出发，都准备就绪。</p>
        <small>蒹葭服务器专用 · Packy 0.1.0</small>
      </div>
    </aside>
    <main>
      <header class="row">
        <span class="eyebrow">MODPACK COMPANION</span>
        <span class="spacer" />
        <span v-if="updating" class="pill">
          <Clock :size="12" />
          后台更新中
        </span>
        <button class="icon-button" aria-label="切换明暗主题" @click="toggleTheme">
          <Sun v-if="theme === 'dark'" :size="18" />
          <Moon v-else :size="18" />
        </button>
      </header>
      <!-- wait 模式只显示一个页面，首次挂载不执行入场动画 -->
      <div class="client-pages">
        <AnimatePresence mode="wait" :initial="false">
          <motion.div
            v-if="page === 'home'"
            key="home"
            class="view-panel client-page"
            data-page="home"
            v-bind="pageMotion"
          >
            <div class="intro">
              <span class="pill">
                {{
                  updating
                    ? backgroundEnabled
                      ? '你可以关闭窗口，更新将在后台继续'
                      : '请保持窗口打开，更新正在进行'
                    : '专注于你的下一次冒险'
                }}
              </span>
              <h1>
                {{ status?.phase === 'completed' ? '一切就绪，出发吧。' : '更新就绪，轻松出发。' }}
              </h1>
              <p class="muted">
                {{
                  updating
                    ? backgroundEnabled
                      ? '无需停留在这里，完成后会通过系统通知提醒你。'
                      : '完成后会通过系统通知提醒你，关闭窗口将尝试安全停止更新。'
                    : '获取最新内容，让文件下载与校验有序完成。'
                }}
              </p>
            </div>
            <LoadingFeedback :busy="feedbackBusy" :label="loadingLabel" />
            <!-- 骨架覆盖同一张卡片，读取结束只切换透明度，保留内容的布局占位 -->
            <section
              class="card pack-card"
              :class="{ 'is-loading': initializing }"
              :aria-busy="initializing"
              :inert="initializing"
            >
              <Transition name="feedback">
                <div v-if="initializing" class="pack-skeleton" aria-hidden="true">
                  <div class="pack-art skeleton" />
                  <div class="pack-info skeleton-info">
                    <div class="skeleton skeleton-title" />
                    <div class="skeleton skeleton-line" />
                    <div class="skeleton skeleton-line" />
                  </div>
                </div>
              </Transition>
              <div class="pack-art">
                <Box :size="76" :stroke-width="1.2" />
                <div class="art-dot" />
              </div>
              <div class="pack-info">
                <div class="eyebrow">YOUR MODPACK</div>
                <h2 :title="packName">{{ packName }}</h2>
                <div class="row version-row">
                  <span class="version-label muted">本地版本</span>
                  <code :title="snapshot?.state.installed_release_id">
                    {{ snapshot?.state.installed_release_id.slice(0, 8) || '未读取' }}
                    <template v-if="snapshot">…</template>
                  </code>
                  <span v-if="plan" class="pill">{{ plan.target_version }}</span>
                </div>
                <p class="muted">
                  {{ title }}
                  <template v-if="status?.phase === 'available'">
                    · 新增 {{ changeCounts.add }} / 替换 {{ changeCounts.replace }} / 删除
                    {{ changeCounts.remove }}
                  </template>
                </p>
                <!-- 检查期间保留操作节点与亮度，禁用状态仍阻止重复请求和启动游戏 -->
                <div class="actions row" :class="{ 'is-checking': checking }">
                  <button
                    class="primary"
                    :disabled="pending || active || !desktop"
                    :aria-busy="checking || updating"
                    @click="primaryAction"
                  >
                    <span
                      v-if="showChecking || updating"
                      class="loading-spinner"
                      aria-hidden="true"
                    />
                    <Download v-else-if="status?.phase === 'available' && !checking" :size="16" />
                    <RefreshCw v-else :size="16" />
                    {{ primaryLabel }}
                  </button>
                  <button
                    v-if="canLaunch"
                    :disabled="pending || !desktop"
                    @click="action('launch_game')"
                  >
                    <Gamepad2 :size="16" />
                    {{ status?.phase === 'failed' ? '离线启动' : '打开启动器' }}
                  </button>
                  <button
                    v-if="updating && ['downloading', 'verifying'].includes(status?.phase || '')"
                    :disabled="pending"
                    @click="action('cancel_update', { task_id: status?.task_id || '' })"
                  >
                    取消
                  </button>
                </div>
              </div>
            </section>
            <Transition name="panel">
              <section
                v-if="status?.task_id && (active || retryable || status?.phase === 'completed')"
                class="card progress-card"
              >
                <div class="row">
                  <div class="progress-icon">
                    <CheckCircle2 v-if="status?.phase === 'completed'" :size="21" />
                    <TriangleAlert v-else-if="retryable" :size="21" />
                    <span v-else class="loading-spinner" aria-hidden="true" />
                  </div>
                  <div>
                    <Transition name="feedback" mode="out-in">
                      <strong :key="status?.phase" role="status">{{ title }}</strong>
                    </Transition>
                    <p class="muted">
                      {{ status?.completed_files || 0 }} /
                      {{ status?.total_files || 0 }} 个文件已下载并校验
                    </p>
                  </div>
                  <span class="spacer" />
                  <span
                    v-if="status?.total_bytes || status?.phase === 'completed'"
                    class="percentage"
                  >
                    {{ progress }}%
                  </span>
                  <span v-else class="muted">{{ bytes(status?.downloaded_bytes) }}</span>
                </div>
                <div
                  role="progressbar"
                  :aria-label="title"
                  aria-valuemin="0"
                  aria-valuemax="100"
                  :aria-valuenow="
                    status?.total_bytes || status?.phase === 'completed' ? progress : undefined
                  "
                  class="progress-track"
                  :class="{ indeterminate: active && !status?.total_bytes }"
                >
                  <span :style="{ width: `${progress}%` }" />
                </div>
                <div class="row muted progress-caption">
                  <span>
                    {{ bytes(status?.downloaded_bytes) }} / {{ bytes(status?.total_bytes) }}
                  </span>
                  <span class="spacer" />
                  <span v-if="status?.phase === 'downloading'">
                    {{ bytes(status?.bytes_per_second) }}/s · 最多 4 个文件并发
                  </span>
                  <span
                    v-else-if="status?.phase === 'installing' || status?.phase === 'saving_state'"
                  >
                    正在写入，请等待完成
                  </span>
                  <span v-else-if="status?.phase === 'completed'">已校验并保存版本</span>
                </div>
              </section>
            </Transition>
            <div v-if="status?.error" class="error" role="alert">
              <strong>{{ status.error.message }}</strong>
              <small v-if="status.error.request_id">诊断编号：{{ status.error.request_id }}</small>
            </div>
            <div v-if="error && error !== status?.error?.message" class="error" role="alert">
              {{ error }}
            </div>
            <p v-if="notificationHint" class="muted hint">{{ notificationHint }}</p>
            <Transition name="panel">
              <section v-if="plan" class="card changelog">
                <div class="row">
                  <h2>
                    {{ status?.phase === 'completed' ? '本次更新' : '更新说明' }}
                  </h2>
                  <span class="spacer" />
                  <span class="muted">{{ plan.target_version }}</span>
                </div>
                <p>{{ plan.changelog || '管理员未填写更新说明。' }}</p>
                <details v-if="plan.operations.length">
                  <summary>查看 {{ plan.operations.length }} 项文件变更</summary>
                  <div
                    v-for="operation in plan.operations"
                    :key="operation.mod_id"
                    class="operation"
                  >
                    <span class="pill">{{ operation.action }}</span>
                    <code>
                      {{ operation.old_path }}
                      <span v-if="operation.old_path && operation.target">→</span>
                      {{ operation.target?.path }}
                    </code>
                  </div>
                </details>
              </section>
            </Transition>
          </motion.div>
          <motion.div
            v-else
            key="details"
            class="view-panel client-page"
            data-page="details"
            v-bind="pageMotion"
          >
            <div class="intro">
              <span class="pill">WORKSPACE SETTINGS</span>
              <h1>配置与说明</h1>
              <p class="muted">蒹葭服务器专用更新器，文件路径相对更新器所在目录。</p>
            </div>
            <LoadingFeedback :busy="feedbackBusy" :label="loadingLabel" />
            <section class="card settings">
              <h2>当前整合包</h2>
              <template v-if="snapshot">
                <dl>
                  <dt>整合包名称</dt>
                  <dd>{{ packName }}</dd>
                  <dt>整合包 ID</dt>
                  <dd>{{ snapshot.config.pack_id }}</dd>
                  <dt>更新服务</dt>
                  <dd>{{ snapshot.config.server_url }}</dd>
                  <dt>游戏实例</dt>
                  <dd>{{ snapshot.config.game_dir }}</dd>
                  <dt>启动器</dt>
                  <dd>{{ snapshot.config.launcher_path }}</dd>
                </dl>
              </template>
              <p v-else class="muted">
                请联系整合包管理员检查程序内置配置与 .updater/state.json，然后重启更新器。
              </p>
              <div class="row background-setting">
                <h2 id="background-label">后台运行</h2>
                <span class="spacer" />
                <button
                  role="switch"
                  class="background-switch"
                  :aria-checked="backgroundEnabled"
                  aria-labelledby="background-label"
                  aria-describedby="background-description"
                  :disabled="pending || !snapshot || !desktop"
                  @click="action('set_background_enabled', { enabled: !backgroundEnabled })"
                >
                  <span />
                </button>
              </div>
              <p id="background-description" class="muted">
                {{
                  backgroundEnabled
                    ? '已开启：关闭窗口后隐藏到系统托盘，更新继续进行。'
                    : '已关闭：关闭窗口时安全退出，下载会停止并保留原任务供重试。'
                }}
                更新完成后发送系统通知；安装和保存版本期间不可退出。设置自动保存在本整合包中。
              </p>
            </section>
            <div v-if="error" class="error">{{ error }}</div>
          </motion.div>
        </AnimatePresence>
      </div>
      <footer class="row muted">
        <ShieldCheck :size="15" />
        <span>SHA-512 校验 · 仅处理名单文件</span>
        <span class="spacer" />
        <span v-if="updating">后台任务由 Rust 核心持有</span>
        <span v-else>准备好，再出发</span>
      </footer>
    </main>
    <dialog ref="confirm" class="confirm-dialog">
      <form method="dialog">
        <div class="row">
          <h2>安装 {{ plan?.target_version }}</h2>
          <span class="spacer" />
          <button aria-label="关闭确认" class="icon-button">×</button>
        </div>
        <p class="muted">全部文件下载并校验通过后，才会修改游戏目录。</p>
        <div class="counts">
          <span>
            <Check :size="16" />
            新增 {{ changeCounts.add }}
          </span>
          <span>
            <RefreshCw :size="16" />
            替换 {{ changeCounts.replace }}
          </span>
          <span>
            <WifiOff :size="16" />
            删除 {{ changeCounts.remove }}
          </span>
        </div>
        <div class="dialog-changes">
          <div
            v-for="operation in plan?.operations || []"
            :key="operation.mod_id"
            class="operation"
          >
            <span class="pill">{{ operation.action }}</span>
            <code>
              {{ operation.old_path }}
              <span v-if="operation.old_path && operation.target">→</span>
              {{ operation.target?.path }}
            </code>
          </div>
        </div>
        <p class="muted hint">名单外 Mod 不作处理。失败重试将继续本次更新，不切换到新发布。</p>
        <div class="row">
          <button>稍后再说</button>
          <span class="spacer" />
          <button type="button" class="primary" :disabled="pending" @click="start">
            确认安装
            <ArrowUpRight :size="16" />
          </button>
        </div>
      </form>
    </dialog>
  </div>
</template>
<style scoped>
.shell {
  display: grid;
  grid-template-columns: 220px 1fr;
  min-height: 100vh;
}
aside {
  border-right: 1px solid var(--line);
  padding: 32px 22px;
  display: flex;
  flex-direction: column;
  gap: 26px;
  background: var(--panel);
  position: sticky;
  top: 0;
  height: 100vh;
}
.brand {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 25px;
  font-weight: 750;
  letter-spacing: -1px;
}
.brand-icon {
  background: var(--accent);
  color: var(--panel);
  display: grid;
  place-items: center;
  width: 37px;
  height: 37px;
  border-radius: 11px;
}
nav {
  display: grid;
  gap: 8px;
}
nav button {
  justify-content: flex-start;
  border: none;
  background: transparent;
  font-size: 13px;
  padding: 13px;
}
nav button.selected {
  color: var(--accent);
  background: var(--accent-soft);
}
.sidebar-bottom {
  margin-top: auto;
  font-size: 10px;
  line-height: 1.9;
}
.sidebar-bottom p {
  margin: 13px 0 5px;
}
.sidebar-bottom small {
  font-size: 10px;
}
.status-dot {
  width: 6px;
  height: 6px;
  background: var(--accent);
  border-radius: 50%;
}
.status-dot.is-disabled {
  background: var(--danger);
}
main {
  max-width: 1050px;
  width: 100%;
  margin: 0 auto;
  padding: 30px 42px;
}
/* 页面保持不透明，避免底层合成内容透出 */
.client-page {
  background: var(--bg);
}
header {
  margin-bottom: 52px;
}
header .pill {
  display: flex;
  align-items: center;
  gap: 5px;
}
.intro {
  margin-bottom: 30px;
}
.intro h1 {
  margin-top: 19px;
  margin-bottom: 12px;
}
.intro p {
  font-size: 13px;
}
.pack-card {
  position: relative;
  display: flex;
  gap: 30px;
  padding: 25px;
  margin-bottom: 22px;
  overflow: hidden;
}
.pack-art {
  flex: 0 0 160px;
  min-height: 210px;
  border-radius: 13px;
  background: linear-gradient(145deg, #274b3e, #132c23);
  color: #8cdbb9;
  display: grid;
  place-items: center;
  position: relative;
}
.art-dot {
  position: absolute;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #89dfb9;
  right: 18px;
  top: 18px;
  box-shadow: 0 0 25px #89dfb9;
}
.pack-info {
  padding: 10px 0;
  flex: 1;
  min-width: 0;
}
.pack-info h2 {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  margin: 14px 0 14px;
}
.pack-info p {
  min-height: 3.8em;
  font-size: 12px;
  line-height: 1.9;
}
.version-row {
  min-height: 26px;
  font-size: 11px;
  flex-wrap: wrap;
}
.actions {
  margin-top: 15px;
  flex-wrap: wrap;
  min-height: 39px;
}
/* 加载图标与操作图标等宽，反馈出现时相邻按钮保持原位 */
.actions .loading-spinner {
  width: 16px;
  height: 16px;
}
.actions.is-checking button:disabled {
  opacity: 1;
}
.actions button {
  font-size: 12px;
  padding: 10px 13px;
}
.progress-card {
  padding: 22px;
  margin-bottom: 22px;
}
.progress-card strong {
  font-size: 13px;
}
.progress-card p {
  font-size: 11px;
  margin: 5px 0 0;
}
.progress-icon {
  width: 42px;
  height: 42px;
  display: grid;
  place-items: center;
  background: var(--accent-soft);
  color: var(--accent);
  border-radius: 11px;
}
.percentage {
  font-weight: 650;
  color: var(--accent);
  font-size: 20px;
}
.progress-track {
  height: 6px;
  background: var(--line);
  border-radius: 4px;
  margin: 22px 0 12px;
  overflow: hidden;
}
.progress-track span {
  display: block;
  height: 100%;
  background: var(--accent);
  border-radius: 4px;
  transition:
    width 300ms var(--motion-ease),
    background-color var(--motion-normal) ease;
}
/* 骨架不进入文档流，移除动画节点时不会改变卡片或后续说明的位置 */
.pack-skeleton {
  position: absolute;
  inset: 25px;
  display: flex;
  gap: inherit;
  pointer-events: none;
}
.pack-card > .pack-art,
.pack-card > .pack-info {
  transition: opacity var(--motion-normal) ease;
}
.pack-card.is-loading > .pack-art,
.pack-card.is-loading > .pack-info {
  opacity: 0;
}
.skeleton-info {
  display: grid;
  align-content: center;
  gap: 20px;
}
.skeleton-title {
  height: 27px;
  width: 55%;
}
.skeleton-line {
  height: 15px;
  width: 85%;
}
.progress-track.indeterminate span {
  width: 35% !important;
  animation: slide 1.5s infinite ease-in-out;
}
.progress-caption {
  font-size: 10px;
}
.changelog {
  padding: 25px;
  margin-top: 22px;
}
.changelog h2 {
  margin: 0;
  font-size: 15px;
}
.changelog p {
  white-space: pre-wrap;
  font-size: 13px;
  line-height: 1.9;
  color: var(--muted);
  margin: 20px 0;
}
.changelog .row > span {
  font-size: 11px;
}
summary {
  font-size: 12px;
  cursor: pointer;
  color: var(--accent);
  padding: 10px 0;
}
.operation {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 0;
  border-bottom: 1px solid var(--line);
  font-size: 11px;
  overflow-wrap: anywhere;
}
.operation code {
  min-width: 0;
  overflow-wrap: anywhere;
}
.operation .pill {
  font-size: 9px;
  flex-shrink: 0;
}
.hint {
  font-size: 11px;
  line-height: 1.8;
}
.error {
  font-size: 12px;
  line-height: 1.7;
}
.error small {
  display: block;
  font-size: 10px;
  margin-top: 5px;
}
.background-setting {
  margin-top: 25px;
}
.background-setting h2 {
  margin: 0;
}
.background-switch {
  width: 44px;
  height: 26px;
  padding: 3px;
  border-radius: 20px;
  justify-content: flex-start;
  background: var(--panel-soft);
}
.background-switch[aria-checked='true'] {
  background: var(--accent);
}
.background-switch span {
  width: 18px;
  height: 18px;
  flex-shrink: 0;
  border-radius: 50%;
  background: var(--text);
  transition: transform var(--motion-fast) var(--motion-ease);
}
.background-switch[aria-checked='true'] span {
  background: var(--panel);
  transform: translateX(17px);
}
.settings {
  padding: 28px;
}
.settings h2 {
  margin-top: 25px;
  font-size: 16px;
}
.settings h2:first-child {
  margin-top: 0;
}
.settings p {
  font-size: 12px;
  line-height: 1.9;
}
dl {
  display: grid;
  grid-template-columns: 110px 1fr;
  gap: 16px;
  font-size: 12px;
}
dt {
  color: var(--muted);
}
dd {
  margin: 0;
  overflow-wrap: anywhere;
}
.confirm-dialog {
  max-width: 600px;
  width: calc(100vw - 60px);
  padding: 28px;
  border: 1px solid var(--line);
  border-radius: 18px;
  background: var(--panel);
  color: var(--text);
  box-shadow: 0 30px 100px #0004;
}
.confirm-dialog[open] {
  animation: dialog-enter 220ms var(--motion-ease);
}
@keyframes dialog-enter {
  from {
    opacity: 0;
    transform: translateY(10px) scale(0.98);
  }
  to {
    opacity: 1;
    transform: translateY(0) scale(1);
  }
}
.confirm-dialog::backdrop {
  background: #06150c66;
  backdrop-filter: blur(4px);
}
.confirm-dialog h2 {
  margin: 0;
}
.confirm-dialog p {
  font-size: 12px;
  line-height: 1.8;
}
.confirm-dialog > .row {
  margin-top: 20px;
}
.counts {
  display: flex;
  gap: 18px;
  margin: 22px 0;
  color: var(--accent);
  font-size: 12px;
}
.counts span {
  display: flex;
  gap: 6px;
  align-items: center;
}
.dialog-changes {
  max-height: 250px;
  overflow-y: auto;
}
footer {
  font-size: 10px;
  margin-top: 38px;
}
@keyframes slide {
  from {
    transform: translateX(-100%);
  }
  to {
    transform: translateX(400%);
  }
}
@media (max-width: 900px) {
  .shell {
    grid-template-columns: 180px 1fr;
  }
  main {
    padding: 26px;
  }
  .pack-card {
    gap: 20px;
  }
  .pack-art {
    flex-basis: 110px;
  }
  header {
    margin-bottom: 35px;
  }
  .sidebar-bottom .row {
    gap: 5px;
  }
  .version-row {
    gap: 7px;
  }
}
</style>
