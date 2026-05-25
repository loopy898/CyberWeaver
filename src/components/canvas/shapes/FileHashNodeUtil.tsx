import { ShapeUtil, HTMLContainer, TLBaseShape, T, Rectangle2d, type Geometry2d, type TLResizeInfo } from 'tldraw'
import type { RecordProps } from '@tldraw/tlschema'

export type FileHashNodeShape = TLBaseShape<'file-hash', {
  w: number
  h: number
  hashValue: string
  algorithm: 'MD5' | 'SHA1' | 'SHA256' | 'SHA512'
  label: string
  fileName?: string
  malwareClassification?: string
  confidence: number
}>

export class FileHashNodeUtil extends ShapeUtil<FileHashNodeShape> {
  static override type = 'file-hash' as const

  static override props: RecordProps<FileHashNodeShape> = {
    w: T.number,
    h: T.number,
    hashValue: T.string,
    algorithm: T.literalEnum('MD5', 'SHA1', 'SHA256', 'SHA512'),
    label: T.string,
    fileName: T.string.optional(),
    malwareClassification: T.string.optional(),
    confidence: T.number,
  }

  getDefaultProps(): FileHashNodeShape['props'] {
    return {
      w: 220,
      h: 90,
      hashValue: '',
      algorithm: 'SHA256',
      label: '',
      confidence: 1.0,
    }
  }

  getGeometry(shape: FileHashNodeShape): Geometry2d {
    return new Rectangle2d({
      width: shape.props.w,
      height: shape.props.h,
      isFilled: true,
    })
  }

  component(shape: FileHashNodeShape) {
    const { hashValue, algorithm, label, fileName, malwareClassification, confidence } = shape.props
    const truncated = hashValue.length > 12 ? hashValue.slice(0, 12) + '...' : hashValue

    return (
      <HTMLContainer>
        <div style={{
          width: '100%', height: '100%',
          border: '2px solid #F59E0B',
          borderRadius: 8,
          background: '#FFFBEB',
          padding: '10px 12px',
          fontFamily: 'monospace',
          display: 'flex', flexDirection: 'column', gap: 4,
          position: 'relative',
        }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
            <span style={{ fontSize: 18 }}>📄</span>
            <span style={{ fontWeight: 600, fontSize: 12, color: '#92400E' }}>{truncated || 'File Hash'}</span>
            <span style={{
              marginLeft: 'auto', fontSize: 10, padding: '1px 6px',
              borderRadius: 4, background: '#FDE68A', color: '#78350F',
            }}>{algorithm}</span>
          </div>
          {label && <div style={{ fontSize: 11, color: '#64748B' }}>{label}</div>}
          <div style={{ display: 'flex', gap: 8, fontSize: 10, color: '#94A3B8', marginTop: 'auto' }}>
            {fileName && <span>📁 {fileName}</span>}
            {malwareClassification && <span>⚠️ {malwareClassification}</span>}
            <span style={{ marginLeft: 'auto' }}>conf: {(confidence * 100).toFixed(0)}%</span>
          </div>
          <div style={{
            position: 'absolute', top: -8, right: 8,
            fontSize: 9, color: '#F59E0B', background: '#FEF3C7',
            padding: '1px 6px', borderRadius: 4,
          }}>HASH</div>
        </div>
      </HTMLContainer>
    )
  }

  indicator(shape: FileHashNodeShape) {
    return <rect width={shape.props.w} height={shape.props.h} rx={8} />
  }

  override onResize(_shape: FileHashNodeShape, info: TLResizeInfo<FileHashNodeShape>) {
    return {
      props: {
        w: Math.max(100, info.initialBounds.width * info.scaleX),
        h: Math.max(60, info.initialBounds.height * info.scaleY),
      },
    }
  }
}
