import { useState, useEffect } from 'react'
import type { TLShapeId } from 'tldraw'
import { useInvestigationStore } from '../../stores/investigation'
import {
  NODE_TYPE_SHAPES,
  NODE_TYPE_LABELS,
  NODE_ICONS,
  type NodeTypeShape,
} from '../../lib/constants'

// ---------------------------------------------------------------------------
// 字段定义
// ---------------------------------------------------------------------------
interface FieldDef {
  key: string
  label: string
  type: 'text' | 'number' | 'select' | 'multiline'
  options?: string[]
}

const NODE_FIELDS: Record<NodeTypeShape, FieldDef[]> = {
  'ip-address': [
    { key: 'address', label: 'IP 地址', type: 'text' },
    { key: 'label', label: '标签', type: 'text' },
    { key: 'geo', label: '地理位置', type: 'text' },
    { key: 'asn', label: 'ASN', type: 'text' },
    { key: 'reputation', label: '信誉', type: 'select', options: ['clean', 'suspicious', 'malicious', 'unknown'] },
    { key: 'confidence', label: '置信度', type: 'number' },
  ],
  domain: [
    { key: 'domain', label: '域名', type: 'text' },
    { key: 'label', label: '标签', type: 'text' },
    { key: 'registrar', label: '注册商', type: 'text' },
    { key: 'creationDate', label: '创建日期', type: 'text' },
    { key: 'reputation', label: '信誉', type: 'select', options: ['clean', 'suspicious', 'malicious', 'unknown'] },
    { key: 'confidence', label: '置信度', type: 'number' },
  ],
  'file-hash': [
    { key: 'hashValue', label: '哈希值', type: 'text' },
    { key: 'algorithm', label: '算法', type: 'select', options: ['MD5', 'SHA1', 'SHA256', 'SHA512'] },
    { key: 'label', label: '标签', type: 'text' },
    { key: 'fileName', label: '文件名', type: 'text' },
    { key: 'malwareClassification', label: '恶意分类', type: 'text' },
    { key: 'confidence', label: '置信度', type: 'number' },
  ],
  process: [
    { key: 'processName', label: '进程名', type: 'text' },
    { key: 'pid', label: 'PID', type: 'number' },
    { key: 'label', label: '标签', type: 'text' },
    { key: 'user', label: '用户', type: 'text' },
    { key: 'commandLine', label: '命令行', type: 'multiline' },
    { key: 'confidence', label: '置信度', type: 'number' },
  ],
  malware: [
    { key: 'familyName', label: '家族名', type: 'text' },
    { key: 'label', label: '标签', type: 'text' },
    { key: 'aliases', label: '别名', type: 'text' },
    { key: 'malwareType', label: '恶意类型', type: 'text' },
    { key: 'confidence', label: '置信度', type: 'number' },
  ],
  ttp: [
    { key: 'mitreId', label: 'MITRE ID', type: 'text' },
    { key: 'name', label: '名称', type: 'text' },
    { key: 'label', label: '标签', type: 'text' },
    { key: 'tactic', label: '战术', type: 'text' },
    { key: 'description', label: '描述', type: 'multiline' },
    { key: 'confidence', label: '置信度', type: 'number' },
  ],
  'threat-actor': [
    { key: 'name', label: '名称', type: 'text' },
    { key: 'label', label: '标签', type: 'text' },
    { key: 'aliases', label: '别名', type: 'text' },
    { key: 'motivation', label: '动机', type: 'text' },
    { key: 'sophistication', label: '复杂度', type: 'select', options: ['low', 'medium', 'high', 'advanced'] },
    { key: 'confidence', label: '置信度', type: 'number' },
  ],
  asset: [
    { key: 'hostname', label: '主机名', type: 'text' },
    { key: 'label', label: '标签', type: 'text' },
    { key: 'os', label: '操作系统', type: 'text' },
    { key: 'ipAddresses', label: 'IP 地址', type: 'text' },
    { key: 'criticality', label: '关键度', type: 'select', options: ['low', 'medium', 'high', 'critical'] },
    { key: 'confidence', label: '置信度', type: 'number' },
  ],
}

// ---------------------------------------------------------------------------
// 样式常量
// ---------------------------------------------------------------------------
const panelStyle: React.CSSProperties = {
  position: 'absolute',
  right: 16,
  top: 16,
  width: 280,
  maxHeight: 'calc(100vh - 32px)',
  overflowY: 'auto',
  background: 'rgba(15, 23, 42, 0.92)',
  backdropFilter: 'blur(12px)',
  WebkitBackdropFilter: 'blur(12px)',
  borderRadius: 12,
  border: '1px solid rgba(255, 255, 255, 0.08)',
  boxShadow: '0 8px 32px rgba(0, 0, 0, 0.4)',
  padding: 16,
  zIndex: 1000,
  color: '#e2e8f0',
  fontSize: 13,
  fontFamily: 'system-ui, -apple-system, sans-serif',
  pointerEvents: 'auto',
}

const headerStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'space-between',
  marginBottom: 14,
  paddingBottom: 10,
  borderBottom: '1px solid rgba(255, 255, 255, 0.08)',
}

const fieldLabelStyle: React.CSSProperties = {
  fontSize: 11,
  color: '#94a3b8',
  marginBottom: 3,
  fontWeight: 500,
}

const valueStyle: React.CSSProperties = {
  fontSize: 13,
  color: '#e2e8f0',
  wordBreak: 'break-all',
  minHeight: 18,
  lineHeight: 1.4,
}

const inputStyle: React.CSSProperties = {
  width: '100%',
  boxSizing: 'border-box',
  padding: '6px 8px',
  background: 'rgba(255, 255, 255, 0.06)',
  border: '1px solid rgba(255, 255, 255, 0.12)',
  borderRadius: 6,
  color: '#e2e8f0',
  fontSize: 12,
  fontFamily: 'inherit',
}

const btnBase: React.CSSProperties = {
  padding: '4px 10px',
  borderRadius: 6,
  border: 'none',
  fontSize: 12,
  cursor: 'pointer',
  fontWeight: 500,
  whiteSpace: 'nowrap',
}

// ---------------------------------------------------------------------------
// 组件
// ---------------------------------------------------------------------------
export function PropertyPanel() {
  const editor = useInvestigationStore((s) => s.editor)
  const [selectedShapeId, setSelectedShapeId] = useState<TLShapeId | null>(null)
  const [editing, setEditing] = useState(false)
  const [editProps, setEditProps] = useState<Record<string, unknown>>({})
  const [refreshKey, setRefreshKey] = useState(0)

  // 监听 editor.store 变化，同步选中状态并感知外部数据更新
  useEffect(() => {
    if (!editor) return

    // 初始选中同步
    const initialIds = editor.getSelectedShapeIds()
    if (initialIds.length === 1) {
      setSelectedShapeId(initialIds[0])
    }

    const unlisten = editor.store.listen(() => {
      const ids = editor.getSelectedShapeIds()
      const id = ids.length === 1 ? ids[0] : null
      setSelectedShapeId((prev) => (prev !== id ? id : prev))
      setRefreshKey((k) => k + 1)
    })

    return unlisten
  }, [editor])

  // 选中节点变化时，退出编辑模式
  useEffect(() => {
    setEditing(false)
  }, [selectedShapeId])

  // ------------------------------------------------------------------
  // 空态
  // ------------------------------------------------------------------
  if (!editor) return null

  const shape = selectedShapeId ? editor.getShape(selectedShapeId) : null
  const nodeType =
    shape && NODE_TYPE_SHAPES.includes(shape.type as NodeTypeShape)
      ? (shape.type as NodeTypeShape)
      : null

  if (!nodeType || !shape) return null

  const props = shape.props as Record<string, unknown>
  const fields = NODE_FIELDS[nodeType]

  // ------------------------------------------------------------------
  // 值展示
  // ------------------------------------------------------------------
  const displayValue = (field: FieldDef, value: unknown): string => {
    if (value === undefined || value === null || value === '') return '—'
    if (field.key === 'confidence' && typeof value === 'number') {
      return `${(value * 100).toFixed(0)}%`
    }
    return String(value)
  }

  // ------------------------------------------------------------------
  // 操作回调
  // ------------------------------------------------------------------
  const handleStartEdit = () => {
    const p = { ...shape.props } as Record<string, unknown>
    // 置信度在编辑状态下以 0-100 整数呈现
    if (typeof p.confidence === 'number') {
      p.confidence = Math.round(p.confidence * 100)
    }
    setEditProps(p)
    setEditing(true)
  }

  const handleSave = () => {
    if (!editor || !shape) return
    const p = { ...editProps }
    if (typeof p.confidence === 'number') {
      p.confidence = p.confidence / 100
    }
    editor.updateShape({
      id: shape.id,
      type: shape.type,
      props: p,
    } as any)
    setEditing(false)
  }

  const handleCancel = () => {
    setEditing(false)
  }

  const handleFieldChange = (key: string, rawValue: string) => {
    setEditProps((prev) => {
      const field = fields.find((f) => f.key === key)
      if (!field) return prev

      if (field.type === 'number') {
        const num = rawValue === '' ? 0 : Number(rawValue)
        return { ...prev, [key]: Number.isNaN(num) ? 0 : num }
      }
      return { ...prev, [key]: rawValue }
    })
  }

  // ------------------------------------------------------------------
  // 渲染
  // ------------------------------------------------------------------
  // refreshKey 确保外部数据更新时面板重新读取 editor 中的最新 props
  void refreshKey

  return (
    <div style={panelStyle}>
      {/* 头部 */}
      <div style={headerStyle}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <span style={{ fontSize: 18 }}>{NODE_ICONS[nodeType]}</span>
          <span style={{ fontWeight: 600, fontSize: 14 }}>
            {NODE_TYPE_LABELS[nodeType]}
          </span>
        </div>
        <div style={{ display: 'flex', gap: 6 }}>
          {editing ? (
            <>
              <button
                onClick={handleSave}
                style={{ ...btnBase, background: '#3B82F6', color: '#fff' }}
              >
                保存
              </button>
              <button
                onClick={handleCancel}
                style={{
                  ...btnBase,
                  background: 'rgba(255,255,255,0.1)',
                  color: '#e2e8f0',
                }}
              >
                取消
              </button>
            </>
          ) : (
            <button
              onClick={handleStartEdit}
              style={{
                ...btnBase,
                background: 'rgba(255,255,255,0.08)',
                color: '#94a3b8',
              }}
            >
              编辑
            </button>
          )}
        </div>
      </div>

      {/* 字段列表 */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
        {fields.map((field) => {
          const rawValue = editing ? editProps[field.key] : props[field.key]

          return (
            <div key={field.key}>
              <div style={fieldLabelStyle}>{field.label}</div>
              {editing ? (
                renderEditInput(field, rawValue, handleFieldChange)
              ) : (
                <div style={valueStyle}>{displayValue(field, rawValue)}</div>
              )}
            </div>
          )
        })}
      </div>
    </div>
  )
}

// ---------------------------------------------------------------------------
// 编辑态输入控件
// ---------------------------------------------------------------------------
function renderEditInput(
  field: FieldDef,
  rawValue: unknown,
  onChange: (key: string, value: string) => void,
) {
  if (field.type === 'select' && field.options) {
    return (
      <select
        value={String(rawValue ?? '')}
        onChange={(e) => onChange(field.key, e.target.value)}
        style={inputStyle}
      >
        {field.options.map((opt) => (
          <option key={opt} value={opt}>
            {opt}
          </option>
        ))}
      </select>
    )
  }

  if (field.type === 'multiline') {
    return (
      <textarea
        value={String(rawValue ?? '')}
        onChange={(e) => onChange(field.key, e.target.value)}
        style={{ ...inputStyle, minHeight: 52, resize: 'vertical' }}
        rows={2}
      />
    )
  }

  if (field.type === 'number') {
    const displayValue =
      rawValue !== undefined && rawValue !== null ? String(rawValue) : ''
    return (
      <input
        type="number"
        value={displayValue}
        onChange={(e) => onChange(field.key, e.target.value)}
        style={inputStyle}
      />
    )
  }

  return (
    <input
      type="text"
      value={String(rawValue ?? '')}
      onChange={(e) => onChange(field.key, e.target.value)}
      style={inputStyle}
    />
  )
}
