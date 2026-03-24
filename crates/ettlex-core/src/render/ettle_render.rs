//! Ettle render — simplified for Slice 03 (EP construct removed).

use crate::errors::Result;
use crate::ops::Store;

/// Render an Ettle to Markdown
///
/// Generates a simple Markdown representation of an Ettle title.
/// EP-based rendering has been retired along with the EP construct.
///
/// # Arguments
/// * `store` - Reference to the Store
/// * `ettle_id` - ID of the Ettle to render
///
/// # Returns
/// Markdown string representation
///
/// # Errors
/// * `NotFound` - If Ettle doesn't exist
pub fn render_ettle(store: &Store, ettle_id: &str) -> Result<String> {
    let ettle = store.get_ettle(ettle_id)?;

    let mut output = String::new();

    // Title
    output.push_str(&format!("# {}\n\n", ettle.title));
    output.push_str("*(EP content retired in Slice 03 — use relations for structural queries)*\n");

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Ettle;

    #[test]
    fn test_render_ettle_basic() {
        let mut store = Store::new();
        let ettle = Ettle::new("ettle-1".to_string(), "Test Ettle".to_string());
        store.insert_ettle(ettle);

        let output = render_ettle(&store, "ettle-1").unwrap();
        assert!(output.contains("# Test Ettle"));
    }
}
