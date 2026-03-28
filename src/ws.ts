import { isSupportedNodeType, type PersistedNode } from './graphNodes.ts'

type RawGraphNode = {
  id?: unknown
  node_type?: unknown
  x?: unknown
  y?: unknown
  content?: unknown
}

type RawGraphEdge = {
  id?: unknown
  source_id?: unknown
  target_id?: unknown
  relation?: unknown
}

type RawGraphUpdate = {
  type?: unknown
  delta?: {
    added_nodes?: RawGraphNode[]
    updated_nodes?: RawGraphNode[]
    added_edges?: RawGraphEdge[]
    updated_edges?: RawGraphEdge[]
  }
}

type RawGraphSnapshot = {
  nodes?: RawGraphNode[]
  edges?: RawGraphEdge[]
}

export type GraphUpdateMessage = {
  addedNodes: PersistedNode[]
  updatedNodes: PersistedNode[]
  addedEdges: GraphEdge[]
  updatedEdges: GraphEdge[]
}

type RawToolResult = {
  type?: unknown
  tool?: unknown
  ok?: unknown
  message?: unknown
}

type RawAgentToken = {
  type?: unknown
  token?: unknown
}

export type GraphEdge = {
  id: string
  sourceId: string
  targetId: string
  relation: string
}

export type ToolResultEvent = {
  tool: string
  ok: boolean
  message: string
}

export type AgentTokenEvent = {
  token: string
}

export type GraphSnapshot = {
  nodes: PersistedNode[]
  edges: GraphEdge[]
}

type RawForensicsSummary = {
  node_count?: unknown
  edge_count?: unknown
  component_count?: unknown
  finding_count?: unknown
}

type RawForensicsFinding = {
  node_id?: unknown
  title?: unknown
  relation?: unknown
  evidence?: unknown
}

type RawForensicsReport = {
  generated_at?: unknown
  summary?: RawForensicsSummary
  findings?: RawForensicsFinding[]
  markdown?: unknown
}

export type ForensicsFinding = {
  nodeId: string
  title: string
  relation: string
  evidence: string
}

export type ForensicsReport = {
  generatedAt: string
  summary: {
    nodeCount: number
    edgeCount: number
    componentCount: number
    findingCount: number
  }
  findings: ForensicsFinding[]
  markdown: string
}

function normalizeGraphNode(node: RawGraphNode): PersistedNode | null {
  if (typeof node.id !== 'string' || typeof node.node_type !== 'string') return null
  if (!isSupportedNodeType(node.node_type)) return null

  return {
    id: node.id,
    type: node.node_type,
    x: typeof node.x === 'number' ? node.x : 0,
    y: typeof node.y === 'number' ? node.y : 0,
    content: typeof node.content === 'string' ? node.content : '',
  }
}

function normalizeNodes(nodes: unknown): PersistedNode[] {
  if (!Array.isArray(nodes)) return []
  return nodes.map((node) => normalizeGraphNode(node as RawGraphNode)).filter(Boolean) as PersistedNode[]
}

function normalizeGraphEdge(edge: RawGraphEdge): GraphEdge | null {
  if (typeof edge.id !== 'string') return null
  if (typeof edge.source_id !== 'string' || typeof edge.target_id !== 'string') return null
  return {
    id: edge.id,
    sourceId: edge.source_id,
    targetId: edge.target_id,
    relation: typeof edge.relation === 'string' ? edge.relation : 'related_to',
  }
}

function normalizeEdges(edges: unknown): GraphEdge[] {
  if (!Array.isArray(edges)) return []
  return edges.map((edge) => normalizeGraphEdge(edge as RawGraphEdge)).filter(Boolean) as GraphEdge[]
}

function normalizeNumber(value: unknown) {
  return typeof value === 'number' && Number.isFinite(value) ? value : 0
}

function normalizeFinding(finding: RawForensicsFinding): ForensicsFinding | null {
  if (typeof finding.node_id !== 'string') return null
  return {
    nodeId: finding.node_id,
    title: typeof finding.title === 'string' ? finding.title : '未命名发现',
    relation: typeof finding.relation === 'string' ? finding.relation : 'related_to',
    evidence: typeof finding.evidence === 'string' ? finding.evidence : '',
  }
}

export function createDebugCreateNodeCommand() {
  return JSON.stringify({ type: 'debug_create_node' })
}

export function createToolExecutionCommand(tool: string, params: Record<string, unknown>) {
  return JSON.stringify({
    type: 'tool_execution',
    tool,
    params,
  })
}

export function parseGraphUpdateMessage(raw: string): GraphUpdateMessage {
  try {
    const parsed = JSON.parse(raw) as RawGraphUpdate
    if (parsed.type !== 'graph_update' || !parsed.delta) {
      return { addedNodes: [], updatedNodes: [], addedEdges: [], updatedEdges: [] }
    }

    return {
      addedNodes: normalizeNodes(parsed.delta.added_nodes),
      updatedNodes: normalizeNodes(parsed.delta.updated_nodes),
      addedEdges: normalizeEdges(parsed.delta.added_edges),
      updatedEdges: normalizeEdges(parsed.delta.updated_edges),
    }
  } catch {
    return { addedNodes: [], updatedNodes: [], addedEdges: [], updatedEdges: [] }
  }
}

export function parseToolResultMessage(raw: string): ToolResultEvent | null {
  try {
    const parsed = JSON.parse(raw) as RawToolResult
    if (parsed.type !== 'tool_result') return null
    if (typeof parsed.tool !== 'string' || typeof parsed.ok !== 'boolean') return null
    return {
      tool: parsed.tool,
      ok: parsed.ok,
      message: typeof parsed.message === 'string' ? parsed.message : '',
    }
  } catch {
    return null
  }
}

export function parseAgentTokenMessage(raw: string): AgentTokenEvent | null {
  try {
    const parsed = JSON.parse(raw) as RawAgentToken
    if (parsed.type !== 'agent_token') return null
    if (typeof parsed.token !== 'string') return null
    return { token: parsed.token }
  } catch {
    return null
  }
}

export function parseGraphSnapshot(raw: string): GraphSnapshot {
  try {
    const parsed = JSON.parse(raw) as RawGraphSnapshot
    return {
      nodes: normalizeNodes(parsed.nodes),
      edges: normalizeEdges(parsed.edges),
    }
  } catch {
    return { nodes: [], edges: [] }
  }
}

export function parseForensicsReport(raw: string): ForensicsReport {
  try {
    const parsed = JSON.parse(raw) as RawForensicsReport
    const findings = Array.isArray(parsed.findings)
      ? parsed.findings.map((finding) => normalizeFinding(finding)).filter(Boolean) as ForensicsFinding[]
      : []

    return {
      generatedAt: typeof parsed.generated_at === 'string' ? parsed.generated_at : '',
      summary: {
        nodeCount: normalizeNumber(parsed.summary?.node_count),
        edgeCount: normalizeNumber(parsed.summary?.edge_count),
        componentCount: normalizeNumber(parsed.summary?.component_count),
        findingCount: normalizeNumber(parsed.summary?.finding_count),
      },
      findings,
      markdown: typeof parsed.markdown === 'string' ? parsed.markdown : '',
    }
  } catch {
    return {
      generatedAt: '',
      summary: {
        nodeCount: 0,
        edgeCount: 0,
        componentCount: 0,
        findingCount: 0,
      },
      findings: [],
      markdown: '',
    }
  }
}

export function createNodeSearchIndex(nodes: PersistedNode[], query: string) {
  const normalizedQuery = query.trim().toLowerCase()
  if (!normalizedQuery) return nodes

  return nodes.filter((node) => {
    const haystack = `${node.id} ${node.type} ${node.content}`.toLowerCase()
    return haystack.includes(normalizedQuery)
  })
}

export function buildReportSummaryText(report: ForensicsReport) {
  return `生成于 ${report.generatedAt || '未知时间'} | 节点 ${report.summary.nodeCount} | 边 ${report.summary.edgeCount} | 发现 ${report.summary.findingCount}`
}
