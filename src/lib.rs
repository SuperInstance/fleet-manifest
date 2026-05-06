//! fleet-manifest — SuperInstance Fleet Inventory
//!
//! The living specification for what each vessel owns, what it trusts,
//! and how it communicates. This is the fleet's shared memory of itself.
//!
//! ## Design Principle
//! "The fleet knows itself." No central registry — every agent has a copy.
//! Trust relationships are encoded as Pythagorean48 vectors.
//! Laman rigidity (E = 2V-3) determines provably self-coordinating subgroups.
//!
//! ## Current Fleet (2026-05-06)
//!
//! | Vessel | Role | Trust | Status |
//! |--------|------|-------|--------|
//! | Oracle1 🔮 | Keeper/Primary | 1.00 | Online (Oracle Cloud, ARM64) |
//! | Forgemaster ⚒️ | GPU/Constraint Theory | 0.85 | Online (RTX 4050) |
//! | JetsonClaw1 ⚡ | Edge/Orin | 0.75 | Offline (2026-05-04) |
//! | CCC 🦀 | Research/Slides | 0.70 | Online (Kimi K2.5) |
//! | Test Probe 🔬 | Testing | 0.50 | Online |
//!
//! ## Repository Inventory
//!
//! Math foundations:
//! - `fleet-coordinate` — ZHC consensus + Laman rigidity + beam equilibrium
//! - `pythagorean48-codes` — 48-direction trust encoding (shared codebook)
//! - `holonomy-consensus` — O(C·L) consensus (FM's implementation)
//!
//! Implementation:
//! - `cocapn-glue-core` — Keeper↔Fleet binary wire protocol
//! - `aboracle` — FM-instinct agents (work-queue, beachcomb, health, mud-agent)
//! - `spline-physics` — Beam physics (multi-segment joints, Newton-Raphson)
//!
//! Documentation:
//! - `constraint-theory-ecosystem` — 8-chapter cookbook + SPEC.md
//! - `flux-research` — Dissertations, case studies, ArXiv papers
//!
//! Services:
//! - keeper (8900), agent-api (8901), holodeck (7778), MUD (7777), PLATO (8847)
//!

use serde::{Deserialize, Serialize};

/// One vessel in the fleet
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Vessel {
    /// Unique vessel name (e.g., "oracle1", "fm", "jc1")
    pub name: String,
    /// Human-readable role description
    pub role: String,
    /// Trust weight (0.0 - 1.0) — used for task routing priority
    pub trust: f64,
    /// Repository URL (if applicable)
    pub repo: Option<String>,
    /// Current status
    pub status: VesselStatus,
    /// Communication endpoint (if known)
    pub endpoint: Option<String>,
    /// Pythagorean48 trust vector index (0-47)
    /// Maps trust weight to a direction on the unit circle
    pub trust_vector: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum VesselStatus {
    Online,
    Offline,
    Unknown,
}

impl Vessel {
    pub fn new(name: &str, role: &str, trust: f64) -> Self {
        let trust_vector = ((trust * 47.0) as u8).min(47);
        Self {
            name: name.to_string(),
            role: role.to_string(),
            trust,
            repo: None,
            status: VesselStatus::Unknown,
            endpoint: None,
            trust_vector,
        }
    }

    pub fn with_repo(mut self, repo: &str) -> Self {
        self.repo = Some(repo.to_string());
        self
    }

    pub fn with_status(mut self, status: VesselStatus) -> Self {
        self.status = status;
        self
    }

    pub fn with_endpoint(mut self, endpoint: &str) -> Self {
        self.endpoint = Some(endpoint.to_string());
        self
    }

    pub fn is_trusted(&self, threshold: f64) -> bool {
        self.trust >= threshold
    }
}

/// The complete fleet manifest
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub version: String,
    pub last_updated: String,
    pub vessels: Vec<Vessel>,
}

impl Manifest {
    pub fn new() -> Self {
        Self {
            version: "0.1.0".to_string(),
            last_updated: chrono_timestamp(),
            vessels: Vec::new(),
        }
    }

    pub fn current_fleet() -> Self {
        let vessels = vec![
            Vessel::new("oracle1", "Keeper/Primary — GLM-5.1, Oracle Cloud ARM64", 1.00)
                .with_repo("SuperInstance/oracle1-workspace")
                .with_status(VesselStatus::Online)
                .with_endpoint("http://localhost:8900"),
            Vessel::new("fm", "Forgemaster — RTX 4050 GPU, Constraint Theory, LLVM", 0.85)
                .with_repo("SuperInstance/forgemaster")
                .with_status(VesselStatus::Online),
            Vessel::new("jc1", "JetsonClaw1 — Edge Orin, HDC cognition, bottle-fleet-coordination", 0.75)
                .with_repo("SuperInstance/jetsonclaw1-vessel")
                .with_status(VesselStatus::Offline)
                .with_endpoint("http://146.7.52.185:8847"),
            Vessel::new("ccc", "CCC — Kimi K2.5, research assistant, slide maker", 0.70)
                .with_repo("SuperInstance/cocapn")
                .with_status(VesselStatus::Online),
            Vessel::new("test-probe", "Test Probe — integration testing", 0.50)
                .with_status(VesselStatus::Online),
        ];
        Self {
            version: "0.1.0".to_string(),
            last_updated: chrono_timestamp(),
            vessels,
        }
    }

    /// Get vessels above a trust threshold
    pub fn trusted(&self, threshold: f64) -> Vec<&Vessel> {
        self.vessels.iter().filter(|v| v.trust >= threshold).collect()
    }

    /// Number of vessels
    pub fn size(&self) -> usize {
        self.vessels.len()
    }

    /// Check if fleet is Laman-rigid (E = 2V - 3)
    /// Returns (is_rigid, edge_count, expected_edges)
    pub fn lamant_rigid(&self) -> (bool, usize, usize) {
        let V = self.vessels.len();
        // Assume all pairs are connected for now (complete graph)
        let E = V * (V - 1) / 2;
        let expected = 2 * V - 3;
        let is_rigid = (E as f64 / expected as f64 - 1.0).abs() < 0.05;
        (is_rigid, E, expected)
    }
}

impl Default for Manifest {
    fn default() -> Self { Self::new() }
}

fn chrono_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{}", secs)
}

/// Repository in the SuperInstance ecosystem
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Repo {
    pub name: String,
    pub url: String,
    pub description: String,
    pub domain: RepoDomain,
    pub stars: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RepoDomain {
    Math,      // fleet-coordinate, pythagorean48-codes, holonomy-consensus
    Protocol,  // cocapn-glue-core, aboracle
    Physics,   // spline-physics, constraint-theory-llvm
    Docs,      // constraint-theory-ecosystem, flux-research
    Services,  // keeper, holodeck, PLATO room server
}

impl Manifest {
    pub fn current_repos() -> Vec<Repo> {
        vec![
            Repo {
                name: "fleet-coordinate".to_string(),
                url: "https://github.com/SuperInstance/fleet-coordinate".to_string(),
                description: "ZHC consensus + Laman rigidity + beam equilibrium + H1 emergence".to_string(),
                domain: RepoDomain::Math,
                stars: 0,
            },
            Repo {
                name: "pythagorean48-codes".to_string(),
                url: "https://github.com/SuperInstance/pythagorean48-codes".to_string(),
                description: "48 exact direction vectors for trust encoding — shared codebook".to_string(),
                domain: RepoDomain::Math,
                stars: 0,
            },
            Repo {
                name: "holonomy-consensus".to_string(),
                url: "https://github.com/SuperInstance/holonomy-consensus".to_string(),
                description: "O(C·L) consensus — FM's implementation of zero-holonomy consensus".to_string(),
                domain: RepoDomain::Math,
                stars: 0,
            },
            Repo {
                name: "cocapn-glue-core".to_string(),
                url: "https://github.com/SuperInstance/cocapn-glue-core".to_string(),
                description: "Keeper↔Fleet binary wire protocol — TILE, HEARTBEAT, REGISTER messages".to_string(),
                domain: RepoDomain::Protocol,
                stars: 0,
            },
            Repo {
                name: "aboracle".to_string(),
                url: "https://github.com/SuperInstance/aboracle".to_string(),
                description: "FM-instinct agents — work-queue, beachcomb, health-system, mud-agent".to_string(),
                domain: RepoDomain::Protocol,
                stars: 0,
            },
            Repo {
                name: "spline-physics".to_string(),
                url: "https://github.com/SuperInstance/spline-physics".to_string(),
                description: "Beam physics — multi-segment joints, Newton-Raphson, 21 tests passing".to_string(),
                domain: RepoDomain::Physics,
                stars: 0,
            },
            Repo {
                name: "constraint-theory-llvm".to_string(),
                url: "https://github.com/SuperInstance/constraint-theory-llvm".to_string(),
                description: "LLVM backend — CDCL → AVX-512, analog spline computing, 210 tests".to_string(),
                domain: RepoDomain::Physics,
                stars: 0,
            },
            Repo {
                name: "constraint-theory-ecosystem".to_string(),
                url: "https://github.com/SuperInstance/constraint-theory-ecosystem".to_string(),
                description: "8-chapter cookbook, 12 recipes with GUARD DSL + FLUX-C examples".to_string(),
                domain: RepoDomain::Docs,
                stars: 0,
            },
            Repo {
                name: "flux-research".to_string(),
                url: "https://github.com/SuperInstance/flux-research".to_string(),
                description: "Dissertations, case studies, ArXiv papers — fleet math, marine cert".to_string(),
                domain: RepoDomain::Docs,
                stars: 0,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_fleet() {
        let manifest = Manifest::current_fleet();
        assert_eq!(manifest.size(), 5);
        let trusted = manifest.trusted(0.80);
        assert!(trusted.len() >= 2); // oracle1 and fm at minimum
    }

    #[test]
    fn test_lamant_rigid() {
        let manifest = Manifest::current_fleet();
        let (is_rigid, E, expected) = manifest.lamant_rigid();
        // V=5, E=10, expected=7 → ratio=1.43 → not rigid (too many edges)
        assert!(!is_rigid, "complete graph of 5 is over-rigid, not Laman-rigid");
        assert_eq!(E, 10);
    }

    #[test]
    fn test_vessel_is_trusted() {
        let v = Vessel::new("test", "role", 0.85);
        assert!(v.is_trusted(0.80));
        assert!(!v.is_trusted(0.90));
    }

    #[test]
    fn test_trust_vector_mapping() {
        let v = Vessel::new("oracle1", "keeper", 1.00);
        assert_eq!(v.trust_vector, 47); // max trust → max vector index
    }
}
