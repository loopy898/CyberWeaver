import { useCallback, useEffect, useMemo, useState, type CSSProperties, type FormEvent } from 'react'
import { createShapeId } from 'tldraw'
import { useLLM, type ExtractedEntity, type ExtractedRelation } from '../../hooks/useLLM'
import { NODE_COLORS, NODE_ICONS, NODE_TYPE_LABELS } from '../../lib/constants'
import { nodeTypeToShapeType } from '../../lib/shape-mapper'
import { useInvestigationStore } from '../../stores/investigation'
import type { NodeTypeShape } from '../../lib/constants'

const RELATION_LABELS: Record<string, string> = {
  connects_to: '网络连接',
  resolves_to: 'DNS解析',
  creates: '创建',
  belongs_to: '归属于',
  uses: '使用',
  targets: '攻击目标',
  contains: '包含',
}

type TabId = 'extract' | 'config'
type PhaseId = 'input' | 'entities' | 'relations'

type EditableEntity = ExtractedEntity & {
  selected: boolean
  editedLabel: string
}

function isNodeTypeShape(value: string): value is NodeTypeShape {
  return value in NODE_TYPE_LABELS
}

function toShapeType(nodeType: string): NodeTypeShape | null {
  const shapeType = nodeTypeToShapeType(nodeType)
  return isNodeTypeShape(shapeType) ? shapeType : null
}

function asString(value: unknown): string | undefined {
  return typeof value === 'string' && value.trim() ? value : undefined
}

function asNumber(value: unknown): number | undefined {
  return typeof value === 'number' && Number.isFinite(value) ? value : undefined
}

function asStringArray(value: unknown): string[] {
  if (!Array.isArray(value)) return []
  return value.filter((item): item is string => typeof item === 'string' && item.trim().length > 0)
}

function buildShapeProps(entity: ExtractedEntity, editedLabel: string): Record<string, unknown> | null {
  const shapeType = toShapeType(entity.node_type)
  if (!shapeType) return null

  const props = entity.properties
  const baseProps = {
    w: 220,
    h: 90,
    label: editedLabel || entity.label,
    confidence: entity.confidence,
  }

  switch (shapeType) {
    case 'ip-address':
      return {
        ...baseProps,
        address: asString(props.address) ?? entity.label,
        geo: asString(props.geo_location),
        asn: asString(props.asn),
        reputation: asString(props.reputation) ?? 'unknown',
      }
    case 'domain':
      return {
        ...baseProps,
        domain: asString(props.domain) ?? entity.label,
        registrar: asString(props.registrar),
        creationDate: asString(props.creation_date),
        reputation: 'unknown',
      }
    case 'file-hash': {
      const algorithm = (asString(props.algorithm) ?? 'sha256').toUpperCase()
      const normalizedAlgorithm =
        algorithm === 'MD5' || algorithm === 'SHA1' || algorithm === 'SHA256' || algorithm === 'SHA512'
          ? algorithm
          : 'SHA256'

      return {
        ...baseProps,
        hashValue: asString(props.hash_value) ?? entity.label,
        algorithm: normalizedAlgorithm,
        fileName: asString(props.file_name),
        malwareClassification: asString(props.malware_classification),
      }
    }
    case 'process':
      return {
        ...baseProps,
        processName: asString(props.process_name) ?? entity.label,
        pid: asNumber(props.pid) ?? 0,
        user: asString(props.user),
        commandLine: asString(props.command_line),
      }
    case 'malware': {
      const aliases = asStringArray(props.aliases)
      return {
        ...baseProps,
        familyName: asString(props.family_name) ?? entity.label,
        aliases: aliases.length > 0 ? aliases.join(', ') : undefined,
        malwareType: asString(props.malware_type),
      }
    }
    case 'ttp':
      return {
        ...baseProps,
        mitreId: asString(props.mitre_id) ?? entity.label,
        name: entity.label,
        tactic: asString(props.tactic),
        description: entity.description || undefined,
      }
    case 'threat-actor': {
      const aliases = asStringArray(props.aliases)
      const sophistication = asString(props.sophistication)
      const normalizedSophistication =
        sophistication === 'low' || sophistication === 'medium' || sophistication === 'high' || sophistication === 'advanced'
          ? sophistication
          : 'medium'

      return {
        ...baseProps,
        name: asString(props.name) ?? entity.label,
        aliases: aliases.length > 0 ? aliases.join(', ') : undefined,
        motivation: asString(props.motivation),
        sophistication: normalizedSophistication,
      }
    }
    case 'asset': {
      const criticality = asString(props.criticality)
      const ipAddresses = asStringArray(props.ip_addresses)
      const normalizedCriticality =
        criticality === 'low' || criticality === 'medium' || criticality === 'high' || criticality === 'critical'
          ? criticality
          : 'medium'

      return {
        ...baseProps,
        hostname: asString(props.hostname) ?? entity.label,
        os: asString(props.os),
        ipAddresses: ipAddresses.length > 0 ? ipAddresses.join(', ') : undefined,
        criticality: normalizedCriticality,
      }
    }
    default:
      return null
  }
}

export function AiPanel() {
  const editor = useInvestigationStore((s) => s.editor)
  const { isLoading, error, configureLlm, getLlmConfig, extractFromText, extractRelations } = useLLM()

  const [activeTab, setActiveTab] = useState<TabId>('extract')
  const [phase, setPhase] = useState<PhaseId>('input')
  const [inputText, setInputText] = useState('')
  const [entities, setEntities] = useState<EditableEntity[]>([])
  const [relations, setRelations] = useState<ExtractedRelation[]>([])

  const [apiBase, setApiBase] = useState('')
  const [apiKey, setApiKey] = useState('')
  const [model, setModel] = useState('gpt-4o')
  const [configStatus, setConfigStatus] = useState<string | null>(null)

  useEffect(() => {
    let cancelled = false

    void getLlmConfig()
      .then((config) => {
        if (cancelled) return
        setApiBase(config.api_base)
        setModel(config.model || 'gpt-4o')
        setConfigStatus(config.configured ? `已配置 (${config.model})` : '未配置')
      })
      .catch(() => {})

    return () => {
      cancelled = true
    }
  }, [getLlmConfig])

  const selectedEntities = useMemo(() => entities.filter((entity) => entity.selected), [entities])

  const handleExtractEntities = useCallback(async () => {
    if (!inputText.trim()) return

    try {
      const result = await extractFromText(inputText)
      setEntities(
        result.map((entity) => ({
          ...entity,
          selected: true,
          editedLabel: entity.label,
        })),
      )
      setRelations([])
      setPhase('entities')
    } catch {}
  }, [extractFromText, inputText])

  const handleExtractRelations = useCallback(async () => {
    if (selectedEntities.length < 2) return

    try {
      const result = await extractRelations(inputText, selectedEntities)
      setRelations(result)
      setPhase('relations')
    } catch {}
  }, [extractRelations, inputText, selectedEntities])

  const handleGenerateGraph = useCallback(() => {
    if (!editor || selectedEntities.length === 0) return

    const viewport = editor.getViewportPageBounds()
    const spacing = 250
    const cols = Math.ceil(Math.sqrt(selectedEntities.length))
    const totalRows = Math.ceil(selectedEntities.length / cols)
    const startX = viewport.x + Math.max((viewport.w - cols * spacing) / 2, 48)
    const startY = viewport.y + Math.max((viewport.h - totalRows * spacing) / 2, 48)
    const createdShapeIds = new Map<number, string>()

    selectedEntities.forEach((entity, index) => {
      const shapeType = toShapeType(entity.node_type)
      const shapeProps = buildShapeProps(entity, entity.editedLabel)
      if (!shapeType || !shapeProps) return

      const col = index % cols
      const row = Math.floor(index / cols)
      const id = createShapeId()

      editor.createShape({
        id,
        type: shapeType,
        x: startX + col * spacing,
        y: startY + row * spacing,
        props: shapeProps,
      } as any)

      createdShapeIds.set(index, id)
    })

    relations.forEach((relation) => {
      const sourceId = createdShapeIds.get(relation.source_index)
      const targetId = createdShapeIds.get(relation.target_index)
      if (!sourceId || !targetId) return

      editor.createShape({
        id: createShapeId(),
        type: 'arrow',
        props: {
          start: {
            type: 'binding',
            boundShapeId: sourceId,
            normalizedAnchor: { x: 0.5, y: 0.5 },
            isExact: false,
          },
          end: {
            type: 'binding',
            boundShapeId: targetId,
            normalizedAnchor: { x: 0.5, y: 0.5 },
            isExact: false,
          },
          text: relation.label || RELATION_LABELS[relation.relation_type] || relation.relation_type,
          color: 'black',
        } as any,
      })
    })

    setPhase('input')
    setEntities([])
    setRelations([])
    setInputText('')
  }, [editor, relations, selectedEntities])

  const handleSaveConfig = useCallback(async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()

    try {
      const result = await configureLlm({ apiBase, apiKey, model })
      setConfigStatus(result.configured ? `已配置 (${result.model})` : '未配置')
      setApiKey('')
    } catch {}
  }, [apiBase, apiKey, configureLlm, model])

  const canExtractRelations = selectedEntities.length >= 2
  const canGenerateGraph = selectedEntities.length > 0

  return (
    <div style={panelStyle}>
      <div style={{ display: 'flex', gap: 4 }}>
        <button style={tabStyle(activeTab, 'extract')} onClick={() => setActiveTab('extract')}>
          提取
        </button>
        <button style={tabStyle(activeTab, 'config')} onClick={() => setActiveTab('config')}>
          配置
        </button>
      </div>

      {error ? <div style={errorStyle}>{error}</div> : null}

      {activeTab === 'extract' ? (
        <>
          {phase === 'input' ? (
            <>
              <textarea
                style={{ ...inputStyle, minHeight: 132, resize: 'vertical', fontSize: 11, lineHeight: 1.5 }}
                placeholder="粘贴威胁报告、APT 分析文章..."
                value={inputText}
                onChange={(event) => setInputText(event.target.value)}
              />
              <button style={primaryButtonStyle} onClick={handleExtractEntities} disabled={isLoading || !inputText.trim()}>
                {isLoading ? '提取中...' : '提取实体'}
              </button>
            </>
          ) : null}

          {phase === 'entities' ? (
            <>
              <div style={hintStyle}>
                已提取 {entities.length} 个实体，当前选择 {selectedEntities.length} 个。
              </div>
              <div style={listStyle}>
                {entities.map((entity, index) => {
                  const shapeType = toShapeType(entity.node_type)
                  const nodeLabel = shapeType ? NODE_TYPE_LABELS[shapeType] : entity.node_type
                  const nodeIcon = shapeType ? NODE_ICONS[shapeType] : '•'
                  const nodeColor = shapeType ? NODE_COLORS[shapeType] : '#94a3b8'

                  return (
                    <div key={`${entity.node_type}-${entity.label}-${index}`} style={entityCardStyle(entity.selected)}>
                      <input
                        type="checkbox"
                        checked={entity.selected}
                        onChange={() => {
                          setEntities((prev) =>
                            prev.map((current, currentIndex) =>
                              currentIndex === index ? { ...current, selected: !current.selected } : current,
                            ),
                          )
                        }}
                      />
                      <div style={{ flex: 1, minWidth: 0 }}>
                        <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 6 }}>
                          <span style={{ fontSize: 15 }}>{nodeIcon}</span>
                          <span
                            style={{
                              fontSize: 10,
                              fontWeight: 700,
                              letterSpacing: 0.2,
                              color: nodeColor,
                              textTransform: 'uppercase',
                            }}
                          >
                            {nodeLabel}
                          </span>
                        </div>
                        <input
                          style={{ ...inputStyle, marginBottom: 4, fontWeight: 600 }}
                          value={entity.editedLabel}
                          onChange={(event) => {
                            const nextValue = event.target.value
                            setEntities((prev) =>
                              prev.map((current, currentIndex) =>
                                currentIndex === index ? { ...current, editedLabel: nextValue } : current,
                              ),
                            )
                          }}
                        />
                        <div style={metaStyle}>置信度 {Math.round(entity.confidence * 100)}%</div>
                        <div style={confidenceTrackStyle}>
                          <div
                            style={{
                              ...confidenceFillStyle,
                              width: `${Math.max(0, Math.min(100, Math.round(entity.confidence * 100)))}%`,
                            }}
                          />
                        </div>
                        {entity.description ? <div style={descriptionStyle}>{entity.description}</div> : null}
                      </div>
                    </div>
                  )
                })}
              </div>
              <div style={buttonRowStyle}>
                <button style={primaryButtonStyle} onClick={handleExtractRelations} disabled={isLoading || !canExtractRelations}>
                  {isLoading ? '提取中...' : '提取关系'}
                </button>
                <button style={successButtonStyle} onClick={handleGenerateGraph} disabled={!canGenerateGraph}>
                  直接生成图谱
                </button>
                <button
                  style={ghostButtonStyle}
                  onClick={() => {
                    setPhase('input')
                    setEntities([])
                    setRelations([])
                  }}
                >
                  返回
                </button>
              </div>
            </>
          ) : null}

          {phase === 'relations' ? (
            <>
              <div style={hintStyle}>已提取 {relations.length} 条关系。</div>
              <div style={listStyle}>
                {relations.map((relation, index) => (
                  <div key={`${relation.relation_type}-${relation.source_index}-${relation.target_index}-${index}`} style={relationCardStyle}>
                    <div style={{ display: 'flex', gap: 6, alignItems: 'center', flexWrap: 'wrap' }}>
                      <span style={{ color: '#c4b5fd' }}>
                        [{selectedEntities[relation.source_index]?.editedLabel || relation.source_index}]
                      </span>
                      <span style={{ color: '#64748b' }}>→</span>
                      <span style={{ color: '#fbbf24' }}>
                        [{selectedEntities[relation.target_index]?.editedLabel || relation.target_index}]
                      </span>
                      <span style={{ color: '#94a3b8' }}>
                        <span style={relationTypeTagStyle}>
                          {RELATION_LABELS[relation.relation_type] || relation.relation_type}
                        </span>
                      </span>
                    </div>
                    {relation.label ? <div style={descriptionStyle}>{relation.label}</div> : null}
                    <div style={metaStyle}>置信度 {Math.round(relation.confidence * 100)}%</div>
                  </div>
                ))}
              </div>
              <div style={buttonRowStyle}>
                <button style={successButtonStyle} onClick={handleGenerateGraph} disabled={!canGenerateGraph}>
                  生成图谱
                </button>
                <button style={ghostButtonStyle} onClick={() => setPhase('entities')}>
                  返回实体
                </button>
              </div>
            </>
          ) : null}
        </>
      ) : null}

      {activeTab === 'config' ? (
        <form onSubmit={handleSaveConfig} style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
          <div style={hintStyle}>支持 OpenAI API 和任何兼容接口（如 Ollama: http://localhost:11434/v1）</div>

          <label style={labelStyle}>
            API Base URL
            <input
              style={inputStyle}
              type="text"
              placeholder="https://api.openai.com"
              value={apiBase}
              onChange={(event) => setApiBase(event.target.value)}
            />
          </label>

          <label style={labelStyle}>
            API Key
            <input
              style={inputStyle}
              type="password"
              placeholder="sk-..."
              value={apiKey}
              onChange={(event) => setApiKey(event.target.value)}
            />
          </label>

          <label style={labelStyle}>
            Model
            <input
              style={inputStyle}
              type="text"
              placeholder="gpt-4o"
              value={model}
              onChange={(event) => setModel(event.target.value)}
            />
          </label>

          <button type="submit" style={primaryButtonStyle} disabled={isLoading}>
            保存配置
          </button>

          {configStatus ? (
            <div
              style={{
                ...statusStyle,
                background: configStatus.startsWith('已配置') ? 'rgba(34, 197, 94, 0.14)' : 'rgba(248, 113, 113, 0.14)',
                color: configStatus.startsWith('已配置') ? '#4ade80' : '#f87171',
              }}
            >
              {configStatus}
            </div>
          ) : null}
        </form>
      ) : null}
    </div>
  )
}

const panelStyle: CSSProperties = {
  position: 'fixed',
  right: 16,
  top: 320,
  width: 300,
  maxHeight: 'calc(100vh - 336px)',
  overflowY: 'auto',
  background: 'rgba(30, 30, 40, 0.95)',
  backdropFilter: 'blur(12px)',
  WebkitBackdropFilter: 'blur(12px)',
  borderRadius: 12,
  border: '1px solid rgba(255, 255, 255, 0.08)',
  boxShadow: '0 8px 32px rgba(0, 0, 0, 0.35)',
  padding: 16,
  zIndex: 1000,
  color: '#e2e8f0',
  fontSize: 13,
  display: 'flex',
  flexDirection: 'column',
  gap: 12,
  pointerEvents: 'auto',
}

const inputStyle: CSSProperties = {
  width: '100%',
  boxSizing: 'border-box',
  padding: '8px 10px',
  background: 'rgba(255, 255, 255, 0.06)',
  border: '1px solid rgba(255, 255, 255, 0.12)',
  borderRadius: 6,
  color: '#e2e8f0',
  fontSize: 12,
  fontFamily: 'inherit',
}

const labelStyle: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: 6,
  fontSize: 11,
  color: '#cbd5e1',
}

const hintStyle: CSSProperties = {
  fontSize: 11,
  color: '#94a3b8',
  lineHeight: 1.5,
}

const confidenceTrackStyle: CSSProperties = {
  width: '100%',
  height: 6,
  marginTop: 6,
  borderRadius: 999,
  background: 'rgba(148, 163, 184, 0.2)',
  overflow: 'hidden',
}

const confidenceFillStyle: CSSProperties = {
  height: '100%',
  borderRadius: 999,
  background: 'linear-gradient(90deg, #38bdf8, #22c55e)',
}

const errorStyle: CSSProperties = {
  color: '#fca5a5',
  fontSize: 11,
  padding: '6px 8px',
  background: 'rgba(248, 113, 113, 0.12)',
  borderRadius: 6,
}

const statusStyle: CSSProperties = {
  fontSize: 11,
  padding: '6px 8px',
  borderRadius: 6,
}

const listStyle: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: 8,
  maxHeight: 320,
  overflowY: 'auto',
}

const buttonRowStyle: CSSProperties = {
  display: 'flex',
  gap: 8,
  flexWrap: 'wrap',
}

const metaStyle: CSSProperties = {
  fontSize: 10,
  color: '#94a3b8',
}

const descriptionStyle: CSSProperties = {
  marginTop: 4,
  fontSize: 10,
  color: '#64748b',
  lineHeight: 1.45,
}

function tabStyle(activeTab: TabId, tab: TabId): CSSProperties {
  const isActive = activeTab === tab
  return {
    flex: 1,
    padding: '8px 0',
    textAlign: 'center',
    cursor: 'pointer',
    borderRadius: 8,
    fontSize: 12,
    fontWeight: 700,
    background: isActive ? 'rgba(59, 130, 246, 0.2)' : 'transparent',
    color: isActive ? '#bfdbfe' : '#94a3b8',
    border: '1px solid rgba(255, 255, 255, 0.06)',
  }
}

function entityCardStyle(selected: boolean): CSSProperties {
  return {
    display: 'flex',
    gap: 8,
    alignItems: 'flex-start',
    padding: 8,
    background: selected ? 'rgba(59, 130, 246, 0.14)' : 'rgba(255, 255, 255, 0.04)',
    borderRadius: 8,
    border: '1px solid rgba(255, 255, 255, 0.06)',
  }
}

const relationCardStyle: CSSProperties = {
  padding: 8,
  background: 'rgba(255, 255, 255, 0.04)',
  borderRadius: 8,
  border: '1px solid rgba(255, 255, 255, 0.06)',
  fontSize: 11,
}

const relationTypeTagStyle: CSSProperties = {
  display: 'inline-flex',
  alignItems: 'center',
  padding: '2px 8px',
  borderRadius: 999,
  background: 'rgba(59, 130, 246, 0.18)',
  color: '#bfdbfe',
  fontSize: 10,
  fontWeight: 700,
}

const baseButtonStyle: CSSProperties = {
  padding: '8px 14px',
  borderRadius: 6,
  border: 'none',
  color: '#f8fafc',
  cursor: 'pointer',
  fontSize: 12,
  fontWeight: 700,
}

const primaryButtonStyle: CSSProperties = {
  ...baseButtonStyle,
  background: 'rgba(59, 130, 246, 0.72)',
}

const successButtonStyle: CSSProperties = {
  ...baseButtonStyle,
  background: 'rgba(34, 197, 94, 0.72)',
}

const ghostButtonStyle: CSSProperties = {
  ...baseButtonStyle,
  background: 'rgba(255, 255, 255, 0.1)',
  color: '#cbd5e1',
}
