export function formatJson(text: string, indentation = 2): string {
  const parsed = JSON.parse(text)
  return JSON.stringify(parsed, null, indentation)
}

export function compactJson(text: string): string {
  const parsed = JSON.parse(text)
  return JSON.stringify(parsed)
}

export function validateJson(text: string): { valid: boolean; error: string | null } {
  try {
    JSON.parse(text)
    return { valid: true, error: null }
  } catch (e) {
    return { valid: false, error: (e as Error).message }
  }
}

export function tryParseJson(text: string): { success: boolean; data: unknown; error: string | null } {
  try {
    const data = JSON.parse(text)
    return { success: true, data, error: null }
  } catch (e) {
    return { success: false, data: null, error: (e as Error).message }
  }
}

export function countJsonNodes(text: string): number {
  const result = tryParseJson(text)
  if (!result.success) return 0
  let count = 0
  const traverse = (obj: unknown): void => {
    if (obj === null || obj === undefined) return
    count++
    if (Array.isArray(obj)) {
      obj.forEach(traverse)
    } else if (typeof obj === 'object') {
      Object.values(obj as Record<string, unknown>).forEach(traverse)
    }
  }
  traverse(result.data)
  return count
}

export function getValueByPath(obj: unknown, path: (string | number)[]): unknown {
  let current = obj
  for (const key of path) {
    if (current === null || current === undefined) return undefined
    if (typeof current === 'object') {
      current = (current as Record<string | number, unknown>)[key]
    } else {
      return undefined
    }
  }
  return current
}

export function getValueType(value: unknown): 'array' | 'object' | 'none' {
  if (Array.isArray(value)) return 'array'
  if (value !== null && typeof value === 'object') return 'object'
  return 'none'
}
