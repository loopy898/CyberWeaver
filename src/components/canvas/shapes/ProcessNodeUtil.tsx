import { ShapeUtil, HTMLContainer, TLBaseShape, T, Rectangle2d, type Geometry2d, type TLResizeInfo } from 'tldraw'
import type { RecordProps } from '@tldraw/tlschema'

export type ProcessNodeShape = TLBaseShape<'process', {
  w: number
  h: number
  processName: string
  pid: number
  label: string
  user?: string
  commandLine?: string
  confidence: number
}>

export class ProcessNodeUtil extends ShapeUtil<ProcessNodeShape> {
  static override type = 'process' as const

  static override props: RecordProps<ProcessNodeShape> = {
    w: T.number,
    h: T.number,
    processName: T.string,
    pid: T.number,
    label: T.string,
    user: T.string.optional(),
    commandLine: T.string.optional(),
    confidence: T.number,
  }

  getDefaultProps(): ProcessNodeShape['props'] {
    return {
      w: 220,
      h: 90,
      processName: '',
      pid: 0,
      label: '',
      confidence: 1.0,
    }
  }

  getGeometry(shape: ProcessNodeShape): Geometry2d {
    return new Rectangle2d({
      width: shape.props.w,
      height: shape.props.h,
      isFilled: true,
    })
  }

  component(shape: ProcessNodeShape) {
    const { processName, pid, label, user, commandLine, confidence } = shape.props
    const truncatedCmd = commandLine && commandLine.length > 30
      ? commandLine.slice(0, 30) + '...'
      : commandLine

    return (
      <HTMLContainer>
        <div style={{
          width: '100%', height: '100%',
          border: '2px solid #6B7280',
          borderRadius: 8,
          background: '#F9FAFB',
          padding: '10px 12px',
          fontFamily: 'monospace',
          display: 'flex', flexDirection: 'column', gap: 4,
          position: 'relative',
        }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
            <span style={{ fontSize: 18 }}>⚙️</span>
            <span style={{ fontWeight: 600, fontSize: 13, color: '#374151' }}>{processName || 'Process'}</span>
            <span style={{
              marginLeft: 'auto', fontSize: 10, padding: '1px 6px',
              borderRadius: 4, background: '#E5E7EB', color: '#374151',
            }}>PID:{pid}</span>
          </div>
          {label && <div style={{ fontSize: 11, color: '#64748B' }}>{label}</div>}
          <div style={{ display: 'flex', flexDirection: 'column', gap: 2, fontSize: 10, color: '#9CA3AF', marginTop: 'auto' }}>
            {user && <span>👤 {user}</span>}
            {truncatedCmd && <span>💻 {truncatedCmd}</span>}
            <span style={{ alignSelf: 'flex-end' }}>conf: {(confidence * 100).toFixed(0)}%</span>
          </div>
          <div style={{
            position: 'absolute', top: -8, right: 8,
            fontSize: 9, color: '#6B7280', background: '#F3F4F6',
            padding: '1px 6px', borderRadius: 4,
          }}>PROCESS</div>
        </div>
      </HTMLContainer>
    )
  }

  indicator(shape: ProcessNodeShape) {
    return <rect width={shape.props.w} height={shape.props.h} rx={8} />
  }

  override onResize(_shape: ProcessNodeShape, info: TLResizeInfo<ProcessNodeShape>) {
    return {
      props: {
        w: Math.max(100, info.initialBounds.width * info.scaleX),
        h: Math.max(60, info.initialBounds.height * info.scaleY),
      },
    }
  }
}
