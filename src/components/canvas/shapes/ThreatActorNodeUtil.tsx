import { ShapeUtil, HTMLContainer, TLBaseShape, T, Rectangle2d, type Geometry2d, type TLResizeInfo } from 'tldraw'
import type { RecordProps } from '@tldraw/tlschema'

export type ThreatActorNodeShape = TLBaseShape<'threat-actor', {
  w: number
  h: number
  name: string
  label: string
  aliases?: string
  motivation?: string
  sophistication: 'low' | 'medium' | 'high' | 'advanced'
  confidence: number
}>

export class ThreatActorNodeUtil extends ShapeUtil<ThreatActorNodeShape> {
  static override type = 'threat-actor' as const

  static override props: RecordProps<ThreatActorNodeShape> = {
    w: T.number,
    h: T.number,
    name: T.string,
    label: T.string,
    aliases: T.string.optional(),
    motivation: T.string.optional(),
    sophistication: T.literalEnum('low', 'medium', 'high', 'advanced'),
    confidence: T.number,
  }

  getDefaultProps(): ThreatActorNodeShape['props'] {
    return {
      w: 220,
      h: 90,
      name: '',
      label: '',
      sophistication: 'medium',
      confidence: 1.0,
    }
  }

  getGeometry(shape: ThreatActorNodeShape): Geometry2d {
    return new Rectangle2d({
      width: shape.props.w,
      height: shape.props.h,
      isFilled: true,
    })
  }

  component(shape: ThreatActorNodeShape) {
    const { name, label, aliases, motivation, sophistication, confidence } = shape.props
    const sophColor: Record<string, string> = {
      low: '#10B981',
      medium: '#F59E0B',
      high: '#EF4444',
      advanced: '#7C3AED',
    }

    return (
      <HTMLContainer>
        <div style={{
          width: '100%', height: '100%',
          border: '2px solid #EC4899',
          borderRadius: 8,
          background: '#FDF2F8',
          padding: '10px 12px',
          fontFamily: 'monospace',
          display: 'flex', flexDirection: 'column', gap: 4,
          position: 'relative',
        }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
            <span style={{ fontSize: 18 }}>👤</span>
            <span style={{ fontWeight: 600, fontSize: 13, color: '#9D174D' }}>{name || 'Threat Actor'}</span>
            <span style={{
              marginLeft: 'auto', fontSize: 10, padding: '1px 6px',
              borderRadius: 4, background: sophColor[sophistication] ?? '#6B7280', color: '#fff',
            }}>{sophistication}</span>
          </div>
          {label && <div style={{ fontSize: 11, color: '#64748B' }}>{label}</div>}
          <div style={{ display: 'flex', gap: 8, fontSize: 10, color: '#94A3B8', marginTop: 'auto' }}>
            {aliases && <span>aka: {aliases}</span>}
            {motivation && <span>🎯 {motivation}</span>}
            <span style={{ marginLeft: 'auto' }}>conf: {(confidence * 100).toFixed(0)}%</span>
          </div>
          <div style={{
            position: 'absolute', top: -8, right: 8,
            fontSize: 9, color: '#EC4899', background: '#FCE7F3',
            padding: '1px 6px', borderRadius: 4,
          }}>ACTOR</div>
        </div>
      </HTMLContainer>
    )
  }

  indicator(shape: ThreatActorNodeShape) {
    return <rect width={shape.props.w} height={shape.props.h} rx={8} />
  }

  override onResize(_shape: ThreatActorNodeShape, info: TLResizeInfo<ThreatActorNodeShape>) {
    return {
      props: {
        w: Math.max(100, info.initialBounds.width * info.scaleX),
        h: Math.max(60, info.initialBounds.height * info.scaleY),
      },
    }
  }
}
