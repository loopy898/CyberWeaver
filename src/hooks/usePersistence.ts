import { useEffect, useRef, useState, useCallback, type MutableRefObject } from 'react'
import type { Editor } from 'tldraw'
import { invoke } from '@tauri-apps/api/core'
import {
  filterCustomShapes,
  nodeDataToShape,
  shapeToNodeData,
  toDbId,
  type CustomShapeRecord,
} from '../lib/shape-mapper'
import type { NodeData } from '../types/domain'
import { SYNC_DEBOUNCE_MS } from '../lib/constants'
import { useInvestigationStore } from '../stores/investigation'

/**
 * Load persisted nodes from the SQLite database into the tldraw canvas,
 * and continuously sync canvas mutations back to the database.
 *
 * Relation (arrow) sync is deferred to Phase 3.
 */
export function usePersistence(editorRef: MutableRefObject<Editor | null>) {
  const isLoadingRef = useRef(false)
  const [isLoading, setIsLoading] = useState(true)
  const currentId = useInvestigationStore((s) => s.currentId)

  /**
   * Maps canvas shape IDs to DB node IDs for shapes created during the
   * current session (create_node generates its own UUID).
   */
  const nodeIdMapRef = useRef(new Map<string, string>())

  /** Resolve the DB node ID for a given canvas shape ID. */
  const getNodeId = useCallback((shapeId: string): string => {
    return nodeIdMapRef.current.get(shapeId) ?? toDbId(shapeId)
  }, [])

  // ---------------------------------------------------------------------------
  // Load phase — pull nodes from DB and create shapes on the canvas.
  // ---------------------------------------------------------------------------
  useEffect(() => {
    const loadFromDb = async () => {
      const editor = editorRef.current
      if (!editor) return

      isLoadingRef.current = true
      setIsLoading(true)

      try {
        const investigationId = currentId ?? 'default'
        const nodes = await invoke<NodeData[]>('get_nodes', { investigationId })

        const shapesToCreate: ReturnType<typeof nodeDataToShape>[] = []
        for (const node of nodes) {
          const shapeParams = nodeDataToShape(node)
          if (shapeParams) {
            shapesToCreate.push(shapeParams)
          }
        }

        if (shapesToCreate.length > 0) {
          for (const params of shapesToCreate) {
            if (params) {
              // NOTE: tldraw's createShape has a fine-grained discriminated-union
              // signature; casting through any is safe here because the shape
              // types are already registered via customShapeUtils.
              // eslint-disable-next-line @typescript-eslint/no-explicit-any
              editor.createShape(params as any)
            }
          }
        }
        console.log(`Loaded ${shapesToCreate.length} nodes from DB (investigation: ${investigationId})`)
      } catch (error) {
        console.error('Failed to load nodes from DB:', error)
      } finally {
        isLoadingRef.current = false
        setIsLoading(false)
      }
    }

    // Delay execution to let the editor mount.
    const timer = setTimeout(loadFromDb, 500)
    return () => clearTimeout(timer)
  }, [editorRef, currentId])

  // ---------------------------------------------------------------------------
  // Sync phase — watch canvas changes and persist to DB.
  // ---------------------------------------------------------------------------
  useEffect(() => {
    const editor = editorRef.current
    if (!editor) return

    let saveTimeout: ReturnType<typeof setTimeout> | undefined

    /**
     * Process a batch of shape changes (add / update / remove) and sync to DB.
     */
    const processChanges = async (
      added: CustomShapeRecord[],
      updated: CustomShapeRecord[],
      removed: CustomShapeRecord[],
    ) => {
      const investigationId = currentId ?? 'default'

      // ---- 1. Handle new shapes ----
      for (const shape of added) {
        try {
          const params = shapeToNodeData(shape, investigationId)
          if (!params) continue

          const created = await invoke<NodeData>('create_node', {
            investigationId,
            nodeType: params.nodeType,
            label: params.label,
            posX: params.posX,
            posY: params.posY,
          })

          // Remember the mapping so future updates/deletes target the correct row.
          nodeIdMapRef.current.set(shape.id, created.id)

          // Apply the full properties (create_node only sets defaults).
          const fullParams = shapeToNodeData(shape, investigationId)
          if (fullParams) {
            await invoke<NodeData>('update_node', {
              id: created.id,
              label: fullParams.label,
              confidence: fullParams.confidence,
              properties: fullParams.properties,
              posX: fullParams.posX,
              posY: fullParams.posY,
            }).catch((err) => {
              console.warn('Failed to apply full properties after create:', err)
            })
          }
        } catch (error) {
          console.error('Failed to create node for shape:', shape.id, error)
        }
      }

      // ---- 2. Handle updated shapes ----
      for (const shape of updated) {
        try {
          const nodeId = getNodeId(shape.id)
          const params = shapeToNodeData(shape, investigationId)
          if (!params) continue

          await invoke<NodeData>('update_node', {
            id: nodeId,
            label: params.label,
            confidence: params.confidence,
            properties: params.properties,
            posX: params.posX,
            posY: params.posY,
          })
        } catch (error) {
          console.error('Failed to update node for shape:', shape.id, error)
        }
      }

      // ---- 3. Handle removed shapes ----
      for (const shape of removed) {
        try {
          const nodeId = getNodeId(shape.id)
          await invoke('delete_node', { id: nodeId })
          // Clean up mapping.
          nodeIdMapRef.current.delete(shape.id)
        } catch (error) {
          // The node might not exist in DB (e.g. was never persisted).
          console.warn('Failed to delete node:', shape.id, error)
          nodeIdMapRef.current.delete(shape.id)
        }
      }
    }

    const unsubscribe = editor.store.listen((entry) => {
      // Skip while loading from DB.
      if (isLoadingRef.current) return

      // Collect changed records. The store diff contains TLRecord values
      // whose exact union type determines whether they pass isCustomShape.
      const rawAdded = Object.values(entry.changes.added ?? {})
      const rawUpdated = (Object.values(entry.changes.updated ?? {}) as unknown[][])
        .map((pair) => (Array.isArray(pair) ? pair[1] : undefined))
      const rawRemoved = Object.values(entry.changes.removed ?? {})

      // Filter to our custom shapes only. TLRecord is a wide union so we use
      // the non-predicate isCustomShape check inside filterCustomShapes.
      const added = filterCustomShapes(rawAdded)
      const updated = filterCustomShapes(rawUpdated)
      const removed = filterCustomShapes(rawRemoved)

      if (added.length === 0 && updated.length === 0 && removed.length === 0) {
        return
      }

      // Debounce: accumulate changes and flush after SYNC_DEBOUNCE_MS.
      if (saveTimeout) clearTimeout(saveTimeout)
      saveTimeout = setTimeout(() => {
        processChanges(added, updated, removed)
      }, SYNC_DEBOUNCE_MS)
    })

    return () => {
      unsubscribe()
      if (saveTimeout) clearTimeout(saveTimeout)
    }
  }, [editorRef, currentId, getNodeId])

  return { isLoading }
}
