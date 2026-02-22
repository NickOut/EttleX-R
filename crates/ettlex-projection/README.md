# ettlex-projection

**EttleX Projection - Multi-Format Export**

Export EttleX semantic architectures to multiple output formats.

## Status

🚧 **Stub Crate** - Not yet implemented

This crate is a placeholder for future projection/export functionality.

## Overview

EttleX Projection will provide format-specific exporters for transforming EttleX semantic models into various output formats for consumption by external tools and stakeholders.

## Planned Modules

### `json` - JSON Export

Structured JSON export for programmatic consumption.

**Planned outputs**:

- Full tree export with all Ettles and EPs
- Single Ettle export with metadata
- EPT export (ordered list of EP IDs)
- Snapshot manifest export

**Example output**:

```json
{
  "schema_version": 1,
  "root_ettle_id": "ettle:root",
  "ettles": [
    {
      "id": "ettle:root",
      "title": "EttleX Product",
      "eps": [
        {
          "id": "ep:root:0",
          "ordinal": 0,
          "normative": true,
          "why": "...",
          "what": "...",
          "how": "..."
        }
      ]
    }
  ],
  "links": [
    {
      "parent_ep": "ep:root:1",
      "child_ettle": "ettle:store"
    }
  ]
}
```

### `markdown` - Markdown Export

Human-readable documentation export.

**Current implementation**: Basic Markdown rendering exists in `ettlex-core/src/render/`

**Planned enhancements**:

- Template-based rendering
- Custom CSS/styling
- Multi-page documentation sites
- Navigation generation
- Cross-reference linking

### `archimate` - ArchiMate Export

Export to ArchiMate XML for enterprise architecture tools.

**Planned mappings**:

- Ettle → ArchiMate Element
- EP → ArchiMate Relationship
- Refinement tree → Layered view

**Target tools**:

- Archi (open source)
- Enterprise Architect
- BiZZdesign Enterprise Studio

## Usage (Planned)

```rust
use ettlex_projection::{json, markdown, archimate};
use ettlex_core::ops::store::Store;

// Load tree from database
let store = load_tree(&conn)?;

// Export to JSON
let json_output = json::export_tree(&store, "ettle:root")?;
std::fs::write("output.json", json_output)?;

// Export to Markdown
let md_output = markdown::export_bundle(&store, "ettle:leaf")?;
std::fs::write("output.md", md_output)?;

// Export to ArchiMate
let archimate_xml = archimate::export_model(&store, "ettle:root")?;
std::fs::write("model.archimate", archimate_xml)?;
```

## Dependencies (Planned)

- `ettlex-core` - Domain models
- `serde` / `serde_json` - JSON serialization
- `askama` or `tera` - Template engine for Markdown
- `quick-xml` - ArchiMate XML generation

## Future Work

- [ ] Implement JSON exporter
- [ ] Enhance Markdown exporter with templates
- [ ] Implement ArchiMate XML exporter
- [ ] Add Mermaid diagram generation
- [ ] Add PlantUML export
- [ ] Support custom templates
- [ ] Add filtering/projection options
