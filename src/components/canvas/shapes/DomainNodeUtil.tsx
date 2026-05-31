import { ShapeUtil, HTMLContainer, TLBaseShape, T, Rectangle2d, type Geometry2d, type TLResizeInfo } from 'tldraw'
import type { RecordProps } from '@tldraw/tlschema'

export type DomainNodeShape = TLBaseShape<'domain', {
  w: number
  h: number
  domain: string
  label: string
  registrar?: string
  creationDate?: string
  reputation: 'clean' | 'suspicious' | 'malicious' | 'unknown'
  confidence: number
}>

export class DomainNodeUtil extends ShapeUtil<DomainNodeShape> {
  static override type = 'domain' as const

  static override props: RecordProps<DomainNodeShape> = {
    w: T.number,
    h: T.number,
    domain: T.string,
    label: T.string,
    registrar: T.string.optional(),
    creationDate: T.string.optional(),
    reputation: T.literalEnum('clean', 'suspicious', 'malicious', 'unknown'),
    confidence: T.number,
  }

  getDefaultProps(): DomainNodeShape['props'] {
    return {
      w: 220,
      h: 90,
      domain: '',
      label: '',
      reputation: 'unknown',
      confidence: 1.0,
    }
  }

  getGeometry(shape: DomainNodeShape): Geometry2d {
    return new Rectangle2d({
      width: shape.props.w,
      height: shape.props.h,
      isFilled: true,
    })
  }

  component(shape: DomainNodeShape) {
    const { domain, label, registrar, creationDate, reputation, confidence } = shape.props
    const repColor: Record<string, string> = {
      clean: '#10B981',
      suspicious: '#F59E0B',
      malicious: '#EF4444',
      unknown: '#6B7280',
    }

    return (
      <HTMLContainer>
        <div style={{
          width: '100%', height: '100%',
          border: '2px solid #10B981',
          borderRadius: 8,
          background: '#ECFDF5',
          padding: '10px 12px',
          fontFamily: 'monospace',
          display: 'flex', flexDirection: 'column', gap: 4,
          position: 'relative',
        }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
            <span style={{ fontSize: 18 }}>🔗</span>
            <span style={{ fontWeight: 600, fontSize: 13, color: '#065F46' }}>{domain || 'Domain'}</span>
            <span style={{
              marginLeft: 'auto', fontSize: 10, padding: '1px 6px',
              borderRadius: 4, background: repColor[reputation] ?? '#6B7280', color: '#fff',
            }}>{reputation}</span>
          </div>
          {label && <div style={{ fontSize: 11, color: '#64748B' }}>{label}</div>}
          <div style={{ display: 'flex', gap: 8, fontSize: 10, color: '#94A3B8', marginTop: 'auto' }}>
            {registrar && <span>🏢 {registrar}</span>}
            {creationDate && <span>📅 {creationDate}</span>}
            <span style={{ marginLeft: 'auto' }}>conf: {(confidence * 100).toFixed(0)}%</span>
          </div>
          <div style={{
            position: 'absolute', top: -8, right: 8,
            fontSize: 9, color: '#10B981', background: '#D1FAE5',
            padding: '1px 6px', borderRadius: 4,
          }}>DOMAIN</div>
        </div>
      </HTMLContainer>
    )
  }

  indicator(shape: DomainNodeShape) {
    return <rect width={shape.props.w} height={shape.props.h} rx={8} />
  }

  override onResize(_shape: DomainNodeShape, info: TLResizeInfo<DomainNodeShape>) {
    return {
      props: {
        w: Math.max(100, info.initialBounds.width * info.scaleX),
        h: Math.max(60, info.initialBounds.height * info.scaleY),
      },
    }
  }
}
