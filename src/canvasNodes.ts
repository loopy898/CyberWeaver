import { toRichText } from 'tldraw'
import { isSupportedNodeType, type PersistedNode } from './graphNodes.ts'

export type ShapeRecord = {
  id: string
  typeName: string
  type: string
  x: number
  y: number
  props?: Record<string, unknown>
}

const SHAPE_PREFIX = 'shape:'

export function ensureShapeId(id: string) {
  return id.startsWith(SHAPE_PREFIX) ? id : `${SHAPE_PREFIX}${id}`
}

export function toStorageId(id: string) {
  return id.startsWith(SHAPE_PREFIX) ? id.slice(SHAPE_PREFIX.length) : id
}

export function isPersistableShape(record: unknown): record is ShapeRecord {
  if (!record || typeof record !== 'object') return false
  const candidate = record as ShapeRecord
  return candidate.typeName === 'shape' && isSupportedNodeType(candidate.type)
}

export function richTextToPlainText(richText: unknown) {
  if (!richText) return ''
  if (typeof richText === 'string') return richText
  if (typeof richText !== 'object') return ''

  const root = richText as { content?: unknown[] }
  if (!Array.isArray(root.content)) return ''

  return root.content
    .map((block) => {
      if (!block || typeof block !== 'object') return ''
      const paragraph = block as { type?: string; content?: unknown[] }
      if (paragraph.type !== 'paragraph' || !Array.isArray(paragraph.content)) return ''
      return paragraph.content
        .map((part) => {
          if (!part || typeof part !== 'object') return ''
          const textNode = part as { text?: unknown }
          return typeof textNode.text === 'string' ? textNode.text : ''
        })
        .join('')
    })
    .filter(Boolean)
    .join('\n')
}

export function shapeToNode(record: ShapeRecord): PersistedNode {
  const storageId = toStorageId(record.id).replace(/^viz:/, '')
  const richText = record.props?.richText
  const fallbackText = record.props?.text
  const content =
    record.type === 'geo'
      ? `[${String(record.props?.geo ?? 'rectangle')}]`
      : richTextToPlainText(richText) || (typeof fallbackText === 'string' ? fallbackText : '')

  return {
    id: storageId,
    type: record.type,
    x: record.x,
    y: record.y,
    content,
  }
}

export function nodeToShape(node: PersistedNode) {
  const common = {
    id: ensureShapeId(node.id),
    x: Number.isFinite(node.x) ? node.x : 0,
    y: Number.isFinite(node.y) ? node.y : 0,
  }

  switch (node.type) {
    case 'geo':
      return {
        ...common,
        type: 'geo',
        props: {
          geo: 'rectangle',
          w: 200,
          h: 100,
        },
      }
    case 'text':
      return {
        ...common,
        type: 'text',
        props: {
          richText: toRichText(node.content ?? ''),
        },
      }
    case 'note':
      return {
        ...common,
        type: 'note',
        props: {
          richText: toRichText(node.content ?? ''),
        },
      }
    default:
      return null
  }
}
