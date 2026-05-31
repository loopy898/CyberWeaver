import { useEffect, useRef } from 'react'
import { WS_URL, WS_RECONNECT_DELAY_MS } from '../lib/constants'
import { parseGraphUpdateMessage } from '../ws'

export function useWebSocket() {
  const socketRef = useRef<WebSocket | null>(null)
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  useEffect(() => {
    const connect = () => {
      if (socketRef.current?.readyState === WebSocket.OPEN) return

      const socket = new WebSocket(WS_URL)
      socketRef.current = socket

      socket.onopen = () => {
        console.log('WebSocket connected')
      }

      socket.onmessage = (event) => {
        const message = parseGraphUpdateMessage(event.data as string)
        if (message.addedNodes.length > 0 || message.updatedNodes.length > 0) {
          console.log('WebSocket graph update received:', message)
        }
        // Phase 2+ 会实现完整的消息处理（写入 tldraw store）
      }

      socket.onclose = () => {
        console.log('WebSocket disconnected, reconnecting...')
        socketRef.current = null
        reconnectTimerRef.current = setTimeout(connect, WS_RECONNECT_DELAY_MS)
      }

      socket.onerror = (error) => {
        console.error('WebSocket error:', error)
      }
    }

    connect()

    return () => {
      socketRef.current?.close()
      if (reconnectTimerRef.current) clearTimeout(reconnectTimerRef.current)
    }
  }, [])
}
