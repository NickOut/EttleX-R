//! Architectural conformance tests for ettlex-agent-api.
//!
//! SC-49  test_agent_api_only_memory_dep
//! SC-50  test_agent_api_writes_route_through_memory
//! SC-51  test_agent_api_single_boundary_module
//! SC-52  test_agent_api_no_why_what_how_in_logs
//! SC-53  test_agent_api_no_apply_mcp_command

use std::path::Path;

const WORKSPACE_ROOT: &str = env!("CARGO_MANIFEST_DIR");

fn agent_api_root() -> std::path::PathBuf {
    Path::new(WORKSPACE_ROOT).to_path_buf()
}

fn workspace_root() -> std::path::PathBuf {
    Path::new(WORKSPACE_ROOT)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

// ---------------------------------------------------------------------------
// SC-49 — Agent API depends only on ettlex-memory (workspace)
// ---------------------------------------------------------------------------

#[test]
fn test_agent_api_only_memory_dep() {
    let cargo_toml_path = agent_api_root().join("Cargo.toml");
    let contents = std::fs::read_to_string(&cargo_toml_path).expect("should read Cargo.toml");

    // Parse the [dependencies] section
    let toml: toml::Value = contents.parse().expect("should parse Cargo.toml");
    let deps = toml
        .get("dependencies")
        .and_then(|d| d.as_table())
        .expect("[dependencies] should be a table");

    let ettlex_deps: Vec<&str> = deps
        .keys()
        .filter(|k| k.starts_with("ettlex-"))
        .map(|k| k.as_str())
        .collect();

    assert_eq!(
        ettlex_deps,
        vec!["ettlex-memory"],
        "ettlex-memory should be the only ettlex-* dep, found: {:?}",
        ettlex_deps
    );

    // Explicitly assert the forbidden crates are absent
    assert!(
        !deps.contains_key("ettlex-engine"),
        "ettlex-engine must not be in [dependencies]"
    );
    assert!(
        !deps.contains_key("ettlex-store"),
        "ettlex-store must not be in [dependencies]"
    );
    assert!(
        !deps.contains_key("ettlex-core"),
        "ettlex-core must not be in [dependencies]"
    );
}

// ---------------------------------------------------------------------------
// SC-50 — All write operations route through MemoryManager
// ---------------------------------------------------------------------------

#[test]
fn test_agent_api_writes_route_through_memory() {
    let ops_dir = agent_api_root().join("src").join("operations");

    for file_name in &["ettle.rs", "relation.rs", "group.rs"] {
        let path = ops_dir.join(file_name);
        let contents = std::fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("should read {}", path.display()));

        // No direct ettlex_engine import
        assert!(
            !contents.contains("ettlex_engine::"),
            "{file_name} must not import directly from ettlex_engine"
        );
        // No direct apply_command from engine
        assert!(
            !contents.contains("ettlex_engine::commands::command::apply_command"),
            "{file_name} must not call ettlex_engine apply_command directly"
        );
    }
}

// ---------------------------------------------------------------------------
// SC-51 — Exactly one boundary mapping module
// ---------------------------------------------------------------------------

#[test]
fn test_agent_api_single_boundary_module() {
    let mapping_path = agent_api_root()
        .join("src")
        .join("boundary")
        .join("mapping.rs");

    assert!(
        mapping_path.exists(),
        "src/boundary/mapping.rs must exist at {}",
        mapping_path.display()
    );

    // Operations files must not contain From<ExError> or error conversion impls
    let ops_dir = agent_api_root().join("src").join("operations");
    for file_name in &["ettle.rs", "relation.rs", "group.rs"] {
        let path = ops_dir.join(file_name);
        let contents = std::fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("should read {}", path.display()));

        assert!(
            !contents.contains("impl From<ExError>"),
            "{file_name} must not contain From<ExError> impl — use boundary/mapping.rs"
        );
        assert!(
            !contents.contains("impl From<ettlex_memory::ExError>"),
            "{file_name} must not contain error type conversion logic"
        );
    }
}

// ---------------------------------------------------------------------------
// SC-52 — No WHY/WHAT/HOW content in log output
// ---------------------------------------------------------------------------

#[test]
fn test_agent_api_no_why_what_how_in_logs() {
    let ops_dir = agent_api_root().join("src").join("operations");

    for file_name in &["ettle.rs", "relation.rs", "group.rs"] {
        let path = ops_dir.join(file_name);
        let contents = std::fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("should read {}", path.display()));

        // Check that log_op_* macros do not include why/what/how as field names
        // We scan for patterns like: why = ..., what = ..., how = ...
        // inside log_op_start/end/error invocations.
        // Simple check: no log field named "why", "what", or "how" in logging macros.
        let log_lines: Vec<&str> = contents.lines().filter(|l| l.contains("log_op_")).collect();

        for line in &log_lines {
            assert!(
                !line.contains("why ="),
                "WHY content must not appear as a log field in {file_name}: {line}"
            );
            assert!(
                !line.contains("what ="),
                "WHAT content must not appear as a log field in {file_name}: {line}"
            );
            assert!(
                !line.contains("how ="),
                "HOW content must not appear as a log field in {file_name}: {line}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// SC-53 — No apply_mcp_command reference in workspace
// ---------------------------------------------------------------------------

#[test]
fn test_agent_api_no_apply_mcp_command() {
    // Walk all Rust source files in the workspace
    let ws_root = workspace_root();
    let crates_dir = ws_root.join("crates");

    let mut found: Vec<String> = Vec::new();
    visit_rs_files(&crates_dir, &mut |path, contents| {
        if contents.contains("apply_mcp_command") {
            found.push(path.to_string_lossy().to_string());
        }
    });

    assert!(
        found.is_empty(),
        "apply_mcp_command found in workspace source files (it was retired in Slice 02): {:?}",
        found
    );
}

fn visit_rs_files(dir: &Path, visitor: &mut impl FnMut(&Path, &str)) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Skip target directories and tests directories
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                if name == "target" || name == "tests" {
                    continue;
                }
                visit_rs_files(&path, visitor);
            } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
                if let Ok(contents) = std::fs::read_to_string(&path) {
                    visitor(&path, &contents);
                }
            }
        }
    }
}
