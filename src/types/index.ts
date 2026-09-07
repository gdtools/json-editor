export type EditorMode = 'tree' | 'text' | 'table'

export type ThemeMode = 'light' | 'dark'

export interface EditorPanelData {
  content: string
  mode: EditorMode
}

export interface StatusInfo {
  line: number
  column: number
  count: number
  isValid: boolean
  error: string | null
}

/** A button of the generic confirm dialog. */
export interface DialogButton {
  key: string
  label: string
  primary?: boolean
  danger?: boolean
}
