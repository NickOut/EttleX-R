//! Relation and Group record types for the store layer (Slice 02).

use serde::{Deserialize, Serialize};

/// A full Relation record as stored in the `relations` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationRecord {
    pub id: String,
    pub source_ettle_id: String,
    pub target_ettle_id: String,
    pub relation_type: String,
    pub properties_json: String,
    pub created_at: String,
    pub tombstoned_at: Option<String>,
}

/// Options for listing Relations.
#[derive(Debug, Clone, Default)]
pub struct RelationListOpts {
    pub source_ettle_id: Option<String>,
    pub target_ettle_id: Option<String>,
    pub relation_type: Option<String>,
    pub include_tombstoned: bool,
}

/// A full Group record as stored in the `groups` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupRecord {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub tombstoned_at: Option<String>,
}

/// A full GroupMember record as stored in the `group_members` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMemberRecord {
    pub id: String,
    pub group_id: String,
    pub ettle_id: String,
    pub created_at: String,
    pub tombstoned_at: Option<String>,
}

/// A Relation Type Registry entry.
#[derive(Debug, Clone)]
pub struct RelationTypeEntry {
    pub relation_type: String,
    pub properties_json: String,
    pub created_at: String,
    pub tombstoned_at: Option<String>,
}
