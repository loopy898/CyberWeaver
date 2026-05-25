use tauri_app_lib::models::domain::*;

// ---------------------------------------------------------------------------
// NodeType tests
// ---------------------------------------------------------------------------

#[test]
fn test_node_type_display_names() {
    assert_eq!(NodeType::IpAddress.display_name(), "IP 地址");
    assert_eq!(NodeType::Domain.display_name(), "域名");
    assert_eq!(NodeType::FileHash.display_name(), "文件哈希");
    assert_eq!(NodeType::Process.display_name(), "进程");
    assert_eq!(NodeType::Malware.display_name(), "恶意软件");
    assert_eq!(NodeType::Ttp.display_name(), "攻击技术");
    assert_eq!(NodeType::ThreatActor.display_name(), "威胁组织");
    assert_eq!(NodeType::Asset.display_name(), "资产");
}

#[test]
fn test_node_type_icons() {
    assert_eq!(NodeType::IpAddress.icon(), "ip");
    assert_eq!(NodeType::Domain.icon(), "domain");
    assert_eq!(NodeType::FileHash.icon(), "file-hash");
    assert_eq!(NodeType::Process.icon(), "process");
    assert_eq!(NodeType::Malware.icon(), "malware");
    assert_eq!(NodeType::Ttp.icon(), "ttp");
    assert_eq!(NodeType::ThreatActor.icon(), "threat-actor");
    assert_eq!(NodeType::Asset.icon(), "asset");
}

#[test]
fn test_node_type_serde_roundtrip() {
    let types = vec![
        NodeType::IpAddress,
        NodeType::Domain,
        NodeType::FileHash,
        NodeType::Process,
        NodeType::Malware,
        NodeType::Ttp,
        NodeType::ThreatActor,
        NodeType::Asset,
    ];

    for nt in types {
        let json = serde_json::to_string(&nt).unwrap();
        let deserialized: NodeType = serde_json::from_str(&json).unwrap();
        assert_eq!(nt, deserialized);
    }
}

#[test]
fn test_node_type_serde_is_snake_case() {
    // Verify that NodeType serializes using snake_case (the serde rename_all)
    let json = serde_json::to_string(&NodeType::IpAddress).unwrap();
    assert_eq!(json, r#""ip_address""#);

    let json = serde_json::to_string(&NodeType::ThreatActor).unwrap();
    assert_eq!(json, r#""threat_actor""#);

    let json = serde_json::to_string(&NodeType::FileHash).unwrap();
    assert_eq!(json, r#""file_hash""#);
}

// ---------------------------------------------------------------------------
// RelationType tests
// ---------------------------------------------------------------------------

#[test]
fn test_relation_type_display_names() {
    assert_eq!(RelationType::ConnectsTo.display_name(), "网络连接");
    assert_eq!(RelationType::ResolvesTo.display_name(), "DNS 解析");
    assert_eq!(RelationType::Creates.display_name(), "创建");
    assert_eq!(RelationType::BelongsTo.display_name(), "归属于");
    assert_eq!(RelationType::Uses.display_name(), "使用技术");
    assert_eq!(RelationType::Targets.display_name(), "攻击目标");
    assert_eq!(RelationType::Contains.display_name(), "包含");
}

#[test]
fn test_relation_type_is_directed() {
    // All relation types are directed in this model
    assert!(RelationType::ConnectsTo.is_directed());
    assert!(RelationType::ResolvesTo.is_directed());
    assert!(RelationType::Creates.is_directed());
    assert!(RelationType::BelongsTo.is_directed());
    assert!(RelationType::Uses.is_directed());
    assert!(RelationType::Targets.is_directed());
    assert!(RelationType::Contains.is_directed());
}

#[test]
fn test_relation_type_serde_roundtrip() {
    let types = vec![
        RelationType::ConnectsTo,
        RelationType::ResolvesTo,
        RelationType::Creates,
        RelationType::BelongsTo,
        RelationType::Uses,
        RelationType::Targets,
        RelationType::Contains,
    ];

    for rt in types {
        let json = serde_json::to_string(&rt).unwrap();
        let deserialized: RelationType = serde_json::from_str(&json).unwrap();
        assert_eq!(rt, deserialized);
    }
}

#[test]
fn test_relation_type_serde_is_snake_case() {
    let json = serde_json::to_string(&RelationType::ConnectsTo).unwrap();
    assert_eq!(json, r#""connects_to""#);

    let json = serde_json::to_string(&RelationType::ResolvesTo).unwrap();
    assert_eq!(json, r#""resolves_to""#);

    let json = serde_json::to_string(&RelationType::BelongsTo).unwrap();
    assert_eq!(json, r#""belongs_to""#);
}

// ---------------------------------------------------------------------------
// HashAlgorithm tests
// ---------------------------------------------------------------------------

#[test]
fn test_hash_algorithm_default() {
    let algo = HashAlgorithm::default();
    assert_eq!(algo, HashAlgorithm::MD5);
}

#[test]
fn test_hash_algorithm_serde_lowercase() {
    let json = serde_json::to_string(&HashAlgorithm::MD5).unwrap();
    assert_eq!(json, r#""md5""#);

    let json = serde_json::to_string(&HashAlgorithm::SHA1).unwrap();
    assert_eq!(json, r#""sha1""#);

    let json = serde_json::to_string(&HashAlgorithm::SHA256).unwrap();
    assert_eq!(json, r#""sha256""#);
}

#[test]
fn test_hash_algorithm_serde_roundtrip() {
    let algos = vec![
        HashAlgorithm::MD5,
        HashAlgorithm::SHA1,
        HashAlgorithm::SHA256,
    ];
    for algo in algos {
        let json = serde_json::to_string(&algo).unwrap();
        let back: HashAlgorithm = serde_json::from_str(&json).unwrap();
        assert_eq!(algo, back);
    }
}

// ---------------------------------------------------------------------------
// Reputation tests
// ---------------------------------------------------------------------------

#[test]
fn test_reputation_serde_lowercase() {
    let json = serde_json::to_string(&Reputation::Clean).unwrap();
    assert_eq!(json, r#""clean""#);

    let json = serde_json::to_string(&Reputation::Malicious).unwrap();
    assert_eq!(json, r#""malicious""#);

    let json = serde_json::to_string(&Reputation::Unknown).unwrap();
    assert_eq!(json, r#""unknown""#);
}

#[test]
fn test_reputation_serde_roundtrip() {
    let reps = vec![
        Reputation::Clean,
        Reputation::Suspicious,
        Reputation::Malicious,
        Reputation::Unknown,
    ];
    for rep in reps {
        let json = serde_json::to_string(&rep).unwrap();
        let back: Reputation = serde_json::from_str(&json).unwrap();
        assert_eq!(rep, back);
    }
}

// ---------------------------------------------------------------------------
// TypeSpecificProps tests
// ---------------------------------------------------------------------------

#[test]
fn test_ip_address_props_default() {
    let props = IpAddressProps::default();
    assert!(props.address.is_empty());
    assert!(props.version.is_none());
    assert!(props.geo_location.is_none());
    assert!(props.asn.is_none());
    assert!(props.isp.is_none());
    assert!(props.reputation.is_none());
}

#[test]
fn test_ip_address_props_serde() {
    let props = IpAddressProps {
        address: "10.0.0.1".to_string(),
        version: Some("IPv4".to_string()),
        geo_location: None,
        asn: None,
        isp: None,
        reputation: Some(Reputation::Malicious),
    };

    let json = serde_json::to_string(&props).unwrap();
    let back: IpAddressProps = serde_json::from_str(&json).unwrap();
    assert_eq!(back.address, "10.0.0.1");
    assert_eq!(back.version, Some("IPv4".to_string()));
    assert_eq!(back.reputation, Some(Reputation::Malicious));
    assert!(back.geo_location.is_none());
}

#[test]
fn test_type_specific_props_tagged_enum_ip() {
    let ip_props = IpAddressProps {
        address: "192.168.1.1".to_string(),
        version: Some("IPv4".to_string()),
        geo_location: None,
        asn: None,
        isp: None,
        reputation: Some(Reputation::Suspicious),
    };

    let props = TypeSpecificProps::IpAddress(ip_props);
    let json = serde_json::to_string(&props).unwrap();
    let deserialized: TypeSpecificProps = serde_json::from_str(&json).unwrap();

    match deserialized {
        TypeSpecificProps::IpAddress(p) => {
            assert_eq!(p.address, "192.168.1.1");
            assert_eq!(p.reputation, Some(Reputation::Suspicious));
        }
        _ => panic!("Expected IpAddress variant"),
    }
}

#[test]
fn test_type_specific_props_tagged_enum_domain() {
    let domain_props = DomainProps {
        domain: "evil.com".to_string(),
        registrar: Some("GoDaddy".to_string()),
        creation_date: None,
    };

    let props = TypeSpecificProps::Domain(domain_props);
    let json = serde_json::to_string(&props).unwrap();
    let deserialized: TypeSpecificProps = serde_json::from_str(&json).unwrap();

    match deserialized {
        TypeSpecificProps::Domain(p) => {
            assert_eq!(p.domain, "evil.com");
            assert_eq!(p.registrar, Some("GoDaddy".to_string()));
        }
        _ => panic!("Expected Domain variant"),
    }
}

#[test]
fn test_type_specific_props_tagged_enum_file_hash() {
    let fh_props = FileHashProps {
        hash_value: "d41d8cd98f00b204e9800998ecf8427e".to_string(),
        algorithm: HashAlgorithm::MD5,
        file_name: Some("malware.exe".to_string()),
        file_size: Some(1024),
        file_type: Some("PE32".to_string()),
        malware_classification: Some("trojan".to_string()),
    };

    let props = TypeSpecificProps::FileHash(fh_props);
    let json = serde_json::to_string(&props).unwrap();
    let deserialized: TypeSpecificProps = serde_json::from_str(&json).unwrap();

    match deserialized {
        TypeSpecificProps::FileHash(p) => {
            assert_eq!(p.hash_value, "d41d8cd98f00b204e9800998ecf8427e");
            assert_eq!(p.algorithm, HashAlgorithm::MD5);
            assert_eq!(p.file_name, Some("malware.exe".to_string()));
            assert_eq!(p.file_size, Some(1024));
        }
        _ => panic!("Expected FileHash variant"),
    }
}

#[test]
fn test_type_specific_props_tagged_enum_malware() {
    let malware_props = MalwareProps {
        family_name: "Emotet".to_string(),
        aliases: vec!["Geodo".to_string(), "Heodo".to_string()],
        malware_type: Some("Banking Trojan".to_string()),
        first_seen: None,
    };

    let props = TypeSpecificProps::Malware(malware_props);
    let json = serde_json::to_string(&props).unwrap();
    let deserialized: TypeSpecificProps = serde_json::from_str(&json).unwrap();

    match deserialized {
        TypeSpecificProps::Malware(p) => {
            assert_eq!(p.family_name, "Emotet");
            assert_eq!(p.aliases.len(), 2);
            assert!(p.aliases.contains(&"Geodo".to_string()));
        }
        _ => panic!("Expected Malware variant"),
    }
}

#[test]
fn test_type_specific_props_tagged_enum_ttp() {
    let ttp_props = TtpProps {
        mitre_id: "T1059.001".to_string(),
        tactic: Some("Execution".to_string()),
        platform: vec!["Windows".to_string(), "macOS".to_string()],
        data_source: vec!["Process".to_string(), "Command".to_string()],
    };

    let props = TypeSpecificProps::Ttp(ttp_props);
    let json = serde_json::to_string(&props).unwrap();
    let deserialized: TypeSpecificProps = serde_json::from_str(&json).unwrap();

    match deserialized {
        TypeSpecificProps::Ttp(p) => {
            assert_eq!(p.mitre_id, "T1059.001");
            assert_eq!(p.tactic, Some("Execution".to_string()));
            assert_eq!(p.platform.len(), 2);
        }
        _ => panic!("Expected Ttp variant"),
    }
}

#[test]
fn test_type_specific_props_tagged_enum_threat_actor() {
    let ta_props = ThreatActorProps {
        name: "APT29".to_string(),
        aliases: vec!["Cozy Bear".to_string()],
        motivation: Some("Espionage".to_string()),
        sophistication: Some("Advanced".to_string()),
        targets: vec!["Government".to_string(), "Technology".to_string()],
    };

    let props = TypeSpecificProps::ThreatActor(ta_props);
    let json = serde_json::to_string(&props).unwrap();
    let deserialized: TypeSpecificProps = serde_json::from_str(&json).unwrap();

    match deserialized {
        TypeSpecificProps::ThreatActor(p) => {
            assert_eq!(p.name, "APT29");
            assert_eq!(p.motivation, Some("Espionage".to_string()));
            assert_eq!(p.targets.len(), 2);
        }
        _ => panic!("Expected ThreatActor variant"),
    }
}

#[test]
fn test_type_specific_props_tagged_enum_asset() {
    let asset_props = AssetProps {
        hostname: "SRV-DC01".to_string(),
        os: Some("Windows Server 2019".to_string()),
        ip_addresses: vec!["10.0.0.1".to_string(), "10.0.0.2".to_string()],
        owner: Some("IT Department".to_string()),
        criticality: Some("High".to_string()),
    };

    let props = TypeSpecificProps::Asset(asset_props);
    let json = serde_json::to_string(&props).unwrap();
    let deserialized: TypeSpecificProps = serde_json::from_str(&json).unwrap();

    match deserialized {
        TypeSpecificProps::Asset(p) => {
            assert_eq!(p.hostname, "SRV-DC01");
            assert_eq!(p.os, Some("Windows Server 2019".to_string()));
            assert_eq!(p.ip_addresses.len(), 2);
            assert_eq!(p.criticality, Some("High".to_string()));
        }
        _ => panic!("Expected Asset variant"),
    }
}

#[test]
fn test_type_specific_props_tagged_enum_process() {
    let proc_props = ProcessProps {
        process_name: "powershell.exe".to_string(),
        pid: Some(1234),
        command_line: Some("powershell -enc ...".to_string()),
        parent_process: Some("explorer.exe".to_string()),
        user: Some("Administrator".to_string()),
    };

    let props = TypeSpecificProps::Process(proc_props);
    let json = serde_json::to_string(&props).unwrap();
    let deserialized: TypeSpecificProps = serde_json::from_str(&json).unwrap();

    match deserialized {
        TypeSpecificProps::Process(p) => {
            assert_eq!(p.process_name, "powershell.exe");
            assert_eq!(p.pid, Some(1234));
            assert_eq!(p.user, Some("Administrator".to_string()));
        }
        _ => panic!("Expected Process variant"),
    }
}

#[test]
fn test_type_specific_props_tagged_enum_json_structure() {
    // Verify the tagged enum serializes with "type" and "data" fields
    let props = TypeSpecificProps::IpAddress(IpAddressProps {
        address: "10.0.0.1".to_string(),
        version: None,
        geo_location: None,
        asn: None,
        isp: None,
        reputation: None,
    });

    let json = serde_json::to_string(&props).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["type"], "IpAddress");
    assert!(parsed["data"].is_object());
    assert_eq!(parsed["data"]["address"], "10.0.0.1");
}

// ---------------------------------------------------------------------------
// NodeData / RelationData transfer struct tests
// ---------------------------------------------------------------------------

#[test]
fn test_node_data_serde_roundtrip() {
    let ip_props = IpAddressProps {
        address: "10.0.0.1".to_string(),
        version: Some("IPv4".to_string()),
        geo_location: None,
        asn: None,
        isp: None,
        reputation: Some(Reputation::Malicious),
    };

    let nd = NodeData {
        id: "node-001".to_string(),
        node_type: NodeType::IpAddress,
        label: "10.0.0.1".to_string(),
        description: "Suspicious IP".to_string(),
        confidence: 0.9,
        properties: TypeSpecificProps::IpAddress(ip_props),
        pos_x: 100.0,
        pos_y: 200.0,
        investigation_id: "inv-001".to_string(),
        created_at: Some("2024-01-01 00:00:00".to_string()),
        updated_at: None,
    };

    let json = serde_json::to_string(&nd).unwrap();
    let back: NodeData = serde_json::from_str(&json).unwrap();

    assert_eq!(back.id, "node-001");
    assert_eq!(back.node_type, NodeType::IpAddress);
    assert_eq!(back.label, "10.0.0.1");
    assert!((back.confidence - 0.9f32).abs() < f32::EPSILON);
    assert!((back.pos_x - 100.0).abs() < f64::EPSILON);
    assert!((back.pos_y - 200.0).abs() < f64::EPSILON);
    assert_eq!(back.investigation_id, "inv-001");
    assert_eq!(back.created_at, Some("2024-01-01 00:00:00".to_string()));
    assert!(back.updated_at.is_none());

    match back.properties {
        TypeSpecificProps::IpAddress(p) => {
            assert_eq!(p.address, "10.0.0.1");
            assert_eq!(p.reputation, Some(Reputation::Malicious));
        }
        _ => panic!("Expected IpAddress"),
    }
}

#[test]
fn test_relation_data_serde_roundtrip() {
    let rd = RelationData {
        id: "rel-001".to_string(),
        relation_type: RelationType::ConnectsTo,
        source_node_id: "node-a".to_string(),
        target_node_id: "node-b".to_string(),
        label: "HTTP connection".to_string(),
        confidence: 0.8,
        first_seen: Some("2024-01-01 00:00:00".to_string()),
        last_seen: Some("2024-01-02 00:00:00".to_string()),
        investigation_id: "inv-001".to_string(),
    };

    let json = serde_json::to_string(&rd).unwrap();
    let back: RelationData = serde_json::from_str(&json).unwrap();

    assert_eq!(back.id, "rel-001");
    assert_eq!(back.relation_type, RelationType::ConnectsTo);
    assert_eq!(back.source_node_id, "node-a");
    assert_eq!(back.target_node_id, "node-b");
    assert_eq!(back.label, "HTTP connection");
    assert!((back.confidence - 0.8f32).abs() < f32::EPSILON);
    assert_eq!(back.first_seen, Some("2024-01-01 00:00:00".to_string()));
    assert_eq!(back.last_seen, Some("2024-01-02 00:00:00".to_string()));
}
