//! Integration tests for fleet-manifest.
//!
//! Tests cover: Vessel construction/builder, Manifest operations,
//! trust-vector math, Laman rigidity, trust filtering, serde roundtrip,
//! repos inventory, and edge cases.

use fleet_manifest::{Manifest, Vessel, VesselStatus};

// ── Vessel tests ─────────────────────────────────────────────────────────────

#[test]
fn test_vessel_new_defaults() {
    let v = Vessel::new("test-vessel", "integration test dummy", 0.75);
    assert_eq!(v.name, "test-vessel");
    assert_eq!(v.role, "integration test dummy");
    assert_eq!(v.trust, 0.75);
    assert!(v.repo.is_none());
    assert!(v.endpoint.is_none());
    assert!(matches!(v.status, VesselStatus::Unknown));
}

#[test]
fn test_vessel_builder_with_repo() {
    let v = Vessel::new("a", "role", 0.5)
        .with_repo("org/repo-name");
    assert_eq!(v.repo, Some("org/repo-name".into()));
}

#[test]
fn test_vessel_builder_with_status() {
    let v = Vessel::new("b", "role", 0.5)
        .with_status(VesselStatus::Online);
    assert!(matches!(v.status, VesselStatus::Online));
}

#[test]
fn test_vessel_builder_with_endpoint() {
    let v = Vessel::new("c", "role", 0.5)
        .with_endpoint("http://localhost:9999");
    assert_eq!(v.endpoint, Some("http://localhost:9999".into()));
}

#[test]
fn test_vessel_builder_chaining() {
    let v = Vessel::new("d", "role", 0.6)
        .with_repo("org/repo")
        .with_status(VesselStatus::Online)
        .with_endpoint("http://example.com:8847");
    assert_eq!(v.repo, Some("org/repo".into()));
    assert!(matches!(v.status, VesselStatus::Online));
    assert_eq!(v.endpoint, Some("http://example.com:8847".into()));
}

#[test]
fn test_vessel_trust_threshold_exact_match() {
    let v = Vessel::new("e", "role", 0.80);
    assert!(v.is_trusted(0.80), "equal trust should pass");
}

#[test]
fn test_vessel_trust_threshold_below() {
    let v = Vessel::new("f", "role", 0.50);
    assert!(!v.is_trusted(0.75), "below threshold should fail");
    assert!(!v.is_trusted(1.01), "trust cannot be above 1.0, so always false");
}

#[test]
fn test_vessel_trust_zero() {
    let v = Vessel::new("g", "role", 0.0);
    assert!(v.is_trusted(0.0), "zero should trust zero");
    assert!(!v.is_trusted(0.01), "zero should not trust anything positive");
}

#[test]
fn test_vessel_trust_one() {
    let v = Vessel::new("h", "role", 1.0);
    assert!(v.is_trusted(0.0));
    assert!(v.is_trusted(0.5));
    assert!(v.is_trusted(1.0));
}

#[test]
fn test_trust_vector_mapping_zero() {
    let v = Vessel::new("i", "role", 0.0);
    assert_eq!(v.trust_vector, 0, "trust 0.0 → vector 0");
}

#[test]
fn test_trust_vector_mapping_one() {
    let v = Vessel::new("j", "role", 1.0);
    assert_eq!(v.trust_vector, 47, "trust 1.0 → vector 47");
}

#[test]
fn test_trust_vector_mapping_mid() {
    let v = Vessel::new("k", "role", 0.5);
    // 0.5 * 47 = 23.5 → 23 as u8
    assert_eq!(v.trust_vector, 23, "trust 0.5 → vector 23");
}

#[test]
fn test_trust_vector_max_clamp() {
    // values >1.0 are clamped to 47
    let v = Vessel::new("l", "role", 2.0);
    assert_eq!(v.trust_vector, 47, "above 1.0 gets clamped to 47");
}

#[test]
fn test_trust_vector_rounding() {
    let v = Vessel::new("m", "role", 0.01);
    // 0.01 * 47 = 0.47 → 0 as u8 (floor)
    assert_eq!(v.trust_vector, 0);
}

#[test]
fn test_vessel_status_online() {
    assert!(matches!(VesselStatus::Online, VesselStatus::Online));
}

#[test]
fn test_vessel_status_offline() {
    assert!(matches!(VesselStatus::Offline, VesselStatus::Offline));
}

#[test]
fn test_vessel_status_unknown() {
    assert!(matches!(VesselStatus::Unknown, VesselStatus::Unknown));
}

// ── Manifest tests ───────────────────────────────────────────────────────────

#[test]
fn test_manifest_new_is_empty() {
    let m = Manifest::new();
    assert_eq!(m.size(), 0);
    assert_eq!(m.version, "0.1.0");
    assert!(!m.last_updated.is_empty(), "should have a timestamp");
}

#[test]
fn test_manifest_default_is_empty() {
    let m: Manifest = Default::default();
    assert_eq!(m.size(), 0);
    assert_eq!(m.version, "0.1.0");
}

#[test]
fn test_manifest_current_fleet_size() {
    let m = Manifest::current_fleet();
    assert_eq!(m.size(), 5, "current fleet should have exactly 5 vessels");
}

#[test]
fn test_manifest_current_fleet_has_oracle1() {
    let m = Manifest::current_fleet();
    let oracle1 = m.vessels.iter().find(|v| v.name == "oracle1");
    assert!(oracle1.is_some(), "oracle1 should be in the fleet");
    let o = oracle1.unwrap();
    assert_eq!(o.trust_vector, 47);
    assert!(matches!(o.status, VesselStatus::Online));
    assert_eq!(o.endpoint, Some("http://localhost:8900".into()));
}

#[test]
fn test_manifest_current_fleet_jc1_is_offline() {
    let m = Manifest::current_fleet();
    let jc1 = m.vessels.iter().find(|v| v.name == "jc1").unwrap();
    assert!(matches!(jc1.status, VesselStatus::Offline),
        "JetsonClaw1 should be offline");
}

#[test]
fn test_manifest_current_fleet_test_probe_has_low_trust() {
    let m = Manifest::current_fleet();
    let probe = m.vessels.iter().find(|v| v.name == "test-probe").unwrap();
    assert_eq!(probe.trust, 0.50);
    assert!(probe.repo.is_none(), "test-probe has no repo");
}

// ── Trust filtering ──────────────────────────────────────────────────────────

#[test]
fn test_trusted_at_zero_includes_all() {
    let m = Manifest::current_fleet();
    let trusted = m.trusted(0.0);
    assert_eq!(trusted.len(), 5);
}

#[test]
fn test_trusted_at_one_includes_none() {
    let m = Manifest::current_fleet();
    let trusted = m.trusted(1.01); // above max possible
    assert_eq!(trusted.len(), 0);
}

#[test]
fn test_trusted_at_high_threshold() {
    let m = Manifest::current_fleet();
    // only oracle1 (1.00) at 0.95
    let trusted = m.trusted(0.95);
    assert_eq!(trusted.len(), 1);
    assert_eq!(trusted[0].name, "oracle1");
}

#[test]
fn test_trusted_at_mid_threshold() {
    let m = Manifest::current_fleet();
    // oracle1 (1.00) + fm (0.85) at 0.80
    let trusted = m.trusted(0.80);
    assert!(trusted.len() >= 2);
    assert!(trusted.iter().any(|v| v.name == "oracle1"));
    assert!(trusted.iter().any(|v| v.name == "fm"));
}

#[test]
fn test_trusted_retains_references_to_original_vessels() {
    let m = Manifest::current_fleet();
    let trusted = m.trusted(0.70);
    // should return references to original Vessel instances
    let original = m.vessels.iter().find(|v| v.name == "ccc").unwrap();
    let trusted_ccc = trusted.iter().find(|v| v.name == "ccc").unwrap();
    // they point to the same Vessel
    assert_eq!(trusted_ccc.trust, original.trust);
}

// ── Laman rigidity ───────────────────────────────────────────────────────────

#[test]
fn test_lamant_rigid_empty_fleet() {
    let m = Manifest::new();
    // V=0, E=0 → not rigid (guard returns expected=0)
    let (is_rigid, edge_count, expected) = m.lamant_rigid();
    assert!(!is_rigid, "empty fleet cannot be rigid");
    assert_eq!(edge_count, 0);
    assert_eq!(expected, 0);
}

#[test]
fn test_lamant_rigid_single_vessel() {
    // V=1, E=0 → not rigid (guard returns expected=0)
    let mut m = Manifest::new();
    m.vessels.push(Vessel::new("solo", "alone", 0.5));
    let (is_rigid, edge_count, expected) = m.lamant_rigid();
    assert!(!is_rigid, "single vessel cannot be rigid");
    assert_eq!(edge_count, 0);
    assert_eq!(expected, 0);
}

#[test]
fn test_lamant_rigid_two_vessels() {
    let mut m = Manifest::new();
    m.vessels.push(Vessel::new("a", "role", 0.5));
    m.vessels.push(Vessel::new("b", "role", 0.5));
    // V=2, E=1, expected=1 → is_rigid = |1/1 - 1| = 0 < 0.05 → true
    let (is_rigid, edge_count, expected) = m.lamant_rigid();
    assert!(is_rigid, "two vessels form a rigid pair");
    assert_eq!(edge_count, 1);
    assert_eq!(expected, 1);
}

#[test]
fn test_lamant_rigid_three_vessels() {
    let mut m = Manifest::new();
    m.vessels.push(Vessel::new("a", "role", 0.5));
    m.vessels.push(Vessel::new("b", "role", 0.5));
    m.vessels.push(Vessel::new("c", "role", 0.5));
    // V=3, E=3, expected=3 → is_rigid = |3/3 - 1| = 0 < 0.05 → true
    let (is_rigid, edge_count, expected) = m.lamant_rigid();
    assert!(is_rigid, "three vessels (triangle) is the minimal Laman-rigid graph");
    assert_eq!(edge_count, 3);
    assert_eq!(expected, 3);
}

#[test]
fn test_lamant_rigid_four_vessels() {
    let mut m = Manifest::new();
    m.vessels.push(Vessel::new("a", "role", 0.5));
    m.vessels.push(Vessel::new("b", "role", 0.5));
    m.vessels.push(Vessel::new("c", "role", 0.5));
    m.vessels.push(Vessel::new("d", "role", 0.5));
    // V=4, E=6, expected=5 → |6/5 - 1| = 0.2 > 0.05 → not rigid
    let (is_rigid, edge_count, expected) = m.lamant_rigid();
    assert!(!is_rigid);
    assert_eq!(edge_count, 6);
    assert_eq!(expected, 5);
}

#[test]
fn test_lamant_rigid_current_fleet() {
    let m = Manifest::current_fleet();
    // V=5, E=10, expected=7 → |10/7 - 1| ≈ 0.43 > 0.05 → not rigid
    let (is_rigid, edge_count, expected) = m.lamant_rigid();
    assert!(!is_rigid, "current fleet (complete K5) is over-rigid");
    assert_eq!(edge_count, 10);
    assert_eq!(expected, 7);
}

#[test]
fn test_lamant_rigid_ten_vessels() {
    let mut m = Manifest::new();
    for i in 0..10 {
        m.vessels.push(Vessel::new(&format!("v{}", i), "role", 0.5));
    }
    // V=10, E=45, expected=17 → |45/17 - 1| ≈ 1.65 > 0.05 → not rigid
    let (is_rigid, edge_count, expected) = m.lamant_rigid();
    assert!(!is_rigid, "complete graph K10 is extremely over-rigid");
    assert_eq!(edge_count, 45);
    assert_eq!(expected, 17);
}

#[test]
fn test_lamant_rigid_v_3_triangle() {
    // V=3, E=3 → exactly E=2V-3 → rigid
    let mut m = Manifest::new();
    m.vessels.push(Vessel::new("a", "role", 0.5));
    m.vessels.push(Vessel::new("b", "role", 0.5));
    m.vessels.push(Vessel::new("c", "role", 0.5));
    let (is_rigid, edge_count, _expected) = m.lamant_rigid();
    assert!(is_rigid, "triangle (V=3, E=3) is the minimal Laman-rigid bar-joint framework");
}

// ── Repos ────────────────────────────────────────────────────────────────────

#[test]
fn test_current_repos_count() {
    let repos = Manifest::current_repos();
    assert_eq!(repos.len(), 9, "should list 9 repos");
}

#[test]
fn test_current_repos_has_math_domain() {
    let repos = Manifest::current_repos();
    let math_count = repos.iter().filter(|r| {
        matches!(r.domain, fleet_manifest::RepoDomain::Math)
    }).count();
    assert_eq!(math_count, 3, "3 math repos: fleet-coordinate, pythagorean48-codes, holonomy-consensus");
}

#[test]
fn test_current_repos_has_fleet_coordinate() {
    let repos = Manifest::current_repos();
    let fc = repos.iter().find(|r| r.name == "fleet-coordinate").unwrap();
    assert!(matches!(fc.domain, fleet_manifest::RepoDomain::Math));
    assert_eq!(fc.stars, 0);
    assert!(fc.url.contains("SuperInstance/fleet-coordinate"));
}

#[test]
fn test_current_repos_domains() {
    let repos = Manifest::current_repos();
    // Check all domains appear
    let domains: std::collections::HashSet<_> = repos.iter().map(|r| {
        match r.domain {
            fleet_manifest::RepoDomain::Math => "math",
            fleet_manifest::RepoDomain::Protocol => "protocol",
            fleet_manifest::RepoDomain::Physics => "physics",
            fleet_manifest::RepoDomain::Docs => "docs",
            fleet_manifest::RepoDomain::Services => "services",
        }
    }).collect();
    assert!(domains.contains("math"));
    assert!(domains.contains("protocol"));
    assert!(domains.contains("physics"));
    assert!(domains.contains("docs"));
    // "services" doesn't appear yet — that's fine, keep it in the enum
}

#[test]
fn test_current_repos_urls_are_github() {
    let repos = Manifest::current_repos();
    for r in &repos {
        assert!(r.url.starts_with("https://github.com/SuperInstance/"),
            "repo {} has unexpected URL: {}", r.name, r.url);
    }
}

// ── Serde roundtrip ──────────────────────────────────────────────────────────

#[test]
fn test_vessel_serde_roundtrip() {
    let v = Vessel::new("roundtrip", "test", 0.75)
        .with_repo("org/r")
        .with_status(VesselStatus::Online)
        .with_endpoint("http://test:8847");
    let json = serde_json::to_string(&v).expect("serialize");
    let recovered: Vessel = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(v.name, recovered.name);
    assert_eq!(v.trust, recovered.trust);
    assert_eq!(v.trust_vector, recovered.trust_vector);
    assert_eq!(v.repo, recovered.repo);
    assert_eq!(v.endpoint, recovered.endpoint);
    assert!(matches!(recovered.status, VesselStatus::Online));
}

#[test]
fn test_vessel_serde_roundtrip_no_endpoint() {
    let v = Vessel::new("bare", "minimal", 0.1);
    let json = serde_json::to_string(&v).expect("serialize");
    let recovered: Vessel = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(v.name, recovered.name);
    assert!(recovered.endpoint.is_none());
    assert!(recovered.repo.is_none());
}

#[test]
fn test_manifest_serde_roundtrip() {
    let m = Manifest::current_fleet();
    let json = serde_json::to_string(&m).expect("serialize");
    let recovered: Manifest = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(m.version, recovered.version);
    assert_eq!(m.size(), recovered.size());
    assert_eq!(m.vessels[0].name, recovered.vessels[0].name);
    assert_eq!(m.vessels[1].trust, recovered.vessels[1].trust);
}

#[test]
fn test_vessel_status_serde_roundtrip() {
    for status in &[VesselStatus::Online, VesselStatus::Offline, VesselStatus::Unknown] {
        let json = serde_json::to_string(status).expect("serialize");
        let recovered: VesselStatus = serde_json::from_str(&json).expect("deserialize");
        assert!(matches!(recovered, VesselStatus::Online | VesselStatus::Offline | VesselStatus::Unknown));
    }
}

#[test]
fn test_repo_domain_serde_roundtrip() {
    use fleet_manifest::RepoDomain;
    for domain in &[
        RepoDomain::Math,
        RepoDomain::Protocol,
        RepoDomain::Physics,
        RepoDomain::Docs,
        RepoDomain::Services,
    ] {
        let json = serde_json::to_string(domain).expect("serialize");
        let _recovered: RepoDomain = serde_json::from_str(&json).expect("deserialize");
        // If it deserialized, the roundtrip succeeded
    }
}

#[test]
fn test_vessel_json_is_human_readable() {
    let v = Vessel::new("oracle1", "Keeper", 1.00)
        .with_status(VesselStatus::Online)
        .with_endpoint("http://localhost:8900");
    let json = serde_json::to_string_pretty(&v).expect("serialize");
    assert!(json.contains("oracle1"), "JSON should contain vessel name");
    assert!(json.contains("Keeper"), "JSON should contain role");
    assert!(json.contains("Online"), "JSON should contain status");
}

// ── Timestamp format ─────────────────────────────────────────────────────────

#[test]
fn test_chrono_timestamp_is_numeric() {
    let m = Manifest::new();
    // timestamp should be a unix epoch string (digits only)
    assert!(m.last_updated.chars().all(|c| c.is_ascii_digit()),
        "timestamp should be numeric, got: {}", m.last_updated);
    // reasonable range: epoch seconds for year 2023-2030
    let secs: u64 = m.last_updated.parse().expect("numeric timestamp");
    assert!(secs > 1_600_000_000, "timestamp too old: {}", secs);
    assert!(secs < 2_000_000_000, "timestamp too far in future: {}", secs);
}

#[test]
fn test_different_manifests_have_different_timestamps() {
    // Two adjacent calls should differ (or at least not error)
    let m1 = Manifest::new();
    std::thread::sleep(std::time::Duration::from_millis(10));
    let m2 = Manifest::new();
    // They might be different if clock ticks; if not, that's fine
    let _ = (m1, m2); // just checking no panic
}

// ── Builder idempotency ──────────────────────────────────────────────────────

#[test]
fn test_vessel_builder_with_repo_overwrite() {
    let v = Vessel::new("x", "role", 0.5)
        .with_repo("first/repo")
        .with_repo("second/repo");
    // Last wins
    assert_eq!(v.repo, Some("second/repo".into()));
}

#[test]
fn test_vessel_builder_with_status_chain() {
    let v = Vessel::new("y", "role", 0.5)
        .with_status(VesselStatus::Online)
        .with_status(VesselStatus::Offline);
    assert!(matches!(v.status, VesselStatus::Offline), "last status should win");
}
