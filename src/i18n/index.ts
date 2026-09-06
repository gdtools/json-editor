import { ref, watch } from 'vue'

export type Lang = 'zh-CN' | 'en-US'

const STORAGE_KEY = 'json-editor-lang'

function loadLang(): Lang {
  try {
    const s = localStorage.getItem(STORAGE_KEY)
    if (s === 'zh-CN' || s === 'en-US') return s
  } catch {
    // ignore
  }
  const nav = (navigator.language || 'en-US').toLowerCase()
  return nav.startsWith('zh') ? 'zh-CN' : 'en-US'
}

// 模块级单例：所有组件共享同一语言状态
export const lang = ref<Lang>(loadLang())

watch(
  lang,
  (v) => {
    try {
      localStorage.setItem(STORAGE_KEY, v)
    } catch {
      // ignore quota errors
    }
    document.documentElement.setAttribute('lang', v)
  },
  { immediate: true }
)

type Dict = Record<string, string>

export const messages: Record<Lang, Dict> = {
  'zh-CN': {
    'toolbar.new': '新建',
    'toolbar.open': '打开',
    'toolbar.openFile': '打开文件',
    'toolbar.openUrl': '从 URL 打开',
    'toolbar.save': '保存',
    'toolbar.copy': '复制',
    'toolbar.about': '关于',
    'toolbar.theme': '切换主题',
    'toolbar.lang': '切换语言',
    'about.version': 'JsonEditor v0.1.4',
    'panel.treeView': '树视图',
    'panel.valid': '有效',
    'panel.invalid': '无效',
    'panel.nodes': '个节点',
    'modal.title': '从 URL 打开',
    'modal.close': '关闭',
    'modal.hint': '在下方粘贴 curl 命令，系统会自动解析 URL、请求方法、请求头（含 Authorization）与请求体。',
    'modal.placeholder': "curl -X GET 'https://api.example.com/data' -H 'Authorization: Bearer token123'",
    'modal.emptyError': '请粘贴 curl 命令',
    'modal.cancel': '取消',
    'modal.load': '加载',
    'modal.loading': '加载中…',
  },
  'en-US': {
    'toolbar.new': 'New',
    'toolbar.open': 'Open',
    'toolbar.openFile': 'Open File',
    'toolbar.openUrl': 'Open from URL',
    'toolbar.save': 'Save',
    'toolbar.copy': 'Copy',
    'toolbar.about': 'About',
    'toolbar.theme': 'Toggle Theme',
    'toolbar.lang': 'Switch Language',
    'about.version': 'JsonEditor v0.1.4',
    'panel.treeView': 'Tree View',
    'panel.valid': 'Valid',
    'panel.invalid': 'Invalid',
    'panel.nodes': 'nodes',
    'modal.title': 'Open from URL',
    'modal.close': 'Close',
    'modal.hint': 'Paste a curl command below. URL, method, headers (including Authorization), and body will be parsed automatically.',
    'modal.placeholder': "curl -X GET 'https://api.example.com/data' -H 'Authorization: Bearer token123'",
    'modal.emptyError': 'Please paste a curl command',
    'modal.cancel': 'Cancel',
    'modal.load': 'Load',
    'modal.loading': 'Loading...',
  },
}

export function t(key: string): string {
  const dict = messages[lang.value]
  return dict[key] ?? key
}

export function toggleLang() {
  lang.value = lang.value === 'zh-CN' ? 'en-US' : 'zh-CN'
}
