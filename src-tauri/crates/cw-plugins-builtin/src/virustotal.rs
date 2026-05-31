use cw_plugin_sdk::{
    DiscoveredNode, DiscoveredRelation, InvestigationTool, ParameterType, ToolInput, ToolManifest,
    ToolOutput, ToolParameter,
};
use serde_json::json;

pub struct VirusTotalTool;

impl InvestigationTool for VirusTotalTool {
    fn manifest(&self) -> ToolManifest {
        ToolManifest {
            name: "virustotal_ip_lookup".to_string(),
            display_name: "VirusTotal IP 查询".to_string(),
            description:
                "Query VirusTotal for IP address reputation, related domains, and malware detections"
                    .to_string(),
            version: "0.1.0".to_string(),
            author: "CyberWeaver".to_string(),
            parameters: vec![ToolParameter {
                name: "ip_address".to_string(),
                parameter_type: ParameterType::String,
                description: "IPv4 or IPv6 address to inspect".to_string(),
                required: true,
                default_value: None,
            }],
            input_types: vec!["ip_address".to_string()],
            output_types: vec!["domain".to_string(), "malware".to_string()],
        }
    }

    fn execute(&self, input: ToolInput) -> Result<ToolOutput, String> {
        let ip_address = input
            .params
            .get("ip_address")
            .and_then(|value| value.as_str())
            .ok_or_else(|| "missing required parameter: ip_address".to_string())?;

        let related_domain = format!("edge-{}.example", ip_address.replace('.', "-"));
        let malware_family = "RedLine Stealer";

        Ok(ToolOutput {
            new_nodes: vec![
                DiscoveredNode {
                    node_type: "malware".to_string(),
                    label: malware_family.to_string(),
                    description: format!(
                        "Simulated VirusTotal detection family associated with {ip_address}"
                    ),
                    properties: json!({
                        "detection_type": "family",
                        "last_analysis_stats": {
                            "malicious": 12,
                            "suspicious": 3,
                            "harmless": 45
                        },
                        "sample_sha256": "N/A",
                        "source": "VirusTotal (simulated)"
                    }),
                    confidence: 0.7,
                },
                DiscoveredNode {
                    node_type: "domain".to_string(),
                    label: related_domain.clone(),
                    description: format!(
                        "Simulated domain observed resolving to or communicating with {ip_address}"
                    ),
                    properties: json!({
                        "first_seen": "2026-05-01T00:00:00Z",
                        "last_seen": "2026-05-25T00:00:00Z",
                        "resolution": ip_address,
                        "registrar": "N/A",
                        "source": "VirusTotal (simulated)"
                    }),
                    confidence: 0.6,
                },
            ],
            new_relations: vec![DiscoveredRelation {
                source_label: ip_address.to_string(),
                target_label: related_domain.clone(),
                relation_type: "connects_to".to_string(),
                label: format!("{ip_address} connects_to {related_domain}"),
                confidence: 0.6,
            }],
            enriched_properties: json!({
                "ip_address": ip_address,
                "as_owner": "N/A",
                "country": "N/A",
                "analysis_source": "VirusTotal (simulated)",
                "reputation": "malicious"
            }),
            text_summary: format!(
                "VirusTotal simulation indicates {ip_address} has malicious reputation signals, is linked to malware family {malware_family}, and is associated with domain {related_domain}."
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::VirusTotalTool;
    use cw_plugin_sdk::{InvestigationTool, ToolInput};
    use serde_json::json;

    #[test]
    fn manifest_matches_expected_contract() {
        let manifest = VirusTotalTool.manifest();
        assert_eq!(manifest.name, "virustotal_ip_lookup");
        assert_eq!(manifest.input_types, vec!["ip_address"]);
        assert_eq!(manifest.output_types, vec!["domain", "malware"]);
        assert_eq!(manifest.parameters.len(), 1);
        assert_eq!(manifest.parameters[0].name, "ip_address");
        assert!(manifest.parameters[0].required);
    }

    #[test]
    fn execute_returns_simulated_nodes_and_relations() {
        let output = VirusTotalTool
            .execute(ToolInput {
                node_id: None,
                params: json!({ "ip_address": "8.8.8.8" }),
            })
            .expect("tool execution should succeed");

        assert_eq!(output.new_nodes.len(), 2);
        assert_eq!(output.new_relations.len(), 1);
        assert!(output.text_summary.contains("8.8.8.8"));
        assert_eq!(output.new_nodes[0].node_type, "malware");
        assert_eq!(output.new_nodes[0].confidence, 0.7);
        assert_eq!(output.new_nodes[1].node_type, "domain");
        assert_eq!(output.new_nodes[1].confidence, 0.6);
        assert_eq!(output.new_relations[0].relation_type, "connects_to");
    }
}
