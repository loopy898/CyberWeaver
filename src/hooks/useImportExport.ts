import { useCallback, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'

export type ExportFormat = 'json_canvas' | 'stix' | 'attack_flow' | 'html_report'
export type ImportFormat = 'json_canvas' | 'stix' | 'attack_flow'

export interface ReportExportConfig {
  title: string
  author: string
  organization: string
  include_ioc_list: boolean
  include_graph_summary: boolean
}

export interface ImportResult {
  nodes_imported: number
  relations_imported: number
  errors: string[]
}

interface ExportParams {
  format: ExportFormat
  investigationId: string
  config: ReportExportConfig
}

interface ImportParams {
  format: ImportFormat
  investigationId: string
  json: string
}

interface ExportResult {
  fileName: string
  method: 'tauri' | 'browser'
}

const EXPORT_COMMANDS: Record<ExportFormat, string> = {
  json_canvas: 'export_json_canvas',
  stix: 'export_stix',
  attack_flow: 'export_attack_flow',
  html_report: 'export_report',
}

const IMPORT_COMMANDS: Record<ImportFormat, string> = {
  json_canvas: 'import_json_canvas',
  stix: 'import_stix',
  attack_flow: 'import_attack_flow',
}

const EXPORT_SUFFIXES: Record<ExportFormat, string> = {
  json_canvas: '.json',
  stix: '.stix.json',
  attack_flow: '.attack-flow.json',
  html_report: '-report.html',
}

const MIME_TYPES: Record<ExportFormat, string> = {
  json_canvas: 'application/json',
  stix: 'application/json',
  attack_flow: 'application/json',
  html_report: 'text/html;charset=utf-8',
}

const FILE_FILTERS: Record<ExportFormat, Array<{ name: string; extensions: string[] }>> = {
  json_canvas: [{ name: 'JSON Canvas', extensions: ['json'] }],
  stix: [{ name: 'STIX 2.1', extensions: ['json'] }],
  attack_flow: [{ name: 'Attack Flow', extensions: ['json'] }],
  html_report: [{ name: 'HTML Report', extensions: ['html'] }],
}

const IMPORT_ACCEPT: Record<ImportFormat, string> = {
  json_canvas: '.json,application/json',
  stix: '.json,application/json',
  attack_flow: '.json,application/json',
}

const runtimeImport = new Function('specifier', 'return import(specifier)') as <T = unknown>(
  specifier: string,
) => Promise<T>

function normalizeInvestigationId(value: string): string {
  const trimmed = value.trim()
  const safe = trimmed.replace(/[^a-zA-Z0-9_-]+/g, '-').replace(/^-+|-+$/g, '')
  return safe || 'default'
}

function downloadTextFile(content: string, fileName: string, mimeType: string) {
  if (typeof window === 'undefined' || typeof document === 'undefined') {
    throw new Error('当前环境不支持浏览器文件下载')
  }

  const blob = new Blob([content], { type: mimeType })
  const url = URL.createObjectURL(blob)
  const anchor = document.createElement('a')
  anchor.href = url
  anchor.download = fileName
  anchor.style.display = 'none'
  document.body.appendChild(anchor)
  anchor.click()
  anchor.remove()
  window.setTimeout(() => URL.revokeObjectURL(url), 0)
}

async function chooseFileWithBrowser(accept: string): Promise<File | null> {
  if (typeof document === 'undefined') {
    throw new Error('当前环境不支持浏览器文件选择')
  }

  return new Promise<File | null>((resolve) => {
    const input = document.createElement('input')
    input.type = 'file'
    input.accept = accept
    input.style.display = 'none'
    input.onchange = () => {
      resolve(input.files?.[0] ?? null)
      input.remove()
    }
    document.body.appendChild(input)
    input.click()
  })
}

async function trySaveWithTauri(
  content: string,
  fileName: string,
  format: ExportFormat,
): Promise<boolean> {
  try {
    const [{ save }, { writeTextFile }] = await Promise.all([
      runtimeImport<{ save: (options?: Record<string, unknown>) => Promise<string | null> }>(
        '@tauri-apps/plugin-dialog',
      ),
      runtimeImport<{ writeTextFile: (path: string, data: string) => Promise<void> }>(
        '@tauri-apps/plugin-fs',
      ),
    ])

    const path = await save({
      defaultPath: fileName,
      filters: FILE_FILTERS[format],
    })

    if (!path) return false

    await writeTextFile(path, content)
    return true
  } catch {
    return false
  }
}

async function tryOpenWithTauri(format: ImportFormat): Promise<{ name: string; text: string } | null> {
  try {
    const [{ open }, { readTextFile }] = await Promise.all([
      runtimeImport<{ open: (options?: Record<string, unknown>) => Promise<string | string[] | null> }>(
        '@tauri-apps/plugin-dialog',
      ),
      runtimeImport<{ readTextFile: (path: string) => Promise<string> }>(
        '@tauri-apps/plugin-fs',
      ),
    ])

    const selected = await open({
      multiple: false,
      filters: FILE_FILTERS[format],
    })

    const path = typeof selected === 'string' ? selected : Array.isArray(selected) ? selected[0] : null
    if (!path) return null

    const text = await readTextFile(path)
    const name = path.split('/').pop() || path
    return { name, text }
  } catch {
    return null
  }
}

export function getExportCommand(format: ExportFormat): string {
  return EXPORT_COMMANDS[format]
}

export function getImportCommand(format: ImportFormat): string {
  return IMPORT_COMMANDS[format]
}

export function getDefaultExportFileName(format: ExportFormat, investigationId: string): string {
  return `cyberweaver-${normalizeInvestigationId(investigationId)}${EXPORT_SUFFIXES[format]}`
}

export function useImportExport() {
  const [isExporting, setIsExporting] = useState(false)
  const [isImporting, setIsImporting] = useState(false)

  const exportData = useCallback(async ({ format, investigationId, config }: ExportParams): Promise<ExportResult> => {
    setIsExporting(true)
    try {
      const content =
        format === 'html_report'
          ? await invoke<string>(getExportCommand(format), {
              investigation_id: investigationId,
              config,
            })
          : await invoke<string>(getExportCommand(format), {
              investigation_id: investigationId,
            })

      const fileName = getDefaultExportFileName(format, investigationId)
      const savedByTauri = await trySaveWithTauri(content, fileName, format)

      if (savedByTauri) {
        return {
          fileName,
          method: 'tauri',
        }
      }

      downloadTextFile(content, fileName, MIME_TYPES[format])
      return {
        fileName,
        method: 'browser',
      }
    } finally {
      setIsExporting(false)
    }
  }, [])

  const selectImportFile = useCallback(async (format: ImportFormat): Promise<{ fileName: string; content: string } | null> => {
    const tauriFile = await tryOpenWithTauri(format)
    if (tauriFile) {
      return {
        fileName: tauriFile.name,
        content: tauriFile.text,
      }
    }

    const browserFile = await chooseFileWithBrowser(IMPORT_ACCEPT[format])
    if (!browserFile) return null

    return {
      fileName: browserFile.name,
      content: await browserFile.text(),
    }
  }, [])

  const importData = useCallback(async ({ format, investigationId, json }: ImportParams): Promise<ImportResult> => {
    setIsImporting(true)
    try {
      return await invoke<ImportResult>(getImportCommand(format), {
        investigation_id: investigationId,
        json,
      })
    } finally {
      setIsImporting(false)
    }
  }, [])

  return {
    isExporting,
    isImporting,
    exportData,
    selectImportFile,
    importData,
  }
}
