import { Tldraw, defaultShapeUtils, type Editor } from 'tldraw'
import 'tldraw/tldraw.css'
import { useCallback, useRef } from 'react'
import { usePersistence } from '../../hooks/usePersistence'
import { useWebSocket } from '../../hooks/useWebSocket'
import { RelationEdgeUtil } from './edges'
import { customShapeUtils } from './shapes'
import { NodeTool } from './tools/NodeTool'
import { RelationTool } from './tools/RelationTool'
import { TraversalPanel } from '../panels/TraversalPanel'
import { PropertyPanel } from '../panels/PropertyPanel'
import { AiPanel } from '../panels/AiPanel'
import ExportPanel from '../panels/ExportPanel'
import { useInvestigationStore } from '../../stores/investigation'

export function CyberCanvas() {
  const editorRef = useRef<Editor | null>(null)
  const setEditor = useInvestigationStore((s) => s.setEditor)

  // WebSocket 连接（后续 Phase 完整实现）
  useWebSocket()

  // 画布 ↔ DB 持久化同步
  usePersistence(editorRef)

  const handleMount = useCallback((editor: Editor) => {
    editorRef.current = editor
    setEditor(editor)
  }, [setEditor])

  return (
    <div style={{ position: 'fixed', inset: 0 }}>
      <Tldraw
        onMount={handleMount}
        shapeUtils={[...defaultShapeUtils, ...customShapeUtils, RelationEdgeUtil]}
      />
      <AiPanel />
      <NodeTool />
      <RelationTool />
      <TraversalPanel />
      <PropertyPanel />
      <ExportPanel />
    </div>
  )
}
