use std::process::Command;

fn workspace_root() -> std::path::PathBuf {
    // CARGO_MANIFEST_DIR = /path/to/workspace/crates/ettlex-errors
    // parent = /path/to/workspace/crates
    // parent = /path/to/workspace  (workspace root)
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap() // crates/
        .parent()
        .unwrap() // workspace root
        .to_path_buf()
}

// SC-14: println! rejected by CI script
#[test]
fn test_check_banned_patterns_rejects_println() {
    let root = workspace_root();
    let script = root.join("scripts/check_banned_patterns.sh");
    let content = std::fs::read_to_string(&script).unwrap();
    assert!(
        content.contains("println!"),
        "Script should check for println!"
    );
}

// SC-15: tracing_subscriber init rejected by CI script
#[test]
fn test_check_banned_patterns_rejects_tracing_subscriber_init() {
    let root = workspace_root();
    let script = root.join("scripts/check_banned_patterns.sh");
    let content = std::fs::read_to_string(&script).unwrap();
    assert!(
        content.contains("tracing_subscriber"),
        "Script should check for tracing_subscriber"
    );
}

// SC-16: ettlex-errors has no dep on ettlex-core
#[test]
fn test_ettlex_errors_no_core_dep() {
    let cargo_toml = include_str!("../Cargo.toml");
    // Should not contain ettlex-core as a dependency (ettlex-core-types is allowed)
    let without_types = cargo_toml.replace("ettlex-core-types", "");
    assert!(
        !without_types.contains("ettlex-core"),
        "ettlex-errors must not depend on ettlex-core. Found in Cargo.toml:\n{}",
        cargo_toml
    );
}

// SC-17: ettlex-logging has no dep on ettlex-core (read from filesystem)
#[test]
fn test_ettlex_logging_no_core_dep() {
    let root = workspace_root();
    let cargo_toml_path = root.join("crates/ettlex-logging/Cargo.toml");
    let content = std::fs::read_to_string(&cargo_toml_path).unwrap();
    // Must not have ettlex-core as dependency (ettlex-core-types is fine)
    let without_types = content.replace("ettlex-core-types", "");
    assert!(
        !without_types.contains("ettlex-core"),
        "ettlex-logging must not depend on ettlex-core. Found:\n{}",
        content
    );
}

// SC-18: EttleXError not in workspace source (excluding this conformance test file itself)
#[test]
fn test_ettlex_x_error_not_in_workspace() {
    let root = workspace_root();
    let output = Command::new("grep")
        .args([
            "-r",
            "--include=*.rs",
            "--exclude=conformance_tests.rs",
            "EttleXError",
            "crates/",
        ])
        .current_dir(&root)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.is_empty(),
        "EttleXError must not exist in workspace source files. Found:\n{}",
        stdout
    );
}

// SC-19: Store public API returns ExError (grep for EttleXError in store)
#[test]
fn test_store_public_api_no_ettlex_x_error() {
    let root = workspace_root();
    let output = Command::new("grep")
        .args([
            "-r",
            "--include=*.rs",
            "EttleXError",
            "crates/ettlex-store/src/",
        ])
        .current_dir(&root)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.is_empty(),
        "ettlex-store must not reference EttleXError. Found:\n{}",
        stdout
    );
}

// SC-20: Engine public API returns ExError
#[test]
fn test_engine_public_api_no_ettlex_x_error() {
    let root = workspace_root();
    let output = Command::new("grep")
        .args([
            "-r",
            "--include=*.rs",
            "EttleXError",
            "crates/ettlex-engine/src/",
        ])
        .current_dir(&root)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.is_empty(),
        "ettlex-engine must not reference EttleXError. Found:\n{}",
        stdout
    );
}

// SC-21: No direct tracing_subscriber::init outside ettlex-logging
#[test]
fn test_no_direct_tracing_subscriber_init() {
    let root = workspace_root();
    let output = Command::new("grep")
        .args([
            "-r",
            "--include=*.rs",
            "tracing_subscriber.*init()",
            "crates/",
        ])
        .current_dir(&root)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Filter out ettlex-logging/src/ lines and this conformance test file itself
    let violations: Vec<&str> = stdout
        .lines()
        .filter(|l| !l.contains("ettlex-logging/src/") && !l.contains("conformance_tests.rs"))
        .collect();
    assert!(
        violations.is_empty(),
        "Direct tracing_subscriber::init() found outside ettlex-logging:\n{}",
        violations.join("\n")
    );
}

// SC-22: No println! in non-test source (run the CI script)
#[test]
fn test_no_println_in_non_test_code() {
    let root = workspace_root();
    let script = root.join("scripts/check_banned_patterns.sh");
    let output = Command::new("bash")
        .arg(&script)
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "check_banned_patterns.sh failed:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );
}

// SC-23: From<EttleXError> bridge does not exist
#[test]
fn test_from_bridge_not_in_workspace() {
    let root = workspace_root();
    let output = Command::new("grep")
        .args([
            "-r",
            "--include=*.rs",
            "--exclude=conformance_tests.rs",
            "From<EttleXError>",
            "crates/",
        ])
        .current_dir(&root)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.is_empty(),
        "From<EttleXError> bridge must be removed. Found:\n{}",
        stdout
    );
}

// SC-24: EttleXError enum not defined
#[test]
fn test_ettlex_x_error_enum_not_defined() {
    let root = workspace_root();
    let output = Command::new("grep")
        .args([
            "-r",
            "--include=*.rs",
            "--exclude=conformance_tests.rs",
            "enum EttleXError",
            "crates/",
        ])
        .current_dir(&root)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.is_empty(),
        "EttleXError enum must be removed. Found:\n{}",
        stdout
    );
}

// SC-25: RequestId, TraceId, RequestContext in ettlex-core-types
#[test]
fn test_core_types_correlation_types_present() {
    use ettlex_core_types::{RequestContext, RequestId, TraceId};
    let _rid = RequestId::new();
    let _tid = TraceId::new();
    // Just verifying they compile
    let _ = std::mem::size_of::<RequestContext>();
}

// SC-26: Sensitive<T> in ettlex-core-types
#[test]
fn test_core_types_sensitive_t_present() {
    use ettlex_core_types::Sensitive;
    let s: Sensitive<String> = Sensitive::new("secret".to_string());
    let _ = s;
}

// SC-27: ettlex-core-types has no workspace crate deps
#[test]
fn test_core_types_no_workspace_deps() {
    let root = workspace_root();
    let cargo_toml =
        std::fs::read_to_string(root.join("crates/ettlex-core-types/Cargo.toml")).unwrap();
    // Check that no other ettlex-* crate appears as a dependency (not just the package name)
    // Dependencies would appear as e.g. `ettlex-errors = ...` or `ettlex-logging = ...`
    // We exclude the [package] section by checking for dependency-like patterns
    let has_ettlex_dep = cargo_toml
        .lines()
        .filter(|l| !l.trim_start().starts_with("name"))
        .any(|l| {
            let l = l.trim_start();
            (l.starts_with("ettlex-errors")
                || l.starts_with("ettlex-logging")
                || l.starts_with("ettlex-core")
                || l.starts_with("ettlex-store")
                || l.starts_with("ettlex-engine")
                || l.starts_with("ettlex-mcp")
                || l.starts_with("ettlex-cli"))
                && l.contains('=')
        });
    assert!(
        !has_ettlex_dep,
        "ettlex-core-types must not depend on any workspace crate. Found:\n{}",
        cargo_toml
    );
}
