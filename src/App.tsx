import { invoke } from '@tauri-apps/api/core'
import { useRef } from 'react'
import { Tldraw, toRichText } from 'tldraw'
import 'tldraw/tldraw.css'

type PersistedNode = {
  id: string
  type: string
  x: number
  y: number
  content: string
}

type ShapeRecord = {
  id: string
  typeName: string
  type: string
  x: number
  y: number
  props?: Record<string, unknown>
}

const ALLOWED_TYPES = new Set(['geo', 'text', 'note'])
const SHAPE_PREFIX = 'shape:'

function ensureShapeId(id: string) {
  return id.startsWith(SHAPE_PREFIX) ? id : `${SHAPE_PREFIX}${id}`
}

function toStorageId(id: string) {
  return id.startsWith(SHAPE_PREFIX) ? id.slice(SHAPE_PREFIX.length) : id
}

function isPersistableShape(record: unknown): record is ShapeRecord {
  if (!record || typeof record !== 'object') return false
  const candidate = record as ShapeRecord
  return candidate.typeName === 'shape' && ALLOWED_TYPES.has(candidate.type)
}

function richTextToPlainText(richText: unknown) {
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

function shapeToNode(record: ShapeRecord): PersistedNode {
  const richText = record.props?.richText
  const fallbackText = record.props?.text
  const content =
    record.type === 'geo'
      ? `[${String(record.props?.geo ?? 'rectangle')}]`
      : richTextToPlainText(richText) || (typeof fallbackText === 'string' ? fallbackText : '')

  return {
    id: toStorageId(record.id),
    type: record.type,
    x: record.x,
    y: record.y,
    content,
  }
}

function nodeToShape(node: PersistedNode) {
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

function App() {
  const isLoadingRef = useRef(false)
  const hasLoadedRef = useRef(false)

  const handleMount = (editor: any) => {
    if (isLoadingRef.current || hasLoadedRef.current) return

    const loadFromDb = async () => {
      isLoadingRef.current = true
      try {
        const nodes = await invoke<PersistedNode[]>('get_nodes')
        for (const node of nodes) {
          const shape = nodeToShape(node)
          if (!shape) continue
          try {
            editor.createShape(shape)
          } catch (error) {
            console.error('Failed to create shape from DB node', node, error)
          }
        }
      } catch (error) {
        console.error('Failed to load nodes from DB', error)
      } finally {
        isLoadingRef.current = false
        hasLoadedRef.current = true
      }
    }

    loadFromDb()

    let saveTimeout: ReturnType<typeof setTimeout> | undefined

    editor.store.listen((entry: any) => {
      if (isLoadingRef.current || !hasLoadedRef.current) return

      if (saveTimeout) clearTimeout(saveTimeout)

      saveTimeout = setTimeout(async () => {
        const addedShapes = Object.values(entry.changes.added ?? {}).filter(isPersistableShape)
        const updatedShapes = Object.values(entry.changes.updated ?? {})
          .map((pair: any) => (Array.isArray(pair) ? pair[1] : undefined))
          .filter(isPersistableShape)
        const removedShapes = Object.values(entry.changes.removed ?? {}).filter(isPersistableShape)

        const saveCalls = [...addedShapes, ...updatedShapes].map((shape) =>
          invoke('save_node', { node: shapeToNode(shape) })
        )
        const deleteCalls = removedShapes.map((shape) =>
          invoke('delete_node', { id: toStorageId(shape.id) })
        )

        try {
          await Promise.all([...saveCalls, ...deleteCalls])
        } catch (error) {
          console.error('Failed to sync shape changes', error)
        }
      }, 250)
    })
  }

  return (
    <div style={{ position: 'fixed', inset: 0 }}>
      <Tldraw onMount={handleMount} />
    </div>
  )
}

export default App
