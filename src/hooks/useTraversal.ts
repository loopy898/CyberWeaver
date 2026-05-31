import { invoke } from '@tauri-apps/api/core'

// ---------------------------------------------------------------------------
// Types — match the Rust transfer structs in commands/traversal_cmd.rs
// ---------------------------------------------------------------------------

export interface TraversalPathData {
  node_ids: string[]
  relation_ids: string[]
  relation_types: string[]
}

export interface ExpandNodeResult {
  paths: TraversalPathData[]
  total_hops: number[]
}

export interface GraphSummary {
  node_count: number
  edge_count: number
  node_ids: string[]
}

// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------

export function useTraversal() {
  /** 展开节点邻域：从 start_node_id 出发，在 max_hops 跳内查找所有可达路径 */
  const expandNode = async (
    investigationId: string,
    startNodeId: string,
    maxHops: number,
    relationTypeFilter?: string,
  ): Promise<ExpandNodeResult> => {
    return invoke<ExpandNodeResult>('expand_node', {
      investigationId,
      startNodeId,
      maxHops,
      relationTypeFilter: relationTypeFilter ?? null,
    })
  }

  /** 查找两个节点之间的最短路径 */
  const findPath = async (
    investigationId: string,
    fromNodeId: string,
    toNodeId: string,
    maxHops: number,
  ): Promise<TraversalPathData | null> => {
    return invoke<TraversalPathData | null>('find_path', {
      investigationId,
      fromNodeId,
      toNodeId,
      maxHops,
    })
  }

  /** 获取节点的连通分量 */
  const getComponent = async (
    investigationId: string,
    nodeId: string,
  ): Promise<string[]> => {
    return invoke<string[]>('get_component', { investigationId, nodeId })
  }

  /** 获取图概览（节点数、边数、所有节点 ID 列表） */
  const getGraphSummary = async (
    investigationId: string,
  ): Promise<GraphSummary> => {
    return invoke<GraphSummary>('get_graph_summary', { investigationId })
  }

  return { expandNode, findPath, getComponent, getGraphSummary }
}
