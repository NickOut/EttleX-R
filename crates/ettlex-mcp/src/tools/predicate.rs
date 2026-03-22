//! Handler for `constraint_predicates.*` tool group.

use ettlex_core::policy_provider::PolicyProvider;
use ettlex_memory::commands::engine_query::{apply_engine_query, EngineQuery};
use ettlex_store::cas::FsStore;
use rusqlite::Connection;
use serde_json::{json, Value};

use crate::error::{McpError, McpResult, MCP_INVALID_INPUT};

/// Handle `constraint_predicates.preview`.
///
/// Params: `{ profile_ref?: String, context: Object, candidates: Array<String> }`
pub fn handle_predicate_preview(
    params: &Value,
    conn: &Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
) -> McpResult {
    let profile_ref = params
        .get("profile_ref")
        .and_then(Value::as_str)
        .map(String::from);

    let context = match params.get("context") {
        Some(v) => v.clone(),
        None => return McpResult::Err(McpError::new(MCP_INVALID_INPUT, "missing 'context' field")),
    };

    let candidates: Vec<String> = match params.get("candidates") {
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        Some(Value::Null) | None => vec![],
        _ => {
            return McpResult::Err(McpError::new(
                MCP_INVALID_INPUT,
                "'candidates' must be an array of strings",
            ))
        }
    };

    match apply_engine_query(
        EngineQuery::ConstraintPredicatesPreview {
            profile_ref,
            context,
            candidates,
        },
        conn,
        cas,
        Some(policy_provider),
    ) {
        Ok(result) => {
            use ettlex_memory::commands::engine_query::EngineQueryResult;
            use ettlex_memory::commands::read_tools::PreviewStatus;
            if let EngineQueryResult::PredicatePreview(r) = result {
                let status = match r.status {
                    PreviewStatus::Selected => "Selected",
                    PreviewStatus::NoMatch => "NoMatch",
                    PreviewStatus::Ambiguous => "Ambiguous",
                    PreviewStatus::RoutedForApproval => "RoutedForApproval",
                };
                McpResult::Ok(json!({
                    "status": status,
                    "selected": r.selected,
                    "candidates": r.candidates,
                }))
            } else {
                McpResult::Err(McpError::new("Internal", "unexpected result variant"))
            }
        }
        Err(e) => McpResult::Err(McpError::from_ex_error(e)),
    }
}
