<script setup lang="ts">
import { computed, onMounted, onBeforeUnmount, ref, watch } from 'vue';
import {
  ArrowDownToLine,
  ArrowRight,
  Check,
  CheckCircle2,
  FileText,
  Info,
  LockKeyhole,
  Moon,
  Plus,
  RefreshCw,
  ShieldCheck,
  Sun,
  Trash2,
  TriangleAlert,
  X,
} from '@lucide/vue';
import { theme, setTheme, toggleTheme } from './shared/theme';
import DownloadSettingsPanel from './shared/DownloadSettingsPanel.vue';
import { useUpdater } from './composables/useUpdater';
const { snapshot, status, error, pending, pendingCommand, initializing, active, desktop, action } =
  useUpdater();
const page = ref<'update' | 'settings'>('update');
const tab = ref<'notes' | 'files'>('notes');
const workspace = ref<HTMLElement>();
let navigationAnimations: Animation[] = [];
function stopNavigationMotion() {
  navigationAnimations.forEach((animation) => animation.cancel());
  navigationAnimations = [];
}
watch(
  [page, tab],
  ([nextPage], [previousPage]) => {
    stopNavigationMotion();
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;
    const selector =
      nextPage === previousPage
        ? '.release-notes, .file-list'
        : ':scope > section:not(.pack-header), :scope > .notice';
    workspace.value?.querySelectorAll<HTMLElement>(selector).forEach((element) => {
      navigationAnimations.push(
        element.animate(
          [
            { opacity: 0.55, transform: `translateX(${nextPage === 'settings' ? 8 : -8}px)` },
            { opacity: 1, transform: 'translateX(0)' },
          ],
          { duration: 180, easing: 'cubic-bezier(0.16, 1, 0.3, 1)' },
        ),
      );
    });
  },
  { flush: 'post' },
);
onMounted(() => {
  const preference = window.matchMedia('(prefers-reduced-motion: reduce)');
  preference.addEventListener('change', stopNavigationMotion);
  onBeforeUnmount(() => preference.removeEventListener('change', stopNavigationMotion));
});
onBeforeUnmount(stopNavigationMotion);
const confirmation = ref<HTMLDialogElement>();
const launchMessage = ref('');
const branding = ref({
  logo: './branding/logo.png',
  light: './branding/background-light.webp',
  dark: './branding/background-dark.webp',
});
onMounted(async () => {
  if (window.packy) {
    try {
      const overrides = await window.packy.branding();
      await Promise.all(
        Object.entries(overrides).map(async ([key, url]) => {
          if (!['logo', 'light', 'dark'].includes(key) || !url) return;
          const image = new Image();
          image.src = url;
          try {
            await image.decode();
            branding.value[key as keyof typeof branding.value] = url;
          } catch {
            /* 文件头有效但内容损坏时保留默认素材 */
          }
        }),
      );
    } catch {
      /* 默认素材不依赖品牌读取成功 */
    }
  }
});
const phase = computed(() => status.value?.phase || 'idle');
const plan = computed(() => snapshot.value?.plan);
const packName = computed(() => snapshot.value?.config.pack_name || '蒹葭服务器整合包');
const background = computed(() => snapshot.value?.preferences.background_enabled ?? true);
const checking = computed(
  () => phase.value === 'checking' || pendingCommand.value === 'check_update',
);
const critical = computed(() => ['installing', 'saving_state'].includes(phase.value));
const transferring = computed(() => ['downloading', 'verifying'].includes(phase.value));
const completed = computed(() => phase.value === 'completed');
const retryable = computed(
  () => !!status.value?.task_id && ['failed', 'cancelled'].includes(phase.value),
);
const busy = computed(() => active.value || pending.value || initializing.value);
const failed = computed(() => !!error.value || !!status.value?.error || phase.value === 'failed');
const errorMessage = computed(
  () =>
    error.value ||
    (retryable.value
      ? '更新未完成，请重试原任务；详细原因已记录到客户端日志'
      : '操作未完成，请重试；详细原因已记录到客户端日志'),
);
const labels: Record<string, string> = {
  idle: '准备就绪',
  checking: '正在检查更新',
  available: '发现可用更新',
  ready: '已经是最新版本',
  downloading: '正在下载文件',
  verifying: '正在校验文件',
  installing: '正在安装更新',
  saving_state: '正在保存版本',
  completed: '更新完成',
  failed: '更新未完成',
  cancelled: '更新已取消',
};
const title = computed(() =>
  initializing.value
    ? '正在连接更新核心'
    : checking.value
      ? labels.checking
      : labels[phase.value] || '准备就绪',
);
const operations = computed(() => plan.value?.operations || []);
const counts = computed(() => ({
  ADD: operations.value.filter((o) => o.action === 'ADD').length,
  REPLACE: operations.value.filter((o) => o.action === 'REPLACE').length,
  REMOVE: operations.value.filter((o) => o.action === 'REMOVE').length,
}));
const actionNames = { ADD: '新增', REPLACE: '替换', REMOVE: '移除' };
const downloadSize = computed(() => {
  let total = 0;
  for (const op of operations.value)
    if (op.target) {
      if (op.target.size == null) return null;
      total += op.target.size;
    }
  return total;
});
const progress = computed(() =>
  status.value?.total_bytes
    ? Math.min(100, Math.round((status.value.downloaded_bytes / status.value.total_bytes) * 100))
    : null,
);
const step = computed(
  () =>
    ({ downloading: 0, verifying: 1, installing: 2, saving_state: 2, completed: 3 })[phase.value] ??
    -1,
);
const installedKind = computed(() =>
  snapshot.value?.state.installed_release_id &&
  plan.value?.target_release_id !== snapshot.value.state.installed_release_id
    ? '本地发布 ID'
    : '本地版本',
);
const installedLabel = computed(() => {
  const id = snapshot.value?.state.installed_release_id;
  if (!id) return '尚未读取';
  return plan.value?.target_release_id === id ? plan.value.target_version : `${id.slice(0, 8)}…`;
});
function bytes(value: number | null | undefined) {
  if (value == null) return '大小未知';
  if (value < 1024) return `${value} B`;
  const i = Math.min(2, Math.floor(Math.log(value) / Math.log(1024)) - 1);
  return `${(value / 1024 ** (i + 1)).toFixed(1)} ${['KiB', 'MiB', 'GiB'][i]}`;
}
function filename(value?: string) {
  return value?.split('/').pop() || '—';
}
function openConfirmation() {
  if (!busy.value && phase.value === 'available') confirmation.value?.showModal();
}
async function startUpdate() {
  confirmation.value?.close();
  await action('start_update');
}
async function launch() {
  launchMessage.value = '';
  await action('launch_game');
  if (!error.value) launchMessage.value = '已请求打开启动器';
}
function cancel() {
  void action('cancel_update', { task_id: status.value?.task_id || '' });
}
watch(phase, () => {
  launchMessage.value = '';
  if (phase.value !== 'available') confirmation.value?.close();
});
</script>

<template>
  <div class="app-shell">
    <header class="app-navigation">
      <div class="brand" aria-label="Packy">
        <img :src="branding.logo" alt="" />
        <span>Packy</span>
      </div>
      <nav aria-label="主导航">
        <button :aria-current="page === 'update' ? 'page' : undefined" @click="page = 'update'">
          更新
        </button>
        <button :aria-current="page === 'settings' ? 'page' : undefined" @click="page = 'settings'">
          配置与说明
        </button>
      </nav>
      <button
        class="icon-button theme-toggle"
        :aria-label="theme === 'light' ? '切换深色主题' : '切换浅色主题'"
        @click="toggleTheme"
      >
        <Sun v-if="theme === 'light'" :size="20" />
        <Moon v-else :size="20" />
      </button>
    </header>

    <main ref="workspace" class="workspace">
      <section class="pack-header" :class="{ compact: page === 'settings' }" aria-label="整合包">
        <div class="landscape" aria-hidden="true">
          <img :src="branding.light" class="landscape-light" alt="" />
          <img :src="branding.dark" class="landscape-dark" alt="" />
        </div>
        <div class="pack-heading">
          <h1>{{ packName }}</h1>
          <div class="version-line">
            <span>{{ installedKind }}</span>
            <strong :title="snapshot?.state.installed_release_id">{{ installedLabel }}</strong>
            <template
              v-if="
                plan &&
                !completed &&
                plan.target_release_id !== snapshot?.state.installed_release_id
              "
            >
              <ArrowRight :size="18" />
              <span>目标</span>
              <strong>{{ plan.target_version }}</strong>
            </template>
          </div>
          <p
            v-if="page === 'update'"
            class="status-label"
            :class="{ warning: failed || phase === 'cancelled' }"
          >
            <TriangleAlert v-if="failed" :size="16" />
            <CheckCircle2 v-else-if="completed || phase === 'ready'" :size="16" />
            <RefreshCw v-else :size="16" :class="{ spinning: checking }" />
            {{ title }}
          </p>
        </div>
      </section>

      <template v-if="page === 'update'">
        <p v-if="!desktop" class="notice">
          <Info :size="18" />
          浏览器仅用于界面预览，更新功能请在桌面应用中使用
        </p>
        <section v-if="failed" class="error-panel" role="alert">
          <TriangleAlert :size="24" />
          <div>
            <h2>{{ retryable ? '更新未完成' : '暂时无法继续' }}</h2>
            <p>{{ errorMessage }}</p>
            <p v-if="retryable" class="secondary">已保留原更新任务，重试将继续使用原清单</p>
            <p class="secondary">反馈问题时，请附上 Packy/logs/client.log</p>
          </div>
        </section>
        <section v-if="checking || initializing" class="waiting panel" aria-live="polite">
          <RefreshCw class="spinning" :size="28" />
          <h2>{{ title }}</h2>
          <p class="secondary">
            {{ initializing ? '正在读取整合包信息与任务状态' : '正在获取更新清单，请稍候' }}
          </p>
        </section>

        <section
          v-else-if="transferring || critical || completed"
          class="progress-panel panel"
          aria-label="更新进度"
        >
          <ol class="steps">
            <li
              v-for="(name, i) in ['下载', '校验', '安装', '完成']"
              :key="name"
              :class="{ done: i < step || completed, current: i === step }"
            >
              <span>
                <Check v-if="i < step || completed" :size="16" />
                <template v-else>{{ i + 1 }}</template>
              </span>
              {{ name }}
            </li>
          </ol>
          <div v-if="completed" class="completion">
            <CheckCircle2 :size="36" />
            <div>
              <h2>更新完成</h2>
              <p>版本已保存，可以打开启动器</p>
            </div>
          </div>
          <template v-else>
            <div class="progress-description">
              <div>
                <h2>{{ critical ? '正在写入文件，请勿关闭程序' : title }}</h2>
                <p class="secondary">
                  {{
                    critical
                      ? '安装与保存版本阶段不可取消'
                      : `已处理 ${status?.completed_files || 0} / ${status?.total_files || 0} 个文件`
                  }}
                </p>
              </div>
              <LockKeyhole v-if="critical" :size="24" />
              <strong v-else-if="progress !== null && phase === 'downloading'">
                {{ progress }}%
              </strong>
            </div>
            <div
              class="progress-track"
              role="progressbar"
              :aria-label="title"
              :aria-valuenow="
                critical || phase === 'verifying' ? undefined : (progress ?? undefined)
              "
              :aria-valuemin="0"
              :aria-valuemax="100"
            >
              <span
                :class="{ indeterminate: critical || phase === 'verifying' || progress === null }"
                :style="{
                  width:
                    !critical && phase === 'downloading' && progress !== null
                      ? `${progress}%`
                      : undefined,
                }"
              />
            </div>
            <div v-if="phase === 'downloading'" class="transfer-numbers secondary">
              <span>{{ bytes(status?.downloaded_bytes) }} / {{ bytes(status?.total_bytes) }}</span>
              <span>{{ bytes(status?.bytes_per_second || 0) }}/s</span>
            </div>
          </template>
        </section>

        <template v-if="plan && !checking && !initializing">
          <section class="change-summary" aria-label="更新摘要">
            <div>
              <Plus class="add" :size="20" />
              <span>
                新增
                <strong>{{ counts.ADD }}</strong>
              </span>
            </div>
            <div>
              <RefreshCw class="replace" :size="20" />
              <span>
                替换
                <strong>{{ counts.REPLACE }}</strong>
              </span>
            </div>
            <div>
              <Trash2 class="remove" :size="20" />
              <span>
                移除
                <strong>{{ counts.REMOVE }}</strong>
              </span>
            </div>
            <div>
              <ArrowDownToLine :size="20" />
              <span>
                下载大小
                <strong>{{ bytes(downloadSize) }}</strong>
              </span>
            </div>
          </section>
          <section class="changes panel">
            <div class="content-tabs" role="group" aria-label="更新详情">
              <button :aria-pressed="tab === 'notes'" @click="tab = 'notes'">更新内容</button>
              <button :aria-pressed="tab === 'files'" @click="tab = 'files'">
                变更清单
                <span>{{ operations.length }}</span>
              </button>
            </div>
            <div v-if="tab === 'notes'" class="release-notes">
              <FileText :size="20" />
              <div>
                <h2>更新说明</h2>
                <p>{{ plan.changelog || '维护者未填写本次更新说明' }}</p>
              </div>
            </div>
            <ul class="file-list" aria-label="文件变更">
              <li v-for="op in operations" :key="op.mod_id">
                <span class="file-icon"><FileText :size="20" /></span>
                <div class="file-description">
                  <strong>{{ filename(op.target?.path || op.old_path) }}</strong>
                  <div v-if="tab === 'files'" class="file-paths secondary">
                    <span v-if="op.old_path">{{ op.old_path }}</span>
                    <ArrowRight v-if="op.action === 'REPLACE'" :size="14" />
                    <span v-if="op.target">{{ op.target.path }}</span>
                  </div>
                </div>
                <span class="badge" :class="op.action.toLowerCase()">
                  {{ actionNames[op.action] }}
                </span>
              </li>
              <li v-if="!operations.length" class="empty-list">
                当前版本与发布清单一致，无需修改文件
              </li>
            </ul>
          </section>
        </template>
        <section v-else-if="!checking && !initializing && !failed" class="empty-state panel">
          <ShieldCheck :size="32" />
          <h2>{{ phase === 'cancelled' ? '检查已取消' : '准备好，再出发' }}</h2>
          <p class="secondary">检查服务器发布的更新，确认变更后再安装</p>
        </section>

        <footer class="action-bar">
          <span class="safety">
            <ShieldCheck :size="17" />
            仅更新清单内文件
          </span>
          <label class="background-control">
            <span>后台运行</span>
            <input
              type="checkbox"
              role="switch"
              :checked="background"
              :disabled="pending || initializing || !snapshot"
              @change="
                action('set_background_enabled', {
                  enabled: ($event.target as HTMLInputElement).checked,
                })
              "
            />
          </label>
          <div class="actions">
            <button
              v-if="checking || transferring"
              :disabled="pendingCommand === 'cancel_update'"
              @click="cancel"
            >
              {{
                pendingCommand === 'cancel_update'
                  ? '正在停止…'
                  : checking
                    ? '取消检查'
                    : '取消更新'
              }}
            </button>
            <button v-else-if="critical" disabled>
              <LockKeyhole :size="16" />
              {{ title }}
            </button>
            <button
              v-else-if="retryable"
              class="primary"
              :disabled="pending"
              @click="action('retry_update', { task_id: status?.task_id })"
            >
              <RefreshCw :size="17" />
              重试原任务
            </button>
            <template v-else>
              <button
                :class="{ primary: !snapshot?.can_launch && phase !== 'available' }"
                :disabled="busy || !snapshot"
                @click="action('check_update')"
              >
                <RefreshCw :size="17" />
                {{ plan ? '重新检查' : '检查更新' }}
              </button>
              <button
                v-if="phase === 'available'"
                class="primary"
                :disabled="busy"
                @click="openConfirmation"
              >
                <ArrowDownToLine :size="17" />
                查看并安装
              </button>
              <button
                v-else-if="snapshot?.can_launch"
                class="primary"
                :disabled="busy"
                @click="launch"
              >
                打开启动器
                <ArrowRight :size="17" />
              </button>
            </template>
          </div>
          <p v-if="launchMessage" class="launch-message" role="status">{{ launchMessage }}</p>
        </footer>
      </template>

      <section v-else class="settings-grid">
        <div class="panel preferences">
          <h2>外观</h2>
          <p class="secondary">选择应用界面主题</p>
          <div class="theme-options" role="group" aria-label="外观">
            <button :aria-pressed="theme === 'light'" @click="setTheme('light')">
              <Sun :size="18" />
              浅色
            </button>
            <button :aria-pressed="theme === 'dark'" @click="setTheme('dark')">
              <Moon :size="18" />
              深色
            </button>
          </div>
          <div class="setting-divider" />
          <label class="background-setting">
            <span>
              <strong>后台运行</strong>
              <span class="secondary">关闭窗口后隐藏到托盘</span>
            </span>
            <input
              type="checkbox"
              role="switch"
              :checked="background"
              :disabled="pending || !snapshot"
              @change="
                action('set_background_enabled', {
                  enabled: ($event.target as HTMLInputElement).checked,
                })
              "
            />
          </label>
          <p class="secondary">安装与保存版本时，关闭窗口不会中断更新</p>
        </div>
        <div class="panel pack-information">
          <h2>整合包信息</h2>
          <p class="secondary">由整合包维护者配置，客户端不可修改</p>
          <dl>
            <div
              v-for="(value, key) in {
                名称: packName,
                游戏目录: snapshot?.config.game_dir,
                启动器: snapshot?.config.launcher_path,
                服务地址: snapshot?.config.server_url,
              }"
              :key="key"
            >
              <dt>
                <LockKeyhole :size="14" />
                {{ key }}
              </dt>
              <dd>{{ value || '尚未读取' }}</dd>
            </div>
          </dl>
          <p class="secondary resource-note">维护者可替换更新器图标和明暗背景，重启后生效</p>
        </div>
        <DownloadSettingsPanel v-if="snapshot?.download_defaults" :snapshot="snapshot" />
        <p class="secondary app-version">Packy 0.2.0 · 蒹葭服务器整合包更新器</p>
      </section>
    </main>

    <dialog
      ref="confirmation"
      aria-labelledby="confirm-title"
      @click="$event.target === confirmation && confirmation?.close()"
    >
      <div class="dialog-heading">
        <h2 id="confirm-title">确认更新</h2>
        <button class="icon-button" aria-label="关闭确认框" @click="confirmation?.close()">
          <X :size="20" />
        </button>
      </div>
      <p class="dialog-version">
        <span :title="snapshot?.state.installed_release_id">
          {{ installedKind }} {{ installedLabel }}
        </span>
        <ArrowRight :size="18" />
        {{ plan?.target_version }}
      </p>
      <p class="secondary">将执行以下变更：</p>
      <ul class="confirmation-list">
        <li v-for="op in operations" :key="op.mod_id">
          <span class="badge" :class="op.action.toLowerCase()">{{ actionNames[op.action] }}</span>
          <div>
            <span v-if="op.old_path">{{ op.old_path }}</span>
            <ArrowRight v-if="op.action === 'REPLACE'" :size="16" />
            <span v-if="op.target">{{ op.target.path }}</span>
          </div>
        </li>
      </ul>
      <p class="safety">
        <ShieldCheck :size="18" />
        清单外文件保持不变
      </p>
      <div class="dialog-actions">
        <button autofocus @click="confirmation?.close()">取消</button>
        <button class="primary" :disabled="busy || phase !== 'available'" @click="startUpdate">
          开始更新
        </button>
      </div>
    </dialog>
  </div>
</template>
