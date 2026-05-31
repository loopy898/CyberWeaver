import { ShapeUtil, HTMLContainer, TLBaseShape, T, Rectangle2d, type Geometry2d, type TLResizeInfo } from 'tldraw'
import type { RecordProps } from '@tldraw/tlschema'

export type IpNodeShape = TLBaseShape<'ip-address', {
  w: number
  h: number
  address: string
  label: string
  geo?: string
  asn?: string
  reputation: 'clean' | 'suspicious' | 'malicious' | 'unknown'
  confidence: number
}>

export class IpNodeUtil extends ShapeUtil<IpNodeShape> {
  static override type = 'ip-address' as const

  static override props: RecordProps<IpNodeShape> = {
    w: T.number,
    h: T.number,
    address: T.string,
    label: T.string,
    geo: T.string.optional(),
    asn: T.string.optional(),
    reputation: T.literalEnum('clean', 'suspicious', 'malicious', 'unknown'),
    confidence: T.number,
  }

  getDefaultProps(): IpNodeShape['props'] {
    return {
      w: 220,
      h: 90,
      address: '',
      label: '',
      reputation: 'unknown',
      confidence: 1.0,
    }
  }

  getGeometry(shape: IpNodeShape): Geometry2d {
    return new Rectangle2d({
      width: shape.props.w,
      height: shape.props.h,
      isFilled: true,
    })
  }

  component(shape: IpNodeShape) {
    const { address, label, geo, reputation, confidence } = shape.props
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
          border: '2px solid #3B82F6',
          borderRadius: 8,
          background: '#EFF6FF',
          padding: '10px 12px',
          fontFamily: 'monospace',
          display: 'flex', flexDirection: 'column', gap: 4,
          position: 'relative',
        }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
            <span style={{ fontSize: 18 }}>🌐</span>
            <span style={{ fontWeight: 600, fontSize: 13, color: '#1E40AF' }}>{address || 'IP Address'}</span>
            <span style={{
              marginLeft: 'auto', fontSize: 10, padding: '1px 6px',
              borderRadius: 4, background: repColor[reputation] ?? '#6B7280', color: '#fff',
            }}>{reputation}</span>
          </div>
          {label && <div style={{ fontSize: 11, color: '#64748B' }}>{label}</div>}
          <div style={{ display: 'flex', gap: 8, fontSize: 10, color: '#94A3B8', marginTop: 'auto' }}>
            {geo && <span>📍 {geo}</span>}
            <span style={{ marginLeft: 'auto' }}>conf: {(confidence * 100).toFixed(0)}%</span>
          </div>
          <div style={{
            position: 'absolute', top: -8, right: 8,
            fontSize: 9, color: '#3B82F6', background: '#DBEAFE',
            padding: '1px 6px', borderRadius: 4,
          }}>IP</div>
        </div>
      </HTMLContainer>
    )
  }

  indicator(shape: IpNodeShape) {
    return <rect width={shape.props.w} height={shape.props.h} rx={8} />
  }

  override onResize(_shape: IpNodeShape, info: TLResizeInfo<IpNodeShape>) {
    return {
      props: {
        w: Math.max(100, info.initialBounds.width * info.scaleX),
        h: Math.max(60, info.initialBounds.height * info.scaleY),
      },
    }
  }
}
