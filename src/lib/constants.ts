export const WS_URL = 'ws://127.0.0.1:3000/ws'

export const CANVAS_GRID_SIZE = 20

export const SYNC_DEBOUNCE_MS = 250

export const WS_RECONNECT_DELAY_MS = 1500

// ---------------------------------------------------------------------------
// 关系连线颜色映射
// ---------------------------------------------------------------------------
export const RELATION_COLORS: Record<string, string> = {
  connects: '#3B82F6',
  resolves: '#10B981',
  references: '#6B7280',
  spawns: '#F59E0B',
  creates: '#8B5CF6',
  modifies: '#EC4899',
  reads: '#06B6D4',
  deletes: '#EF4444',
  authenticates: '#84CC16',
  belongs_to: '#F97316',
  depends_on: '#A855F7',
  triggers: '#EC4899',
}

// ---------------------------------------------------------------------------
// 关系类型中文标签
// ---------------------------------------------------------------------------
export const RELATION_LABELS: Record<string, string> = {
  connects: '连接',
  resolves: '解析',
  references: '引用',
  spawns: '派生',
  creates: '创建',
  modifies: '修改',
  reads: '读取',
  deletes: '删除',
  authenticates: '认证',
  belongs_to: '归属',
  depends_on: '依赖',
  triggers: '触发',
}

// ---------------------------------------------------------------------------
// 节点类型中文标签
// ---------------------------------------------------------------------------
export const NODE_TYPE_LABELS: Record<string, string> = {
  'ip-address': 'IP 地址',
  'domain': '域名',
  'file-hash': '文件哈希',
  'process': '进程',
  'malware': '恶意软件',
  'ttp': '攻击技术',
  'threat-actor': '威胁组织',
  'asset': '资产',
}

export const NODE_TYPE_SHAPES = [
  'ip-address',
  'domain',
  'file-hash',
  'process',
  'malware',
  'ttp',
  'threat-actor',
  'asset',
] as const

export type NodeTypeShape = (typeof NODE_TYPE_SHAPES)[number]

// ---------------------------------------------------------------------------
// 节点图标（Emoji）
// ---------------------------------------------------------------------------
export const NODE_ICONS: Record<NodeTypeShape, string> = {
  'ip-address': '🌐',
  domain: '🔗',
  'file-hash': '📄',
  process: '⚙️',
  malware: '🦠',
  ttp: '⚔️',
  'threat-actor': '👤',
  asset: '🖥️',
}

// ---------------------------------------------------------------------------
// 节点颜色
// ---------------------------------------------------------------------------
export const NODE_COLORS: Record<NodeTypeShape, string> = {
  'ip-address': '#3B82F6',
  domain: '#10B981',
  'file-hash': '#F59E0B',
  process: '#6B7280',
  malware: '#EF4444',
  ttp: '#8B5CF6',
  'threat-actor': '#EC4899',
  asset: '#14B8A6',
}
