// ---------------------------------------------------------------------------
// Domain types matching Rust backend models.
//
// Tauri auto-applies camelCase to command return-structs (NodeData, RelationData)
// top-level fields. Nested structs inside TypeSpecificProps use their own serde
// config (snake_case, matching Rust field names exactly).
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Node types
// ---------------------------------------------------------------------------

/** Node type values from Rust backend (serde rename_all = "snake_case"). */
export type NodeTypeStr =
  | 'ip_address'
  | 'domain'
  | 'file_hash'
  | 'process'
  | 'malware'
  | 'ttp'
  | 'threat_actor'
  | 'asset'

/** Rust Reputation enum — lowercased via serde(rename_all = "lowercase"). */
export type Reputation = 'clean' | 'suspicious' | 'malicious' | 'unknown'

/** Rust HashAlgorithm enum — lowercased via serde(rename_all = "lowercase"). */
export type HashAlgorithm = 'md5' | 'sha1' | 'sha256'

// ---------------------------------------------------------------------------
// Type-specific property interfaces
//
// Field names match Rust struct field names (snake_case).
// Tauri does NOT recursively rename these — they use default serde serialization.
// ---------------------------------------------------------------------------

export interface IpAddressProps {
  address: string
  version?: string | null
  geo_location?: string | null
  asn?: string | null
  isp?: string | null
  reputation?: Reputation | null
}

export interface DomainProps {
  domain: string
  registrar?: string | null
  creation_date?: string | null
}

export interface FileHashProps {
  hash_value: string
  algorithm: HashAlgorithm
  file_name?: string | null
  file_size?: number | null
  file_type?: string | null
  malware_classification?: string | null
}

export interface ProcessProps {
  process_name: string
  pid?: number | null
  command_line?: string | null
  parent_process?: string | null
  user?: string | null
}

export interface MalwareProps {
  family_name: string
  aliases: string[]
  malware_type?: string | null
  first_seen?: string | null
}

export interface TtpProps {
  mitre_id: string
  tactic?: string | null
  platform: string[]
  data_source: string[]
}

export interface ThreatActorProps {
  name: string
  aliases: string[]
  motivation?: string | null
  sophistication?: string | null
  targets: string[]
}

export interface AssetProps {
  hostname: string
  os?: string | null
  ip_addresses: string[]
  owner?: string | null
  criticality?: string | null
}

// ---------------------------------------------------------------------------
// Tagged enum — matches Rust TypeSpecificProps (serde tag="type", content="data")
// ---------------------------------------------------------------------------

export type TypeSpecificProps =
  | { type: 'IpAddress'; data: IpAddressProps }
  | { type: 'Domain'; data: DomainProps }
  | { type: 'FileHash'; data: FileHashProps }
  | { type: 'Process'; data: ProcessProps }
  | { type: 'Malware'; data: MalwareProps }
  | { type: 'Ttp'; data: TtpProps }
  | { type: 'ThreatActor'; data: ThreatActorProps }
  | { type: 'Asset'; data: AssetProps }

// ---------------------------------------------------------------------------
// Relation types
// ---------------------------------------------------------------------------

/** Relation type values from Rust backend (serde rename_all = "snake_case"). */
export type RelationTypeStr =
  | 'connects_to'
  | 'resolves_to'
  | 'creates'
  | 'belongs_to'
  | 'uses'
  | 'targets'
  | 'contains'

// ---------------------------------------------------------------------------
// Transfer structs — match Rust NodeData / RelationData
//
// Top-level fields use camelCase via Tauri auto-conversion:
//   node_type → nodeType,  pos_x → posX,  investigation_id → investigationId,
//   created_at → createdAt,  updated_at → updatedAt
// ---------------------------------------------------------------------------

export interface NodeData {
  id: string
  nodeType: NodeTypeStr
  label: string
  description: string
  confidence: number
  properties: TypeSpecificProps
  posX: number
  posY: number
  investigationId: string
  createdAt?: string | null
  updatedAt?: string | null
}

export interface RelationData {
  id: string
  relationType: RelationTypeStr
  sourceNodeId: string
  targetNodeId: string
  label: string
  confidence: number
  firstSeen?: string | null
  lastSeen?: string | null
  investigationId: string
}
