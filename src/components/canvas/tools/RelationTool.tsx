import { useCallback, useEffect, useRef, useState } from 'react'
import { createShapeId } from 'tldraw'
import { useInvestigationStore } from '../../../stores/investigation'
import { RELATION_LABELS, RELATION_COLORS } from '../../../lib/constants'

const RELATION_TYPES = [
  { value: 'connects', label: '连接', icon: '🔗' },
  { value: 'resolves', label: '解析', icon: '🔍' },
  { value: 'references', label: '引用', icon: '📎' },
  { value: 'spawns', label: '派生', icon: '🌱' },
  { value: 'creates', label: '创建', icon: '📝' },
  { value: 'modifies', label: '修改', icon: '✏️' },
  { value: 'belongs_to', label: '归属', icon: '🏷️' },
  { value: 'depends_on', label: '依赖', icon: '🔧' },
]

export function RelationTool() {
  const editor = useInvestigationStore((s) => s.editor)

  // ---- UI state ----
  const [mode, setMode] = useState<'idle' | 'selecting-source' | 'selecting-target'>('idle')
  const [sourceId, setSourceId] = useState<string | null>(null)
  const [relationType, setRelationType] = useState('connects')
  const [isOpen, setIsOpen] = useState(false)

  // ---- Refs to keep listener closure in sync ----
  const modeRef = useRef(mode)
  const sourceIdRef = useRef<string | null>(null)
  const relationTypeRef = useRef(relationType)

  useEffect(() => { modeRef.current = mode }, [mode])
  useEffect(() => { sourceIdRef.current = sourceId }, [sourceId])
  useEffect(() => { relationTypeRef.current = relationType }, [relationType])

  // ---- Listen for shape selection changes when in selecting mode ----
  useEffect(() => {
    if (!editor) return

    const prevSelectionRef = { current: new Set(editor.getSelectedShapeIds()) }
    let handling = false

    const unsubscribe = editor.store.listen(() => {
      if (handling) return

      const currentMode = modeRef.current
      if (currentMode === 'idle') return

      const currentIds = editor.getSelectedShapeIds()
      const prevIds = prevSelectionRef.current

      // 找出新选中的 shape id
      const newIds = currentIds.filter((id) => !prevIds.has(id))
      prevSelectionRef.current = new Set(currentIds)

      if (newIds.length === 0) return

      // 遍历新选中的 shape，跳过 arrow 类型
      for (const id of newIds) {
        const shape = editor.getShape(id)
        if (!shape || shape.type === 'arrow') continue

        handling = true
        try {
          if (currentMode === 'selecting-source') {
            // 第一步：记录源节点
            sourceIdRef.current = id
            modeRef.current = 'selecting-target'
            setSourceId(id)
            setMode('selecting-target')
            editor.selectNone()
          } else if (currentMode === 'selecting-target' && sourceIdRef.current && id !== sourceIdRef.current) {
            // 第二步：创建连线
            const arrowId = createShapeId()
            const label = RELATION_LABELS[relationTypeRef.current] || relationTypeRef.current
            const color = RELATION_COLORS[relationTypeRef.current] || '#6B7280'

            editor.createShape({
              id: arrowId,
              type: 'arrow',
              props: {
                start: {
                  type: 'binding',
                  boundShapeId: sourceIdRef.current,
                  normalizedAnchor: { x: 0.5, y: 0.5 },
                  isExact: false,
                },
                end: {
                  type: 'binding',
                  boundShapeId: id,
                  normalizedAnchor: { x: 0.5, y: 0.5 },
                  isExact: false,
                },
                text: label,
                color,
              } as any,
            })

            // 重置状态
            sourceIdRef.current = null
            modeRef.current = 'idle'
            setSourceId(null)
            setMode('idle')
            editor.selectNone()
          }
        } catch (e) {
          console.error('Failed to create relation arrow:', e)
        } finally {
          handling = false
        }

        break // 一次只处理一个选择
      }
    })

    return unsubscribe
  }, [editor])

  // ---- 用户操作 ----
  const handleStartRelation = useCallback((relType: string) => {
    setRelationType(relType)
    setMode('selecting-source')
    setIsOpen(false)
  }, [])

  const handleCancel = useCallback(() => {
    sourceIdRef.current = null
    modeRef.current = 'idle'
    setSourceId(null)
    setMode('idle')
  }, [])

  // editor 尚未挂载时不渲染
  if (!editor) return null

  return (
    <div
      style={{
        position: 'absolute',
        top: mode !== 'idle' ? 80 : 68,
        left: 16,
        zIndex: 1000,
        display: 'flex',
        flexDirection: 'column',
        gap: 4,
      }}
    >
      {/* 模式状态条 */}
      {mode !== 'idle' && (
        <div
          style={{
            padding: '8px 12px',
            borderRadius: 8,
            background: 'rgba(15, 23, 42, 0.9)',
            color: '#f8fafc',
            fontSize: 11,
            boxShadow: '0 4px 12px rgba(15, 23, 42, 0.15)',
            display: 'flex',
            alignItems: 'center',
            gap: 8,
          }}
        >
          {mode === 'selecting-source' && <span>🔍 点击源节点</span>}
          {mode === 'selecting-target' && <span>🎯 点击目标节点</span>}
          <button
            onClick={handleCancel}
            style={{
              background: '#EF444433',
              border: 'none',
              color: '#EF4444',
              borderRadius: 4,
              cursor: 'pointer',
              fontSize: 11,
              padding: '2px 8px',
            }}
          >
            取消
          </button>
        </div>
      )}

      {/* 主按钮 */}
      {mode === 'idle' && (
        <>
          <button
            onClick={() => setIsOpen(!isOpen)}
            style={{
              width: 44,
              height: 44,
              borderRadius: 10,
              border: 'none',
              background: 'rgba(15, 23, 42, 0.9)',
              color: '#f8fafc',
              fontSize: 20,
              cursor: 'pointer',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              boxShadow: '0 4px 12px rgba(15, 23, 42, 0.15)',
            }}
            title="添加关系连线"
          >
            &rarr;
          </button>

          {isOpen && (
            <div
              style={{
                display: 'flex',
                flexDirection: 'column',
                gap: 2,
                padding: 6,
                borderRadius: 10,
                background: 'rgba(15, 23, 42, 0.9)',
                boxShadow: '0 4px 20px rgba(15, 23, 42, 0.25)',
                backdropFilter: 'blur(8px)',
              }}
            >
              {RELATION_TYPES.map((rt) => (
                <button
                  key={rt.value}
                  onClick={() => handleStartRelation(rt.value)}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 8,
                    padding: '8px 12px',
                    border: 'none',
                    borderRadius: 6,
                    background: 'transparent',
                    color: '#f8fafc',
                    fontSize: 12,
                    cursor: 'pointer',
                    whiteSpace: 'nowrap',
                  }}
                >
                  <span>{rt.icon}</span>
                  <span>{rt.label}</span>
                  <span
                    style={{
                      width: 8,
                      height: 8,
                      borderRadius: '50%',
                      background: RELATION_COLORS[rt.value] || '#6B7280',
                      marginLeft: 'auto',
                    }}
                  />
                </button>
              ))}
            </div>
          )}
        </>
      )}
    </div>
  )
}
