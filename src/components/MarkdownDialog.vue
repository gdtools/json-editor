<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import MarkdownIt from 'markdown-it'
import { t } from '../i18n'

const props = withDefaults(defineProps<{
  visible?: boolean
  content?: string
  theme?: string
}>(), {
  visible: false,
  content: '',
  theme: 'light',
})

const emit = defineEmits<{
  'update:visible': [value: boolean]
}>()

/**
 * html: false  -> raw HTML in the JSON value is escaped (content is untrusted).
 * linkify: true -> bare URLs become links.
 * breaks: true  -> single newlines become <br>, which suits Windows-style values.
 */
const md = new MarkdownIt({
  html: false,
  linkify: true,
  breaks: true,
  typographer: false,
})

/** JSON string values commonly use Windows CRLF ("\r\n"); normalize to LF. */
const normalized = computed(() => (props.content ?? '').replace(/\r\n|\r/g, '\n'))

const renderedHtml = computed(() => md.render(normalized.value))

const charCount = computed(() => (props.content ?? '').length)

const copied = ref(false)

function close() {
  emit('update:visible', false)
}

async function copyRaw() {
  const text = normalized.value
  try {
    await navigator.clipboard.writeText(text)
  } catch {
    // Fallback when async clipboard access is unavailable
    const ta = document.createElement('textarea')
    ta.value = text
    ta.style.position = 'fixed'
    ta.style.opacity = '0'
    document.body.appendChild(ta)
    ta.select()
    document.execCommand('copy')
    document.body.removeChild(ta)
  }
  copied.value = true
  window.setTimeout(() => {
    copied.value = false
  }, 1200)
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') close()
}

watch(() => props.visible, (v) => {
  if (v) {
    copied.value = false
    window.addEventListener('keydown', onKeydown)
  } else {
    window.removeEventListener('keydown', onKeydown)
  }
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKeydown)
})
</script>

<template>
  <Teleport to="body">
    <div v-if="visible" class="md-overlay" @click.self="close">
      <div class="md-dialog" :class="{ 'md-dark': theme === 'dark' }">
        <header class="md-header">
          <span class="md-title">{{ t('markdown.title') }}</span>
          <button class="md-close" :title="t('markdown.close')" @click="close">×</button>
        </header>
        <div class="md-body" v-html="renderedHtml" />
        <footer class="md-footer">
          <span class="md-meta">{{ charCount }} chars</span>
          <button class="md-btn" @click="copyRaw">
            {{ copied ? t('toast.copied') : t('markdown.copyRaw') }}
          </button>
        </footer>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.md-overlay {
  position: fixed;
  inset: 0;
  z-index: 10000;
  background: rgba(0, 0, 0, 0.45);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 40px;
}

.md-dialog {
  display: flex;
  flex-direction: column;
  width: min(920px, 100%);
  max-height: 100%;
  background: #ffffff;
  color: #1a1a1a;
  border-radius: 8px;
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.25);
  overflow: hidden;
}

.md-dialog.md-dark {
  background: #252526;
  color: #d4d4d4;
}

.md-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 14px;
  border-bottom: 1px solid #e5e7eb;
}

.md-dark .md-header {
  border-bottom-color: #3c3c3c;
}

.md-title {
  font-size: 13px;
  font-weight: 600;
}

.md-close {
  border: none;
  background: transparent;
  color: inherit;
  font-size: 20px;
  line-height: 1;
  padding: 0 4px;
  cursor: pointer;
  opacity: 0.7;
}

.md-close:hover {
  opacity: 1;
}

.md-body {
  flex: 1;
  overflow: auto;
  padding: 16px 20px;
  font-size: 14px;
  line-height: 1.7;
}

.md-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 14px;
  border-top: 1px solid #e5e7eb;
}

.md-dark .md-footer {
  border-top-color: #3c3c3c;
}

.md-meta {
  font-size: 11px;
  opacity: 0.6;
}

.md-btn {
  border: 1px solid #d1d5db;
  background: #f9fafb;
  color: inherit;
  border-radius: 4px;
  padding: 4px 10px;
  font-size: 12px;
  cursor: pointer;
}

.md-btn:hover {
  background: #f3f4f6;
}

.md-dark .md-btn {
  background: #3d3d3d;
  border-color: #4f4f4f;
}

.md-dark .md-btn:hover {
  background: #4b4b4b;
}

/* ---------------------------------------------------------------------------
   Markdown typography.
   Content is injected via v-html, so scoped styles need :deep() to apply.
   --------------------------------------------------------------------------- */
.md-body :deep(> *:first-child) {
  margin-top: 0;
}

.md-body :deep(> *:last-child) {
  margin-bottom: 0;
}

.md-body :deep(h1),
.md-body :deep(h2),
.md-body :deep(h3),
.md-body :deep(h4),
.md-body :deep(h5),
.md-body :deep(h6) {
  margin: 20px 0 10px;
  line-height: 1.35;
  font-weight: 600;
}

.md-body :deep(h1) {
  font-size: 22px;
  padding-bottom: 6px;
  border-bottom: 1px solid #e5e7eb;
}

.md-body :deep(h2) {
  font-size: 18px;
  padding-bottom: 5px;
  border-bottom: 1px solid #e5e7eb;
}

.md-body :deep(h3) {
  font-size: 16px;
}

.md-body :deep(h4) {
  font-size: 14px;
}

.md-body :deep(h5),
.md-body :deep(h6) {
  font-size: 13px;
}

.md-dark :deep(h1),
.md-dark :deep(h2) {
  border-bottom-color: #3c3c3c;
}

.md-body :deep(p) {
  margin: 0 0 12px;
}

.md-body :deep(ul),
.md-body :deep(ol) {
  margin: 0 0 12px;
  padding-left: 24px;
}

.md-body :deep(li) {
  margin: 4px 0;
}

.md-body :deep(li > ul),
.md-body :deep(li > ol) {
  margin: 4px 0;
}

.md-body :deep(blockquote) {
  margin: 0 0 12px;
  padding: 4px 14px;
  border-left: 3px solid #d1d5db;
  color: #6b7280;
}

.md-dark :deep(blockquote) {
  border-left-color: #4f4f4f;
  color: #9ca3af;
}

.md-body :deep(pre) {
  margin: 0 0 12px;
  padding: 12px 14px;
  overflow: auto;
  background: #f6f8fa;
  border-radius: 6px;
}

.md-body :deep(code) {
  font-family: consolas, menlo, monaco, monospace;
  font-size: 12.5px;
}

.md-body :deep(:not(pre) > code) {
  padding: 2px 5px;
  background: rgba(0, 0, 0, 0.06);
  border-radius: 4px;
}

.md-dark :deep(pre) {
  background: #1b1b1b;
}

.md-dark :deep(:not(pre) > code) {
  background: rgba(255, 255, 255, 0.12);
}

.md-body :deep(a) {
  color: #2563eb;
  text-decoration: none;
}

.md-body :deep(a:hover) {
  text-decoration: underline;
}

.md-dark :deep(a) {
  color: #58a6ff;
}

.md-body :deep(table) {
  margin: 0 0 12px;
  border-collapse: collapse;
}

.md-body :deep(th),
.md-body :deep(td) {
  border: 1px solid #d1d5db;
  padding: 6px 10px;
  text-align: left;
}

.md-body :deep(th) {
  background: rgba(0, 0, 0, 0.04);
  font-weight: 600;
}

.md-dark :deep(th),
.md-dark :deep(td) {
  border-color: #4f4f4f;
}

.md-dark :deep(th) {
  background: rgba(255, 255, 255, 0.08);
}

.md-body :deep(hr) {
  margin: 18px 0;
  border: none;
  border-top: 1px solid #e5e7eb;
}

.md-dark :deep(hr) {
  border-top-color: #3c3c3c;
}

.md-body :deep(img) {
  max-width: 100%;
}
</style>
