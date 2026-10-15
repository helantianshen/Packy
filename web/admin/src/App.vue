<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import {
  Box,
  Layers,
  Package,
  Compass,
  Download,
  Settings,
  Sun,
  Moon,
  LogOut,
  ShieldCheck,
} from '@lucide/vue';
import '@arco-design/web-vue/dist/arco.css';
import { theme, toggleTheme } from '../../shared/theme';
import { account, api, can, login, logout } from './api';
import { useAction } from './useAction';
import LoadingFeedback from '../../shared/LoadingFeedback.vue';
import PacksView from './views/PacksView.vue';
import ModsView from './views/ModsView.vue';
import ModrinthView from './views/ModrinthView.vue';
import SourcesView from './views/SourcesView.vue';
import SystemView from './views/SystemView.vue';
const { busy, error, success, run } = useAction();
const username = ref(''),
  password = ref(''),
  register = ref(false),
  registrationEnabled = ref(false),
  active = ref('packs');
const views = computed(() =>
  [
    {
      id: 'packs',
      title: '整合包',
      icon: Layers,
      visible: can('pack:read'),
      component: PacksView,
    },
    {
      id: 'mods',
      title: 'Mod 目录',
      icon: Package,
      visible: can('mod:read'),
      component: ModsView,
    },
    {
      id: 'discover',
      title: '发现 Mod',
      icon: Compass,
      visible: can('mod:read'),
      component: ModrinthView,
    },
    {
      id: 'sources',
      title: '下载源',
      icon: Download,
      visible: can('source:read'),
      component: SourcesView,
    },
    {
      id: 'system',
      title: '系统管理',
      icon: Settings,
      visible: can('user:read', true) || can('permission:read', true) || can('settings:read', true),
      component: SystemView,
    },
  ].filter((view) => view.visible),
);
const selected = computed(
  () => views.value.find((view) => view.id === active.value) || views.value[0],
);
async function registrationStatus() {
  registrationEnabled.value = (
    await api<{ registration_enabled: boolean }>('/auth/registration-status')
  ).registration_enabled;
}
onMounted(() => run(registrationStatus));
async function submit() {
  if (register.value) {
    await api('/auth/register', 'POST', {
      username: username.value,
      password: password.value,
    });
    register.value = false;
    password.value = '';
    await registrationStatus();
  } else {
    await login(username.value, password.value);
    password.value = '';
  }
}
</script>
<template>
  <Transition name="view" mode="out-in">
    <div v-if="!account" key="auth" class="auth-shell">
      <div class="auth-story">
        <div class="brand">
          <Box :size="28" />
          Packy
          <span>CONSOLE</span>
        </div>
        <div>
          <div class="eyebrow">YOUR WORLD, WELL MANAGED</div>
          <h1>
            每一次更新，
            <br />
            都有条不紊。
          </h1>
          <p>
            管理 Mod、准备版本、发布整合包。
            <br />
            让玩家只需专注于下一次冒险。
          </p>
        </div>
        <span class="muted">Rust · Tauri · 可靠的文件校验</span>
      </div>
      <div class="auth-side">
        <button class="theme-toggle icon-button" aria-label="切换明暗主题" @click="toggleTheme">
          <Sun v-if="theme === 'dark'" :size="19" />
          <Moon v-else :size="19" />
        </button>
        <form
          :aria-busy="busy"
          class="auth-card"
          @submit.prevent="run(submit, register ? '注册成功，请登录' : '')"
        >
          <span class="pill">PACKY CONSOLE</span>
          <Transition name="feedback" mode="out-in">
            <h2 :key="String(register)">{{ register ? '创建普通账号' : '欢迎回来' }}</h2>
          </Transition>
          <p class="muted">
            {{ register ? '注册后由超管配置业务权限。' : '登录以管理整合包与发布版本。' }}
          </p>
          <label>
            账号
            <input
              v-model="username"
              required
              minlength="3"
              maxlength="32"
              autocomplete="username"
              placeholder="输入账号"
            />
          </label>
          <label>
            密码
            <input
              v-model="password"
              type="password"
              required
              minlength="8"
              :autocomplete="register ? 'new-password' : 'current-password'"
              placeholder="输入密码"
            />
          </label>
          <div v-if="error" class="error" role="alert">{{ error }}</div>
          <div v-if="success" class="success">{{ success }}</div>
          <LoadingFeedback :busy="busy" label="正在连接管理服务…" />
          <button class="primary" :disabled="busy" :aria-busy="busy">
            <span v-if="busy" class="loading-spinner" aria-hidden="true" />
            {{ busy ? '请稍候…' : register ? '注册普通账号' : '登录管理后台' }}
          </button>
          <button
            v-if="registrationEnabled || register"
            type="button"
            class="text-button"
            @click="
              register = !register;
              error = '';
              success = '';
            "
          >
            {{ register ? '返回登录' : '还没有账号？注册' }}
          </button>
          <p v-else class="muted small">当前未开放注册，请联系整合包管理员。</p>
        </form>
      </div>
    </div>
    <div v-else key="workspace" class="admin-shell">
      <aside>
        <div class="brand">
          <span class="logo"><Box :size="23" /></span>
          Packy
          <span>CONSOLE</span>
        </div>
        <div class="eyebrow nav-label">工作空间</div>
        <nav>
          <button
            v-for="view in views"
            :key="view.id"
            :class="{ active: selected?.id === view.id }"
            @click="active = view.id"
          >
            <component :is="view.icon" :size="18" />
            {{ view.title }}
          </button>
        </nav>
        <div class="sidebar-bottom">
          <div class="row">
            <ShieldCheck :size="17" />
            <strong>{{ account.username }}</strong>
          </div>
          <small class="muted">
            {{ account.roles.includes('super_admin') ? '超级管理员' : '普通用户' }}
          </small>
          <div class="row">
            <button class="icon-button" aria-label="切换明暗主题" @click="toggleTheme">
              <Sun v-if="theme === 'dark'" :size="17" />
              <Moon v-else :size="17" />
            </button>
            <button :disabled="busy" @click="run(logout)">
              <LogOut :size="15" />
              退出登录
            </button>
          </div>
        </div>
      </aside>
      <main>
        <header class="row">
          <span class="eyebrow">{{ selected?.title || '工作空间' }} / PACKY</span>
          <span class="spacer" />
          <span class="pill">
            {{ account.roles.includes('super_admin') ? 'SUPER ADMIN' : 'MEMBER' }}
          </span>
        </header>
        <Transition name="view" mode="out-in">
          <div v-if="selected" :key="selected.id" class="view-panel">
            <component :is="selected.component" />
          </div>
        </Transition>
        <section v-if="!selected" class="card empty">
          <h2>账号已登录</h2>
          <p>当前没有管理权限，请联系超管授权。</p>
        </section>
        <div v-if="error" class="error" role="alert">{{ error }}</div>
        <footer class="muted">Packy Console · 管理变更由服务端授权与校验</footer>
      </main>
    </div>
  </Transition>
</template>
<style>
.admin-shell {
  display: grid;
  grid-template-columns: 230px 1fr;
  min-height: 100vh;
}
.admin-shell aside {
  position: sticky;
  top: 0;
  height: 100vh;
  display: flex;
  flex-direction: column;
  padding: 30px 22px;
  background: var(--panel);
  border-right: 1px solid var(--line);
}
.brand {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 24px;
  font-weight: 750;
  letter-spacing: -0.7px;
}
.brand > span:last-child {
  font-size: 8px;
  letter-spacing: 0.1em;
  color: var(--muted);
  align-self: flex-end;
  padding-bottom: 4px;
}
.brand .logo {
  width: 34px;
  height: 34px;
  display: grid;
  place-items: center;
  background: var(--accent);
  color: var(--panel);
  border-radius: 10px;
}
.nav-label {
  margin: 45px 10px 15px;
}
.admin-shell nav {
  display: grid;
  gap: 7px;
}
.admin-shell nav button {
  justify-content: flex-start;
  border: 0;
  background: transparent;
  padding: 13px;
  font-size: 13px;
}
.admin-shell nav button.active {
  background: var(--accent-soft);
  color: var(--accent);
  font-weight: 600;
}
.sidebar-bottom {
  margin-top: auto;
  display: grid;
  gap: 14px;
  font-size: 12px;
}
.admin-shell main {
  padding: 32px 40px;
  min-width: 0;
  max-width: 1500px;
  width: 100%;
  margin: 0 auto;
}
.admin-shell header {
  margin-bottom: 48px;
}
.view-heading {
  margin-bottom: 27px;
}
.view-heading h1 {
  margin: 0 0 10px;
}
.view-heading p {
  font-size: 13px;
  line-height: 1.7;
  margin: 0;
}
.section {
  padding: 25px;
  margin: 20px 0;
}
.section h2 {
  margin-bottom: 16px;
}
.section p.muted {
  font-size: 12px;
  line-height: 1.8;
}
.form-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 18px;
  margin: 16px 0;
}
.form-grid .full {
  grid-column: 1/-1;
}
.inline {
  display: inline-flex;
  align-items: center;
  gap: 7px;
}
.inline input {
  width: auto;
}
.table-scroll {
  overflow-x: auto;
}
table {
  width: 100%;
  border-collapse: collapse;
  text-align: left;
  font-size: 13px;
}
th {
  font-size: 11px;
  color: var(--muted);
  font-weight: 500;
  padding: 12px 10px;
  background: var(--panel-soft);
}
td {
  padding: 14px 10px;
  border-bottom: 1px solid var(--line);
}
td button {
  padding: 7px 11px;
  font-size: 12px;
}
td code {
  font-size: 11px;
}
.pagination {
  margin-top: 20px;
  font-size: 12px;
}
.success {
  color: var(--accent);
  padding: 12px 15px;
  border-radius: 10px;
  background: var(--accent-soft);
  margin: 15px 0;
}
.error {
  margin: 15px 0;
}
footer {
  font-size: 10px;
  margin: 35px 0 0;
}
.auth-shell {
  display: grid;
  grid-template-columns: 1fr 1fr;
  min-height: 100vh;
}
.auth-story {
  padding: 60px;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  background: linear-gradient(145deg, #1d3b31, #10281f);
  color: #ecfff5;
}
.auth-story .eyebrow {
  color: #83bea4;
  margin-bottom: 20px;
}
.auth-story h1 {
  font-size: 48px;
  line-height: 1.35;
  font-weight: 650;
}
.auth-story p {
  font-size: 14px;
  line-height: 2;
  color: #afc7bb;
}
.auth-side {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 35px;
  position: relative;
}
.theme-toggle {
  position: absolute;
  top: 35px;
  right: 35px;
}
.auth-card {
  width: 100%;
  max-width: 340px;
  display: grid;
  gap: 20px;
}
.auth-card h2 {
  font-size: 29px;
  margin: 5px 0 0;
}
.auth-card > p {
  font-size: 12px;
  margin: 0 0 10px;
}
.text-button {
  border: none;
  background: none !important;
  font-size: 12px;
}
.small {
  font-size: 11px;
}
code {
  font-family: 'Cascadia Code', Consolas, monospace;
}
.arco-modal {
  background: var(--panel);
  color: var(--text);
}
.arco-modal-title,
.arco-modal-body {
  color: var(--text);
}
@media (max-width: 1000px) {
  .admin-shell {
    grid-template-columns: 190px 1fr;
  }
  .admin-shell main {
    padding: 25px;
  }
  .auth-story {
    padding: 35px;
  }
  .auth-story h1 {
    font-size: 37px;
  }
  .form-grid {
    grid-template-columns: 1fr;
  }
}
</style>
