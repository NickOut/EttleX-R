//! Seed parser with validation
//!
//! Parses YAML and validates schema version, ordinal uniqueness, and referential integrity

#![allow(clippy::result_large_err)]

use crate::errors::{seed_validation, Result};
use crate::seed::format_v0::SeedV0;
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

/// Parse a seed file from a path
pub fn parse_seed_file(path: &Path) -> Result<SeedV0> {
    parse_seed_file_with_db(path, None)
}

/// Parse a seed file from a path with optional database context for cross-seed validation
pub fn parse_seed_file_with_db(path: &Path, conn: Option<&Connection>) -> Result<SeedV0> {
    let content = fs::read_to_string(path)
        .map_err(|e| seed_validation(&format!("Failed to read seed file: {}", e)))?;

    parse_seed_str_with_db(&content, conn)
}

/// Parse a seed from a string
pub fn parse_seed_str(content: &str) -> Result<SeedV0> {
    parse_seed_str_with_db(content, None)
}

/// Parse a seed from a string with optional database context for cross-seed validation
pub fn parse_seed_str_with_db(content: &str, conn: Option<&Connection>) -> Result<SeedV0> {
    // Parse YAML
    let seed: SeedV0 = serde_yaml::from_str(content)
        .map_err(|e| seed_validation(&format!("YAML parse error: {}", e)))?;

    // Validate seed
    validate_seed(&seed, conn)?;

    Ok(seed)
}

/// Validate a parsed seed
fn validate_seed(seed: &SeedV0, conn: Option<&Connection>) -> Result<()> {
    // Validate schema version
    if seed.schema_version != 0 {
        return Err(seed_validation(&format!(
            "Unsupported schema_version: {}. Expected 0",
            seed.schema_version
        )));
    }

    // Validate ordinal uniqueness within each Ettle
    for ettle in &seed.ettles {
        let mut ordinals = HashSet::new();
        for ep in &ettle.eps {
            if !ordinals.insert(ep.ordinal) {
                return Err(seed_validation(&format!(
                    "Duplicate ordinal {} in ettle {}",
                    ep.ordinal, ettle.id
                )));
            }
        }
    }

    // Validate referential integrity for links
    let ettle_ids: HashSet<&String> = seed.ettles.iter().map(|e| &e.id).collect();
    let ep_ids: HashSet<&String> = seed
        .ettles
        .iter()
        .flat_map(|e| e.eps.iter().map(|ep| &ep.id))
        .collect();

    for link in &seed.links {
        // Check parent exists (check database first if available)
        if !ettle_ids.contains(&link.parent) {
            if let Some(conn) = conn {
                // Check if parent exists in database
                let exists: bool = conn
                    .query_row(
                        "SELECT 1 FROM ettles WHERE id = ?1 AND deleted = 0",
                        [&link.parent],
                        |_| Ok(true),
                    )
                    .unwrap_or(false);

                if !exists {
                    return Err(seed_validation(&format!(
                        "Link references non-existent parent Ettle: {}",
                        link.parent
                    )));
                }
            } else {
                return Err(seed_validation(&format!(
                    "Link references non-existent parent Ettle: {}",
                    link.parent
                )));
            }
        }

        // Check parent_ep exists (check database first if available)
        if !ep_ids.contains(&link.parent_ep) {
            if let Some(conn) = conn {
                // Check if parent EP exists in database
                let exists: bool = conn
                    .query_row(
                        "SELECT 1 FROM eps WHERE id = ?1 AND deleted = 0",
                        [&link.parent_ep],
                        |_| Ok(true),
                    )
                    .unwrap_or(false);

                if !exists {
                    return Err(seed_validation(&format!(
                        "Link references non-existent parent EP: {}",
                        link.parent_ep
                    )));
                }
            } else {
                return Err(seed_validation(&format!(
                    "Link references non-existent parent EP: {}",
                    link.parent_ep
                )));
            }
        }

        // Check child exists
        if !ettle_ids.contains(&link.child) {
            return Err(seed_validation(&format!(
                "Link references non-existent child Ettle: {}",
                link.child
            )));
        }
    }

    // Validate that parent_ep belongs to parent Ettle
    let ettle_eps: HashMap<&String, HashSet<&String>> = seed
        .ettles
        .iter()
        .map(|e| (&e.id, e.eps.iter().map(|ep| &ep.id).collect()))
        .collect();

    for link in &seed.links {
        if let Some(eps) = ettle_eps.get(&link.parent) {
            if !eps.contains(&link.parent_ep) {
                return Err(seed_validation(&format!(
                    "Link EP {} does not belong to parent Ettle {}",
                    link.parent_ep, link.parent
                )));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_seed() {
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

        let result = parse_seed_str(yaml);
        assert!(result.is_ok());
    }

    #[test]
    fn test_reject_invalid_schema_version() {
        let yaml = r#"
schema_version: 99
project:
  name: test
ettles: []
links: []
"#;

        let result = parse_seed_str(yaml);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("schema_version"));
    }

    #[test]
    fn test_reject_duplicate_ordinals() {
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
      - id: ep:1:1
        ordinal: 0
        normative: false
        why: "Why2"
        what: "What2"
        how: "How2"
links: []
"#;

        let result = parse_seed_str(yaml);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Duplicate ordinal"));
    }

    #[test]
    fn test_reject_missing_reference() {
        let yaml = r#"
schema_version: 0
project:
  name: test
ettles:
  - id: ettle:1
    title: "Parent"
    eps:
      - id: ep:1:0
        ordinal: 0
        normative: true
        why: "Why"
        what: "What"
        how: "How"
links:
  - parent: ettle:1
    parent_ep: ep:1:0
    child: ettle:nonexistent
"#;

        let result = parse_seed_str(yaml);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("non-existent"));
    }

    #[test]
    fn test_what_polymorphism_stable() {
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
        what: "Plain string"
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
          text: "Plain string"
        how: "How"
links: []
"#;

        let seed1 = parse_seed_str(yaml1).unwrap();
        let seed2 = parse_seed_str(yaml2).unwrap();

        // Both should parse to the same WHAT content
        assert_eq!(seed1.ettles[0].eps[0].what, seed2.ettles[0].eps[0].what);
    }
}
