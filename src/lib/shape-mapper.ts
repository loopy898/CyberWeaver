import type { NodeData, TypeSpecificProps } from '../types/domain'
import { NODE_TYPE_SHAPES } from './constants'
import type { NodeTypeShape } from './constants'

// ---------------------------------------------------------------------------
// Shape-type ↔ Rust node_type (snake_case) bidirectional maps
// ---------------------------------------------------------------------------

const SHAPE_TO_NODE_TYPE: Record<NodeTypeShape, string> = {
  'ip-address': 'ip_address',
  'domain': 'domain',
  'file-hash': 'file_hash',
  'process': 'process',
  'malware': 'malware',
  'ttp': 'ttp',
  'threat-actor': 'threat_actor',
  'asset': 'asset',
}

const NODE_TYPE_TO_SHAPE: Record<string, NodeTypeShape> = {}
for (const [shape, node] of Object.entries(SHAPE_TO_NODE_TYPE)) {
  NODE_TYPE_TO_SHAPE[node] = shape as NodeTypeShape
}

const CUSTOM_SHAPE_TYPES: ReadonlySet<string> = new Set(NODE_TYPE_SHAPES)

// ---------------------------------------------------------------------------
// Tag names for the TypeSpecificProps enum (PascalCase, matches Rust variants)
// ---------------------------------------------------------------------------

const SHAPE_TO_TAG: Record<NodeTypeShape, string> = {
  'ip-address': 'IpAddress',
  'domain': 'Domain',
  'file-hash': 'FileHash',
  'process': 'Process',
  'malware': 'Malware',
  'ttp': 'Ttp',
  'threat-actor': 'ThreatActor',
  'asset': 'Asset',
}

// ---------------------------------------------------------------------------
// ID helpers
// ---------------------------------------------------------------------------

const SHAPE_PREFIX = 'shape:'

export function toShapeId(dbId: string): string {
  return dbId.startsWith(SHAPE_PREFIX) ? dbId : `${SHAPE_PREFIX}${dbId}`
}

export function toDbId(shapeId: string): string {
  return shapeId.startsWith(SHAPE_PREFIX) ? shapeId.slice(SHAPE_PREFIX.length) : shapeId
}

// ---------------------------------------------------------------------------
// Public conversion helpers
// ---------------------------------------------------------------------------

export function shapeTypeToNodeType(shapeType: string): string {
  return SHAPE_TO_NODE_TYPE[shapeType as NodeTypeShape] ?? shapeType
}

export function nodeTypeToShapeType(nodeType: string): NodeTypeShape {
  return NODE_TYPE_TO_SHAPE[nodeType] ?? (nodeType as NodeTypeShape)
}

// ---------------------------------------------------------------------------
// isCustomShape
// ---------------------------------------------------------------------------

/** Minimal shape-record shape used by the persistence layer. */
export interface CustomShapeRecord {
  typeName: 'shape'
  id: string
  type: string
  x: number
  y: number
  props: Record<string, unknown>
}

export function isCustomShape(record: unknown): boolean {
  if (!record || typeof record !== 'object') return false
  const r = record as Record<string, unknown>
  return (
    r.typeName === 'shape' &&
    typeof r.type === 'string' &&
    CUSTOM_SHAPE_TYPES.has(r.type)
  )
}

/**
 * Filter an array of tldraw store records, returning only custom shape records.
 * TLRecord is a wide union — this helper narrows to {@link CustomShapeRecord}.
 */
export function filterCustomShapes(records: unknown[]): CustomShapeRecord[] {
  return records.filter((r): r is CustomShapeRecord => isCustomShape(r)) as CustomShapeRecord[]
}

// ---------------------------------------------------------------------------
// getPrimaryLabel – extract the main identifying field from shape props
// ---------------------------------------------------------------------------

export function getPrimaryLabel(shapeType: string, props: Record<string, unknown>): string {
  switch (shapeType) {
    case 'ip-address':
      return String(props.address ?? '')
    case 'domain':
      return String(props.domain ?? '')
    case 'file-hash':
      return String(props.hashValue ?? '')
    case 'process':
      return String(props.processName ?? '')
    case 'malware':
      return String(props.familyName ?? '')
    case 'ttp':
      return String(props.mitreId ?? '')
    case 'threat-actor':
      return String(props.name ?? '')
    case 'asset':
      return String(props.hostname ?? '')
    default:
      return ''
  }
}

// ---------------------------------------------------------------------------
// nodeDataToShape — convert Rust NodeData → params for editor.createShape()
// ---------------------------------------------------------------------------

interface CreateShapeParams {
  id: string
  type: NodeTypeShape
  x: number
  y: number
  props: Record<string, unknown>
}

export function nodeDataToShape(node: NodeData): CreateShapeParams | null {
  const shapeType = NODE_TYPE_TO_SHAPE[node.nodeType]
  if (!shapeType) {
    console.warn(`Unknown nodeType: ${node.nodeType}`)
    return null
  }

  const { data } = node.properties
  const base = {
    id: toShapeId(node.id),
    type: shapeType,
    x: node.posX,
    y: node.posY,
  }

  // Each shape has its own default w/h and type-specific prop mapping.
  // The Rust data fields are snake_case; shape props use camelCase + abbreviated names.
  switch (shapeType) {
    case 'ip-address': {
      const d = data as import('../types/domain').IpAddressProps
      return {
        ...base,
        props: {
          w: 220,
          h: 90,
          address: d.address,
          label: node.label,
          geo: d.geo_location ?? undefined,
          asn: d.asn ?? undefined,
          reputation: d.reputation ?? 'unknown',
          confidence: node.confidence,
        },
      }
    }
    case 'domain': {
      const d = data as import('../types/domain').DomainProps
      return {
        ...base,
        props: {
          w: 220,
          h: 90,
          domain: d.domain,
          label: node.label,
          registrar: d.registrar ?? undefined,
          creationDate: d.creation_date ?? undefined,
          reputation: 'unknown' as string,
          confidence: node.confidence,
        },
      }
    }
    case 'file-hash': {
      const d = data as import('../types/domain').FileHashProps
      const algoMap: Record<string, string> = { md5: 'MD5', sha1: 'SHA1', sha256: 'SHA256' }
      return {
        ...base,
        props: {
          w: 220,
          h: 90,
          hashValue: d.hash_value,
          algorithm: algoMap[d.algorithm] ?? 'SHA256',
          label: node.label,
          fileName: d.file_name ?? undefined,
          malwareClassification: d.malware_classification ?? undefined,
          confidence: node.confidence,
        },
      }
    }
    case 'process': {
      const d = data as import('../types/domain').ProcessProps
      return {
        ...base,
        props: {
          w: 220,
          h: 90,
          processName: d.process_name,
          pid: d.pid ?? 0,
          label: node.label,
          user: d.user ?? undefined,
          commandLine: d.command_line ?? undefined,
          confidence: node.confidence,
        },
      }
    }
    case 'malware': {
      const d = data as import('../types/domain').MalwareProps
      return {
        ...base,
        props: {
          w: 220,
          h: 90,
          familyName: d.family_name,
          label: node.label,
          aliases: d.aliases.length > 0 ? d.aliases.join(', ') : undefined,
          malwareType: d.malware_type ?? undefined,
          confidence: node.confidence,
        },
      }
    }
    case 'ttp': {
      const d = data as import('../types/domain').TtpProps
      return {
        ...base,
        props: {
          w: 220,
          h: 90,
          mitreId: d.mitre_id,
          name: node.label,
          label: node.label,
          tactic: d.tactic ?? undefined,
          description: undefined,
          confidence: node.confidence,
        },
      }
    }
    case 'threat-actor': {
      const d = data as import('../types/domain').ThreatActorProps
      return {
        ...base,
        props: {
          w: 220,
          h: 90,
          name: d.name,
          label: node.label,
          aliases: d.aliases.length > 0 ? d.aliases.join(', ') : undefined,
          motivation: d.motivation ?? undefined,
          sophistication: d.sophistication ?? 'medium',
          confidence: node.confidence,
        },
      }
    }
    case 'asset': {
      const d = data as import('../types/domain').AssetProps
      return {
        ...base,
        props: {
          w: 220,
          h: 90,
          hostname: d.hostname,
          label: node.label,
          os: d.os ?? undefined,
          ipAddresses: d.ip_addresses.length > 0 ? d.ip_addresses.join(', ') : undefined,
          criticality: d.criticality ?? 'medium',
          confidence: node.confidence,
        },
      }
    }
    default:
      return null
  }
}

// ---------------------------------------------------------------------------
// shapeToNodeData — convert a canvas shape record → params for Rust commands
// ---------------------------------------------------------------------------

export interface NodeDataParams {
  fixedId: string
  investigationId: string
  nodeType: string
  label: string
  confidence: number
  /** JSON string of TypeSpecificProps (tagged enum) */
  properties: string
  posX: number
  posY: number
}

export function shapeToNodeData(
  shape: CustomShapeRecord,
  investigationId: string,
): NodeDataParams | null {
  const nodeType = SHAPE_TO_NODE_TYPE[shape.type as NodeTypeShape]
  if (!nodeType) {
    console.warn(`Unknown shape type: ${shape.type}`)
    return null
  }

  const props = shape.props
  const fixedId = toDbId(shape.id)
  const label = String(props.label ?? '')
  const confidence = typeof props.confidence === 'number' ? props.confidence : 1.0

  // Build the TypeSpecificProps tagged-enum JSON.
  const tag = SHAPE_TO_TAG[shape.type as NodeTypeShape]
  const data = buildRustData(shape.type as NodeTypeShape, props)
  // Cast through unknown: buildRustData returns the correct field set for the
  // given shape type, but the discriminated union can't statically verify it.
  const properties = { type: tag, data } as unknown as TypeSpecificProps
  const propertiesJson = JSON.stringify(properties)

  return {
    fixedId,
    investigationId,
    nodeType,
    label,
    confidence,
    properties: propertiesJson,
    posX: shape.x,
    posY: shape.y,
  }
}

// ---------------------------------------------------------------------------
// buildRustData – convert shape props → the "data" part of TypeSpecificProps
//
// Produces an object whose keys match the Rust struct field names (snake_case),
// ready for JSON serialization into the tagged-enum format.
// ---------------------------------------------------------------------------

function buildRustData(shapeType: NodeTypeShape, props: Record<string, unknown>): Record<string, unknown> {
  switch (shapeType) {
    case 'ip-address':
      return {
        address: String(props.address ?? ''),
        version: null,
        geo_location: coerceString(props.geo),
        asn: coerceString(props.asn),
        isp: null,
        reputation: coerceReputation(props.reputation),
      }
    case 'domain':
      return {
        domain: String(props.domain ?? ''),
        registrar: coerceString(props.registrar),
        creation_date: coerceString(props.creationDate),
      }
    case 'file-hash': {
      const algoLower = String(props.algorithm ?? 'sha256').toLowerCase()
      return {
        hash_value: String(props.hashValue ?? ''),
        algorithm: algoLower === 'sha512' ? 'sha256' : algoLower,
        file_name: coerceString(props.fileName),
        file_size: null,
        file_type: null,
        malware_classification: coerceString(props.malwareClassification),
      }
    }
    case 'process':
      return {
        process_name: String(props.processName ?? ''),
        pid: typeof props.pid === 'number' ? props.pid : null,
        command_line: coerceString(props.commandLine),
        parent_process: null,
        user: coerceString(props.user),
      }
    case 'malware':
      return {
        family_name: String(props.familyName ?? ''),
        aliases: splitJoined(props.aliases),
        malware_type: coerceString(props.malwareType),
        first_seen: null,
      }
    case 'ttp':
      return {
        mitre_id: String(props.mitreId ?? ''),
        tactic: coerceString(props.tactic),
        platform: [],
        data_source: [],
      }
    case 'threat-actor':
      return {
        name: String(props.name ?? ''),
        aliases: splitJoined(props.aliases),
        motivation: coerceString(props.motivation),
        sophistication: coerceString(props.sophistication),
        targets: [],
      }
    case 'asset':
      return {
        hostname: String(props.hostname ?? ''),
        os: coerceString(props.os),
        ip_addresses: splitJoined(props.ipAddresses),
        owner: null,
        criticality: coerceString(props.criticality),
      }
    default:
      return {}
  }
}

// ---------------------------------------------------------------------------
// Tiny helpers
// ---------------------------------------------------------------------------

function coerceString(value: unknown): string | null {
  if (value === null || value === undefined || value === '') return null
  return String(value)
}

function splitJoined(value: unknown): string[] {
  if (typeof value === 'string') {
    return value
      .split(/[,;，；]\s*/)
      .map((s) => s.trim())
      .filter(Boolean)
  }
  if (Array.isArray(value)) {
    return value.map((v) => String(v)).filter(Boolean)
  }
  return []
}

function coerceReputation(value: unknown): string | null {
  const v = coerceString(value)
  if (!v) return null
  const valid = new Set(['clean', 'suspicious', 'malicious', 'unknown'])
  return valid.has(v.toLowerCase()) ? v.toLowerCase() : null
}
