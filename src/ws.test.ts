import assert from 'node:assert/strict'
import test from 'node:test'

import {
  createToolExecutionCommand,
  buildReportSummaryText,
  createNodeSearchIndex,
  parseAgentTokenMessage,
  parseForensicsReport,
  parseGraphUpdateMessage,
  parseGraphSnapshot,
  parseToolResultMessage,
} from './ws.ts'

test('parses added note node from graph_update message', () => {
  const result = parseGraphUpdateMessage(
    JSON.stringify({
      type: 'graph_update',
      delta: {
        added_nodes: [
          { id: '1', node_type: 'note', x: 10, y: 20, content: 'hello' },
        ],
        updated_nodes: [],
      },
    })
  )

  assert.equal(result.addedNodes.length, 1)
  assert.equal(result.addedEdges.length, 0)
  assert.deepEqual(result.addedNodes[0], {
    id: '1',
    type: 'note',
    x: 10,
    y: 20,
    content: 'hello',
  })
})

test('ignores unsupported pushed node types', () => {
  const result = parseGraphUpdateMessage(
    JSON.stringify({
      type: 'graph_update',
      delta: {
        added_nodes: [
          { id: '1', node_type: 'arrow', x: 0, y: 0, content: '' },
        ],
        updated_nodes: [],
      },
    })
  )

  assert.equal(result.addedNodes.length, 0)
})

test('parses graph edges from graph_update payload', () => {
  const result = parseGraphUpdateMessage(
    JSON.stringify({
      type: 'graph_update',
      delta: {
        added_nodes: [],
        updated_nodes: [],
        added_edges: [{ id: 'e1', source_id: 'n1', target_id: 'n2', relation: 'scan_result' }],
        updated_edges: [],
      },
    })
  )

  assert.equal(result.addedEdges.length, 1)
  assert.deepEqual(result.addedEdges[0], {
    id: 'e1',
    sourceId: 'n1',
    targetId: 'n2',
    relation: 'scan_result',
  })
})

test('parses tool_result event', () => {
  const result = parseToolResultMessage(
    JSON.stringify({
      type: 'tool_result',
      tool: 'scan_port',
      ok: true,
      message: 'done',
    })
  )

  assert.deepEqual(result, {
    tool: 'scan_port',
    ok: true,
    message: 'done',
  })
})

test('parses agent_token event', () => {
  const result = parseAgentTokenMessage(
    JSON.stringify({
      type: 'agent_token',
      token: 'thinking',
    })
  )

  assert.deepEqual(result, { token: 'thinking' })
})

test('builds tool execution command payload', () => {
  const raw = createToolExecutionCommand('scan_port', { target_id: 'n1' })
  const parsed = JSON.parse(raw)
  assert.equal(parsed.type, 'tool_execution')
  assert.equal(parsed.tool, 'scan_port')
  assert.equal(parsed.params.target_id, 'n1')
})

test('builds timestamp_convert command payload', () => {
  const raw = createToolExecutionCommand('timestamp_convert', { value: '1711603200' })
  const parsed = JSON.parse(raw)
  assert.equal(parsed.type, 'tool_execution')
  assert.equal(parsed.tool, 'timestamp_convert')
  assert.equal(parsed.params.value, '1711603200')
})

test('builds reverse_geocode command payload', () => {
  const raw = createToolExecutionCommand('reverse_geocode', { latitude: 39.9042, longitude: 116.4074 })
  const parsed = JSON.parse(raw)
  assert.equal(parsed.type, 'tool_execution')
  assert.equal(parsed.tool, 'reverse_geocode')
  assert.equal(parsed.params.latitude, 39.9042)
  assert.equal(parsed.params.longitude, 116.4074)
})

test('parses graph snapshot payload', () => {
  const result = parseGraphSnapshot(
    JSON.stringify({
      nodes: [{ id: 'n1', node_type: 'note', x: 10, y: 20, content: 'host' }],
      edges: [{ id: 'e1', source_id: 'n1', target_id: 'n2', relation: 'linked' }],
    })
  )

  assert.equal(result.nodes.length, 1)
  assert.equal(result.edges.length, 1)
  assert.equal(result.nodes[0]?.id, 'n1')
  assert.equal(result.edges[0]?.sourceId, 'n1')
})

test('parses forensics report payload', () => {
  const report = parseForensicsReport(
    JSON.stringify({
      generated_at: '2026-03-27T12:00:00Z',
      summary: {
        node_count: 2,
        edge_count: 1,
        component_count: 0,
        finding_count: 1,
      },
      findings: [
        {
          node_id: 'scan:seed-host',
          title: '扫描发现',
          relation: 'scan_result',
          evidence: 'seed-host 开放端口: 22, 80, 443',
        },
      ],
      markdown: '# CyberWeaver 自动化取证报告',
    })
  )

  assert.equal(report.summary.nodeCount, 2)
  assert.equal(report.summary.findingCount, 1)
  assert.equal(report.findings[0]?.nodeId, 'scan:seed-host')
  assert.equal(report.findings[0]?.title, '扫描发现')
})

test('creates searchable node index from id and content', () => {
  const result = createNodeSearchIndex([
    { id: 'seed-host', type: 'note', x: 0, y: 0, content: 'Host: 192.168.1.10 sshd' },
    { id: 'scan:seed-host', type: 'note', x: 0, y: 0, content: '开放端口: 22, 80, 443' },
  ], '22')

  assert.equal(result.length, 1)
  assert.equal(result[0]?.id, 'scan:seed-host')
})

test('builds human readable report summary text', () => {
  const summary = buildReportSummaryText({
    generatedAt: '2026-03-27T12:00:00Z',
    summary: {
      nodeCount: 4,
      edgeCount: 3,
      componentCount: 2,
      findingCount: 1,
    },
    findings: [],
    markdown: '# report',
  })

  assert.match(summary, /节点 4/)
  assert.match(summary, /发现 1/)
})
