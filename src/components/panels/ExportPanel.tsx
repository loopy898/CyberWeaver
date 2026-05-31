import { useMemo, useState, type CSSProperties } from 'react'
import { useInvestigationStore } from '../../stores/investigation'
import {
  useImportExport,
  type ExportFormat,
  type ImportFormat,
  type ImportResult,
  type ReportExportConfig,
} from '../../hooks/useImportExport'

type TabId = 'export' | 'import'

const EXPORT_FORMAT_LABELS: Record<ExportFormat, string> = {
  json_canvas: 'JSON Canvas',
  stix: 'STIX 2.1',
  attack_flow: 'Attack Flow',
  html_report: 'HTML Report',
}

const IMPORT_FORMAT_LABELS: Record<ImportFormat, string> = {
  json_canvas: 'JSON Canvas',
  stix: 'STIX 2.1',
  attack_flow: 'Attack Flow',
}

const DEFAULT_REPORT_CONFIG: ReportExportConfig = {
  title: 'CyberWeaver Threat Investigation Report',
  author: '',
  organization: '',
  include_ioc_list: true,
  include_graph_summary: true,
}

function getErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}

export default function ExportPanel() {
  const currentId = useInvestigationStore((s) => s.currentId)
  const editor = useInvestigationStore((s) => s.editor)
  const investigationId = currentId ?? 'default'
  const { isExporting, isImporting, exportData, selectImportFile, importData } = useImportExport()

  const [activeTab, setActiveTab] = useState<TabId>('export')
  const [exportFormat, setExportFormat] = useState<ExportFormat>('json_canvas')
  const [importFormat, setImportFormat] = useState<ImportFormat>('json_canvas')
  const [reportConfig, setReportConfig] = useState<ReportExportConfig>(DEFAULT_REPORT_CONFIG)
  const [selectedImportFile, setSelectedImportFile] = useState<string>('')
  const [importPayload, setImportPayload] = useState<string>('')
  const [status, setStatus] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [importResult, setImportResult] = useState<ImportResult | null>(null)

  const statusTone = useMemo<'success' | 'error' | null>(() => {
    if (error) return 'error'
    if (status) return 'success'
    return null
  }, [error, status])

  const handleExport = async () => {
    setError(null)
    setStatus(null)

    try {
      const result = await exportData({
        format: exportFormat,
        investigationId,
        config: reportConfig,
      })

      const methodLabel = result.method === 'tauri' ? '系统保存对话框' : '浏览器下载'
      setStatus(`已导出 ${result.fileName}（${methodLabel}）`)
    } catch (err) {
      setError(`导出失败：${getErrorMessage(err)}`)
    }
  }

  const handleChooseFile = async () => {
    setError(null)
    setStatus(null)
    setImportResult(null)

    try {
      const file = await selectImportFile(importFormat)
      if (!file) return

      setSelectedImportFile(file.fileName)
      setImportPayload(file.content)
      setStatus(`已选择文件：${file.fileName}`)
    } catch (err) {
      setError(`读取文件失败：${getErrorMessage(err)}`)
    }
  }

  const handleImport = async () => {
    if (!importPayload.trim()) {
      setError('请先选择导入文件')
      setStatus(null)
      return
    }

    setError(null)
    setStatus(null)

    try {
      const result = await importData({
        format: importFormat,
        investigationId,
        json: importPayload,
      })
      setImportResult(result)
      setStatus(`导入完成：节点 ${result.nodes_imported}，关系 ${result.relations_imported}`)
    } catch (err) {
      setImportResult(null)
      setError(`导入失败：${getErrorMessage(err)}`)
    }
  }

  const handleRefreshCanvas = () => {
    if (typeof window !== 'undefined') {
      window.location.reload()
    }
  }

  if (!editor) return null

  return (
    <div style={panelStyle}>
      <div style={{ display: 'flex', gap: 4, marginBottom: 12 }}>
        <button style={tabStyle(activeTab === 'export')} onClick={() => setActiveTab('export')}>
          导出
        </button>
        <button style={tabStyle(activeTab === 'import')} onClick={() => setActiveTab('import')}>
          导入
        </button>
      </div>

      <div style={metaTextStyle}>调查 ID：{investigationId}</div>

      {statusTone ? (
        <div style={statusBoxStyle(statusTone)}>
          {error ?? status}
        </div>
      ) : null}

      {activeTab === 'export' ? (
        <div style={sectionStyle}>
          <div style={labelStyle}>导出格式</div>
          <div style={formatGridStyle}>
            {Object.entries(EXPORT_FORMAT_LABELS).map(([format, label]) => (
              <button
                key={format}
                style={formatButtonStyle(exportFormat === format)}
                onClick={() => setExportFormat(format as ExportFormat)}
              >
                {label}
              </button>
            ))}
          </div>

          {exportFormat === 'html_report' ? (
            <div style={sectionStyle}>
              <div style={labelStyle}>报告配置</div>

              <label style={fieldStyle}>
                标题
                <input
                  style={inputStyle}
                  value={reportConfig.title}
                  onChange={(event) => setReportConfig((prev) => ({ ...prev, title: event.target.value }))}
                  placeholder="报告标题"
                />
              </label>

              <label style={fieldStyle}>
                作者
                <input
                  style={inputStyle}
                  value={reportConfig.author}
                  onChange={(event) => setReportConfig((prev) => ({ ...prev, author: event.target.value }))}
                  placeholder="分析师姓名"
                />
              </label>

              <label style={fieldStyle}>
                组织
                <input
                  style={inputStyle}
                  value={reportConfig.organization}
                  onChange={(event) => setReportConfig((prev) => ({ ...prev, organization: event.target.value }))}
                  placeholder="组织名称"
                />
              </label>

              <label style={checkboxRowStyle}>
                <input
                  type="checkbox"
                  checked={reportConfig.include_ioc_list}
                  onChange={(event) =>
                    setReportConfig((prev) => ({ ...prev, include_ioc_list: event.target.checked }))
                  }
                />
                <span>包含 IOC 清单</span>
              </label>

              <label style={checkboxRowStyle}>
                <input
                  type="checkbox"
                  checked={reportConfig.include_graph_summary}
                  onChange={(event) =>
                    setReportConfig((prev) => ({ ...prev, include_graph_summary: event.target.checked }))
                  }
                />
                <span>包含图谱摘要</span>
              </label>
            </div>
          ) : null}

          <button style={primaryButtonStyle} onClick={handleExport} disabled={isExporting}>
            {isExporting ? '导出中...' : '导出'}
          </button>

          <div style={hintStyle}>
            优先使用 Tauri 保存对话框；若插件未安装，则自动回退到浏览器下载。
          </div>
        </div>
      ) : null}

      {activeTab === 'import' ? (
        <div style={sectionStyle}>
          <div style={labelStyle}>导入格式</div>
          <div style={formatGridStyle}>
            {Object.entries(IMPORT_FORMAT_LABELS).map(([format, label]) => (
              <button
                key={format}
                style={formatButtonStyle(importFormat === format)}
                onClick={() => setImportFormat(format as ImportFormat)}
              >
                {label}
              </button>
            ))}
          </div>

          <button style={secondaryButtonStyle} onClick={handleChooseFile} disabled={isImporting}>
            选择文件
          </button>

          <div style={fileInfoStyle}>
            {selectedImportFile ? `当前文件：${selectedImportFile}` : '尚未选择文件'}
          </div>

          <button style={primaryButtonStyle} onClick={handleImport} disabled={isImporting || !importPayload.trim()}>
            {isImporting ? '导入中...' : '开始导入'}
          </button>

          {importResult ? (
            <div style={resultBoxStyle}>
              <div>导入节点：{importResult.nodes_imported}</div>
              <div>导入关系：{importResult.relations_imported}</div>
              <div>错误数：{importResult.errors.length}</div>
              {importResult.errors.length > 0 ? (
                <ul style={errorListStyle}>
                  {importResult.errors.map((item, index) => (
                    <li key={`${item}-${index}`}>{item}</li>
                  ))}
                </ul>
              ) : (
                <div style={hintStyle}>未返回错误。</div>
              )}
            </div>
          ) : null}

          <button
            style={secondaryButtonStyle}
            onClick={handleRefreshCanvas}
            disabled={!importResult}
            title="导入后建议刷新页面，让持久化层重新加载节点"
          >
            刷新画布
          </button>

          <div style={hintStyle}>
            导入完成后建议点击“刷新画布”或手动刷新页面，以重新加载数据库中的图谱数据。
          </div>
        </div>
      ) : null}
    </div>
  )
}

const panelStyle: CSSProperties = {
  position: 'fixed',
  right: 16,
  top: 650,
  width: 300,
  maxHeight: 'calc(100vh - 666px)',
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
  fontSize: 12,
  display: 'flex',
  flexDirection: 'column',
  gap: 10,
}

const sectionStyle: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: 10,
}

const metaTextStyle: CSSProperties = {
  color: '#94a3b8',
  fontSize: 11,
}

const labelStyle: CSSProperties = {
  color: '#cbd5e1',
  fontSize: 11,
  fontWeight: 600,
}

const fieldStyle: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: 6,
  color: '#cbd5e1',
  fontSize: 11,
}

const inputStyle: CSSProperties = {
  width: '100%',
  boxSizing: 'border-box',
  padding: '8px 10px',
  background: 'rgba(255, 255, 255, 0.06)',
  border: '1px solid rgba(255, 255, 255, 0.12)',
  borderRadius: 8,
  color: '#e2e8f0',
  fontSize: 12,
  fontFamily: 'inherit',
}

const formatGridStyle: CSSProperties = {
  display: 'grid',
  gridTemplateColumns: 'repeat(2, minmax(0, 1fr))',
  gap: 8,
}

const checkboxRowStyle: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: 8,
  color: '#cbd5e1',
  fontSize: 12,
}

const fileInfoStyle: CSSProperties = {
  padding: '10px 12px',
  borderRadius: 8,
  background: 'rgba(255, 255, 255, 0.04)',
  border: '1px solid rgba(255, 255, 255, 0.06)',
  color: '#cbd5e1',
  fontSize: 11,
  wordBreak: 'break-all',
}

const hintStyle: CSSProperties = {
  color: '#94a3b8',
  fontSize: 11,
  lineHeight: 1.5,
}

const resultBoxStyle: CSSProperties = {
  padding: 12,
  borderRadius: 10,
  background: 'rgba(15, 23, 42, 0.6)',
  border: '1px solid rgba(255, 255, 255, 0.08)',
  display: 'flex',
  flexDirection: 'column',
  gap: 6,
}

const errorListStyle: CSSProperties = {
  margin: 0,
  paddingLeft: 18,
  color: '#fca5a5',
  fontSize: 11,
  lineHeight: 1.5,
}

function tabStyle(active: boolean): CSSProperties {
  return {
    flex: 1,
    border: 'none',
    borderRadius: 8,
    padding: '8px 12px',
    background: active ? 'rgba(14, 165, 233, 0.22)' : 'rgba(255, 255, 255, 0.05)',
    color: active ? '#e0f2fe' : '#94a3b8',
    fontSize: 12,
    fontWeight: 600,
    cursor: 'pointer',
  }
}

function formatButtonStyle(active: boolean): CSSProperties {
  return {
    border: '1px solid rgba(255, 255, 255, 0.08)',
    borderRadius: 8,
    padding: '8px 10px',
    background: active ? 'rgba(34, 197, 94, 0.18)' : 'rgba(255, 255, 255, 0.04)',
    color: active ? '#bbf7d0' : '#cbd5e1',
    fontSize: 11,
    fontWeight: 600,
    cursor: 'pointer',
  }
}

const baseButtonStyle: CSSProperties = {
  width: '100%',
  border: 'none',
  borderRadius: 8,
  padding: '10px 12px',
  fontSize: 12,
  fontWeight: 600,
  cursor: 'pointer',
}

const primaryButtonStyle: CSSProperties = {
  ...baseButtonStyle,
  background: 'linear-gradient(135deg, #0ea5e9, #0284c7)',
  color: '#f8fafc',
}

const secondaryButtonStyle: CSSProperties = {
  ...baseButtonStyle,
  background: 'rgba(255, 255, 255, 0.08)',
  color: '#e2e8f0',
}

function statusBoxStyle(tone: 'success' | 'error'): CSSProperties {
  return {
    padding: '10px 12px',
    borderRadius: 8,
    background: tone === 'success' ? 'rgba(34, 197, 94, 0.14)' : 'rgba(248, 113, 113, 0.14)',
    color: tone === 'success' ? '#4ade80' : '#fda4af',
    fontSize: 11,
    lineHeight: 1.5,
  }
}
