<script setup lang="ts">
import LoadingFeedback from '../../../shared/LoadingFeedback.vue';
import { onMounted, ref } from 'vue';
import { account, all, api, can } from '../api';
import { useAction } from '../useAction';
import type { SystemSettings } from '../../../shared/generated/SystemSettings';
import type { Account } from '../../../shared/generated/Account';
import type { Role } from '../../../shared/generated/Role';
import type { Permission } from '../../../shared/generated/Permission';
const { busy, error, success, run } = useAction();
const registration = ref(false),
  users = ref<Account[]>([]),
  roles = ref<Role[]>([]),
  permissions = ref<Permission[]>([]);
async function load() {
  if (can('settings:read', true))
    registration.value = (await api<SystemSettings>('/admin/system/settings')).registration_enabled;
  if (can('user:read', true)) users.value = await all<Account>('/admin/system/users');
  if (can('permission:read', true))
    [roles.value, permissions.value] = await Promise.all([
      api<Role[]>('/admin/system/roles'),
      api<Permission[]>('/admin/system/permissions'),
    ]);
}
onMounted(() => run(load));
async function saveUser(user: Account) {
  await api(`/admin/system/users/${user.id}`, 'PATCH', {
    enabled: user.enabled,
    roles: user.roles,
  });
  await load();
}
async function saveRole(role: Role) {
  await api(`/admin/system/roles/${role.code}/permissions`, 'PUT', {
    permissions: role.permissions,
  });
  account.value = await api<Account>('/auth/me');
  await load();
}
</script>
<template>
  <div class="view-heading">
    <h1>系统管理</h1>
    <p class="muted">管理访问权限与注册策略，保存后立即生效。</p>
  </div>
  <LoadingFeedback :busy="busy" />
  <div v-if="error" class="error" role="alert">{{ error }}</div>
  <div v-if="success" class="success" role="status">{{ success }}</div>
  <section v-if="can('settings:read', true)" class="card section">
    <div class="row">
      <div>
        <h2>开放注册</h2>
        <p class="muted">注册账号固定为普通用户，默认没有管理权限。</p>
      </div>
      <span class="spacer" />
      <label class="inline">
        <input type="checkbox" v-model="registration" />
        允许注册
      </label>
      <button
        v-if="can('settings:write', true)"
        :disabled="busy"
        @click="
          run(async () => {
            await api('/admin/system/settings', 'PUT', {
              registration_enabled: registration,
            });
          }, '注册设置已保存')
        "
      >
        保存设置
      </button>
    </div>
  </section>
  <section v-if="can('user:read', true)" class="card section">
    <h2>用户管理</h2>
    <div class="table-scroll">
      <table>
        <thead>
          <tr>
            <th>账号</th>
            <th>角色</th>
            <th>状态</th>
            <th>操作</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="user in users" :key="user.id">
            <td>
              {{ user.username }}
              <span v-if="user.id === account?.id" class="pill">当前账号</span>
            </td>
            <td>
              <label v-for="role in ['user', 'super_admin']" :key="role" class="inline">
                <input type="checkbox" :value="role" v-model="user.roles" />
                {{ role === 'user' ? '普通用户' : '超管' }}
              </label>
            </td>
            <td>
              <label class="inline">
                <input type="checkbox" v-model="user.enabled" />
                启用
              </label>
            </td>
            <td>
              <button
                v-if="can('user:write', true)"
                :disabled="busy"
                @click="run(() => saveUser(user), '账号已保存')"
              >
                保存
              </button>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
  <section v-if="can('permission:read', true)" class="card section">
    <h2>角色权限</h2>
    <p class="muted">系统用户、权限和设置管理始终要求超管角色；至少保留一位启用超管。</p>
    <div class="role-grid">
      <div v-for="role in roles" :key="role.code">
        <h3>{{ role.name }}</h3>
        <div class="permission-list">
          <label v-for="permission in permissions" :key="permission.code" class="inline">
            <input type="checkbox" :value="permission.code" v-model="role.permissions" />
            <span>
              {{ permission.name }}
              <small class="muted">{{ permission.code }}</small>
            </span>
          </label>
        </div>
        <button
          v-if="can('permission:write', true)"
          :disabled="busy"
          @click="run(() => saveRole(role), '角色权限已保存')"
        >
          保存{{ role.name }}权限
        </button>
      </div>
    </div>
  </section>
</template>
<style scoped>
.role-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 40px;
}
.permission-list {
  display: grid;
  gap: 12px;
  margin: 20px 0;
}
small {
  display: block;
  font-size: 10px;
}
.pill {
  margin-left: 10px;
}
</style>
