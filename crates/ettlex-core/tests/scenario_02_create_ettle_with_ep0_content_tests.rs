/// Scenario 2: Create Ettle With EP0 Content
///
/// Tests creating an Ettle with WHY/WHAT/HOW content for EP0.
/// Validates content constraints (empty strings forbidden for WHAT/HOW).
use ettlex_core::errors::EttleXError;
use ettlex_core::ops::{ettle_ops, Store};

#[test]
fn test_scenario_02_happy_create_ettle_with_ep0_content() {
    // GIVEN an empty store
    let mut store = Store::new();

    // WHEN creating an Ettle with WHY/WHAT/HOW for EP0
    let ettle_id = ettle_ops::create_ettle(
        &mut store,
        "API Gateway".to_string(),
        None,
        Some("Need to route requests to microservices".to_string()),
        Some("Central entry point for all API requests".to_string()),
        Some("Use nginx with reverse proxy configuration".to_string()),
    )
    .expect("Should create Ettle with EP0 content");

    // THEN the Ettle exists
    let ettle = store.get_ettle(&ettle_id).expect("Ettle should exist");
    assert_eq!(ettle.title, "API Gateway");

    // AND EP0 has the provided content
    assert_eq!(ettle.ep_ids.len(), 1);
    let ep0 = store.get_ep(&ettle.ep_ids[0]).expect("EP0 should exist");
    assert_eq!(ep0.ordinal, 0);
    assert_eq!(ep0.why, "Need to route requests to microservices");
    assert_eq!(ep0.what, "Central entry point for all API requests");
    assert_eq!(ep0.how, "Use nginx with reverse proxy configuration");
}

#[test]
fn test_scenario_02_error_empty_how_string() {
    // GIVEN an empty store
    let mut store = Store::new();

    // WHEN creating an Ettle with empty HOW string
    let result = ettle_ops::create_ettle(
        &mut store,
        "Test Ettle".to_string(),
        None,
        Some("Why text".to_string()),
        Some("What text".to_string()),
        Some("".to_string()), // Empty HOW
    );

    // THEN it should fail with InvalidHow error
    assert!(result.is_err());
    assert!(matches!(result, Err(EttleXError::InvalidHow { .. })));
}

#[test]
fn test_scenario_02_error_empty_what_string() {
    // GIVEN an empty store
    let mut store = Store::new();

    // WHEN creating an Ettle with empty WHAT string
    let result = ettle_ops::create_ettle(
        &mut store,
        "Test Ettle".to_string(),
        None,
        Some("Why text".to_string()),
        Some("".to_string()), // Empty WHAT
        Some("How text".to_string()),
    );

    // THEN it should fail with InvalidWhat error
    assert!(result.is_err());
    assert!(matches!(result, Err(EttleXError::InvalidWhat { .. })));
}

#[test]
fn test_scenario_02_omitted_content_is_allowed() {
    // GIVEN an empty store
    let mut store = Store::new();

    // WHEN creating an Ettle with only WHY (WHAT/HOW omitted)
    let ettle_id = ettle_ops::create_ettle(
        &mut store,
        "Minimal Content".to_string(),
        None,
        Some("Just the why".to_string()),
        None, // WHAT omitted
        None, // HOW omitted
    )
    .expect("Should allow omitted WHAT/HOW");

    // THEN EP0 has WHY but empty WHAT/HOW
    let ettle = store.get_ettle(&ettle_id).expect("Ettle should exist");
    let ep0 = store.get_ep(&ettle.ep_ids[0]).expect("EP0 should exist");
    assert_eq!(ep0.why, "Just the why");
    assert_eq!(ep0.what, "");
    assert_eq!(ep0.how, "");
}

#[test]
fn test_scenario_02_none_values_create_empty_strings() {
    // GIVEN an empty store
    let mut store = Store::new();

    // WHEN creating an Ettle with all content as None
    let ettle_id =
        ettle_ops::create_ettle(&mut store, "No Content".to_string(), None, None, None, None)
            .expect("Should create with empty content");

    // THEN EP0 has empty strings for all fields
    let ettle = store.get_ettle(&ettle_id).expect("Ettle should exist");
    let ep0 = store.get_ep(&ettle.ep_ids[0]).expect("EP0 should exist");
    assert_eq!(ep0.why, "");
    assert_eq!(ep0.what, "");
    assert_eq!(ep0.how, "");
}
