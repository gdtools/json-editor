import { open as openDialog, save as saveDialog } from '@tauri-apps/plugin-dialog'
import { readTextFile, writeTextFile, readDir } from '@tauri-apps/plugin-fs'

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

export async function saveJsonFile(content: string, defaultName = 'untitled.json'): Promise<string | null> {
  const filePath = await saveDialog({
    filters: JSON_FILTERS,
    defaultPath: defaultName,
  })
  if (!filePath) return null
  await writeTextFile(filePath, content)
  return filePath
}

export async function listJsonFiles(dirPath: string): Promise<string[]> {
  try {
    const entries = await readDir(dirPath)
    const jsonFiles: string[] = []
    for (const entry of entries) {
      if (entry.isDirectory) {
        continue
      }
      if (entry.name.toLowerCase().endsWith('.json')) {
        jsonFiles.push(entry.name)
      }
    }
    return jsonFiles.sort()
  } catch (e) {
    console.error('Failed to list directory:', e)
    return []
  }
}
