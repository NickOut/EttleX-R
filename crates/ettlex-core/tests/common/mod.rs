use ettlex_core::{Ep, Ettle, Store};
use uuid::Uuid;

/// Create a new empty Store for testing
#[allow(dead_code)]
pub fn new_store() -> Store {
    Store::new()
}

/// Create a test Ettle with the given title
///
/// Automatically generates a UUID v7 for the ID and creates EP0.
/// This is a simplified helper - for full CRUD testing, use the actual
/// `create_ettle` operation.
#[allow(dead_code)]
pub fn create_test_ettle(store: &mut Store, title: &str) -> String {
    let id = Uuid::now_v7().to_string();
    let ettle = Ettle::new(id.clone(), title.to_string());

    // Insert the ettle (bypassing CRUD operations for test setup)
    store.insert_ettle(ettle);

    // Create EP0 (ordinal 0)
    let ep0_id = Uuid::now_v7().to_string();
    let ep0 = Ep::new(
        ep0_id.clone(),
        id.clone(),
        0,
        true, // normative
        String::new(),
        String::new(),
        String::new(),
    );
    store.insert_ep(ep0);

    // Add EP0 to ettle's ep_ids
    let ettle = store.get_ettle_mut(&id).unwrap();
    ettle.add_ep_id(ep0_id);

    id
}

/// Create a test EP with the given parameters
///
/// This is a simplified helper that bypasses CRUD operations for test setup.
#[allow(dead_code)]
pub fn create_test_ep(
    store: &mut Store,
    ettle_id: &str,
    ordinal: u32,
    normative: bool,
    why: &str,
    what: &str,
    how: &str,
) -> String {
    let id = Uuid::now_v7().to_string();
    let ep = Ep::new(
        id.clone(),
        ettle_id.to_string(),
        ordinal,
        normative,
        why.to_string(),
        what.to_string(),
        how.to_string(),
    );

    store.insert_ep(ep);

    // Add EP ID to ettle's ep_ids
    let ettle = store.get_ettle_mut(ettle_id).unwrap();
    ettle.add_ep_id(id.clone());

    id
}

/// Setup a simple tree: Root -> Mid -> Leaf
///
/// Returns (root_id, mid_id, leaf_id)
#[allow(dead_code)]
pub fn setup_simple_tree(store: &mut Store) -> (String, String, String) {
    let root_id = create_test_ettle(store, "Root");
    let mid_id = create_test_ettle(store, "Mid");
    let leaf_id = create_test_ettle(store, "Leaf");

    // Link Root -> Mid via EP1
    let ep1_id = create_test_ep(store, &root_id, 1, true, "Why Mid", "What Mid", "How Mid");
    let ep1 = store.get_ep_mut(&ep1_id).unwrap();
    ep1.child_ettle_id = Some(mid_id.clone());

    let mid = store.get_ettle_mut(&mid_id).unwrap();
    mid.parent_id = Some(root_id.clone());

    // Link Mid -> Leaf via EP1
    let ep2_id = create_test_ep(store, &mid_id, 1, true, "Why Leaf", "What Leaf", "How Leaf");
    let ep2 = store.get_ep_mut(&ep2_id).unwrap();
    ep2.child_ettle_id = Some(leaf_id.clone());

    let leaf = store.get_ettle_mut(&leaf_id).unwrap();
    leaf.parent_id = Some(mid_id.clone());

    (root_id, mid_id, leaf_id)
}
