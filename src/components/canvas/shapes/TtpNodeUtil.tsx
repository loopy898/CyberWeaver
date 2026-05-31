import { ShapeUtil, HTMLContainer, TLBaseShape, T, Rectangle2d, type Geometry2d, type TLResizeInfo } from 'tldraw'
import type { RecordProps } from '@tldraw/tlschema'

export type TtpNodeShape = TLBaseShape<'ttp', {
  w: number
  h: number
  mitreId: string
  name: string
  label: string
  tactic?: string
  description?: string
  confidence: number
}>

export class TtpNodeUtil extends ShapeUtil<TtpNodeShape> {
  static override type = 'ttp' as const

  static override props: RecordProps<TtpNodeShape> = {
    w: T.number,
    h: T.number,
    mitreId: T.string,
    name: T.string,
    label: T.string,
    tactic: T.string.optional(),
    description: T.string.optional(),
    confidence: T.number,
  }

  getDefaultProps(): TtpNodeShape['props'] {
    return {
      w: 220,
      h: 90,
      mitreId: '',
      name: '',
      label: '',
      confidence: 1.0,
    }
  }

  getGeometry(shape: TtpNodeShape): Geometry2d {
    return new Rectangle2d({
      width: shape.props.w,
      height: shape.props.h,
      isFilled: true,
    })
  }

  component(shape: TtpNodeShape) {
    const { mitreId, name, label, tactic, description, confidence } = shape.props
    const truncatedDesc = description && description.length > 35
      ? description.slice(0, 35) + '...'
      : description

    return (
      <HTMLContainer>
        <div style={{
          width: '100%', height: '100%',
          border: '2px solid #8B5CF6',
          borderRadius: 8,
          background: '#F5F3FF',
          padding: '10px 12px',
          fontFamily: 'monospace',
          display: 'flex', flexDirection: 'column', gap: 4,
          position: 'relative',
        }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
            <span style={{ fontSize: 18 }}>⚔️</span>
            <span style={{ fontWeight: 600, fontSize: 13, color: '#5B21B6' }}>{mitreId || 'TTP'}</span>
            {tactic && (
              <span style={{
                marginLeft: 'auto', fontSize: 10, padding: '1px 6px',
                borderRadius: 4, background: '#EDE9FE', color: '#5B21B6',
              }}>{tactic}</span>
            )}
          </div>
          {name && <div style={{ fontSize: 12, fontWeight: 500, color: '#6D28D9' }}>{name}</div>}
          {label && <div style={{ fontSize: 11, color: '#64748B' }}>{label}</div>}
          <div style={{ display: 'flex', gap: 8, fontSize: 10, color: '#94A3B8', marginTop: 'auto' }}>
            {truncatedDesc && <span>{truncatedDesc}</span>}
            <span style={{ marginLeft: 'auto' }}>conf: {(confidence * 100).toFixed(0)}%</span>
          </div>
          <div style={{
            position: 'absolute', top: -8, right: 8,
            fontSize: 9, color: '#8B5CF6', background: '#EDE9FE',
            padding: '1px 6px', borderRadius: 4,
          }}>TTP</div>
        </div>
      </HTMLContainer>
    )
  }

  indicator(shape: TtpNodeShape) {
    return <rect width={shape.props.w} height={shape.props.h} rx={8} />
  }

  override onResize(_shape: TtpNodeShape, info: TLResizeInfo<TtpNodeShape>) {
    return {
      props: {
        w: Math.max(100, info.initialBounds.width * info.scaleX),
        h: Math.max(60, info.initialBounds.height * info.scaleY),
      },
    }
  }
}
