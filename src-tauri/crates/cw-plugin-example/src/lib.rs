use cw_plugin_sdk::{
    export_plugin, DiscoveredNode, InvestigationTool, ParameterType, ToolInput, ToolManifest,
    ToolOutput, ToolParameter,
};
use serde_json::json;

pub struct GeoIpLookup;

impl Default for GeoIpLookup {
    fn default() -> Self {
        Self
    }
}

impl InvestigationTool for GeoIpLookup {
    fn manifest(&self) -> ToolManifest {
        ToolManifest {
            name: "geoip_lookup".to_string(),
            display_name: "GeoIP 地理位置查询".to_string(),
            description: "Look up geographical location, ASN, and ISP for an IP address"
                .to_string(),
            version: "0.1.0".to_string(),
            author: "CyberWeaver".to_string(),
            parameters: vec![ToolParameter {
                name: "ip_address".to_string(),
                parameter_type: ParameterType::String,
                description: "IPv4 or IPv6 address to geolocate".to_string(),
                required: true,
                default_value: None,
            }],
            input_types: vec!["ip_address".to_string()],
            output_types: vec!["ip_address".to_string(), "asset".to_string()],
        }
    }

    fn execute(&self, input: ToolInput) -> Result<ToolOutput, String> {
        let ip_address = input
            .params
            .get("ip_address")
            .and_then(|value| value.as_str())
            .ok_or_else(|| "missing required parameter: ip_address".to_string())?;

        Ok(ToolOutput {
            new_nodes: vec![DiscoveredNode {
                node_type: "asset".to_string(),
                label: format!("Geo Profile: {ip_address}"),
                description: format!(
                    "Simulated GeoIP enrichment for {ip_address} covering geography and network ownership"
                ),
                properties: json!({
                    "ip_address": ip_address,
                    "city": "Singapore",
                    "country": "Singapore",
                    "asn": "AS13335",
                    "isp": "Cloudflare, Inc.",
                    "latitude": 1.3521,
                    "longitude": 103.8198,
                    "timezone": "Asia/Singapore",
                    "source": "GeoIP (simulated)"
                }),
                confidence: 0.82,
            }],
            new_relations: vec![],
            enriched_properties: json!({
                "ip_address": ip_address,
                "city": "Singapore",
                "country": "Singapore",
                "asn": "AS13335",
                "isp": "Cloudflare, Inc.",
                "coordinates": {
                    "lat": 1.3521,
                    "lon": 103.8198
                }
            }),
            text_summary: format!(
                "GeoIP simulation places {ip_address} in Singapore, operated by Cloudflare, Inc. under AS13335 at coordinates 1.3521, 103.8198."
            ),
        })
    }
}

export_plugin!(GeoIpLookup);

#[cfg(test)]
mod tests {
    use super::GeoIpLookup;
    use cw_plugin_sdk::{InvestigationTool, ToolInput};
    use serde_json::json;

    #[test]
    fn manifest_matches_expected_contract() {
        let manifest = GeoIpLookup.manifest();
        assert_eq!(manifest.name, "geoip_lookup");
        assert_eq!(manifest.input_types, vec!["ip_address"]);
        assert_eq!(manifest.output_types, vec!["ip_address", "asset"]);
        assert_eq!(manifest.parameters.len(), 1);
        assert_eq!(manifest.parameters[0].name, "ip_address");
        assert!(manifest.parameters[0].required);
    }

    #[test]
    fn execute_returns_simulated_geo_data() {
        let output = GeoIpLookup
            .execute(ToolInput {
                node_id: None,
                params: json!({ "ip_address": "1.1.1.1" }),
            })
            .expect("tool execution should succeed");

        assert_eq!(output.new_nodes.len(), 1);
        assert!(output.new_relations.is_empty());
        assert_eq!(output.new_nodes[0].node_type, "asset");
        assert_eq!(output.new_nodes[0].confidence, 0.82);
        assert!(output.text_summary.contains("1.1.1.1"));
    }
}
