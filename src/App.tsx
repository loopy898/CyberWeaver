import { invoke } from '@tauri-apps/api/core'
import { useEffect, useRef, useState } from 'react'
import { Tldraw } from 'tldraw'
import 'tldraw/tldraw.css'
import { isPersistableShape, nodeToShape, shapeToNode, toStorageId } from './canvasNodes.ts'
import { type PersistedNode } from './graphNodes.ts'
import { createDebugCreateNodeCommand, parseGraphUpdateMessage } from './ws.ts'

function App() {
  const isLoadingRef = useRef(false)
  const hasLoadedRef = useRef(false)
  const editorRef = useRef<any>(null)
  const socketRef = useRef<WebSocket | null>(null)
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const [socketState, setSocketState] = useState<'connecting' | 'open' | 'closed'>('connecting')

  useEffect(() => {
    return () => {
      if (reconnectTimerRef.current) clearTimeout(reconnectTimerRef.current)
      socketRef.current?.close()
    }
  }, [])

  const connectWebSocket = () => {
    if (socketRef.current && socketRef.current.readyState <= WebSocket.OPEN) return

    setSocketState('connecting')
    const socket = new WebSocket('ws://127.0.0.1:3000/ws')
    socketRef.current = socket

    socket.onopen = () => {
      setSocketState('open')
    }

    socket.onmessage = (event) => {
      const editor = editorRef.current
      if (!editor) return

      const update = parseGraphUpdateMessage(String(event.data))
      for (const node of [...update.addedNodes, ...update.updatedNodes]) {
        const shape = nodeToShape(node)
        if (!shape) continue

        const existingShape = editor.getShape(shape.id)
        try {
          if (existingShape) {
            editor.updateShape(shape)
          } else {
            editor.createShape(shape)
          }
        } catch (error) {
          console.error('Failed to apply WebSocket graph update', node, error)
        }
      }
    }

    socket.onerror = (error) => {
      console.error('WebSocket connection error', error)
    }

    socket.onclose = () => {
      setSocketState('closed')
      socketRef.current = null
      reconnectTimerRef.current = setTimeout(() => {
        connectWebSocket()
      }, 1500)
    }
  }

  const sendDebugCreateNode = () => {
    const socket = socketRef.current
    if (!socket || socket.readyState !== WebSocket.OPEN) return
    socket.send(createDebugCreateNodeCommand())
  }

  const handleMount = (editor: any) => {
    if (isLoadingRef.current || hasLoadedRef.current) return
    editorRef.current = editor
    connectWebSocket()

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
      <div
        style={{
          position: 'absolute',
          top: 16,
          right: 16,
          zIndex: 1000,
          display: 'flex',
          gap: 12,
          alignItems: 'center',
          padding: '10px 12px',
          background: 'rgba(15, 23, 42, 0.85)',
          color: '#f8fafc',
          borderRadius: 12,
          boxShadow: '0 10px 30px rgba(15, 23, 42, 0.22)',
          backdropFilter: 'blur(8px)',
        }}
      >
        <span style={{ fontSize: 12 }}>WS: {socketState}</span>
        <button type="button" onClick={sendDebugCreateNode} disabled={socketState !== 'open'}>
          推送测试节点
        </button>
      </div>
      <Tldraw onMount={handleMount} />
    </div>
  )
}

export default App
