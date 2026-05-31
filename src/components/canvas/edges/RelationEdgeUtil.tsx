import {
  BaseBoxShapeUtil,
  HTMLContainer,
  TLBaseShape,
  RecordProps,
  T,
} from 'tldraw'
import { RELATION_COLORS, RELATION_LABELS } from '../../../lib/constants'

export type RelationEdgeShape = TLBaseShape<'relation-edge', {
  w: number
  h: number
  relationType: string
  label: string
  sourceNodeId: string
  targetNodeId: string
  confidence: number
}>

export class RelationEdgeUtil extends BaseBoxShapeUtil<RelationEdgeShape> {
  static override type = 'relation-edge' as const

  static override props: RecordProps<RelationEdgeShape> = {
    w: T.number,
    h: T.number,
    relationType: T.string,
    label: T.string,
    sourceNodeId: T.string,
    targetNodeId: T.string,
    confidence: T.number,
  }

  getDefaultProps(): RelationEdgeShape['props'] {
    return {
      w: 130,
      h: 32,
      relationType: 'connects',
      label: '',
      sourceNodeId: '',
      targetNodeId: '',
      confidence: 1.0,
    }
  }

  component(shape: RelationEdgeShape) {
    const { relationType, label, confidence } = shape.props
    const color = RELATION_COLORS[relationType] || '#6B7280'
    const displayLabel = RELATION_LABELS[relationType] || relationType

    return (
      <HTMLContainer>
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 4,
            padding: '4px 8px',
            borderRadius: 6,
            background: 'rgba(15, 23, 42, 0.85)',
            backdropFilter: 'blur(4px)',
            color: '#f8fafc',
            fontSize: 11,
            fontFamily: 'monospace',
            whiteSpace: 'nowrap',
            border: `1px solid ${color}`,
            userSelect: 'none',
          }}
        >
          <span
            style={{
              width: 8,
              height: 8,
              borderRadius: '50%',
              background: color,
              flexShrink: 0,
            }}
          />
          <span style={{ fontWeight: 600 }}>{displayLabel}</span>
          {label && (
            <span style={{ color: '#94A3B8' }}>&mdash; {label}</span>
          )}
          <span
            style={{
              fontSize: 9,
              color: '#64748B',
              marginLeft: 'auto',
            }}
          >
            {(confidence * 100).toFixed(0)}%
          </span>
        </div>
      </HTMLContainer>
    )
  }

  indicator(shape: RelationEdgeShape) {
    return <rect width={shape.props.w} height={shape.props.h} rx={6} />
  }

  // Disable resizing — this is a read-only label shape
  override canResize = () => false

  // Disable inline editing
  override canEdit = () => false
}
