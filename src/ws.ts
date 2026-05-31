import { isSupportedNodeType, type PersistedNode } from './graphNodes.ts'

type RawGraphNode = {
  id?: unknown
  node_type?: unknown
  x?: unknown
  y?: unknown
  content?: unknown
}

type RawGraphUpdate = {
  type?: unknown
  delta?: {
    added_nodes?: RawGraphNode[]
    updated_nodes?: RawGraphNode[]
  }
}

export type GraphUpdateMessage = {
  addedNodes: PersistedNode[]
  updatedNodes: PersistedNode[]
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

export function createDebugCreateNodeCommand() {
  return JSON.stringify({ type: 'debug_create_node' })
}

export function parseGraphUpdateMessage(raw: string): GraphUpdateMessage {
  try {
    const parsed = JSON.parse(raw) as RawGraphUpdate
    if (parsed.type !== 'graph_update' || !parsed.delta) {
      return { addedNodes: [], updatedNodes: [] }
    }

    return {
      addedNodes: normalizeNodes(parsed.delta.added_nodes),
      updatedNodes: normalizeNodes(parsed.delta.updated_nodes),
    }
  } catch {
    return { addedNodes: [], updatedNodes: [] }
  }
}
