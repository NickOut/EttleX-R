//! Seed digest canonicalization
//!
//! Computes stable SHA256 digests of seeds for reproducibility

use crate::seed::format_v0::{SeedEp, SeedV0};
use serde::Serialize;
use sha2::{Digest, Sha256};

/// Canonical representation of a seed for digest calculation
#[derive(Debug, Clone, Serialize)]
struct CanonicalSeed {
    schema_version: u32,
    project_name: String,
    ettles: Vec<CanonicalEttle>,
    links: Vec<CanonicalLink>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
struct CanonicalEttle {
    id: String,
    title: String,
    eps: Vec<CanonicalEp>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
struct CanonicalEp {
    id: String,
    ordinal: u32,
    normative: bool,
    why: String,
    what: String,
    how: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
struct CanonicalLink {
    parent: String,
    parent_ep: String,
    child: String,
}

/// Compute a stable digest for a seed
///
/// Returns a SHA256 hex digest of the canonicalized seed representation
pub fn compute_seed_digest(seed: &SeedV0) -> String {
    // Canonicalize seed
    let canonical = canonicalize_seed(seed);

    // Serialize to JSON with sorted keys
    let json = serde_json::to_string(&canonical).expect("Failed to serialize canonical seed");

    // Compute SHA256 digest
    let mut hasher = Sha256::new();
    hasher.update(json.as_bytes());
    let result = hasher.finalize();

    hex::encode(result)
}

/// Canonicalize a seed for deterministic digest calculation
fn canonicalize_seed(seed: &SeedV0) -> CanonicalSeed {
    // Sort ettles by ID
    let mut ettles: Vec<CanonicalEttle> = seed
        .ettles
        .iter()
        .map(|e| CanonicalEttle {
            id: e.id.clone(),
            title: e.title.clone(),
            eps: canonicalize_eps(&e.eps),
        })
        .collect();
    ettles.sort_by(|a, b| a.id.cmp(&b.id));

    // Sort links by (parent, parent_ep, child)
    let mut links: Vec<CanonicalLink> = seed
        .links
        .iter()
        .map(|l| CanonicalLink {
            parent: l.parent.clone(),
            parent_ep: l.parent_ep.clone(),
            child: l.child.clone(),
        })
        .collect();
    links.sort();

    CanonicalSeed {
        schema_version: seed.schema_version,
        project_name: seed.project.name.clone(),
        ettles,
        links,
    }
}

/// Canonicalize EPs (sort by ordinal)
fn canonicalize_eps(eps: &[SeedEp]) -> Vec<CanonicalEp> {
    let mut canonical_eps: Vec<CanonicalEp> = eps
        .iter()
        .map(|ep| CanonicalEp {
            id: ep.id.clone(),
            ordinal: ep.ordinal,
            normative: ep.normative,
            why: ep.why.clone(),
            what: ep.what.clone(),
            how: ep.how.clone(),
        })
        .collect();

    canonical_eps.sort_by_key(|ep| ep.ordinal);
    canonical_eps
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seed::parser::parse_seed_str;

    #[test]
    fn test_seed_digest_stable() {
        let yaml = r#"
schema_version: 0
project:
  name: test
ettles:
  - id: ettle:1
    title: "Test"
    eps:
      - id: ep:1:0
        ordinal: 0
        normative: true
        why: "Why"
        what: "What"
        how: "How"
links: []
"#;

        let seed1 = parse_seed_str(yaml).unwrap();
        let seed2 = parse_seed_str(yaml).unwrap();

        let digest1 = compute_seed_digest(&seed1);
        let digest2 = compute_seed_digest(&seed2);

        assert_eq!(digest1, digest2);
        assert_eq!(digest1.len(), 64); // SHA256 is 64 hex chars
    }

    #[test]
    fn test_seed_digest_format_independent() {
        // Same content, different whitespace
        let yaml1 = r#"
schema_version: 0
project:
  name: test
ettles:
  - id: ettle:1
    title: "Test"
    eps:
      - id: ep:1:0
        ordinal: 0
        normative: true
        why: "Why"
        what: "What"
        how: "How"
links: []
"#;

        let yaml2 = r#"
schema_version: 0
project:
    name: test
ettles:
    - id: ettle:1
      title: "Test"
      eps:
          - id: ep:1:0
            ordinal: 0
            normative: true
            why: "Why"
            what: "What"
            how: "How"
links: []
"#;

        let seed1 = parse_seed_str(yaml1).unwrap();
        let seed2 = parse_seed_str(yaml2).unwrap();

        let digest1 = compute_seed_digest(&seed1);
        let digest2 = compute_seed_digest(&seed2);

        assert_eq!(digest1, digest2, "Digests should be format-independent");
    }

    #[test]
    fn test_seed_digest_what_polymorphism() {
        // String format vs typed block format should produce same digest
        let yaml1 = r#"
schema_version: 0
project:
  name: test
ettles:
  - id: ettle:1
    title: "Test"
    eps:
      - id: ep:1:0
        ordinal: 0
        normative: true
        why: "Why"
        what: "Content"
        how: "How"
links: []
"#;

        let yaml2 = r#"
schema_version: 0
project:
  name: test
ettles:
  - id: ettle:1
    title: "Test"
    eps:
      - id: ep:1:0
        ordinal: 0
        normative: true
        why: "Why"
        what:
          text: "Content"
        how: "How"
links: []
"#;

        let seed1 = parse_seed_str(yaml1).unwrap();
        let seed2 = parse_seed_str(yaml2).unwrap();

        let digest1 = compute_seed_digest(&seed1);
        let digest2 = compute_seed_digest(&seed2);

        assert_eq!(
            digest1, digest2,
            "WHAT polymorphism should not affect digest"
        );
    }

    #[test]
    fn test_seed_digest_stable_with_sorting() {
        // Ettles in different order should produce same digest
        let yaml1 = r#"
schema_version: 0
project:
  name: test
ettles:
  - id: ettle:a
    title: "A"
    eps:
      - id: ep:a:0
        ordinal: 0
        normative: true
        why: "Why"
        what: "What"
        how: "How"
  - id: ettle:b
    title: "B"
    eps:
      - id: ep:b:0
        ordinal: 0
        normative: true
        why: "Why"
        what: "What"
        how: "How"
links: []
"#;

        let yaml2 = r#"
schema_version: 0
project:
  name: test
ettles:
  - id: ettle:b
    title: "B"
    eps:
      - id: ep:b:0
        ordinal: 0
        normative: true
        why: "Why"
        what: "What"
        how: "How"
  - id: ettle:a
    title: "A"
    eps:
      - id: ep:a:0
        ordinal: 0
        normative: true
        why: "Why"
        what: "What"
        how: "How"
links: []
"#;

        let seed1 = parse_seed_str(yaml1).unwrap();
        let seed2 = parse_seed_str(yaml2).unwrap();

        let digest1 = compute_seed_digest(&seed1);
        let digest2 = compute_seed_digest(&seed2);

        assert_eq!(
            digest1, digest2,
            "Digest should be stable regardless of Ettle order"
        );
    }
}
