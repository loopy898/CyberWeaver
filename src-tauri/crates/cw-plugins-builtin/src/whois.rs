use cw_plugin_sdk::{
    DiscoveredNode, DiscoveredRelation, InvestigationTool, ParameterType, ToolInput, ToolManifest,
    ToolOutput, ToolParameter,
};
use serde_json::json;

pub struct WhoisTool;

impl InvestigationTool for WhoisTool {
    fn manifest(&self) -> ToolManifest {
        ToolManifest {
            name: "whois_domain_lookup".to_string(),
            display_name: "WHOIS 域名查询".to_string(),
            description:
                "Query WHOIS for domain registration info — registrar, creation date, nameservers"
                    .to_string(),
            version: "0.1.0".to_string(),
            author: "CyberWeaver".to_string(),
            parameters: vec![ToolParameter {
                name: "domain".to_string(),
                parameter_type: ParameterType::String,
                description: "Domain name to inspect".to_string(),
                required: true,
                default_value: None,
            }],
            input_types: vec!["domain".to_string()],
            output_types: vec!["ip_address".to_string(), "domain".to_string()],
        }
    }

    fn execute(&self, input: ToolInput) -> Result<ToolOutput, String> {
        let domain = input
            .params
            .get("domain")
            .and_then(|value| value.as_str())
            .ok_or_else(|| "missing required parameter: domain".to_string())?;

        let ip_address = "203.0.113.42";
        let registrar = "Example Registrar, Inc.";
        let nameserver = format!("ns1.{domain}");

        Ok(ToolOutput {
            new_nodes: vec![
                DiscoveredNode {
                    node_type: "ip_address".to_string(),
                    label: ip_address.to_string(),
                    description: format!("Simulated A record resolution for domain {domain}"),
                    properties: json!({
                        "rdns": "N/A",
                        "hosting_provider": "N/A",
                        "source": "WHOIS (simulated)"
                    }),
                    confidence: 0.65,
                },
                DiscoveredNode {
                    node_type: "domain".to_string(),
                    label: domain.to_string(),
                    description: "Simulated WHOIS registration record".to_string(),
                    properties: json!({
                        "registrar": registrar,
                        "creation_date": "2021-04-12T00:00:00Z",
                        "expiration_date": "2027-04-12T00:00:00Z",
                        "nameservers": [nameserver, format!("ns2.{domain}")],
                        "registrant_org": "N/A",
                        "source": "WHOIS (simulated)"
                    }),
                    confidence: 0.85,
                },
            ],
            new_relations: vec![DiscoveredRelation {
                source_label: domain.to_string(),
                target_label: ip_address.to_string(),
                relation_type: "resolves_to".to_string(),
                label: format!("{domain} resolves_to {ip_address}"),
                confidence: 0.65,
            }],
            enriched_properties: json!({
                "domain": domain,
                "registrar": registrar,
                "creation_date": "2021-04-12T00:00:00Z",
                "nameservers": [format!("ns1.{domain}"), format!("ns2.{domain}")],
                "status": "active"
            }),
            text_summary: format!(
                "WHOIS simulation shows {domain} registered via {registrar}, created on 2021-04-12, using nameservers ns1.{domain} and ns2.{domain}, and resolving to {ip_address}."
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::WhoisTool;
    use cw_plugin_sdk::{InvestigationTool, ToolInput};
    use serde_json::json;

    #[test]
    fn manifest_matches_expected_contract() {
        let manifest = WhoisTool.manifest();
        assert_eq!(manifest.name, "whois_domain_lookup");
        assert_eq!(manifest.input_types, vec!["domain"]);
        assert_eq!(manifest.output_types, vec!["ip_address", "domain"]);
        assert_eq!(manifest.parameters.len(), 1);
        assert_eq!(manifest.parameters[0].name, "domain");
        assert!(manifest.parameters[0].required);
    }

    #[test]
    fn execute_returns_simulated_whois_data() {
        let output = WhoisTool
            .execute(ToolInput {
                node_id: None,
                params: json!({ "domain": "example.org" }),
            })
            .expect("tool execution should succeed");

        assert_eq!(output.new_nodes.len(), 2);
        assert_eq!(output.new_relations.len(), 1);
        assert!(output.text_summary.contains("example.org"));
        assert_eq!(output.new_nodes[0].node_type, "ip_address");
        assert_eq!(output.new_nodes[1].node_type, "domain");
        assert_eq!(output.new_relations[0].relation_type, "resolves_to");
    }
}
