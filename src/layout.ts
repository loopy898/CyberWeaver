import ELK, { type ElkNode, type ElkExtendedEdge } from 'elkjs/lib/elk.bundled.js'
import type { PersistedNode } from './graphNodes.ts'
import type { GraphEdge } from './ws.ts'

export type PositionedNode = PersistedNode

const elk = new ELK()

const DEFAULT_LAYOUT_OPTIONS = {
  'elk.algorithm': 'layered',
  'elk.direction': 'RIGHT',
  'elk.layered.spacing.nodeNodeBetweenLayers': '80',
  'elk.spacing.nodeNode': '60',
}

export async function computeLayout(nodes: PersistedNode[], edges: GraphEdge[]): Promise<PositionedNode[]> {
  if (nodes.length === 0) return []

  const graph: ElkNode = {
    id: 'root',
    layoutOptions: DEFAULT_LAYOUT_OPTIONS,
    children: nodes.map((node) => ({
      id: node.id,
      width: 220,
      height: 120,
    })),
    edges: edges.map(
      (edge): ElkExtendedEdge => ({
        id: edge.id,
        sources: [edge.sourceId],
        targets: [edge.targetId],
      })
    ),
  }

  const result = await elk.layout(graph)
  const byId = new Map(result.children?.map((child) => [child.id, child]))

  return nodes.map((node) => {
    const child = byId.get(node.id)
    return {
      ...node,
      x: typeof child?.x === 'number' ? child.x : node.x,
      y: typeof child?.y === 'number' ? child.y : node.y,
    }
  })
}
