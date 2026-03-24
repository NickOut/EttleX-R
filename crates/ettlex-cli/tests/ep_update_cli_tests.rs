//! CLI integration tests for `action_ep_update` Ettle.
//!
//! EP construct retired in Slice 03. The `ettlex ep update` subcommand
//! and its underlying `commands::ep` module no longer exist.
//! This test file is retired; Slice 04 will remove it entirely.
//!
//! Scenario → test mapping:
//!   S-AU-5  test_cli_ep_update_retired (verifies EP command is absent)

#![allow(clippy::unwrap_used)]

// S-AU-5: ep update CLI subcommand has been retired in Slice 03.
// The `commands::ep` module no longer exists; the `ep` variant was removed
// from the CLI Commands enum. There is nothing to test here until Slice 04
// provides updated CLI surface for the Relations model.
#[test]
fn test_cli_ep_update_retired() {
    // No-op: EP construct removed in Slice 03.
    // This test is a placeholder asserting the retirement is in place.
    // Slice 04 will replace this with Relations-based CLI tests.
}
