//! Seed Format v0 schema
//!
//! Defines the YAML structure for seed import

use serde::{Deserialize, Serialize};

/// Top-level seed file structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedV0 {
    /// Schema version (must be 0 for this format)
    pub schema_version: u32,

    /// Project metadata
    pub project: SeedProject,

    /// List of Ettles to import
    pub ettles: Vec<SeedEttle>,

    /// List of parent-child links
    pub links: Vec<SeedLink>,
}

/// Project metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedProject {
    /// Project name
    pub name: String,
}

/// Ettle definition in seed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedEttle {
    /// Ettle ID (stable across imports)
    pub id: String,

    /// Ettle title
    pub title: String,

    /// EPs owned by this Ettle
    pub eps: Vec<SeedEp>,
}

/// EP definition in seed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedEp {
    /// EP ID (stable across imports)
    pub id: String,

    /// Ordinal position within parent Ettle
    pub ordinal: u32,

    /// Whether this EP is normative
    pub normative: bool,

    /// WHY content
    pub why: String,

    /// WHAT content (can be string or typed block)
    #[serde(deserialize_with = "deserialize_what")]
    pub what: String,

    /// HOW content
    pub how: String,
}

/// Link definition in seed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedLink {
    /// Parent Ettle ID
    pub parent: String,

    /// Parent EP ID
    pub parent_ep: String,

    /// Child Ettle ID
    pub child: String,
}

/// Custom deserializer for WHAT field to normalize string vs typed block
fn deserialize_what<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    use std::fmt;

    struct WhatVisitor;

    impl<'de> Visitor<'de> for WhatVisitor {
        type Value = String;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string or a map with 'text' field")
        }

        fn visit_str<E>(self, value: &str) -> Result<String, E>
        where
            E: de::Error,
        {
            Ok(value.to_string())
        }

        fn visit_map<M>(self, mut map: M) -> Result<String, M::Error>
        where
            M: de::MapAccess<'de>,
        {
            let mut text = None;
            while let Some(key) = map.next_key::<String>()? {
                if key == "text" {
                    text = Some(map.next_value()?);
                } else {
                    // Skip unknown fields
                    map.next_value::<serde::de::IgnoredAny>()?;
                }
            }
            text.ok_or_else(|| de::Error::missing_field("text"))
        }
    }

    deserializer.deserialize_any(WhatVisitor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_seed() {
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

        let seed: SeedV0 = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(seed.schema_version, 0);
        assert_eq!(seed.project.name, "test");
        assert_eq!(seed.ettles.len(), 1);
        assert_eq!(seed.ettles[0].id, "ettle:1");
        assert_eq!(seed.ettles[0].eps.len(), 1);
    }

    #[test]
    fn test_what_string_format() {
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
        what: "Simple string"
        how: "How"
links: []
"#;

        let seed: SeedV0 = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(seed.ettles[0].eps[0].what, "Simple string");
    }

    #[test]
    fn test_what_typed_format() {
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
        what:
          text: "Typed block"
        how: "How"
links: []
"#;

        let seed: SeedV0 = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(seed.ettles[0].eps[0].what, "Typed block");
    }
}
