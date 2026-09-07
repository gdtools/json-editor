<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount, watch } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { readTextFile } from '@tauri-apps/plugin-fs'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import JsonEditorPanel from './components/JsonEditorPanel.vue'
import Toolbar from './components/Toolbar.vue'
import TabBar from './components/TabBar.vue'
import OpenUrlModal from './components/OpenUrlModal.vue'
import type { EditorMode, ThemeMode } from './types'
import { t } from './i18n'
import { openJsonFile, saveJsonFile, writeJsonFile } from './utils/file'
import { formatJson, compactJson, validateJson, countJsonNodes, tryParseJson } from './utils/json'
import { usePersistedState } from './composables/usePersistedState'

const SAMPLE_JSON = `{\n  "array": [1, 2, 3],\n  "boolean": true,\n  "color": "gold",\n  "null": null,\n  "number": 123,\n  "object": {\n    "a": "b",\n    "c": "d"\n  },\n  "string": "Hello World"\n}`

// ---------------------------------------------------------------------------
// Tabs
// ---------------------------------------------------------------------------
interface EditorTab {
  id: string
  path: string
  name: string
  content: string
  mode: EditorMode
  dirty: boolean
}

const leftTabs = ref<EditorTab[]>([])
const activeLeftTabId = ref<string | null>(null)

// 右侧草稿（不关联文件）
const rightDraft = usePersistedState('rightDraft', SAMPLE_JSON)
const rightMode = usePersistedState<EditorMode>('rightMode', 'tree')

const theme = usePersistedState<ThemeMode>('theme', 'light')

const leftEditorRef = ref<InstanceType<typeof JsonEditorPanel>>()
const rightEditorRef = ref<InstanceType<typeof JsonEditorPanel>>()
const showOpenUrlModal = ref(false)
const leftSelectionType = ref<'array' | 'object' | 'none'>('none')
const rightSelectionType = ref<'array' | 'object' | 'none'>('none')

const unlistenFns: (() => void)[] = []

// ---------------------------------------------------------------------------
// Active tab helpers
// ---------------------------------------------------------------------------
const activeTab = computed(() => leftTabs.value.find(t => t.id === activeLeftTabId.value) ?? null)

const leftContent = computed({
  get: () => activeTab.value?.content ?? SAMPLE_JSON,
  set: (v: string) => {
    if (activeTab.value) {
      activeTab.value.content = v
      activeTab.value.dirty = true
    }
  }
})

const leftMode = computed({
  get: () => activeTab.value?.mode ?? 'tree',
  set: (m: EditorMode) => {
    if (activeTab.value) {
      activeTab.value.mode = m
    }
  }
})

const fileName = computed(() => activeTab.value?.name ?? 'untitled.json')

// ---------------------------------------------------------------------------
// Recent files
// ---------------------------------------------------------------------------
const recentFiles = ref<Array<{ name: string; path: string }>>([])
const RECENT_KEY = 'json-editor-recent-files'
const MAX_RECENT = 10

function loadRecentFiles() {
  try {
    const raw = localStorage.getItem(RECENT_KEY)
    if (raw) recentFiles.value = JSON.parse(raw)
  } catch {
    recentFiles.value = []
  }
}

function pushRecentFile(path: string) {
  const name = path.split(/[\\/]/).pop() || path
  const list = recentFiles.value.filter(f => f.path !== path)
  list.unshift({ name, path })
  if (list.length > MAX_RECENT) list.length = MAX_RECENT
  recentFiles.value = list
  try {
    localStorage.setItem(RECENT_KEY, JSON.stringify(list))
  } catch {
    // ignore quota errors
  }
}

loadRecentFiles()

// ---------------------------------------------------------------------------
// Folder browser
// ---------------------------------------------------------------------------
const currentDir = ref('')
const dirFiles = ref<string[]>([])
const dirLoading = ref(false)

function getFileDir(filePath: string): string {
  const lastSep = Math.max(filePath.lastIndexOf('/'), filePath.lastIndexOf('\\'))
  if (lastSep <= 0) return ''
  let dir = filePath.substring(0, lastSep)
  if (/^[A-Za-z]:$/.test(dir)) {
    dir = dir + '\\'
  }
  return dir
}

async function loadDirFiles(dirPath: string) {
  if (!dirPath) return
  currentDir.value = dirPath
  dirLoading.value = true
  try {
    await invoke('allow_directory', { path: dirPath })
    const { listJsonFiles } = await import('./utils/file')
    const files = await listJsonFiles(dirPath)
    dirFiles.value = files
  } catch (e) {
    console.error('Failed to load directory:', e)
    dirFiles.value = []
  } finally {
    dirLoading.value = false
  }
}

async function openDirForFile(filePath: string) {
  const dir = getFileDir(filePath)
  if (dir) {
    await loadDirFiles(dir)
  } else {
    currentDir.value = ''
    dirFiles.value = []
  }
}

async function refreshCurrentDir() {
  if (!currentDir.value) return
  await loadDirFiles(currentDir.value)
}

async function handleOpenFileFromFolder(fName: string) {
  if (!currentDir.value) return
  const fullPath = currentDir.value.replace(/[\\/]$/, '') + '/' + fName
  await openFileInTab(fullPath)
}

// ---------------------------------------------------------------------------
// Tab operations
// ---------------------------------------------------------------------------
function generateId(): string {
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 8)
}

async function openFileInTab(path: string, content?: string) {
  // 检查是否已经打开
  const existing = leftTabs.value.find(t => t.path === path)
  if (existing) {
    activeLeftTabId.value = existing.id
    return
  }
  let text = content
  if (text === undefined) {
    try {
      text = await readTextFile(path)
    } catch (e) {
      console.error('Failed to read file:', e)
      alert('无法打开文件：' + path)
      return
    }
  }
  const name = path.split(/[\\/]/).pop() || 'untitled.json'
  const tab: EditorTab = {
    id: generateId(),
    path,
    name,
    content: text,
    mode: 'tree',
    dirty: false,
  }
  leftTabs.value.push(tab)
  activeLeftTabId.value = tab.id
  pushRecentFile(path)
  await openDirForFile(path)
}

function closeTab(id: string) {
  const idx = leftTabs.value.findIndex(t => t.id === id)
  if (idx === -1) return
  leftTabs.value.splice(idx, 1)
  // 如果关闭的是当前激活的 tab，切换到相邻 tab
  if (activeLeftTabId.value === id) {
    if (leftTabs.value.length === 0) {
      activeLeftTabId.value = null
    } else {
      const newIdx = Math.min(idx, leftTabs.value.length - 1)
      activeLeftTabId.value = leftTabs.value[newIdx].id
    }
  }
}

function switchTab(id: string) {
  activeLeftTabId.value = id
}

// ---------------------------------------------------------------------------
// Toolbar handlers
// ---------------------------------------------------------------------------
async function handleNew() {
  // 新建一个未保存的 tab（无路径）
  const tab: EditorTab = {
    id: generateId(),
    path: '',
    name: 'untitled.json',
    content: '{}',
    mode: 'tree',
    dirty: true,
  }
  leftTabs.value.push(tab)
  activeLeftTabId.value = tab.id
}

async function handleOpen() {
  const result = await openJsonFile()
  if (result) {
    await invoke('allow_file', { path: result.path })
    await openFileInTab(result.path, result.content)
  }
}

async function handleOpenRecent(path: string) {
  await invoke('allow_file', { path })
  await openFileInTab(path)
}

async function handleSave() {
  if (!activeTab.value) return
  const tab = activeTab.value
  // 已有路径：直接保存，不弹窗
  if (tab.path) {
    try {
      await writeJsonFile(tab.path, tab.content)
      tab.dirty = false
      pushRecentFile(tab.path)
      await openDirForFile(tab.path)
      showToast(t('toast.saved'))
    } catch (e) {
      console.error('Failed to save file:', e)
      showToast(t('toast.saveFailed') + '：' + (e instanceof Error ? e.message : String(e)))
    }
    return
  }
  // 无路径：弹出另存为
  const path = await saveJsonFile(tab.content, tab.name)
  if (path) {
    tab.path = path
    tab.name = path.split(/[\\/]/).pop() || tab.name
    tab.dirty = false
    pushRecentFile(path)
    await openDirForFile(path)
    showToast(t('toast.saved'))
  }
}

async function handleSaveAs() {
  if (!activeTab.value) return
  const tab = activeTab.value
  const defaultName = tab.name
  const defaultDir = currentDir.value || (tab.path ? getFileDir(tab.path) : undefined)
  const path = await saveJsonFile(tab.content, defaultName, defaultDir)
  if (path) {
    tab.path = path
    tab.name = path.split(/[\\/]/).pop() || tab.name
    tab.dirty = false
    pushRecentFile(path)
    await openDirForFile(path)
  }
}

async function handleOpenUrl() {
  showOpenUrlModal.value = true
}

function handleUrlLoaded(content: string, name: string) {
  // URL 加载也作为 tab 打开（无路径）
  const tab: EditorTab = {
    id: generateId(),
    path: '',
    name,
    content,
    mode: 'tree',
    dirty: true,
  }
  leftTabs.value.push(tab)
  activeLeftTabId.value = tab.id
}

async function handleCopy() {
  try {
    await navigator.clipboard.writeText(leftContent.value)
  } catch (e) {
    console.error('Failed to copy:', e)
  }
}

function handleFormat() {
  try {
    leftContent.value = formatJson(leftContent.value)
  } catch (e) {
    console.error('Failed to format:', e)
  }
}

function handleCompact() {
  try {
    leftContent.value = compactJson(leftContent.value)
  } catch (e) {
    console.error('Failed to compact:', e)
  }
}

function handleRepair() {
  let text = leftContent.value
  text = text.replace(/,\s*([}\]])/g, '$1')
  text = text.replace(/'/g, '"')
  const result = tryParseJson(text)
  if (result.success) {
    leftContent.value = JSON.stringify(result.data, null, 2)
  } else {
    console.error('Failed to repair:', result.error)
  }
}

function handleExpandAll() {
  leftEditorRef.value?.expandAll()
  rightEditorRef.value?.expandAll()
}

function handleCollapseAll() {
  leftEditorRef.value?.collapseAll()
  rightEditorRef.value?.collapseAll()
}

function handleToggleTheme() {
  theme.value = theme.value === 'light' ? 'dark' : 'light'
  document.documentElement.setAttribute('data-theme', theme.value)
}

function copyLeftToRight() {
  const selType = leftEditorRef.value?.getSelectedType()
  if (selType === 'array' || selType === 'object') {
    const value = leftEditorRef.value?.getSelectedValue()
    rightDraft.value = JSON.stringify(value, null, 2)
  } else {
    rightDraft.value = leftContent.value
  }
}

function copyRightToLeft() {
  const selType = rightEditorRef.value?.getSelectedType()
  if (selType === 'array' || selType === 'object') {
    const value = rightEditorRef.value?.getSelectedValue()
    leftContent.value = JSON.stringify(value, null, 2)
  } else {
    leftContent.value = rightDraft.value
  }
}

function getCopyLeftTitle(): string {
  if (leftSelectionType.value === 'array') return t('copyLeftArrayToRight')
  if (leftSelectionType.value === 'object') return t('copyLeftObjectToRight')
  return t('copyLeftToRight')
}

function getCopyRightTitle(): string {
  if (rightSelectionType.value === 'array') return t('copyRightArrayToLeft')
  if (rightSelectionType.value === 'object') return t('copyRightObjectToLeft')
  return t('copyRightToLeft')
}

// ---------------------------------------------------------------------------
// Drag & drop
// ---------------------------------------------------------------------------
const dragOverLeft = ref(false)
const dragOverRight = ref(false)

async function setupDragDrop() {
  try {
    const unlisten = await getCurrentWebview().onDragDropEvent((event) => {
      const payload = event.payload
      if (payload.type === 'over') {
        const container = document.querySelector('.editor-split') as HTMLElement
        if (container) {
          const rect = container.getBoundingClientRect()
          const isLeft = payload.position.x < rect.left + rect.width * splitRatio.value
          dragOverLeft.value = isLeft
          dragOverRight.value = !isLeft
        }
      } else if (payload.type === 'drop') {
        dragOverLeft.value = false
        dragOverRight.value = false
        if (payload.paths.length > 0) {
          const container = document.querySelector('.editor-split') as HTMLElement
          const isLeft = container
            ? payload.position.x < container.getBoundingClientRect().left + container.getBoundingClientRect().width * splitRatio.value
            : true
          loadDroppedFile(payload.paths[0], isLeft ? 'left' : 'right')
        }
      } else {
        dragOverLeft.value = false
        dragOverRight.value = false
      }
    })
    unlistenFns.push(unlisten)
  } catch (e) {
    // non-Tauri env
  }
}

async function loadDroppedFile(path: string, side: 'left' | 'right') {
  try {
    const content = await readTextFile(path)
    if (side === 'left') {
      await openFileInTab(path, content)
    } else {
      rightDraft.value = content
    }
  } catch (e) {
    console.error('Failed to load dropped file:', e)
  }
}

// ---------------------------------------------------------------------------
// Sidebar
// ---------------------------------------------------------------------------
const sidebarWidth = usePersistedState('sidebarWidth', 220)
const isSidebarDragging = ref(false)

function startSidebarDrag(e: MouseEvent) {
  e.preventDefault()
  isSidebarDragging.value = true
  const container = document.querySelector('.left-panel') as HTMLElement
  const onMove = (ev: MouseEvent) => {
    if (!container) return
    const rect = container.getBoundingClientRect()
    const width = ev.clientX - rect.left
    sidebarWidth.value = Math.min(320, Math.max(160, width))
  }
  const onUp = () => {
    isSidebarDragging.value = false
    document.removeEventListener('mousemove', onMove)
    document.removeEventListener('mouseup', onUp)
    document.body.style.cursor = ''
    document.body.style.userSelect = ''
  }
  document.addEventListener('mousemove', onMove)
  document.addEventListener('mouseup', onUp)
  document.body.style.cursor = 'col-resize'
  document.body.style.userSelect = 'none'
}

// Watch active tab to update folder directory
watch(activeLeftTabId, async (newId) => {
  const tab = leftTabs.value.find(t => t.id === newId)
  if (tab?.path) {
    await openDirForFile(tab.path)
  } else {
    currentDir.value = ''
    dirFiles.value = []
  }
})

// ---------------------------------------------------------------------------
// File association
// ---------------------------------------------------------------------------
function normalizePath(input: string): string {
  if (input.startsWith('file://')) {
    let p = decodeURIComponent(input.slice('file://'.length))
    if (/^\/[a-zA-Z]:/.test(p)) p = p.slice(1)
    return p
  }
  return input
}

async function loadFileFromPath(rawPath: string) {
  if (activeTab.value?.path === rawPath) return
  try {
    const path = normalizePath(rawPath)
    console.log('[file-association] loading file:', path)
    const content = await readTextFile(path)
    await openFileInTab(path, content)
    console.log('[file-association] file loaded successfully')
  } catch (e) {
    console.error('[file-association] ERROR:', e)
  }
}

async function setupFileAssociation() {
  try {
    const unlisten = await listen<string[]>('opened', (event) => {
      console.log('[file-association] opened event:', event.payload)
      for (const p of event.payload ?? []) {
        loadFileFromPath(p)
      }
    })
    unlistenFns.push(unlisten)

    await new Promise(resolve => setTimeout(resolve, 100))

    const paths = await invoke<string[]>('opened_paths')
    console.log('[file-association] initial paths:', paths)
    for (const p of paths) {
      await loadFileFromPath(p)
    }
  } catch (e) {
    console.log('[file-association] setup failed (expected in browser):', e)
  }
}

// ---------------------------------------------------------------------------
// Menu shortcuts
// ---------------------------------------------------------------------------
async function setupMenuShortcuts() {
  const handlers: Record<string, () => void> = {
    'menu:new': handleNew,
    'menu:open': handleOpen,
    'menu:open_url': handleOpenUrl,
    'menu:save': handleSave,
  }
  for (const [eventName, handler] of Object.entries(handlers)) {
    // The Tauri IPC bridge may not be ready during onMounted; retry once so a
    // failed registration is never silently swallowed.
    for (let attempt = 0; attempt < 3; attempt++) {
      try {
        const unlisten = await listen(eventName, () => handler())
        unlistenFns.push(unlisten)
        break
      } catch (e) {
        if (attempt === 2) {
          console.warn('[menu] listen failed:', eventName, e)
        } else {
          await new Promise(resolve => setTimeout(resolve, 400))
        }
      }
    }
  }
}

async function setupFocusRefresh() {
  try {
    const unlisten = await listen('tauri://focus', () => {
      refreshCurrentDir()
    })
    unlistenFns.push(unlisten)
  } catch (e) {
    // non-Tauri env
  }
}

// ---------------------------------------------------------------------------
// Resizable split panel
// ---------------------------------------------------------------------------
const splitRatio = usePersistedState('splitRatio', 0.5)
const isDragging = ref(false)

function startDrag(e: MouseEvent) {
  e.preventDefault()
  isDragging.value = true
  const onMove = (ev: MouseEvent) => {
    const container = document.querySelector('.editor-split') as HTMLElement
    if (!container) return
    const rect = container.getBoundingClientRect()
    const ratio = (ev.clientX - rect.left) / rect.width
    splitRatio.value = Math.min(0.9, Math.max(0.1, ratio))
  }
  const onUp = () => {
    isDragging.value = false
    document.removeEventListener('mousemove', onMove)
    document.removeEventListener('mouseup', onUp)
    document.body.style.cursor = ''
    document.body.style.userSelect = ''
  }
  document.addEventListener('mousemove', onMove)
  document.addEventListener('mouseup', onUp)
  document.body.style.cursor = 'col-resize'
  document.body.style.userSelect = 'none'
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------
const leftValidation = computed(() => validateJson(leftContent.value))
const leftNodeCount = computed(() => countJsonNodes(leftContent.value))
const rightValidation = computed(() => validateJson(rightDraft.value))
const rightNodeCount = computed(() => countJsonNodes(rightDraft.value))

// ---------------------------------------------------------------------------
// Toast
// ---------------------------------------------------------------------------
const toastMessage = ref('')
let toastTimer: number | undefined

function showToast(msg: string) {
  toastMessage.value = msg
  if (toastTimer) window.clearTimeout(toastTimer)
  toastTimer = window.setTimeout(() => {
    toastMessage.value = ''
  }, 1600)
}

// ---------------------------------------------------------------------------
// Keyboard shortcuts
// ---------------------------------------------------------------------------
// Registered on `window` in the CAPTURE phase: the vanilla-jsoneditor inner
// handlers (and Tauri's native menu accelerator) would otherwise swallow the
// event before it reaches `document` in the bubble phase.
function onKeyDown(e: KeyboardEvent) {
  if (!(e.ctrlKey || e.metaKey) || e.altKey || e.shiftKey) return
  const isS = e.key === 's' || e.key === 'S' || e.code === 'KeyS'
  if (!isS) return
  e.preventDefault()
  e.stopPropagation()
  if (typeof e.stopImmediatePropagation === 'function') e.stopImmediatePropagation()
  void handleSave()
}

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------
onMounted(() => {
  document.documentElement.setAttribute('data-theme', theme.value)
  setupFileAssociation()
  setupMenuShortcuts()
  setupFocusRefresh()
  setupDragDrop()
  window.addEventListener('keydown', onKeyDown, true)
})

onBeforeUnmount(() => {
  unlistenFns.forEach(fn => fn())
  window.removeEventListener('keydown', onKeyDown, true)
  if (toastTimer) window.clearTimeout(toastTimer)
})
</script>

<template>
  <div class="app" :data-theme="theme">
    <Toolbar
      :mode="leftMode"
      :theme="theme"
      :file-name="fileName"
      :recent-files="recentFiles"
      @new="handleNew"
      @open="handleOpen"
      @open-recent="handleOpenRecent"
      @open-url="handleOpenUrl"
      @save="handleSave"
      @save-as="handleSaveAs"
      @copy="handleCopy"
      @format="handleFormat"
      @compact="handleCompact"
      @repair="handleRepair"
      @expand-all="handleExpandAll"
      @collapse-all="handleCollapseAll"
      @toggle-theme="handleToggleTheme"
      @update:mode="(m) => { if (activeTab) activeTab.mode = m }"
    />
    <div class="app-body">
      <div class="left-panel">
        <div class="sidebar" :style="{ width: sidebarWidth + 'px' }">
          <template v-if="leftTabs.length === 0">
            <div class="sidebar-header">{{ t('welcome.recentFiles') }}</div>
            <div class="sidebar-list">
              <div v-if="recentFiles.length === 0" class="sidebar-empty">{{ t('welcome.noRecent') }}</div>
              <button
                v-for="file in recentFiles"
                :key="file.path"
                class="recent-item"
                @click="handleOpenRecent(file.path)"
              >
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
                </svg>
                <span class="recent-item-name">{{ file.name }}</span>
                <span class="recent-item-path">{{ file.path }}</span>
              </button>
            </div>
          </template>
          <template v-else>
            <div class="folder-header">
              <span class="folder-header-text">{{ currentDir || t('folder.tempFile') }}</span>
              <button
                v-if="currentDir"
                class="folder-refresh"
                @click="refreshCurrentDir"
                :title="t('folder.refresh')"
              >
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M23 4v6h-6M1 20v-6h6M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15" />
                </svg>
              </button>
            </div>
            <div class="folder-list">
              <div v-if="dirLoading" class="folder-empty">{{ t('folder.loading') }}</div>
              <div v-else-if="dirFiles.length === 0" class="folder-empty">{{ t('folder.empty') }}</div>
              <div
                v-for="f in dirFiles"
                :key="f"
                class="folder-item"
                :class="{ active: f === fileName }"
                @click="handleOpenFileFromFolder(f)"
              >
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="folder-icon">
                  <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
                </svg>
                <span class="folder-item-name">{{ f }}</span>
              </div>
            </div>
          </template>
        </div>
        <div class="sidebar-divider" :class="{ dragging: isSidebarDragging }" @mousedown="startSidebarDrag"></div>
        <div class="main-area">
          <TabBar
            v-if="leftTabs.length > 0"
            :tabs="leftTabs"
            :active-id="activeLeftTabId"
            @select="switchTab"
            @close="closeTab"
          />
          <div class="editor-split" :class="{ 'no-tabs': leftTabs.length === 0 }">
            <div
              class="editor-section"
              :class="{ 'drag-over': dragOverLeft }"
              :style="{ flex: `0 0 calc(${splitRatio * 100}% - ${splitRatio * 40}px)` }"
            >
              <template v-if="leftTabs.length > 0">
                <div class="panel-header panel-header-file">
                  <div class="panel-title-group">
                    <span class="panel-title">{{ fileName }}<span v-if="activeTab && activeTab.dirty" class="title-dirty">(*)</span></span>
                    <span v-if="activeTab && activeTab.path" class="panel-path" :title="activeTab.path">{{ activeTab.path }}</span>
                    <span v-else class="panel-path panel-path-empty">{{ t('folder.tempFile') }}</span>
                  </div>
                  <div class="panel-status">
                    <span v-if="leftValidation.valid" class="status-ok">✓ {{ t('panel.valid') }}</span>
                    <span v-else class="status-err">✗ {{ t('panel.invalid') }}</span>
                    <span class="node-count">{{ leftNodeCount }} {{ t('panel.nodes') }}</span>
                  </div>
                </div>
                <JsonEditorPanel
                  ref="leftEditorRef"
                  v-model="leftContent"
                  v-model:mode="leftMode"
                  :theme="theme"
                  label="left"
                  class="editor-wrapper"
                  @selection-change="(t) => leftSelectionType = t"
                />
              </template>
            </div>
            <div class="split-divider">
              <div class="split-actions">
                <button
                  class="split-btn"
                  :title="getCopyLeftTitle()"
                  @click="copyLeftToRight"
                >
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <polyline points="9 18 15 12 9 6" />
                  </svg>
                </button>
                <button
                  class="split-btn"
                  :title="getCopyRightTitle()"
                  @click="copyRightToLeft"
                >
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <polyline points="15 18 9 12 15 6" />
                  </svg>
                </button>
              </div>
              <div class="split-drag-handle" :class="{ dragging: isDragging }" @mousedown="startDrag">
                <svg class="drag-indicator" width="4" height="19" viewBox="0 0 4 19" fill="currentColor"><g transform="translate(-300 -755)"><rect width="2" height="1" transform="translate(300 755)" /><rect width="2" height="1" transform="translate(300 763)" /><rect width="2" height="1" transform="translate(300 757)" /><rect width="2" height="1" transform="translate(300 759)" /><rect width="2" height="1" transform="translate(300 761)" /><rect width="2" height="1" transform="translate(300 765)" /><rect width="2" height="1" transform="translate(300 773)" /><rect width="2" height="1" transform="translate(300 767)" /><rect width="2" height="1" transform="translate(300 769)" /><rect width="2" height="1" transform="translate(300 771)" /><rect width="2" height="1" transform="translate(302 755)" /><rect width="2" height="1" transform="translate(302 763)" /><rect width="2" height="1" transform="translate(302 757)" /><rect width="2" height="1" transform="translate(302 759)" /><rect width="2" height="1" transform="translate(302 761)" /><rect width="2" height="1" transform="translate(302 765)" /><rect width="2" height="1" transform="translate(302 773)" /><rect width="2" height="1" transform="translate(302 767)" /><rect width="2" height="1" transform="translate(302 769)" /><rect width="2" height="1" transform="translate(302 771)" /></g></svg>
              </div>
            </div>
            <div
              class="editor-section"
              :class="{ 'drag-over': dragOverRight }"
              :style="{ flex: `0 0 calc(${(1 - splitRatio) * 100}% - ${(1 - splitRatio) * 40}px)` }"
            >
              <div class="panel-header">
                <span class="panel-title">{{ t('panel.draft') }}</span>
                <div class="panel-status">
                  <span v-if="rightValidation.valid" class="status-ok">✓ {{ t('panel.valid') }}</span>
                  <span v-else class="status-err">✗ {{ t('panel.invalid') }}</span>
                  <span class="node-count">{{ rightNodeCount }} {{ t('panel.nodes') }}</span>
                </div>
              </div>
              <JsonEditorPanel
                ref="rightEditorRef"
                v-model="rightDraft"
                v-model:mode="rightMode"
                :theme="theme"
                label="right"
                class="editor-wrapper"
                @selection-change="(t) => rightSelectionType = t"
              />
            </div>
          </div>
        </div>
      </div>
      <div v-if="!leftValidation.valid && leftValidation.error" class="error-bar">
        <span class="error-icon">⚠</span>
        <span class="error-text">{{ leftValidation.error }}</span>
      </div>
      <OpenUrlModal
        v-if="showOpenUrlModal"
        @close="showOpenUrlModal = false"
        @load="handleUrlLoaded"
      />
    </div>
    <div v-if="toastMessage" class="toast">{{ toastMessage }}</div>
  </div>
</template>

<style>
:root {
  --bg-color: #ffffff;
  --toolbar-bg: #f8f9fa;
  --text-color: #1a1a1a;
  --text-secondary: #6b7280;
  --border-color: #e5e7eb;
  --btn-hover-bg: #f3f4f6;
  --btn-active-bg: #e5e7eb;
  --panel-header-bg: #f8f9fa;
}

:root[data-theme="dark"] {
  --bg-color: #1e1e1e;
  --toolbar-bg: #2d2d2d;
  --text-color: #d4d4d4;
  --text-secondary: #858585;
  --border-color: #3c3c3c;
  --btn-hover-bg: #343434;
  --btn-active-bg: #464646;
  --panel-header-bg: #2d2d2d;
}

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

html,
body,
#app {
  height: 100%;
  width: 100%;
  overflow: hidden;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
  background: var(--bg-color);
  color: var(--text-color);
}

.app {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: var(--bg-color);
}

.app-body {
  display: flex;
  flex: 1;
  overflow: hidden;
}

.left-panel {
  flex: 1;
  display: flex;
  flex-direction: row;
  overflow: hidden;
  min-width: 0;
}

.editor-split {
  display: flex;
  flex: 1;
  overflow: hidden;
}

.editor-split.no-tabs {
  flex: 1;
}

.editor-section {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  position: relative;
  min-width: 0;
}

.editor-section.drag-over {
  outline: 2px dashed var(--accent-color, #3b82f6);
  outline-offset: -2px;
  background: rgba(59, 130, 246, 0.05);
}

.welcome-panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 40px;
  overflow-y: auto;
}

.welcome-title {
  font-size: 20px;
  font-weight: 600;
  color: var(--text-color);
  margin-bottom: 8px;
}

.welcome-subtitle {
  font-size: 13px;
  color: var(--text-secondary);
  margin-bottom: 32px;
}

.recent-list {
  width: 100%;
  max-width: 480px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.recent-header {
  font-size: 11px;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-bottom: 8px;
}

.recent-empty {
  font-size: 13px;
  color: var(--text-secondary);
  padding: 20px;
  text-align: center;
}

.recent-item {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 5px 8px;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: var(--text-color);
  cursor: pointer;
  font-size: 12px;
  text-align: left;
  transition: all 0.15s;
}

.recent-item:hover {
  background: var(--btn-hover-bg);
}

.recent-item svg {
  flex-shrink: 0;
  opacity: 0.6;
}

.recent-item-name {
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.recent-item-path {
  font-size: 10px;
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  margin-left: auto;
  padding-left: 8px;
  max-width: 100px;
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 12px;
  height: 32px;
  background: var(--panel-header-bg);
  border-bottom: 1px solid var(--border-color);
  flex-shrink: 0;
}

.panel-header-file {
  height: 44px;
}

.panel-title-group {
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: 1px;
  flex: 1;
  min-width: 0;
}

.panel-title-group .panel-title {
  flex: none;
  max-width: 100%;
}

.title-dirty {
  color: #ef4444;
  font-weight: 600;
  margin-left: 2px;
}

.panel-path {
  font-size: 10px;
  line-height: 1.35;
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 100%;
}

.panel-path-empty {
  opacity: 0.65;
  font-style: italic;
}

.panel-title {
  font-size: 12px;
  color: var(--text-secondary);
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
  min-width: 0;
}

.toast {
  position: fixed;
  bottom: 28px;
  left: 50%;
  transform: translateX(-50%);
  padding: 8px 18px;
  border-radius: 6px;
  background: rgba(17, 24, 39, 0.92);
  color: #ffffff;
  font-size: 13px;
  z-index: 10000;
  pointer-events: none;
  box-shadow: 0 4px 14px rgba(0, 0, 0, 0.25);
  max-width: 60vw;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.panel-status {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 11px;
  white-space: nowrap;
  flex-shrink: 0;
}

.panel-status .status-ok {
  color: #22c55e;
}

.panel-status .status-err {
  color: #ef4444;
}

.panel-status .node-count {
  color: var(--text-secondary);
}

.editor-wrapper {
  flex: 1;
  overflow: hidden;
}

.split-divider {
  width: 40px;
  display: flex;
  flex-direction: column;
  align-items: center;
  background: var(--toolbar-bg);
  border-left: 1px solid var(--border-color);
  border-right: 1px solid var(--border-color);
  flex-shrink: 0;
}

.split-actions {
  display: flex;
  flex-direction: column;
  gap: 8px;
  justify-content: center;
  align-items: center;
  flex: 0 0 25%;
  flex-shrink: 0;
  width: 100%;
}

.split-drag-handle {
  flex: 1;
  width: 100%;
  cursor: col-resize;
  position: relative;
  transition: background 0.15s;
}

.split-drag-handle:hover,
.split-drag-handle.dragging {
  background: var(--btn-hover-bg);
}

.split-drag-handle::before {
  content: '';
  position: absolute;
  top: 20%;
  bottom: 20%;
  left: 50%;
  width: 2px;
  transform: translateX(-50%);
  background: var(--border-color);
  opacity: 0.6;
  border-radius: 1px;
  transition: opacity 0.15s;
}

.split-drag-handle:hover::before,
.split-drag-handle.dragging::before {
  opacity: 1;
}

.drag-indicator {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  color: var(--text-secondary);
  opacity: 0.5;
  transition: opacity 0.15s;
  pointer-events: none;
  fill: currentColor;
}

.split-drag-handle:hover .drag-indicator,
.split-drag-handle.dragging .drag-indicator {
  opacity: 1;
}

.split-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.15s;
}

.split-btn:hover {
  background: var(--btn-hover-bg);
  color: var(--text-color);
}

.sidebar {
  width: 220px;
  flex-shrink: 0;
  background: var(--panel-header-bg);
  border-right: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.sidebar-divider {
  width: 4px;
  cursor: col-resize;
  background: var(--border-color);
  flex-shrink: 0;
  transition: background 0.15s;
}

.sidebar-divider:hover,
.sidebar-divider.dragging {
  background: var(--text-secondary);
}

.main-area {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-width: 0;
}

.sidebar-header {
  padding: 8px 12px;
  font-size: 11px;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  border-bottom: 1px solid var(--border-color);
  flex-shrink: 0;
}

.sidebar-list {
  flex: 1;
  overflow-y: auto;
  padding: 4px;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.folder-panel {
  height: 160px;
  flex-shrink: 0;
  background: var(--panel-header-bg);
  border-top: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.folder-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 12px;
  font-size: 11px;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  border-bottom: 1px solid var(--border-color);
  flex-shrink: 0;
}

.folder-header-text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  min-width: 0;
}

.folder-refresh {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  flex-shrink: 0;
  transition: all 0.15s;
}

.folder-refresh:hover {
  background: var(--btn-hover-bg);
  color: var(--text-color);
}

.folder-list {
  flex: 1;
  overflow-y: auto;
  padding: 4px;
}

.folder-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 8px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 12px;
  color: var(--text-color);
  transition: background 0.15s;
  white-space: nowrap;
}

.folder-item:hover {
  background: var(--btn-hover-bg);
}

.folder-item.active {
  background: var(--btn-active-bg);
  color: var(--accent-color, #3b82f6);
}

.folder-icon {
  flex-shrink: 0;
  opacity: 0.7;
}

.folder-item-name {
  overflow: hidden;
  text-overflow: ellipsis;
}

.folder-empty {
  padding: 12px;
  font-size: 12px;
  color: var(--text-secondary);
  text-align: center;
}

.error-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 16px;
  background: #fef2f2;
  border-top: 1px solid #fecaca;
  color: #dc2626;
  font-size: 13px;
  flex-shrink: 0;
}

:root[data-theme="dark"] .error-bar {
  background: #451a03;
  border-top: 1px solid #7c2d12;
  color: #fbbf24;
}

.error-icon {
  font-size: 16px;
}

/* vanilla-jsoneditor dark theme overrides */
:root[data-theme="dark"] .jse-main {
  --jse-background-color: #1e1e1e;
  --jse-text-color: #d4d4d4;
  --jse-menu-color: #fff;
  --jse-theme-color: #306eb5;
  --jse-key-color: #9cdcfe;
  --jse-value-color: #d4d4d4;
  --jse-value-color-number: #b5cea8;
  --jse-value-color-boolean: #569cd6;
  --jse-value-color-null: #569cd6;
  --jse-value-color-string: #ce9178;
  --jse-value-color-url: #ce9178;
  --jse-hover-background-color: #343434;
  --jse-selection-background-color: #464646;
  --jse-navigation-bar-background: #656565;
  --jse-navigation-bar-background-highlight: #7e7e7e;
  --jse-panel-button-background-highlight: #7e7e7e;
  --jse-context-menu-background: #4b4b4b;
  --jse-context-menu-background-highlight: #7a7a7a;
  --jse-context-menu-separator-color: #595959;
  --jse-context-menu-pointer-hover-background: hsl(8, 67%, 54%);
  --jse-modal-background: #2f2f2f;
  --jse-modal-overlay-background: rgba(0, 0, 0, 0.5);
  --jse-modal-code-background: #2f2f2f;
  --jse-panel-background: #333333;
  --jse-panel-background-border: 1px solid #464646;
  --jse-panel-border: 1px solid #3c3c3c;
  --jse-input-background: #3d3d3d;
  --jse-input-border: 1px solid #4f4f4f;
  --jse-table-header-background: #333333;
  --jse-table-header-background-highlight: #424242;
  --jse-table-row-odd-background: rgba(255, 255, 255, 0.1);
  --jse-button-background: #808080;
  --jse-button-background-highlight: #7a7a7a;
  --jse-button-color: #e0e0e0;
  --jse-button-secondary-background: #494949;
  --jse-button-secondary-background-highlight: #5d5d5d;
  --jse-a-color: #55abff;
  --jse-a-color-highlight: #4387c9;
  --jse-search-match-background-color: #343434;
  --jse-active-line-background-color: rgba(255, 255, 255, 0.06);
  --jse-tag-background: rgba(0, 0, 0, 0.3);
  --jse-tooltip-background: #4b4b4b;
  --jse-tooltip-border: 1px solid #737373;
  --jse-color-picker-background: #656565;
  --jse-svelte-select-background: #3d3d3d;
  --jse-svelte-select-border: 1px solid #4f4f4f;
}

/* Dark mode: CodeMirror punctuation/bracket color fix */
:root[data-theme="dark"] .jse-text-mode .cm-content,
:root[data-theme="dark"] .jse-text-mode .cm-line {
  color: #d4d4d4;
}

/* Dark mode: soften the divider between line numbers and editor content */
:root[data-theme="dark"] .jse-text-mode .jse-contents .cm-editor .cm-gutters {
  border-right: 1px solid #2a2a2a !important;
}

/* Dark mode: soften CodeMirror indentation guide lines */
:root[data-theme="dark"] .jse-text-mode .cm-editor {
  --indent-marker-bg-color: #303030;
  --indent-marker-active-bg-color: #3a3a3a;
}

/* Dark mode: tree mode punctuation/bracket/index color fix */
:root[data-theme="dark"] .jse-tree-mode .jse-bracket,
:root[data-theme="dark"] .jse-tree-mode .jse-expand,
:root[data-theme="dark"] .jse-tree-mode .jse-meta,
:root[data-theme="dark"] .jse-tree-mode .jse-meta-inner,
:root[data-theme="dark"] .jse-tree-mode .jse-index,
:root[data-theme="dark"] .jse-tree-mode .jse-separator {
  color: rgba(255, 255, 255, 0.6);
}

/* Remove default #d7d7d7 side borders from jsoneditor panels */
.jse-contents,
.jse-status-bar,
.jse-navigation-bar {
  border-left: none !important;
  border-right: none !important;
}

.jse-contents:last-child {
  border-bottom: none !important;
}

:root[data-theme="dark"] .jse-status-bar {
  border-bottom-color: #3c3c3c !important;
}
</style>
