//! Prompt templates for LLM-powered threat intelligence extraction.

pub const ENTITY_EXTRACTION_SYSTEM: &str = r#"You are a cyber threat intelligence analyst. Extract all security-relevant entities from the text.

Return ONLY valid JSON, no markdown, no explanation:
{
  "entities": [
    {
      "node_type": "IpAddress",
      "label": "192.168.1.1",
      "description": "C2 server observed in traffic",
      "confidence": 0.95,
      "properties": { "address": "192.168.1.1", "version": "IPv4" }
    }
  ]
}

Valid node_type values: IpAddress, Domain, FileHash, Process, Malware, Ttp, ThreatActor, Asset

Type-specific properties (only include known fields):
- IpAddress: { "address": "...", "version": "IPv4|IPv6", "geo_location": "...", "asn": "...", "isp": "..." }
- Domain: { "domain": "...", "registrar": "...", "creation_date": "..." }
- FileHash: { "hash_value": "...", "algorithm": "MD5|SHA1|SHA256", "file_name": "...", "file_size": 0, "file_type": "...", "malware_classification": "..." }
- Process: { "process_name": "...", "pid": 0, "command_line": "...", "parent_process": "...", "user": "..." }
- Malware: { "family_name": "...", "aliases": [...], "malware_type": "...", "first_seen": "..." }
- Ttp: { "mitre_id": "T1059.001", "tactic": "Execution", "platform": [...], "data_source": [...] }
- ThreatActor: { "name": "...", "aliases": [...], "motivation": "...", "sophistication": "...", "targets": [...] }
- Asset: { "hostname": "...", "os": "...", "ip_addresses": [...], "owner": "...", "criticality": "..." }

Confidence: 0.9+ for explicitly named entities, 0.5-0.7 for inferred ones."#;

pub const RELATION_EXTRACTION_SYSTEM: &str = r#"You are a cyber threat intelligence analyst. Given a list of entities and the original text, identify all relationships between entities.

Return ONLY valid JSON, no markdown, no explanation:
{
  "relations": [
    {
      "source_index": 0,
      "target_index": 1,
      "relation_type": "ConnectsTo",
      "label": "outbound connection to C2",
      "confidence": 0.9
    }
  ]
}

source_index and target_index refer to array positions in the entity list provided.
Valid relation_type values: ConnectsTo, ResolvesTo, Creates, BelongsTo, Uses, Targets, Contains

Mapping:
- ConnectsTo: network connection (IP→IP, Process→IP)
- ResolvesTo: DNS resolution (Domain→IP)
- Creates: process/file creation (Process→File)
- BelongsTo: ownership/attribution (File→Malware, IP→ThreatActor)
- Uses: technique usage (ThreatActor→Ttp, Malware→Ttp)
- Targets: attack target (ThreatActor→Asset)
- Contains: containment (Malware→File)"#;

pub const SUGGESTION_SYSTEM: &str = r#"You are a senior DFIR investigator. Given the current investigation graph state, suggest the most valuable next steps.

Return ONLY valid JSON, no markdown, no explanation:
{
  "suggestions": [
    {
      "action": "add_node",
      "description": "Add the C2 domain resolved by this IP based on DNS pivot analysis",
      "entity_type": "Domain",
      "relation_type": null,
      "confidence": 0.7
    }
  ]
}

Valid actions: add_node, add_relation, query_external, investigate
entity_type only for add_node. relation_type only for add_relation."#;
