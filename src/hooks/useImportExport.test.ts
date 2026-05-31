import assert from 'node:assert/strict'
import test from 'node:test'

import {
  getDefaultExportFileName,
  getExportCommand,
  getImportCommand,
  type ExportFormat,
  type ImportFormat,
} from './useImportExport.ts'

test('maps export formats to tauri commands', () => {
  const cases: Array<[ExportFormat, string]> = [
    ['json_canvas', 'export_json_canvas'],
    ['stix', 'export_stix'],
    ['attack_flow', 'export_attack_flow'],
    ['html_report', 'export_report'],
  ]

  for (const [format, expected] of cases) {
    assert.equal(getExportCommand(format), expected)
  }
})

test('maps import formats to tauri commands', () => {
  const cases: Array<[ImportFormat, string]> = [
    ['json_canvas', 'import_json_canvas'],
    ['stix', 'import_stix'],
    ['attack_flow', 'import_attack_flow'],
  ]

  for (const [format, expected] of cases) {
    assert.equal(getImportCommand(format), expected)
  }
})

test('builds stable default file names for exports', () => {
  assert.equal(getDefaultExportFileName('json_canvas', 'case-01'), 'cyberweaver-case-01.json')
  assert.equal(getDefaultExportFileName('stix', 'default'), 'cyberweaver-default.stix.json')
  assert.equal(getDefaultExportFileName('attack_flow', 'hunt'), 'cyberweaver-hunt.attack-flow.json')
  assert.equal(getDefaultExportFileName('html_report', 'report'), 'cyberweaver-report-report.html')
})
