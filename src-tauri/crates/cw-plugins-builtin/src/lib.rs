pub mod virustotal;
pub mod whois;

use cw_plugin_sdk::InvestigationTool;
use std::sync::Arc;

pub fn builtin_tools() -> Vec<Arc<dyn InvestigationTool>> {
    vec![
        Arc::new(virustotal::VirusTotalTool),
        Arc::new(whois::WhoisTool),
    ]
}

#[cfg(test)]
mod tests {
    use super::builtin_tools;

    #[test]
    fn builtin_registry_exposes_expected_tools() {
        let manifests: Vec<_> = builtin_tools()
            .into_iter()
            .map(|tool| tool.manifest().name)
            .collect();

        assert_eq!(manifests, vec!["virustotal_ip_lookup", "whois_domain_lookup"]);
    }
}
