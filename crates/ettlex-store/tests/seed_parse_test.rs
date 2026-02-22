// Integration tests for seed parsing
// Covers Gherkin scenarios D.2, D.4, D.5: Seed validation

use std::path::PathBuf;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

#[test]
fn test_parse_minimal_seed() {
    // Given: A minimal valid seed file
    let path = fixtures_dir().join("seed_minimal.yaml");

    // When: We parse it
    let result = ettlex_store::seed::parse_seed_file(&path);

    // Then: Parsing succeeds
    assert!(
        result.is_ok(),
        "Should parse minimal seed: {:?}",
        result.err()
    );

    let seed = result.unwrap();
    assert_eq!(seed.schema_version, 0);
    assert_eq!(seed.ettles.len(), 1);
    assert_eq!(seed.ettles[0].id, "ettle:root");
    assert_eq!(seed.ettles[0].eps.len(), 1);
    assert_eq!(seed.links.len(), 0);
}

#[test]
fn test_parse_full_seed() {
    // Given: A full seed with links
    let path = fixtures_dir().join("seed_full.yaml");

    // When: We parse it
    let result = ettlex_store::seed::parse_seed_file(&path);

    // Then: Parsing succeeds
    assert!(result.is_ok(), "Should parse full seed: {:?}", result.err());

    let seed = result.unwrap();
    assert_eq!(seed.ettles.len(), 2);
    assert_eq!(seed.links.len(), 1);
    assert_eq!(seed.links[0].parent, "ettle:root");
    assert_eq!(seed.links[0].child, "ettle:store");
}

#[test]
fn test_reject_invalid_schema_version() {
    // Given: A seed with invalid schema version
    let path = fixtures_dir().join("seed_invalid_schema_version.yaml");

    // When: We parse it
    let result = ettlex_store::seed::parse_seed_file(&path);

    // Then: Parsing fails
    assert!(result.is_err(), "Should reject invalid schema version");

    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("schema_version"),
        "Error should mention schema_version"
    );
}

#[test]
fn test_reject_duplicate_ordinals() {
    // Given: A seed with duplicate ordinals
    let path = fixtures_dir().join("seed_invalid_duplicate_ordinal.yaml");

    // When: We parse it
    let result = ettlex_store::seed::parse_seed_file(&path);

    // Then: Parsing fails
    assert!(result.is_err(), "Should reject duplicate ordinals");

    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("Duplicate ordinal"),
        "Error should mention duplicate ordinal"
    );
}

#[test]
fn test_what_polymorphism() {
    // Both string and typed block formats should work
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
        what: "String format"
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
          text: "String format"
        how: "How"
links: []
"#;

    let seed1 = ettlex_store::seed::parse_seed_str(yaml1).unwrap();
    let seed2 = ettlex_store::seed::parse_seed_str(yaml2).unwrap();

    assert_eq!(seed1.ettles[0].eps[0].what, seed2.ettles[0].eps[0].what);
}
