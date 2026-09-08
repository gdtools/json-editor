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
    <div
      v-for="tab in tabs"
      :key="tab.id"
      class="tab-item"
      :class="{ active: tab.id === activeId }"
      :title="tab.path || tab.name"
      @mousedown.left.stop="emit('select', tab.id)"
      @mousedown.middle.prevent.stop="emit('close', tab.id)"
      @auxclick.prevent.stop
    >
      <span class="tab-name">{{ tab.name }}</span>
      <span v-if="tab.dirty" class="tab-dirty">•</span>
      <button
        class="tab-close"
        :aria-label="'Close ' + tab.name"
        @mousedown.prevent.stop
        @click.stop="emit('close', tab.id)"
      >
        <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
          <line x1="18" y1="6" x2="6" y2="18" />
          <line x1="6" y1="6" x2="18" y2="18" />
        </svg>
      </button>
    </div>
  </div>
</template>

<style scoped>
.tab-bar {
  display: flex;
  align-items: stretch;
  height: 34px;
  background: var(--toolbar-bg);
  border-bottom: 1px solid var(--border-color);
  flex-shrink: 0;
  overflow-x: auto;
  overflow-y: hidden;
  scrollbar-width: none;
}

.tab-bar::-webkit-scrollbar {
  height: 0;
}

.tab-item {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 0 8px 0 12px;
  border-right: 1px solid var(--border-color);
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  font-size: 12px;
  white-space: nowrap;
  transition: background 0.12s, color 0.12s;
  flex-shrink: 0;
  max-width: 200px;
  user-select: none;
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
  white-space: nowrap;
}

.tab-dirty {
  color: #ef4444;
  font-weight: 700;
  line-height: 1;
  flex-shrink: 0;
}

.tab-close {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  padding: 0;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  flex-shrink: 0;
  opacity: 0.55;
  transition: opacity 0.12s, background 0.12s;
}

.tab-item:hover .tab-close,
.tab-item.active .tab-close {
  opacity: 1;
}

.tab-close:hover {
  background: var(--btn-active-bg);
  color: var(--text-color);
}

:root[data-theme="dark"] .tab-dirty {
  color: #f87171;
}
</style>
