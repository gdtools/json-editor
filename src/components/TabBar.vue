<script setup lang="ts">
interface Tab {
  id: string
  path: string
  name: string
  dirty?: boolean
}

defineProps<{
  tabs: Tab[]
  activeId: string | null
}>()

const emit = defineEmits<{
  select: [id: string]
  close: [id: string]
}>()
</script>

<template>
  <div class="tab-bar">
    <button
      v-for="tab in tabs"
      :key="tab.id"
      class="tab-item"
      :class="{ active: tab.id === activeId }"
      @click="emit('select', tab.id)"
    >
      <span class="tab-name" :title="tab.path || tab.name">
        {{ tab.name }}
        <span v-if="tab.dirty" class="tab-dirty"> (*)</span>
      </span>
      <span class="tab-close" @click.stop="emit('close', tab.id)">×</span>
    </button>
  </div>
</template>

<style scoped>
.tab-bar {
  display: flex;
  align-items: stretch;
  height: 36px;
  background: var(--toolbar-bg);
  border-bottom: 1px solid var(--border-color);
  flex-shrink: 0;
  overflow-x: auto;
  overflow-y: hidden;
}

.tab-item {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 0 12px;
  border: none;
  border-right: 1px solid var(--border-color);
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  font-size: 12px;
  white-space: nowrap;
  transition: background 0.15s;
  flex-shrink: 0;
}

.tab-item:hover {
  background: var(--btn-hover-bg);
  color: var(--text-color);
}

.tab-item.active {
  background: var(--bg-color);
  color: var(--text-color);
  font-weight: 500;
  position: relative;
}

.tab-item.active::after {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 2px;
  background: var(--accent-color, #3b82f6);
}

.tab-name {
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 140px;
}

.tab-close {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  border-radius: 4px;
  font-size: 14px;
  line-height: 1;
  color: var(--text-secondary);
  transition: all 0.15s;
}

.tab-dirty {
  color: #ef4444;
  font-weight: 600;
}

:root[data-theme="dark"] .tab-dirty {
  color: #f87171;
}
</style>
