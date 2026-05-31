//! Report generator — produces formatted investigation reports from graph data.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::db::repositories::node_repo::chrono_now;
use crate::models::domain::{NodeData, NodeType, RelationData, TypeSpecificProps};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportConfig {
    pub title: String,
    pub author: String,
    pub organization: String,
    pub include_ioc_list: bool,
    pub include_graph_summary: bool,
}

pub fn generate_html_report(
    nodes: &[NodeData],
    relations: &[RelationData],
    config: &ReportConfig,
) -> String {
    let generated_at = chrono_now();
    let node_summary_rows = if config.include_graph_summary {
        summarize_node_types(nodes)
            .into_iter()
            .map(|(kind, count)| {
                format!("<tr><td>{}</td><td>{}</td></tr>", escape_html(&kind), count)
            })
            .collect::<Vec<_>>()
            .join("")
    } else {
        String::new()
    };
    let relation_summary_rows = if config.include_graph_summary {
        summarize_relation_types(relations)
            .into_iter()
            .map(|(kind, count)| {
                format!("<tr><td>{}</td><td>{}</td></tr>", escape_html(&kind), count)
            })
            .collect::<Vec<_>>()
            .join("")
    } else {
        String::new()
    };
    let ioc_rows = if config.include_ioc_list {
        collect_ioc_rows(nodes)
            .into_iter()
            .map(|(ioc_type, value, label, confidence)| {
                format!(
                    "<tr><td>{}</td><td>{}</td><td>{}</td><td>{:.2}</td></tr>",
                    escape_html(&ioc_type),
                    escape_html(&value),
                    escape_html(&label),
                    confidence
                )
            })
            .collect::<Vec<_>>()
            .join("")
    } else {
        String::new()
    };

    let graph_summary_section = if config.include_graph_summary {
        format!(
            r#"<section class="panel">
  <h2>图谱摘要</h2>
  <div class="grid two">
    <article class="card">
      <h3>node_type</h3>
      <table>
        <thead><tr><th>类型</th><th>数量</th></tr></thead>
        <tbody>{node_summary_rows}</tbody>
      </table>
    </article>
    <article class="card">
      <h3>relation_type</h3>
      <table>
        <thead><tr><th>类型</th><th>数量</th></tr></thead>
        <tbody>{relation_summary_rows}</tbody>
      </table>
    </article>
  </div>
</section>"#
        )
    } else {
        String::new()
    };

    let ioc_section = if config.include_ioc_list {
        format!(
            r#"<section class="panel">
  <h2>IOC 清单</h2>
  <article class="card">
    <table>
      <thead><tr><th>类型</th><th>值</th><th>标签</th><th>置信度</th></tr></thead>
      <tbody>{ioc_rows}</tbody>
    </table>
  </article>
</section>"#
        )
    } else {
        String::new()
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>{title}</title>
  <style>
    :root {{
      color-scheme: dark;
      --bg: #09111b;
      --bg-accent: #0e1a29;
      --card: rgba(16, 27, 41, 0.9);
      --card-strong: rgba(20, 34, 52, 0.96);
      --ink: #e9f1fb;
      --muted: #8ea3ba;
      --line: rgba(120, 156, 194, 0.22);
      --accent: #4ecdc4;
      --accent-2: #7bb6ff;
      --shadow: 0 24px 80px rgba(0, 0, 0, 0.35);
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      min-height: 100vh;
      color: var(--ink);
      font-family: "Segoe UI", "PingFang SC", "Helvetica Neue", sans-serif;
      background:
        radial-gradient(circle at top right, rgba(78, 205, 196, 0.14), transparent 24%),
        radial-gradient(circle at left center, rgba(123, 182, 255, 0.12), transparent 26%),
        linear-gradient(180deg, var(--bg) 0%, #060c14 100%);
      padding: 40px 20px 56px;
    }}
    .page {{
      max-width: 1200px;
      margin: 0 auto;
    }}
    .hero {{
      background: linear-gradient(135deg, rgba(24, 42, 65, 0.96), rgba(10, 18, 28, 0.96));
      border: 1px solid var(--line);
      border-radius: 24px;
      padding: 32px;
      box-shadow: var(--shadow);
      position: relative;
      overflow: hidden;
    }}
    .hero::after {{
      content: "";
      position: absolute;
      inset: -30% auto auto 60%;
      width: 260px;
      height: 260px;
      background: radial-gradient(circle, rgba(78, 205, 196, 0.18), transparent 70%);
      pointer-events: none;
    }}
    h1, h2, h3 {{ margin: 0; }}
    h1 {{
      font-size: 34px;
      letter-spacing: 0.02em;
      margin-bottom: 12px;
    }}
    h2 {{
      font-size: 22px;
      color: var(--accent);
      margin-bottom: 16px;
    }}
    h3 {{
      font-size: 15px;
      color: var(--accent-2);
      margin-bottom: 12px;
      text-transform: uppercase;
      letter-spacing: 0.08em;
    }}
    .meta {{
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
      gap: 12px;
      margin-top: 18px;
    }}
    .meta-item {{
      background: rgba(255, 255, 255, 0.03);
      border: 1px solid var(--line);
      border-radius: 14px;
      padding: 14px 16px;
    }}
    .meta-item span {{
      display: block;
      font-size: 12px;
      color: var(--muted);
      text-transform: uppercase;
      letter-spacing: 0.08em;
      margin-bottom: 6px;
    }}
    .stats {{
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
      gap: 16px;
      margin: 24px 0 0;
    }}
    .stat {{
      background: linear-gradient(180deg, rgba(255, 255, 255, 0.04), rgba(255, 255, 255, 0.02));
      border: 1px solid var(--line);
      border-radius: 16px;
      padding: 18px;
    }}
    .stat span {{
      display: block;
      color: var(--muted);
      font-size: 13px;
      margin-bottom: 8px;
    }}
    .stat strong {{
      font-size: 30px;
      color: var(--ink);
    }}
    .panel {{
      margin-top: 28px;
    }}
    .grid {{
      display: grid;
      gap: 18px;
    }}
    .grid.two {{
      grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    }}
    .card {{
      background: var(--card);
      border: 1px solid var(--line);
      border-radius: 18px;
      padding: 18px;
      box-shadow: var(--shadow);
    }}
    table {{
      width: 100%;
      border-collapse: collapse;
      background: var(--card-strong);
      border-radius: 14px;
      overflow: hidden;
    }}
    th, td {{
      text-align: left;
      padding: 12px 14px;
      border-bottom: 1px solid var(--line);
      font-size: 14px;
    }}
    th {{
      color: var(--accent);
      background: rgba(78, 205, 196, 0.06);
      font-weight: 700;
    }}
    tr:last-child td {{
      border-bottom: none;
    }}
    .hint {{
      color: var(--muted);
      margin-top: 10px;
      font-size: 13px;
    }}
  </style>
</head>
<body>
  <main class="page">
    <header class="hero">
      <h1>{title}</h1>
      <p class="hint">专业威胁情报视图，聚焦图谱规模、关系分布与 IOC 资产。</p>
      <section class="meta">
        <div class="meta-item"><span>author</span>{author}</div>
        <div class="meta-item"><span>organization</span>{organization}</div>
        <div class="meta-item"><span>generated_at</span>{generated_at}</div>
      </section>
      <section class="stats">
        <div class="stat"><span>nodes</span><strong>{node_count}</strong></div>
        <div class="stat"><span>relations</span><strong>{relation_count}</strong></div>
        <div class="stat"><span>ioc</span><strong>{ioc_count}</strong></div>
      </section>
    </header>
    {graph_summary_section}
    {ioc_section}
  </main>
</body>
</html>"#,
        title = escape_html(&config.title),
        author = escape_html(&config.author),
        organization = escape_html(&config.organization),
        generated_at = escape_html(&generated_at),
        node_count = nodes.len(),
        relation_count = relations.len(),
        ioc_count = collect_ioc_rows(nodes).len(),
        graph_summary_section = graph_summary_section,
        ioc_section = ioc_section,
    )
}

fn summarize_node_types(nodes: &[NodeData]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for node in nodes {
        *counts
            .entry(format!("{:?}", node.node_type).to_lowercase())
            .or_insert(0) += 1;
    }
    counts
}

fn summarize_relation_types(relations: &[RelationData]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for relation in relations {
        *counts
            .entry(format!("{:?}", relation.relation_type).to_lowercase())
            .or_insert(0) += 1;
    }
    counts
}

fn collect_ioc_rows(nodes: &[NodeData]) -> Vec<(String, String, String, f32)> {
    let mut rows = Vec::new();

    for node in nodes {
        match (&node.node_type, &node.properties) {
            (NodeType::IpAddress, TypeSpecificProps::IpAddress(props)) => rows.push((
                "ip_address".to_string(),
                props.address.clone(),
                node.label.clone(),
                node.confidence,
            )),
            (NodeType::Domain, TypeSpecificProps::Domain(props)) => rows.push((
                "domain".to_string(),
                props.domain.clone(),
                node.label.clone(),
                node.confidence,
            )),
            (NodeType::FileHash, TypeSpecificProps::FileHash(props)) => rows.push((
                "file_hash".to_string(),
                props.hash_value.clone(),
                node.label.clone(),
                node.confidence,
            )),
            _ => {}
        }
    }

    rows
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::{generate_html_report, ReportConfig};
    use crate::models::domain::{
        DomainProps, FileHashProps, HashAlgorithm, IpAddressProps, NodeData, NodeType,
        RelationData, RelationType, TypeSpecificProps,
    };

    #[test]
    fn generates_report_with_expected_sections() {
        let nodes = vec![
            NodeData {
                id: "node-ip".to_string(),
                node_type: NodeType::IpAddress,
                label: "8.8.8.8".to_string(),
                description: "resolver".to_string(),
                confidence: 1.0,
                properties: TypeSpecificProps::IpAddress(IpAddressProps {
                    address: "8.8.8.8".to_string(),
                    version: Some("ipv4".to_string()),
                    geo_location: None,
                    asn: None,
                    isp: None,
                    reputation: None,
                }),
                pos_x: 0.0,
                pos_y: 0.0,
                investigation_id: "inv-1".to_string(),
                created_at: None,
                updated_at: None,
            },
            NodeData {
                id: "node-domain".to_string(),
                node_type: NodeType::Domain,
                label: "evil.example".to_string(),
                description: String::new(),
                confidence: 0.8,
                properties: TypeSpecificProps::Domain(DomainProps {
                    domain: "evil.example".to_string(),
                    registrar: None,
                    creation_date: None,
                }),
                pos_x: 0.0,
                pos_y: 0.0,
                investigation_id: "inv-1".to_string(),
                created_at: None,
                updated_at: None,
            },
            NodeData {
                id: "node-hash".to_string(),
                node_type: NodeType::FileHash,
                label: "loader.dll".to_string(),
                description: String::new(),
                confidence: 0.7,
                properties: TypeSpecificProps::FileHash(FileHashProps {
                    hash_value: "deadbeef".to_string(),
                    algorithm: HashAlgorithm::SHA256,
                    file_name: Some("loader.dll".to_string()),
                    file_size: None,
                    file_type: None,
                    malware_classification: None,
                }),
                pos_x: 0.0,
                pos_y: 0.0,
                investigation_id: "inv-1".to_string(),
                created_at: None,
                updated_at: None,
            },
        ];
        let relations = vec![RelationData {
            id: "rel-1".to_string(),
            relation_type: RelationType::ResolvesTo,
            source_node_id: "node-domain".to_string(),
            target_node_id: "node-ip".to_string(),
            label: "dns".to_string(),
            confidence: 1.0,
            first_seen: None,
            last_seen: None,
            investigation_id: "inv-1".to_string(),
        }];
        let config = ReportConfig {
            title: "Incident Report".to_string(),
            author: "Analyst".to_string(),
            organization: "CyberWeaver".to_string(),
            include_ioc_list: true,
            include_graph_summary: true,
        };

        let html = generate_html_report(&nodes, &relations, &config);

        assert!(html.contains("Incident Report"));
        assert!(html.contains("图谱摘要"));
        assert!(html.contains("IOC 清单"));
        assert!(html.contains("evil.example"));
        assert!(html.contains("8.8.8.8"));
        assert!(html.contains("deadbeef"));
        assert!(html.contains("node_type"));
        assert!(html.contains("relation_type"));
    }
}
