use crate::errors::Result;
use crate::ops::Store;
use crate::traversal::ept::compute_ept;

/// Render a leaf bundle to Markdown
///
/// Generates a complete Markdown bundle by computing the EPT from root to leaf,
/// then aggregating the WHY/WHAT/HOW content from all EPs in the path.
///
/// The output includes:
/// - Title showing the full RT path (Root > Mid > Leaf)
/// - Aggregated WHY sections from all EPs
/// - Aggregated WHAT sections from all EPs
/// - Aggregated HOW sections from all EPs
///
/// # Arguments
/// * `store` - Reference to the Store
/// * `leaf_id` - ID of the leaf Ettle
/// * `leaf_ep_ordinal` - Optional ordinal of the leaf EP to end with
///
/// # Returns
/// Markdown string representation of the bundle
///
/// # Errors
/// * `EttleNotFound` - If leaf doesn't exist
/// * `EptMissingMapping` / `EptDuplicateMapping` - EPT computation errors
/// * `EptAmbiguousLeafEp` - If leaf has multiple EPs and ordinal not specified
pub fn render_leaf_bundle(
    store: &Store,
    leaf_id: &str,
    leaf_ep_ordinal: Option<u32>,
) -> Result<String> {
    // Compute EPT
    let ept = compute_ept(store, leaf_id, leaf_ep_ordinal)?;

    let mut output = String::new();

    // Compute RT for title path
    let rt = crate::traversal::rt::compute_rt(store, leaf_id)?;
    let path: Vec<String> = rt
        .iter()
        .filter_map(|ettle_id| store.get_ettle(ettle_id).ok().map(|e| e.title.clone()))
        .collect();

    // Title
    output.push_str(&format!("# Leaf Bundle: {}\n\n", path.join(" > ")));

    // Aggregate WHY sections
    let mut why_sections = Vec::new();
    for ep_id in &ept {
        if let Ok(ep) = store.get_ep(ep_id) {
            if !ep.why.is_empty() {
                why_sections.push(ep.why.clone());
            }
        }
    }

    if !why_sections.is_empty() {
        output.push_str("## WHY (Rationale)\n\n");
        for (i, why) in why_sections.iter().enumerate() {
            output.push_str(&format!("{}. {}\n", i + 1, why));
        }
        output.push('\n');
    }

    // Aggregate WHAT sections
    let mut what_sections = Vec::new();
    for ep_id in &ept {
        if let Ok(ep) = store.get_ep(ep_id) {
            if !ep.what.is_empty() {
                what_sections.push(ep.what.clone());
            }
        }
    }

    if !what_sections.is_empty() {
        output.push_str("## WHAT (Description)\n\n");
        for (i, what) in what_sections.iter().enumerate() {
            output.push_str(&format!("{}. {}\n", i + 1, what));
        }
        output.push('\n');
    }

    // Aggregate HOW sections
    let mut how_sections = Vec::new();
    for ep_id in &ept {
        if let Ok(ep) = store.get_ep(ep_id) {
            if !ep.how.is_empty() {
                how_sections.push(ep.how.clone());
            }
        }
    }

    if !how_sections.is_empty() {
        output.push_str("## HOW (Implementation)\n\n");
        for (i, how) in how_sections.iter().enumerate() {
            output.push_str(&format!("{}. {}\n", i + 1, how));
        }
        output.push('\n');
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Ep, Ettle};

    #[test]
    fn test_render_leaf_bundle_single_ettle() {
        let mut store = Store::new();

        let mut ettle = Ettle::new("root".to_string(), "Root".to_string());
        let ep0 = Ep::new(
            "ep0".to_string(),
            "root".to_string(),
            0,
            true,
            "Why 0".to_string(),
            "What 0".to_string(),
            "How 0".to_string(),
        );

        ettle.add_ep_id("ep0".to_string());
        store.insert_ettle(ettle);
        store.insert_ep(ep0);

        let output = render_leaf_bundle(&store, "root", None).unwrap();

        assert!(output.contains("# Leaf Bundle: Root"));
        assert!(output.contains("Why 0"));
        assert!(output.contains("What 0"));
        assert!(output.contains("How 0"));
    }
}
