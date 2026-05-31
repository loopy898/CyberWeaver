import { useCallback, useEffect, useState } from 'react'
import { useInvestigationStore } from '../../stores/investigation'
import { useTraversal, type TraversalPathData } from '../../hooks/useTraversal'
import { ensureShapeId } from '../../canvasNodes'
import { RELATION_LABELS } from '../../lib/constants'

/** All known relation types for the optional filter dropdown. */
const ALL_RELATION_TYPES = Object.keys(RELATION_LABELS)

export function TraversalPanel() {
  const editor = useInvestigationStore((s) => s.editor)
  const currentId = useInvestigationStore((s) => s.currentId)
  const { expandNode, findPath } = useTraversal()

  // Panel state
  const [isOpen, setIsOpen] = useState(false)

  // Input state
  const [startNodeId, setStartNodeId] = useState('')
  const [targetNodeId, setTargetNodeId] = useState('')
  const [maxHops, setMaxHops] = useState(2)
  const [relationFilter, setRelationFilter] = useState('')

  // Results
  const [paths, setPaths] = useState<TraversalPathData[]>([])
  const [totalHops, setTotalHops] = useState<number[]>([])
  const [shortestPath, setShortestPath] = useState<TraversalPathData | null>(null)
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState('')
  const [activeMode, setActiveMode] = useState<'expand' | 'shortest'>('expand')

  // Auto-fill start node from canvas selection
  useEffect(() => {
    if (!editor) return
    const selectedIds = editor.getSelectedShapeIds()
    if (selectedIds.length === 1) {
      // Strip "shape:" prefix to get the raw DB node ID
      const rawId = selectedIds[0].startsWith('shape:')
        ? selectedIds[0].slice('shape:'.length)
        : selectedIds[0]
      setStartNodeId(rawId)
    }
  }, [editor])

  const investigationId = currentId ?? 'default'

  const handleExpand = useCallback(async () => {
    if (!startNodeId.trim()) {
      setError('请输入起始节点 ID')
      return
    }
    setError('')
    setIsLoading(true)
    setPaths([])
    setShortestPath(null)

    try {
      const result = await expandNode(
        investigationId,
        startNodeId.trim(),
        maxHops,
        relationFilter || undefined,
      )
      setPaths(result.paths)
      setTotalHops(result.total_hops)
    } catch (e) {
      setError(String(e))
    } finally {
      setIsLoading(false)
    }
  }, [investigationId, startNodeId, maxHops, relationFilter, expandNode])

  const handleFindPath = useCallback(async () => {
    if (!startNodeId.trim() || !targetNodeId.trim()) {
      setError('请输入起始节点和目标节点 ID')
      return
    }
    setError('')
    setIsLoading(true)
    setPaths([])
    setShortestPath(null)

    try {
      const result = await findPath(
        investigationId,
        startNodeId.trim(),
        targetNodeId.trim(),
        maxHops,
      )
      setShortestPath(result)
    } catch (e) {
      setError(String(e))
    } finally {
      setIsLoading(false)
    }
  }, [investigationId, startNodeId, targetNodeId, maxHops, findPath])

  /** Click a path to select its nodes on the canvas. */
  const handlePathClick = useCallback(
    (path: TraversalPathData) => {
      if (!editor) return
      const shapeIds = path.node_ids.map(ensureShapeId) as unknown as Parameters<typeof editor.select>
      editor.select(...shapeIds)
      // Zoom to fit the selected nodes
      editor.zoomToSelection({ animation: { duration: 300 } })
    },
    [editor],
  )

  if (!editor) return null

  return (
    <div
      style={{
        position: 'absolute',
        top: 64,
        left: 16,
        zIndex: 999,
        display: 'flex',
        flexDirection: 'column',
        gap: 4,
        maxHeight: 'calc(100vh - 100px)',
        overflow: 'hidden',
      }}
    >
      {/* Toggle button */}
      <button
        onClick={() => setIsOpen(!isOpen)}
        style={{
          width: 44,
          height: 44,
          borderRadius: 10,
          border: 'none',
          background: 'rgba(15, 23, 42, 0.9)',
          color: '#f8fafc',
          fontSize: 18,
          cursor: 'pointer',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          boxShadow: '0 4px 12px rgba(15, 23, 42, 0.15)',
        }}
        title="图遍历"
      >
        {isOpen ? '✕' : '⊿'}
      </button>

      {isOpen && (
        <div
          style={{
            width: 320,
            borderRadius: 10,
            background: 'rgba(15, 23, 42, 0.92)',
            boxShadow: '0 4px 20px rgba(15, 23, 42, 0.3)',
            backdropFilter: 'blur(8px)',
            padding: 12,
            display: 'flex',
            flexDirection: 'column',
            gap: 8,
            color: '#e2e8f0',
            fontSize: 12,
            maxHeight: 'calc(100vh - 160px)',
            overflowY: 'auto',
          }}
        >
          {/* Title */}
          <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 2 }}>
            图遍历
          </div>

          {/* Mode selector */}
          <div style={{ display: 'flex', gap: 4 }}>
            <button
              onClick={() => setActiveMode('expand')}
              style={modeBtnStyle(activeMode === 'expand')}
            >
              展开邻域
            </button>
            <button
              onClick={() => setActiveMode('shortest')}
              style={modeBtnStyle(activeMode === 'shortest')}
            >
              最短路径
            </button>
          </div>

          {/* Start node input */}
          <label style={labelStyle}>
            起始节点 ID
            <input
              value={startNodeId}
              onChange={(e) => setStartNodeId(e.target.value)}
              placeholder="点击画布节点或输入 ID"
              style={inputStyle}
            />
          </label>

          {/* Target node input (only for shortest path mode) */}
          {activeMode === 'shortest' && (
            <label style={labelStyle}>
              目标节点 ID
              <input
                value={targetNodeId}
                onChange={(e) => setTargetNodeId(e.target.value)}
                placeholder="输入目标节点 ID"
                style={inputStyle}
              />
            </label>
          )}

          {/* Max hops */}
          <label style={labelStyle}>
            最大跳数
            <div style={{ display: 'flex', gap: 4 }}>
              {[1, 2, 3, 4].map((n) => (
                <button
                  key={n}
                  onClick={() => setMaxHops(n)}
                  style={chipStyle(maxHops === n)}
                >
                  {n}
                </button>
              ))}
            </div>
          </label>

          {/* Relation type filter */}
          {activeMode === 'expand' && (
            <label style={labelStyle}>
              关系类型过滤（可选）
              <select
                value={relationFilter}
                onChange={(e) => setRelationFilter(e.target.value)}
                style={inputStyle}
              >
                <option value="">全部</option>
                {ALL_RELATION_TYPES.map((rt) => (
                  <option key={rt} value={rt}>
                    {RELATION_LABELS[rt] || rt}
                  </option>
                ))}
              </select>
            </label>
          )}

          {/* Action button */}
          <button
            onClick={activeMode === 'expand' ? handleExpand : handleFindPath}
            disabled={isLoading}
            style={{
              padding: '8px 0',
              border: 'none',
              borderRadius: 6,
              background: isLoading ? '#475569' : '#3B82F6',
              color: '#fff',
              fontSize: 13,
              fontWeight: 600,
              cursor: isLoading ? 'not-allowed' : 'pointer',
              transition: 'background 0.15s',
            }}
          >
            {isLoading
              ? '查询中...'
              : activeMode === 'expand'
                ? '展开'
                : '查找路径'}
          </button>

          {/* Error */}
          {error && (
            <div style={{ color: '#f87171', fontSize: 11 }}>{error}</div>
          )}

          {/* Shortest path result */}
          {shortestPath && (
            <div>
              <div style={{ fontWeight: 600, fontSize: 12, marginBottom: 4 }}>
                最短路径 ({shortestPath.node_ids.length} 个节点)
              </div>
              <PathCard
                path={shortestPath}
                onClick={() => handlePathClick(shortestPath)}
              />
            </div>
          )}

          {/* BFS results */}
          {paths.length > 0 && (
            <div>
              <div
                style={{ fontWeight: 600, fontSize: 12, marginBottom: 4 }}
              >
                邻域路径 ({paths.length} 条)
              </div>
              <div
                style={{
                  display: 'flex',
                  flexDirection: 'column',
                  gap: 4,
                }}
              >
                {paths.map((path, i) => (
                  <PathCard
                    key={i}
                    path={path}
                    hopCount={totalHops[i]}
                    onClick={() => handlePathClick(path)}
                  />
                ))}
              </div>
            </div>
          )}

          {/* Empty state */}
          {!isLoading &&
            !error &&
            paths.length === 0 &&
            !shortestPath && (
              <div style={{ color: '#94a3b8', fontSize: 11, textAlign: 'center', padding: 8 }}>
                点击「展开」或「查找路径」开始图遍历
              </div>
            )}
        </div>
      )}
    </div>
  )
}

// ---------------------------------------------------------------------------
// Sub-components
// ---------------------------------------------------------------------------

function PathCard({
  path,
  hopCount,
  onClick,
}: {
  path: TraversalPathData
  hopCount?: number
  onClick: () => void
}) {
  return (
    <div
      onClick={onClick}
      style={{
        padding: '6px 8px',
        borderRadius: 6,
        background: 'rgba(255, 255, 255, 0.06)',
        cursor: 'pointer',
        border: '1px solid rgba(255, 255, 255, 0.08)',
        transition: 'background 0.15s',
      }}
      onMouseEnter={(e) => {
        e.currentTarget.style.background = 'rgba(59, 130, 246, 0.15)'
      }}
      onMouseLeave={(e) => {
        e.currentTarget.style.background = 'rgba(255, 255, 255, 0.06)'
      }}
    >
      {/* Node sequence with relation arrows */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          flexWrap: 'wrap',
          gap: 2,
          fontSize: 11,
          fontFamily: 'monospace',
        }}
      >
        {path.node_ids.map((nodeId, idx) => (
          <span key={idx} style={{ display: 'flex', alignItems: 'center', gap: 2 }}>
            <span
              style={{
                color: idx === 0 ? '#60a5fa' : idx === path.node_ids.length - 1 ? '#fbbf24' : '#e2e8f0',
                fontWeight: idx === 0 ? 600 : 400,
              }}
              title={nodeId}
            >
              {ellipsis(nodeId)}
            </span>
            {idx < path.node_ids.length - 1 && (
              <span
                style={{
                  color: '#94a3b8',
                  fontSize: 10,
                }}
                title={path.relation_types[idx] ?? ''}
              >
                {relationArrow(path.relation_types[idx] ?? '')}
              </span>
            )}
          </span>
        ))}
      </div>
      {/* Hop count badge */}
      {hopCount !== undefined && (
        <span
          style={{
            display: 'inline-block',
            marginTop: 4,
            padding: '1px 6px',
            borderRadius: 4,
            background: 'rgba(59, 130, 246, 0.2)',
            color: '#93c5fd',
            fontSize: 10,
          }}
        >
          {hopCount} 跳
        </span>
      )}
    </div>
  )
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function ellipsis(id: string, maxLen = 8): string {
  return id.length > maxLen ? id.slice(0, maxLen) + '..' : id
}

function relationArrow(relType: string): string {
  const label = RELATION_LABELS[relType]
  return label ? `--${label}-->` : `--${relType}-->`
}

// ---------------------------------------------------------------------------
// Style factories
// ---------------------------------------------------------------------------

const labelStyle: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: 4,
  fontSize: 11,
  color: '#94a3b8',
}

const inputStyle: React.CSSProperties = {
  padding: '6px 8px',
  borderRadius: 6,
  border: '1px solid rgba(255, 255, 255, 0.12)',
  background: 'rgba(255, 255, 255, 0.06)',
  color: '#e2e8f0',
  fontSize: 12,
  outline: 'none',
  width: '100%',
  boxSizing: 'border-box',
}

function modeBtnStyle(active: boolean): React.CSSProperties {
  return {
    flex: 1,
    padding: '6px 0',
    border: 'none',
    borderRadius: 6,
    background: active ? 'rgba(59, 130, 246, 0.25)' : 'rgba(255, 255, 255, 0.06)',
    color: active ? '#60a5fa' : '#94a3b8',
    fontSize: 12,
    cursor: 'pointer',
    fontWeight: active ? 600 : 400,
    transition: 'background 0.15s',
  }
}

function chipStyle(active: boolean): React.CSSProperties {
  return {
    padding: '4px 10px',
    border: 'none',
    borderRadius: 6,
    background: active ? '#3B82F6' : 'rgba(255, 255, 255, 0.1)',
    color: active ? '#fff' : '#94a3b8',
    fontSize: 12,
    cursor: 'pointer',
    fontWeight: active ? 600 : 400,
    transition: 'background 0.15s',
  }
}
