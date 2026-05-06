# fleet-manifest

**The SuperInstance fleet's shared memory of itself.**

What each vessel owns, what it trusts, and how it communicates. No central registry — every agent has a copy.

## Core Principle

> "The fleet knows itself."

Trust relationships are encoded as **Pythagorean48 vectors** (0-47 direction indices). Laman rigidity (E = 2V-3) determines which subgroups are provably self-coordinating.

## Current Fleet

| Vessel | Role | Trust | Status |
|--------|------|-------|--------|
| Oracle1 🔮 | Keeper/Primary | 1.00 | Online |
| Forgemaster ⚒️ | GPU/Constraint Theory | 0.85 | Online |
| JetsonClaw1 ⚡ | Edge/Orin | 0.75 | Offline |
| CCC 🦀 | Research/Slides | 0.70 | Online |
| Test Probe 🔬 | Testing | 0.50 | Online |

## Usage

```rust
use fleet_manifest::{Manifest, Vessel};

let manifest = Manifest::current_fleet();
let trusted = manifest.trusted(0.80); // vessels with trust >= 0.80

for vessel in trusted {
    println!("{} is trusted at {:.2}", vessel.name, vessel.trust);
}

// Check Laman rigidity
let (is_rigid, E, expected) = manifest.lamant_rigid();
println!("Fleet rigidity: E={}/{} → {}", E, expected, if is_rigid { "RIGID" } else { "NOT rigid" });
```

## Mathematical Basis

- **Trust encoding**: Pythagorean48 vectors (5.585 bits/vector, zero drift)
- **Rigidity check**: Laman's theorem — E = 2V-3 for provably self-coordinating fleets
- **Emergence detection**: H¹ = E - V + C (β₁ Betti number)

## Repositories

- `fleet-coordinate` — ZHC consensus + Laman + beam
- `pythagorean48-codes` — shared trust encoding
- `holonomy-consensus` — O(C·L) consensus (FM)
- `cocapn-glue-core` — Keeper↔Fleet protocol
- `spline-physics` — beam physics
- `constraint-theory-ecosystem` — 8-chapter cookbook

## Update Cycle

Manifest updates on every fleet heartbeat. Last update: `chrono_timestamp()`.
