export type PersistedNode = {
  id: string
  type: string
  x: number
  y: number
  content: string
}

const ALLOWED_TYPES = new Set(['geo', 'text', 'note'])

export function isSupportedNodeType(type: string) {
  return ALLOWED_TYPES.has(type)
}
