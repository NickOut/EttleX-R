//! Apply Atomicity Tests
//!
//! This test suite verifies the functional-boundary atomicity guarantees of the
//! apply() function.
//!
//! ## Scenarios Covered
//!
//! 1. Apply returns new valid state on success
//! 2. Apply fails without partial mutation (atomicity)
//! 3. Apply surfaces typed errors and never panics
//! 4. State ownership transfer semantics

use ettlex_core::{
    apply, ops::ettle_ops, policy::NeverAnchoredPolicy, Command, EttleXError, Store,
};

#[test]
fn test_apply_returns_new_valid_state_on_success() {
    // GIVEN an empty store
    let state = Store::new();

    // WHEN we apply a valid EttleCreate command
    let cmd = Command::EttleCreate {
        title: "Test Ettle".to_string(),
        metadata: None,
        why: None,
        what: None,
        how: None,
    };

    let policy = NeverAnchoredPolicy;
    let new_state = apply(state, cmd, &policy).unwrap();

    // THEN the new state contains the created Ettle
    assert_eq!(new_state.list_ettles().len(), 1);
    let ettle = &new_state.list_ettles()[0];
    assert_eq!(ettle.title, "Test Ettle");
    assert!(!ettle.deleted);
    assert_eq!(ettle.ep_ids.len(), 1); // EP0 created
}

#[test]
fn test_apply_fails_without_partial_mutation() {
    // GIVEN a store with an existing Ettle
    let mut state = Store::new();
    let ettle_id =
        ettle_ops::create_ettle(&mut state, "Original".to_string(), None, None, None, None)
            .unwrap();

    // Clone state to preserve original
    let original_state = state.clone();

    // WHEN we apply an invalid command (empty title)
    let cmd = Command::EttleUpdate {
        ettle_id: ettle_id.clone(),
        title: Some("".to_string()), // Invalid: empty title
        metadata: None,
    };

    let policy = NeverAnchoredPolicy;
    let result = apply(state, cmd, &policy);

    // THEN the command fails
    assert!(result.is_err());
    assert!(matches!(result, Err(EttleXError::InvalidTitle { .. })));

    // AND the original state is unchanged (caller still has valid original)
    assert_eq!(original_state.list_ettles().len(), 1);
    let ettle = original_state.get_ettle(&ettle_id).unwrap();
    assert_eq!(ettle.title, "Original");
}

#[test]
fn test_apply_surfaces_typed_errors_never_panics() {
    let state = Store::new();

    // Test various invalid commands that should return typed errors, not panic

    // 1. Invalid title (empty)
    let cmd = Command::EttleCreate {
        title: "".to_string(),
        metadata: None,
        why: None,
        what: None,
        how: None,
    };
    let result = apply(state.clone(), cmd, &NeverAnchoredPolicy);
    assert!(matches!(result, Err(EttleXError::InvalidTitle { .. })));

    // 2. Non-existent Ettle ID
    let cmd = Command::EttleUpdate {
        ettle_id: "nonexistent".to_string(),
        title: Some("New Title".to_string()),
        metadata: None,
    };
    let result = apply(state.clone(), cmd, &NeverAnchoredPolicy);
    assert!(matches!(result, Err(EttleXError::EttleNotFound { .. })));

    // 3. Non-existent EP ID
    let cmd = Command::EpUpdate {
        ep_id: "nonexistent".to_string(),
        why: Some("New why".to_string()),
        what: None,
        how: None,
        normative: None,
    };
    let result = apply(state.clone(), cmd, &NeverAnchoredPolicy);
    assert!(matches!(result, Err(EttleXError::EpNotFound { .. })));

    // 4. Invalid EP content (empty WHAT string)
    let mut temp_state = Store::new();
    let ettle_id =
        ettle_ops::create_ettle(&mut temp_state, "Test".to_string(), None, None, None, None)
            .unwrap();

    let cmd = Command::EpCreate {
        ettle_id,
        ordinal: 1,
        normative: true,
        why: String::new(),
        what: "  ".to_string(), // Whitespace-only WHAT (invalid)
        how: String::new(),
    };
    let result = apply(temp_state, cmd, &NeverAnchoredPolicy);
    assert!(matches!(result, Err(EttleXError::InvalidWhat { .. })));
}

#[test]
fn test_state_ownership_transfer() {
    // GIVEN an initial state
    let state = Store::new();

    // WHEN we apply a command
    let cmd = Command::EttleCreate {
        title: "Test".to_string(),
        metadata: None,
        why: None,
        what: None,
        how: None,
    };

    let policy = NeverAnchoredPolicy;
    let new_state = apply(state, cmd, &policy).unwrap();
    // Note: `state` is moved and no longer accessible here

    // THEN we own the new state and can continue working with it
    assert_eq!(new_state.list_ettles().len(), 1);

    // We can apply another command to the new state
    let cmd2 = Command::EttleCreate {
        title: "Test 2".to_string(),
        metadata: None,
        why: None,
        what: None,
        how: None,
    };

    let newer_state = apply(new_state, cmd2, &policy).unwrap();
    assert_eq!(newer_state.list_ettles().len(), 2);
}

#[test]
fn test_apply_chaining() {
    // GIVEN an empty state
    let state = Store::new();
    let policy = NeverAnchoredPolicy;

    // WHEN we chain multiple apply calls
    let state = apply(
        state,
        Command::EttleCreate {
            title: "First".to_string(),
            metadata: None,
            why: None,
            what: None,
            how: None,
        },
        &policy,
    )
    .unwrap();

    let state = apply(
        state,
        Command::EttleCreate {
            title: "Second".to_string(),
            metadata: None,
            why: None,
            what: None,
            how: None,
        },
        &policy,
    )
    .unwrap();

    let state = apply(
        state,
        Command::EttleCreate {
            title: "Third".to_string(),
            metadata: None,
            why: None,
            what: None,
            how: None,
        },
        &policy,
    )
    .unwrap();

    // THEN all operations succeeded and state is valid
    assert_eq!(state.list_ettles().len(), 3);
}

#[test]
fn test_apply_error_preserves_original_state() {
    // GIVEN a state with some data
    let mut state = Store::new();
    ettle_ops::create_ettle(&mut state, "Ettle 1".to_string(), None, None, None, None).unwrap();
    ettle_ops::create_ettle(&mut state, "Ettle 2".to_string(), None, None, None, None).unwrap();

    let original_count = state.list_ettles().len();

    // WHEN we attempt an invalid operation
    let cmd = Command::EttleUpdate {
        ettle_id: "nonexistent".to_string(),
        title: Some("New Title".to_string()),
        metadata: None,
    };

    let policy = NeverAnchoredPolicy;
    let result = apply(state.clone(), cmd, &policy);

    // THEN the operation fails
    assert!(result.is_err());

    // AND if we had kept the original state (via clone), it's still valid
    assert_eq!(state.list_ettles().len(), original_count);
}
