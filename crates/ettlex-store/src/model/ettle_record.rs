//! Ettle v2 record types for the store layer.

/// A full Ettle record as stored in the v2 schema.
#[derive(Debug, Clone)]
pub struct EttleRecord {
    pub id: String,
    pub title: String,
    pub why: String,
    pub what: String,
    pub how: String,
    pub reasoning_link_id: Option<String>,
    pub reasoning_link_type: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub tombstoned_at: Option<String>,
}

/// Options for listing Ettles.
#[derive(Debug, Clone)]
pub struct EttleListOpts {
    pub limit: u32,
    pub cursor: Option<EttleCursor>,
    pub include_tombstoned: bool,
}

/// Cursor for Ettle list pagination (created_at, id).
#[derive(Debug, Clone)]
pub struct EttleCursor {
    pub created_at: String,
    pub id: String,
}

/// A paginated page of Ettle list items.
#[derive(Debug, Clone)]
pub struct EttleListPage {
    pub items: Vec<EttleListItem>,
    /// Cursor for the next page, if any. This is an `EttleCursor` encoded as
    /// `{created_at},{id}` and then base64 URL-safe-no-pad encoded.
    pub next_cursor: Option<String>,
}

/// Summary item returned by ettle list.
#[derive(Debug, Clone)]
pub struct EttleListItem {
    pub id: String,
    pub title: String,
    pub tombstoned_at: Option<String>,
}
