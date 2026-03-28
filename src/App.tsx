import { invoke } from '@tauri-apps/api/core'
import { useEffect, useRef, useState } from 'react'
import { Tldraw } from 'tldraw'
import 'tldraw/tldraw.css'
import './App.css'
import { isPersistableShape, nodeToShape, shapeToNode, toStorageId } from './canvasNodes.ts'
import { type PersistedNode } from './graphNodes.ts'
import { computeLayout } from './layout.ts'
import {
  buildReportSummaryText,
  createDebugCreateNodeCommand,
  createNodeSearchIndex,
  createToolExecutionCommand,
  type AgentTokenEvent,
  type ForensicsReport,
  parseAgentTokenMessage,
  parseForensicsReport,
  parseGraphSnapshot,
  parseGraphUpdateMessage,
  parseToolResultMessage,
  type GraphEdge,
} from './ws.ts'

const THOUGHT_NODE_ID = 'agent-thought'

type TimelineEntry = {
  id: string
  kind: 'status' | 'tool' | 'token' | 'graph'
  text: string
  timestamp: string
}

function App() {
  const isLoadingRef = useRef(false)
  const hasLoadedRef = useRef(false)
  const editorRef = useRef<any>(null)
  const graphNodesRef = useRef<Map<string, PersistedNode>>(new Map())
  const graphEdgesRef = useRef<Map<string, GraphEdge>>(new Map())
  const thoughtBufferRef = useRef<string[]>([])
  const isApplyingRemoteRef = useRef(false)
  const snapshotTimerRef = useRef<ReturnType<typeof setInterval> | null>(null)
  const zoomRef = useRef(1)
  const socketRef = useRef<WebSocket | null>(null)
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const [socketState, setSocketState] = useState<'connecting' | 'open' | 'closed'>('connecting')
  const [toolStatus, setToolStatus] = useState('待命')
  const [zoomLevel, setZoomLevel] = useState(1)
  const [graphStats, setGraphStats] = useState({ nodes: 0, edges: 0 })
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; targetId: string } | null>(null)
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null)
  const [timeline, setTimeline] = useState<TimelineEntry[]>([])
  const [report, setReport] = useState<ForensicsReport | null>(null)
  const [searchQuery, setSearchQuery] = useState('')
  const [timestampInput, setTimestampInput] = useState('')
  const [latitudeInput, setLatitudeInput] = useState('')
  const [longitudeInput, setLongitudeInput] = useState('')

  useEffect(() => {
    return () => {
      if (reconnectTimerRef.current) clearTimeout(reconnectTimerRef.current)
      if (snapshotTimerRef.current) clearInterval(snapshotTimerRef.current)
      socketRef.current?.close()
    }
  }, [])

  useEffect(() => {
    if (!contextMenu) return

    const closeMenu = () => setContextMenu(null)
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setContextMenu(null)
    }

    window.addEventListener('click', closeMenu)
    window.addEventListener('keydown', closeOnEscape)
    return () => {
      window.removeEventListener('click', closeMenu)
      window.removeEventListener('keydown', closeOnEscape)
    }
  }, [contextMenu])

  const appendTimeline = (kind: TimelineEntry['kind'], text: string) => {
    setTimeline((current) => {
      const next = [
        {
          id: `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
          kind,
          text,
          timestamp: new Date().toLocaleTimeString('zh-CN', { hour12: false }),
        },
        ...current,
      ]
      return next.slice(0, 10)
    })
  }

  const selectedNode = selectedNodeId ? graphNodesRef.current.get(selectedNodeId) ?? null : null
  const selectedNodeEdges = selectedNodeId
    ? [...graphEdgesRef.current.values()].filter(
        (edge) => edge.sourceId === selectedNodeId || edge.targetId === selectedNodeId
      )
    : []
  const filteredNodes = createNodeSearchIndex([...graphNodesRef.current.values()], searchQuery).slice(0, 6)

  const renderGraphState = async () => {
    const editor = editorRef.current
    if (!editor) return

    const nodes = [...graphNodesRef.current.values()]
    const edges = [...graphEdgesRef.current.values()]
    setGraphStats({ nodes: nodes.length, edges: edges.length })
    const positioned = await computeLayout(nodes, edges)

    const isCompact = zoomRef.current < 0.5
    isApplyingRemoteRef.current = true
    try {
      for (const node of positioned) {
        const normalizedNode: PersistedNode = isCompact
          ? { ...node, type: 'geo', content: '' }
          : { ...node, type: 'note' }
        const shape = nodeToShape(normalizedNode)
        if (!shape) continue
        const shapeId = `shape:${normalizedNode.id}`
        const renderShape = {
          ...shape,
          id: shapeId,
        }

        const existing = editor.getShape(shapeId)
        if (existing) {
          editor.updateShape(renderShape)
        } else {
          editor.createShape(renderShape)
        }
      }
    } finally {
      isApplyingRemoteRef.current = false
    }
  }

  const upsertThoughtBubble = (token: string) => {
    thoughtBufferRef.current.push(token)
    const merged = thoughtBufferRef.current.slice(-8).join('\n')
    const node: PersistedNode = {
      id: THOUGHT_NODE_ID,
      type: 'note',
      x: 60,
      y: 60,
      content: `Agent 思考中...\n${merged}`,
    }
    graphNodesRef.current.set(node.id, node)
    void renderGraphState()
  }

  const connectWebSocket = () => {
    if (socketRef.current && socketRef.current.readyState <= WebSocket.OPEN) return

    setSocketState('connecting')
    const socket = new WebSocket('ws://127.0.0.1:3000/ws')
    socketRef.current = socket

    socket.onopen = () => {
      setSocketState('open')
      setToolStatus('WebSocket 已连接，图谱同步在线')
      appendTimeline('status', 'WebSocket 已连接，开始同步图谱')
      loadGraphSnapshot()
        .then(() => renderGraphState())
        .catch((error) => console.warn('Initial graph snapshot on open failed', error))
    }

    socket.onmessage = (event) => {
      const editor = editorRef.current
      if (!editor) return

      const raw = String(event.data)
      const toolResult = parseToolResultMessage(raw)
      if (toolResult) {
        setToolStatus(`${toolResult.tool}: ${toolResult.ok ? '成功' : '失败'} - ${toolResult.message}`)
        appendTimeline('tool', `${toolResult.tool}: ${toolResult.message}`)
        return
      }

      const tokenEvent: AgentTokenEvent | null = parseAgentTokenMessage(raw)
      if (tokenEvent) {
        upsertThoughtBubble(tokenEvent.token)
        appendTimeline('token', tokenEvent.token)
        return
      }

      const update = parseGraphUpdateMessage(raw)
      for (const node of [...update.addedNodes, ...update.updatedNodes]) {
        graphNodesRef.current.set(node.id, node)
      }
      for (const edge of [...update.addedEdges, ...update.updatedEdges]) {
        graphEdgesRef.current.set(edge.id, edge)
      }
      if (
        update.addedNodes.length > 0 ||
        update.updatedNodes.length > 0 ||
        update.addedEdges.length > 0 ||
        update.updatedEdges.length > 0
      ) {
        appendTimeline(
          'graph',
          `图谱更新：节点 +${update.addedNodes.length} / 边 +${update.addedEdges.length}`
        )
      }
      void renderGraphState()
    }

    socket.onerror = (error) => {
      console.error('WebSocket connection error', error)
    }

    socket.onclose = () => {
      setSocketState('closed')
      setToolStatus('WebSocket 已断开，准备重连')
      appendTimeline('status', 'WebSocket 已断开，系统将在 1.5 秒后重连')
      socketRef.current = null
      reconnectTimerRef.current = setTimeout(() => {
        connectWebSocket()
      }, 1500)
    }
  }

  const pollGraphSnapshot = () => {
    if (snapshotTimerRef.current) clearInterval(snapshotTimerRef.current)
    snapshotTimerRef.current = setInterval(() => {
      loadGraphSnapshot()
        .then(() => renderGraphState())
        .catch((error) => {
          console.warn('Graph snapshot refresh failed', error)
        })
    }, 3000)
  }

  const sendDebugCreateNode = () => {
    const socket = socketRef.current
    if (!socket || socket.readyState !== WebSocket.OPEN) return
    socket.send(createDebugCreateNodeCommand())
    setToolStatus('已发送调试节点创建指令')
    appendTimeline('status', '发送调试节点创建指令')
  }

  const sendScanPort = (targetIdArg?: string) => {
    const socket = socketRef.current
    const editor = editorRef.current
    if (!socket || socket.readyState !== WebSocket.OPEN || !editor) return
    const selectedIds = editor.getSelectedShapeIds()
    const first = selectedIds[0]
    const targetId = targetIdArg ?? (first ? toStorageId(String(first)) : '')
    if (!targetId) {
      setToolStatus('请选择一个节点后再执行 ScanPort')
      return
    }
    socket.send(createToolExecutionCommand('scan_port', { target_id: targetId }))
    setToolStatus(`已发送 scan_port -> ${targetId}`)
    appendTimeline('tool', `发送 scan_port 指令，目标 ${targetId}`)
    setContextMenu(null)
  }

  const sendTimestampConvert = () => {
    const socket = socketRef.current
    if (!socket || socket.readyState !== WebSocket.OPEN) {
      setToolStatus('WebSocket 未连接，无法执行 timestamp_convert')
      return
    }

    const value = timestampInput.trim()
    if (!value) {
      setToolStatus('请输入要转换的时间戳')
      return
    }

    socket.send(createToolExecutionCommand('timestamp_convert', { value }))
    setToolStatus(`已发送 timestamp_convert -> ${value}`)
    appendTimeline('tool', `发送 timestamp_convert 指令，值 ${value}`)
  }

  const sendReverseGeocode = () => {
    const socket = socketRef.current
    if (!socket || socket.readyState !== WebSocket.OPEN) {
      setToolStatus('WebSocket 未连接，无法执行 reverse_geocode')
      return
    }

    const latitude = Number.parseFloat(latitudeInput.trim())
    const longitude = Number.parseFloat(longitudeInput.trim())
    if (!Number.isFinite(latitude) || !Number.isFinite(longitude)) {
      setToolStatus('请输入有效的经纬度数字')
      return
    }
    if (latitude < -90 || latitude > 90) {
      setToolStatus('纬度必须在 -90 到 90 之间')
      return
    }
    if (longitude < -180 || longitude > 180) {
      setToolStatus('经度必须在 -180 到 180 之间')
      return
    }

    socket.send(createToolExecutionCommand('reverse_geocode', { latitude, longitude }))
    setToolStatus(`已发送 reverse_geocode -> ${latitude}, ${longitude}`)
    appendTimeline('tool', `发送 reverse_geocode 指令，坐标 ${latitude}, ${longitude}`)
  }

  const openContextScanMenu = (event: React.MouseEvent<HTMLDivElement>) => {
    const editor = editorRef.current
    if (!editor) return

    const selectedIds = editor.getSelectedShapeIds()
    const first = selectedIds[0]
    if (!first) {
      setContextMenu(null)
      return
    }

    event.preventDefault()
    setContextMenu({
      x: event.clientX,
      y: event.clientY,
      targetId: toStorageId(String(first)),
    })
  }

  const refreshGraphSnapshot = async () => {
    try {
      await loadGraphSnapshot()
      await loadForensicsReport()
      await renderGraphState()
      setToolStatus('图谱快照刷新完成')
      appendTimeline('status', '手动刷新图谱快照完成')
    } catch (error) {
      console.warn('Manual snapshot refresh failed', error)
      setToolStatus('图谱快照刷新失败')
      appendTimeline('status', '手动刷新图谱快照失败')
    }
  }

  const addSeedGraphData = async () => {
    const seedA: PersistedNode = {
      id: 'seed-host',
      type: 'note',
      x: 180,
      y: 160,
      content: 'Host: 192.168.1.10',
    }
    const seedB: PersistedNode = {
      id: 'seed-proc',
      type: 'note',
      x: 420,
      y: 180,
      content: 'Process: sshd',
    }

    graphNodesRef.current.set(seedA.id, seedA)
    graphNodesRef.current.set(seedB.id, seedB)
    graphEdgesRef.current.set('seed-edge-1', {
      id: 'seed-edge-1',
      sourceId: seedA.id,
      targetId: seedB.id,
      relation: 'spawned',
    })

    await Promise.all([
      invoke('save_node', { node: seedA }),
      invoke('save_node', { node: seedB }),
      fetch('http://127.0.0.1:3000/edge', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          source_id: seedA.id,
          target_id: seedB.id,
          relation: 'spawned',
          properties: { source: 'seed' },
        }),
      }).catch((error) => {
        console.warn('Failed to seed edge', error)
      }),
    ])
    await renderGraphState()
  }

  const loadGraphSnapshot = async () => {
    const response = await fetch('http://127.0.0.1:3000/graph')
    if (!response.ok) {
      throw new Error(`graph snapshot request failed: ${response.status}`)
    }
    const raw = await response.text()
    const snapshot = parseGraphSnapshot(raw)

    graphNodesRef.current.clear()
    graphEdgesRef.current.clear()
    for (const node of snapshot.nodes ?? []) {
      graphNodesRef.current.set(node.id, node)
    }
    for (const edge of snapshot.edges ?? []) {
      graphEdgesRef.current.set(edge.id, edge)
    }
  }

  const loadForensicsReport = async () => {
    const response = await fetch('http://127.0.0.1:3000/report')
    if (!response.ok) {
      throw new Error(`forensics report request failed: ${response.status}`)
    }
    const raw = await response.text()
    setReport(parseForensicsReport(raw))
  }

  const exportForensicsReport = async () => {
    try {
      const latest = await fetch('http://127.0.0.1:3000/report')
      if (!latest.ok) {
        throw new Error(`forensics report export failed: ${latest.status}`)
      }
      const parsed = parseForensicsReport(await latest.text())
      setReport(parsed)
      const blob = new Blob([parsed.markdown], { type: 'text/markdown;charset=utf-8' })
      const url = URL.createObjectURL(blob)
      const anchor = document.createElement('a')
      anchor.href = url
      anchor.download = `cyberweaver-report-${Date.now()}.md`
      anchor.click()
      URL.revokeObjectURL(url)
      setToolStatus('取证报告已导出')
      appendTimeline('status', '导出自动化取证报告')
    } catch (error) {
      console.warn('Failed to export forensics report', error)
      setToolStatus('取证报告导出失败')
      appendTimeline('status', '取证报告导出失败')
    }
  }

  const focusNodeFromSearch = async (targetId: string) => {
    const editor = editorRef.current
    const targetNode = graphNodesRef.current.get(targetId)
    if (!editor || !targetNode) return

    setSelectedNodeId(targetId)
    editor.select(`shape:${targetId}`)
    editor.zoomToBounds(
      {
        x: targetNode.x - 120,
        y: targetNode.y - 80,
        w: 260,
        h: 180,
      },
      {
        animation: {
          duration: 220,
        },
      }
    )
    setToolStatus(`已聚焦节点 ${targetId}`)
    appendTimeline('status', `从检索结果聚焦节点 ${targetId}`)
  }

  const handleMount = (editor: any) => {
    if (isLoadingRef.current || hasLoadedRef.current) return
    editorRef.current = editor
    connectWebSocket()
    pollGraphSnapshot()
    editor.on('selection-change', () => {
      const selectedIds = editor.getSelectedShapeIds()
      const first = selectedIds[0]
      setSelectedNodeId(first ? toStorageId(String(first)) : null)
    })
    editor.on('zoom', () => {
      zoomRef.current = editor.getZoomLevel()
      setZoomLevel(zoomRef.current)
      void renderGraphState()
    })

    const loadFromDb = async () => {
      isLoadingRef.current = true
      try {
        await loadGraphSnapshot()
        await loadForensicsReport()
        if (graphNodesRef.current.size === 0) {
          await addSeedGraphData()
          await loadForensicsReport()
        }
        for (const node of graphNodesRef.current.values()) {
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
        void renderGraphState()
      }
    }

    loadFromDb()

    let saveTimeout: ReturnType<typeof setTimeout> | undefined

    editor.store.listen((entry: any) => {
      if (isLoadingRef.current || !hasLoadedRef.current || isApplyingRemoteRef.current) return

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
    <div className="cw-shell" onClick={() => setContextMenu(null)}>
      <div className="cw-bg" aria-hidden="true">
        <div className="cw-orb cw-orb-a" />
        <div className="cw-orb cw-orb-b" />
        <div className="cw-grid" />
      </div>

      <header className="cw-toolbar">
        <div className="cw-brand">
          <strong>CyberWeaver</strong>
          <span>自动化取证编织台</span>
        </div>
        <div className="cw-meta">
          <span className={`cw-chip cw-chip-${socketState}`}>WS {socketState}</span>
          <span className="cw-chip">Zoom {zoomLevel.toFixed(2)}</span>
          <span className="cw-chip">Nodes {graphStats.nodes}</span>
          <span className="cw-chip">Edges {graphStats.edges}</span>
        </div>
        <div className="cw-actions">
          <div className="cw-tool-input-row">
            <input
              className="cw-tool-input"
              value={timestampInput}
              onChange={(event) => setTimestampInput(event.target.value)}
              placeholder="时间戳 / Cocoa 值"
            />
            <button
              type="button"
              className="cw-btn"
              onClick={() => sendTimestampConvert()}
              disabled={socketState !== 'open'}
            >
              转换时间戳
            </button>
          </div>
          <div className="cw-tool-input-row">
            <input
              className="cw-tool-input cw-tool-input-short"
              value={latitudeInput}
              onChange={(event) => setLatitudeInput(event.target.value)}
              placeholder="纬度"
            />
            <input
              className="cw-tool-input cw-tool-input-short"
              value={longitudeInput}
              onChange={(event) => setLongitudeInput(event.target.value)}
              placeholder="经度"
            />
            <button
              type="button"
              className="cw-btn"
              onClick={() => sendReverseGeocode()}
              disabled={socketState !== 'open'}
            >
              经纬度定位
            </button>
          </div>
          <button
            type="button"
            className="cw-btn cw-btn-primary"
            onClick={() => sendDebugCreateNode()}
            disabled={socketState !== 'open'}
          >
            推送测试节点
          </button>
          <button
            type="button"
            className="cw-btn cw-btn-alert"
            onClick={() => sendScanPort()}
            disabled={socketState !== 'open'}
          >
            ScanPort
          </button>
          <button type="button" className="cw-btn" onClick={() => void refreshGraphSnapshot()}>
            刷新快照
          </button>
          <button type="button" className="cw-btn" onClick={() => void exportForensicsReport()}>
            导出报告
          </button>
        </div>
      </header>

      <section className="cw-status">
        <span className="cw-status-label">状态</span>
        <span className="cw-status-text">{toolStatus}</span>
      </section>

      <aside className="cw-sidepanel cw-sidepanel-right">
        <div className="cw-panel-card">
          <div className="cw-panel-head">
            <strong>节点详情</strong>
            <span>{selectedNode ? selectedNode.id : '未选中'}</span>
          </div>
          {selectedNode ? (
            <div className="cw-detail-grid">
              <div className="cw-detail-item">
                <span>类型</span>
                <strong>{selectedNode.type}</strong>
              </div>
              <div className="cw-detail-item">
                <span>坐标</span>
                <strong>
                  {selectedNode.x.toFixed(0)}, {selectedNode.y.toFixed(0)}
                </strong>
              </div>
              <div className="cw-detail-item cw-detail-wide">
                <span>内容</span>
                <strong>{selectedNode.content || '无内容'}</strong>
              </div>
              <div className="cw-detail-item cw-detail-wide">
                <span>关联边</span>
                <strong>{selectedNodeEdges.length > 0 ? selectedNodeEdges.map((edge) => `${edge.relation} (${edge.sourceId} → ${edge.targetId})`).join(' / ') : '暂无'}</strong>
              </div>
            </div>
          ) : (
            <div className="cw-empty-state">在画布上选中一个节点后，这里会显示取证上下文。</div>
          )}
        </div>
        <div className="cw-panel-card cw-panel-gap">
          <div className="cw-panel-head">
            <strong>图谱检索</strong>
            <span>{filteredNodes.length} 条命中</span>
          </div>
          <label className="cw-search">
            <span>关键词</span>
            <input
              value={searchQuery}
              onChange={(event) => setSearchQuery(event.target.value)}
              placeholder="节点 ID、内容、端口"
            />
          </label>
          <div className="cw-search-results">
            {filteredNodes.length > 0 ? (
              filteredNodes.map((node) => (
                <button
                  key={node.id}
                  type="button"
                  className="cw-search-item"
                  onClick={() => void focusNodeFromSearch(node.id)}
                >
                  <strong>{node.id}</strong>
                  <span>{node.content || node.type}</span>
                </button>
              ))
            ) : (
              <div className="cw-empty-state">没有匹配的节点。</div>
            )}
          </div>
        </div>
      </aside>

      <aside className="cw-sidepanel cw-sidepanel-left">
        <div className="cw-panel-card">
          <div className="cw-panel-head">
            <strong>事件时间线</strong>
            <span>最近 10 条</span>
          </div>
          <div className="cw-timeline">
            {timeline.length > 0 ? (
              timeline.map((entry) => (
                <div key={entry.id} className={`cw-timeline-item cw-timeline-${entry.kind}`}>
                  <span className="cw-timeline-time">{entry.timestamp}</span>
                  <span className="cw-timeline-text">{entry.text}</span>
                </div>
              ))
            ) : (
              <div className="cw-empty-state">等待图谱与工具事件。</div>
            )}
          </div>
        </div>
        <div className="cw-panel-card cw-panel-gap">
          <div className="cw-panel-head">
            <strong>取证报告</strong>
            <span>{report ? `${report.summary.findingCount} 项发现` : '未生成'}</span>
          </div>
          {report ? (
            <div className="cw-report">
              <div className="cw-report-summary">{buildReportSummaryText(report)}</div>
              <div className="cw-report-findings">
                {report.findings.length > 0 ? (
                  report.findings.slice(0, 4).map((finding) => (
                    <button
                      key={`${finding.nodeId}-${finding.relation}`}
                      type="button"
                      className="cw-report-finding"
                      onClick={() => void focusNodeFromSearch(finding.nodeId)}
                    >
                      <strong>{finding.title}</strong>
                      <span>{finding.evidence}</span>
                    </button>
                  ))
                ) : (
                  <div className="cw-empty-state">当前暂无结构化发现。</div>
                )}
              </div>
            </div>
          ) : (
            <div className="cw-empty-state">报告尚未载入。</div>
          )}
        </div>
      </aside>

      <div className="cw-canvas" onContextMenu={openContextScanMenu}>
        <Tldraw onMount={handleMount} />
      </div>

      {contextMenu ? (
        <div
          className="cw-context-menu"
          style={{ left: contextMenu.x, top: contextMenu.y }}
          onClick={(event) => event.stopPropagation()}
        >
          <button
            type="button"
            className="cw-context-action"
            onClick={() => sendScanPort(contextMenu.targetId)}
            disabled={socketState !== 'open'}
          >
            ScanPort {contextMenu.targetId}
          </button>
        </div>
      ) : null}
    </div>
  )
}

export default App
