<script setup lang="ts">
import { onMounted, onBeforeUnmount } from 'vue'
import type { DialogButton } from '../types'

const props = withDefaults(defineProps<{
  title?: string
  message?: string
  buttons?: DialogButton[]
  cancelKey?: string
  variant?: 'warning' | 'danger' | 'plain'
}>(), {
  title: '',
  message: '',
  buttons: () => [],
  cancelKey: 'cancel',
  variant: 'warning',
})

const emit = defineEmits<{
  action: [key: string]
}>()

function onKey(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    e.preventDefault()
    e.stopPropagation()
    emit('action', props.cancelKey)
  }
}

onMounted(() => {
  window.addEventListener('keydown', onKey, true)
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKey, true)
})
</script>

<template>
  <Teleport to="body">
    <div class="cd-overlay" @contextmenu.prevent @click.self="emit('action', cancelKey)">
      <div class="cd-dialog" role="alertdialog" aria-modal="true">
        <div class="cd-content">
          <span class="cd-icon" :class="variant">
            <svg v-if="variant === 'danger'" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="12" cy="12" r="9.5" />
              <line x1="15" y1="9" x2="9" y2="15" />
              <line x1="9" y1="9" x2="15" y2="15" />
            </svg>
            <svg v-else-if="variant === 'warning'" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M10.29 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z" />
              <line x1="12" y1="9" x2="12" y2="13" />
              <line x1="12" y1="17" x2="12.01" y2="17" />
            </svg>
            <svg v-else width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="12" cy="12" r="9.5" />
              <circle cx="12" cy="7.6" r="1.05" fill="currentColor" stroke="none" />
              <rect x="11" y="10.5" width="2" height="6.6" rx="0.6" fill="currentColor" stroke="none" />
            </svg>
          </span>
          <div class="cd-text">
            <div class="cd-title">{{ title }}</div>
            <div v-if="message" class="cd-message">{{ message }}</div>
          </div>
        </div>
        <div class="cd-actions">
          <button
            v-for="btn in buttons"
            :key="btn.key"
            class="cd-btn"
            :class="{ primary: btn.primary, danger: btn.danger }"
            @click="emit('action', btn.key)"
          >
            {{ btn.label }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.cd-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.32);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10001;
}

.cd-dialog {
  width: 400px;
  max-width: 90vw;
  padding: 20px;
  border-radius: 10px;
  background: var(--bg-color, #ffffff);
  border: 1px solid var(--border-color, #e5e7eb);
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.28);
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.cd-content {
  display: flex;
  align-items: flex-start;
  gap: 12px;
}

.cd-icon {
  display: flex;
  flex-shrink: 0;
  width: 22px;
  height: 22px;
  margin-top: 1px;
}

.cd-icon.warning {
  color: #f59e0b;
}

.cd-icon.danger {
  color: #ef4444;
}

.cd-icon.plain {
  color: var(--accent-color, #3b82f6);
}

.cd-text {
  min-width: 0;
  flex: 1;
}

.cd-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-color, #1a1a1a);
  margin-bottom: 5px;
}

.cd-message {
  font-size: 13px;
  line-height: 1.55;
  color: var(--text-secondary, #6b7280);
  word-break: break-word;
}

.cd-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.cd-btn {
  padding: 6px 15px;
  border: 1px solid var(--border-color, #e5e7eb);
  border-radius: 6px;
  background: transparent;
  color: var(--text-color, #1a1a1a);
  font-size: 13px;
  cursor: pointer;
  transition: background 0.15s, border-color 0.15s, filter 0.15s;
  min-width: 72px;
}

.cd-btn:hover {
  background: var(--btn-hover-bg, #f3f4f6);
}

.cd-btn.primary {
  background: var(--accent-color, #3b82f6);
  border-color: var(--accent-color, #3b82f6);
  color: #ffffff;
  font-weight: 500;
}

.cd-btn.primary:hover {
  filter: brightness(1.08);
}

.cd-btn.danger {
  background: #ef4444;
  border-color: #ef4444;
  color: #ffffff;
  font-weight: 500;
}

.cd-btn.danger:hover {
  filter: brightness(1.08);
}
</style>
