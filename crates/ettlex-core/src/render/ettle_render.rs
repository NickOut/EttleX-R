use crate::errors::Result;
use crate::ops::{active_eps, Store};

/// Render an Ettle to Markdown
///
/// Generates a Markdown representation of an Ettle, including:
/// - Title as H1
/// - All EPs in ordinal order (ascending)
/// - Each EP with WHY/WHAT/HOW sections
///
/// # Arguments
/// * `store` - Reference to the Store
/// * `ettle_id` - ID of the Ettle to render
///
/// # Returns
/// Markdown string representation
///
/// # Errors
/// * `EttleNotFound` - If Ettle doesn't exist
/// * `EttleDeleted` - If Ettle was deleted
pub fn render_ettle(store: &Store, ettle_id: &str) -> Result<String> {
    let ettle = store.get_ettle(ettle_id)?;

    let mut output = String::new();

    // Title
    output.push_str(&format!("# {}\n\n", ettle.title));

    // Get active EPs (already sorted by ordinal)
    let eps = active_eps(store, ettle)?;

    // Render each EP
    for ep in eps {
        output.push_str(&format!("## EP {}\n\n", ep.ordinal));

        if ep.normative {
            output.push_str("**Normative**: Yes\n\n");
        } else {
            output.push_str("**Normative**: No\n\n");
        }

        if !ep.why.is_empty() {
            output.push_str(&format!("**WHY**: {}\n\n", ep.why));
        }

        if !ep.what.is_empty() {
            output.push_str(&format!("**WHAT**: {}\n\n", ep.what));
        }

        if !ep.how.is_empty() {
            output.push_str(&format!("**HOW**: {}\n\n", ep.how));
        }

        if let Some(ref child_id) = ep.child_ettle_id {
            if let Ok(child) = store.get_ettle(child_id) {
                output.push_str(&format!("**Child**: {}\n\n", child.title));
            }
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Ep, Ettle};

    #[test]
    fn test_render_ettle_basic() {
        let mut store = Store::new();

        let mut ettle = Ettle::new("ettle-1".to_string(), "Test Ettle".to_string());
        let ep0 = Ep::new(
            "ep0".to_string(),
            "ettle-1".to_string(),
            0,
            true,
            "Why text".to_string(),
            "What text".to_string(),
            "How text".to_string(),
        );

        ettle.add_ep_id("ep0".to_string());
        store.insert_ettle(ettle);
        store.insert_ep(ep0);

        let output = render_ettle(&store, "ettle-1").unwrap();

        assert!(output.contains("# Test Ettle"));
        assert!(output.contains("## EP 0"));
        assert!(output.contains("Why text"));
        assert!(output.contains("What text"));
        assert!(output.contains("How text"));
    }
}
