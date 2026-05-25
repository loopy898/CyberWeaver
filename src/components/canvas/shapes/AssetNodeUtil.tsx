import { ShapeUtil, HTMLContainer, TLBaseShape, T, Rectangle2d, type Geometry2d, type TLResizeInfo } from 'tldraw'
import type { RecordProps } from '@tldraw/tlschema'

export type AssetNodeShape = TLBaseShape<'asset', {
  w: number
  h: number
  hostname: string
  label: string
  os?: string
  ipAddresses?: string
  criticality: 'low' | 'medium' | 'high' | 'critical'
  confidence: number
}>

export class AssetNodeUtil extends ShapeUtil<AssetNodeShape> {
  static override type = 'asset' as const

  static override props: RecordProps<AssetNodeShape> = {
    w: T.number,
    h: T.number,
    hostname: T.string,
    label: T.string,
    os: T.string.optional(),
    ipAddresses: T.string.optional(),
    criticality: T.literalEnum('low', 'medium', 'high', 'critical'),
    confidence: T.number,
  }

  getDefaultProps(): AssetNodeShape['props'] {
    return {
      w: 220,
      h: 90,
      hostname: '',
      label: '',
      criticality: 'medium',
      confidence: 1.0,
    }
  }

  getGeometry(shape: AssetNodeShape): Geometry2d {
    return new Rectangle2d({
      width: shape.props.w,
      height: shape.props.h,
      isFilled: true,
    })
  }

  component(shape: AssetNodeShape) {
    const { hostname, label, os, ipAddresses, criticality, confidence } = shape.props
    const critColor: Record<string, string> = {
      low: '#10B981',
      medium: '#F59E0B',
      high: '#EF4444',
      critical: '#7C3AED',
    }

    return (
      <HTMLContainer>
        <div style={{
          width: '100%', height: '100%',
          border: '2px solid #14B8A6',
          borderRadius: 8,
          background: '#F0FDFA',
          padding: '10px 12px',
          fontFamily: 'monospace',
          display: 'flex', flexDirection: 'column', gap: 4,
          position: 'relative',
        }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
            <span style={{ fontSize: 18 }}>🖥️</span>
            <span style={{ fontWeight: 600, fontSize: 13, color: '#0F766E' }}>{hostname || 'Asset'}</span>
            <span style={{
              marginLeft: 'auto', fontSize: 10, padding: '1px 6px',
              borderRadius: 4, background: critColor[criticality] ?? '#6B7280', color: '#fff',
            }}>{criticality}</span>
          </div>
          {label && <div style={{ fontSize: 11, color: '#64748B' }}>{label}</div>}
          <div style={{ display: 'flex', gap: 8, fontSize: 10, color: '#94A3B8', marginTop: 'auto' }}>
            {os && <span>💿 {os}</span>}
            {ipAddresses && <span>🌐 {ipAddresses}</span>}
            <span style={{ marginLeft: 'auto' }}>conf: {(confidence * 100).toFixed(0)}%</span>
          </div>
          <div style={{
            position: 'absolute', top: -8, right: 8,
            fontSize: 9, color: '#14B8A6', background: '#CCFBF1',
            padding: '1px 6px', borderRadius: 4,
          }}>ASSET</div>
        </div>
      </HTMLContainer>
    )
  }

  indicator(shape: AssetNodeShape) {
    return <rect width={shape.props.w} height={shape.props.h} rx={8} />
  }

  override onResize(_shape: AssetNodeShape, info: TLResizeInfo<AssetNodeShape>) {
    return {
      props: {
        w: Math.max(100, info.initialBounds.width * info.scaleX),
        h: Math.max(60, info.initialBounds.height * info.scaleY),
      },
    }
  }
}
