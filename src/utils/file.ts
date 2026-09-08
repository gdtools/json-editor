import { open as openDialog, save as saveDialog } from '@tauri-apps/plugin-dialog'
import { readTextFile, writeTextFile } from '@tauri-apps/plugin-fs'
import { revealItemInDir } from '@tauri-apps/plugin-opener'
import { invoke } from '@tauri-apps/api/core'

const JSON_FILTERS = [
  { name: 'JSON', extensions: ['json'] },
  { name: 'All Files', extensions: ['*'] },
]

export async function openJsonFile(): Promise<{ path: string; content: string } | null> {
  const filePath = await openDialog({
    filters: JSON_FILTERS,
    multiple: false,
  })
  if (!filePath) return null
  const content = await readTextFile(filePath as string)
  return { path: filePath as string, content }
}

export async function saveJsonFile(content: string, defaultName = 'untitled.json', defaultDir?: string): Promise<string | null> {
  const defaultPath = defaultDir ? (defaultDir.replace(/[\\/]$/, '') + '/' + defaultName) : defaultName
  const filePath = await saveDialog({
    filters: JSON_FILTERS,
    defaultPath: defaultPath,
  })
  if (!filePath) return null
  await writeTextFile(filePath, content)
  return filePath
}

export async function writeJsonFile(path: string, content: string): Promise<void> {
  await writeTextFile(path, content)
}

/** One .json entry of a directory, including metadata from the Rust backend. */
export interface DirJsonFile {
  name: string
  path: string
  size: number
  mtime: number
}

/**
 * List all .json files in a directory, newest modification first.
 * Goes through the Rust backend because the fs plugin's readDir exposes
 * neither timestamps nor sizes.
 */
export async function listJsonFiles(dirPath: string): Promise<DirJsonFile[]> {
  try {
    const files = await invoke<DirJsonFile[]>('list_json_files', { dir: dirPath })
    return Array.isArray(files) ? files : []
  } catch (e) {
    console.error('Failed to list directory:', e)
    return []
  }
}

export async function renameJsonFile(from: string, to: string): Promise<void> {
  await invoke('rename_file', { from, to })
}

export async function deleteJsonFile(path: string): Promise<void> {
  await invoke('delete_file', { path })
}

/** Reveal a file (or directory) in Windows Explorer / Finder. */
export async function revealInFileManager(path: string): Promise<void> {
  await revealItemInDir(path)
}

export function joinPath(dir: string, name: string): string {
  return dir.replace(/[\\/]+$/, '') + '\\' + name
}

export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`
}

/** Localised short date-time (e.g. 09-07 14:32). */
export function formatMtime(mtime: number): string {
  if (!mtime) return ''
  const d = new Date(mtime * 1000)
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`
}
