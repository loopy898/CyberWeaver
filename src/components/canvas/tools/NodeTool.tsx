import { useCallback, useState } from 'react'
import { createShapeId } from 'tldraw'
import { useInvestigationStore } from '../../../stores/investigation'
import {
  NODE_TYPE_SHAPES,
  NODE_TYPE_LABELS,
  NODE_ICONS,
  NODE_COLORS,
} from '../../../lib/constants'

// 每种节点类型的默认 props（与 shapes/*NodeUtil.tsx 的 getDefaultProps 对齐）
const KNOWN_DEFAULT_PROPS: Record<string, Record<string, unknown>> = {
  'ip-address': {
    w: 220, h: 90, address: '', label: '', reputation: 'unknown', confidence: 1.0,
  },
  domain: {
    w: 220, h: 90, domain: '', label: '', reputation: 'unknown', confidence: 1.0,
  },
  'file-hash': {
    w: 220, h: 90, hashValue: '', algorithm: 'SHA256', label: '', confidence: 1.0,
  },
  process: {
    w: 220, h: 90, processName: '', pid: 0, label: '', confidence: 1.0,
  },
  malware: {
    w: 220, h: 90, familyName: '', label: '', confidence: 1.0,
  },
  ttp: {
    w: 220, h: 90, mitreId: '', name: '', label: '', confidence: 1.0,
  },
  'threat-actor': {
    w: 220, h: 90, name: '', label: '', sophistication: 'medium', confidence: 1.0,
  },
  asset: {
    w: 220, h: 90, hostname: '', label: '', criticality: 'medium', confidence: 1.0,
  },
}

function getDefaultProps(nodeType: string): Record<string, unknown> {
  return KNOWN_DEFAULT_PROPS[nodeType] ?? { w: 220, h: 90, label: '', confidence: 1.0 }
}

export function NodeTool() {
  const editor = useInvestigationStore((s) => s.editor)
  const [activeType, setActiveType] = useState<string | null>(null)
  const [isOpen, setIsOpen] = useState(false)

  const handleCreateNode = useCallback(
    (nodeType: string) => {
      if (!editor) return

      // 在视口中心创建节点
      const viewport = editor.getViewportPageBounds()
      const centerX = viewport.x + viewport.w / 2
      const centerY = viewport.y + viewport.h / 2

      const id = createShapeId()
      const typeLabel = NODE_TYPE_LABELS[nodeType] || nodeType
      const defaultProps = getDefaultProps(nodeType)

      editor.createShape({
        id,
        type: nodeType as any,
        x: centerX - (defaultProps.w as number) / 2,
        y: centerY - (defaultProps.h as number) / 2,
        props: {
          ...defaultProps,
          label: typeLabel,
        },
      } as any)

      // 选中新创建的节点
      editor.select(id)
      setActiveType(null)
      setIsOpen(false)
    },
    [editor],
  )

  // editor 尚未挂载时不渲染
  if (!editor) return null

  return (
    <div
      style={{
        position: 'absolute',
        top: 16,
        left: 16,
        zIndex: 1000,
        display: 'flex',
        flexDirection: 'column',
        gap: 4,
      }}
    >
      {/* 主按钮：切换工具栏 */}
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
        title="添加线索节点"
      >
        +
      </button>

      {/* 节点类型列表 */}
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
          {NODE_TYPE_SHAPES.map((nodeType) => {
            const isActive = activeType === nodeType
            return (
              <button
                key={nodeType}
                onClick={() => handleCreateNode(nodeType)}
                onMouseEnter={() => setActiveType(nodeType)}
                onMouseLeave={() => setActiveType(null)}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 8,
                  padding: '8px 12px',
                  border: 'none',
                  borderRadius: 6,
                  background: isActive
                    ? NODE_COLORS[nodeType] + '22'
                    : 'transparent',
                  color: '#f8fafc',
                  fontSize: 12,
                  cursor: 'pointer',
                  whiteSpace: 'nowrap',
                  transition: 'background 0.15s',
                }}
              >
                <span style={{ fontSize: 16 }}>{NODE_ICONS[nodeType]}</span>
                <span>{NODE_TYPE_LABELS[nodeType]}</span>
                <span
                  style={{
                    width: 8,
                    height: 8,
                    borderRadius: '50%',
                    background: NODE_COLORS[nodeType],
                    marginLeft: 'auto',
                  }}
                />
              </button>
            )
          })}
        </div>
      )}
    </div>
  )
}
