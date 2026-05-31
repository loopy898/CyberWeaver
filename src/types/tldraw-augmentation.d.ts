import '@tldraw/tlschema'

declare module '@tldraw/tlschema' {
  interface TLGlobalShapePropsMap {
    'ip-address': {
      w: number
      h: number
      address: string
      label: string
      geo?: string
      asn?: string
      reputation: string
      confidence: number
    }
    'domain': {
      w: number
      h: number
      domain: string
      label: string
      registrar?: string
      creationDate?: string
      reputation: string
      confidence: number
    }
    'file-hash': {
      w: number
      h: number
      hashValue: string
      algorithm: string
      label: string
      fileName?: string
      malwareClassification?: string
      confidence: number
    }
    'process': {
      w: number
      h: number
      processName: string
      pid: number
      label: string
      user?: string
      commandLine?: string
      confidence: number
    }
    'malware': {
      w: number
      h: number
      familyName: string
      label: string
      aliases?: string
      malwareType?: string
      confidence: number
    }
    'ttp': {
      w: number
      h: number
      mitreId: string
      name: string
      label: string
      tactic?: string
      description?: string
      confidence: number
    }
    'threat-actor': {
      w: number
      h: number
      name: string
      label: string
      aliases?: string
      motivation?: string
      sophistication: string
      confidence: number
    }
    'asset': {
      w: number
      h: number
      hostname: string
      label: string
      os?: string
      ipAddresses?: string
      criticality: string
      confidence: number
    }
    'relation-edge': {
      w: number
      h: number
      relationType: string
      label: string
      sourceNodeId: string
      targetNodeId: string
      confidence: number
    }
  }
}
