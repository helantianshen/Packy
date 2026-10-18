<script setup lang="ts">
defineProps<{ busy: boolean; label?: string }>();
</script>

<template>
  <!-- 保留反馈行高度，快速请求和状态切换不会推挤下方内容 -->
  <div class="loading-feedback" :aria-busy="busy">
    <Transition name="feedback">
      <span v-if="busy" class="loading-message" role="status">
        <span class="loading-spinner" aria-hidden="true" />
        {{ label || '正在加载，请稍候…' }}
      </span>
    </Transition>
  </div>
</template>
