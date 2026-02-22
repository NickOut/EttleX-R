# ettlex-tauri

**EttleX Tauri Backend - Desktop Application Bridge**

Tauri backend providing command handlers for the EttleX desktop application.

## Status

🚧 **Stub Crate** - Not yet implemented

This crate is a placeholder for future Tauri backend functionality.

## Overview

EttleX Tauri provides the Rust backend for the EttleX desktop application built with Tauri + SvelteKit. It exposes EttleX operations as Tauri commands callable from the frontend.

## Architecture

```
SvelteKit Frontend (apps/ettlex-desktop/)
    ↓ (Tauri IPC)
Tauri Backend (ettlex-tauri)
    ↓
Engine Commands (ettlex-engine)
    ↓
Store/Core Layers
```

## Planned Commands

### Repository Management

- `open_repository(path)` - Open existing repository
- `create_repository(path)` - Initialize new repository
- `get_repository_info()` - Get current repository metadata

### Seed Operations

- `import_seed(path)` - Import seed YAML file
- `list_seeds()` - List imported seeds
- `get_seed_digest(path)` - Compute seed digest without importing

### Tree Navigation

- `get_ettle(id)` - Get Ettle by ID
- `get_ep(id)` - Get EP by ID
- `list_ettles()` - List all Ettles
- `get_children(ettle_id)` - Get child Ettles
- `compute_ept(root_id)` - Compute EPT for root

### Rendering

- `render_ettle(id)` - Render Ettle to Markdown
- `render_bundle(leaf_id, ep_ordinal?)` - Render full bundle

### Snapshots

- `commit_snapshot(root_id, policy_ref, profile_ref)` - Create snapshot
- `list_snapshots()` - List committed snapshots
- `get_snapshot(id)` - Get snapshot by ID
- `diff_snapshots(id1, id2)` - Diff two snapshots

### Validation

- `validate_tree(root_id)` - Validate tree invariants
- `check_cycles(ettle_id)` - Check for cycles

## State Management

Tauri backend maintains application state:

```rust
pub struct AppState {
    pub repo_path: Option<PathBuf>,
    pub conn: Option<rusqlite::Connection>,
    pub cas: Option<FsStore>,
    pub store: Option<Store>,
}
```

State is managed via Tauri's state management:

```rust
use tauri::State;

#[tauri::command]
async fn get_ettle(
    ettle_id: String,
    state: State<'_, AppState>,
) -> Result<Ettle, String> {
    // Access state
    let store = state.store.as_ref()
        .ok_or("No repository open")?;

    // Use engine/store layers
    let ettle = store.get_ettle(&ettle_id)
        .map_err(|e| e.to_string())?;

    Ok(ettle)
}
```

## Frontend Integration (SvelteKit)

### Calling Tauri Commands

```typescript
import { invoke } from '@tauri-apps/api/tauri';

// Import seed
const result = await invoke('import_seed', {
  path: '/path/to/seed.yaml',
});

// Get Ettle
const ettle = await invoke('get_ettle', {
  ettleId: 'ettle:root',
});

// Render bundle
const markdown = await invoke('render_bundle', {
  leafId: 'ettle:snapshot_diff',
  epOrdinal: 0,
});
```

### Event Listeners

```typescript
import { listen } from '@tauri-apps/api/event';

// Listen for operation progress
const unlisten = await listen('operation-progress', (event) => {
  console.log('Progress:', event.payload);
});
```

## Error Handling

Tauri commands use `Result<T, String>` for error handling:

```rust
#[tauri::command]
async fn import_seed(path: String) -> Result<SeedImportResult, String> {
    import_seed_impl(path)
        .map_err(|e| format!("Import failed: {}", e))
}
```

Errors are converted to strings for frontend consumption.

## Testing

Tauri commands include:

- Unit tests for command handlers
- Integration tests with mock state
- Error handling tests
- State management tests

```bash
cargo test -p ettlex-tauri
```

## Command Module Structure

```
ettlex-tauri/
└── src/
    ├── commands/
    │   ├── mod.rs           # Command registry
    │   ├── repository.rs    # Repository commands
    │   ├── seed.rs          # Seed import commands
    │   ├── tree.rs          # Tree navigation commands
    │   ├── render.rs        # Rendering commands
    │   ├── snapshot.rs      # Snapshot commands
    │   └── validation.rs    # Validation commands
    ├── state.rs             # Application state
    └── main.rs              # Tauri app setup
```

## Tauri Configuration

Commands are registered in the Tauri app builder:

```rust
fn main() {
    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            open_repository,
            import_seed,
            get_ettle,
            render_bundle,
            commit_snapshot,
            validate_tree,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

## Desktop App Structure

The desktop app lives in `apps/ettlex-desktop/`:

```
apps/ettlex-desktop/
├── src/              # SvelteKit frontend
├── src-tauri/        # Tauri configuration
│   ├── Cargo.toml    # Depends on ettlex-tauri
│   └── tauri.conf.json
└── package.json
```

## Dependencies (Planned)

- `tauri` - Desktop app framework
- `ettlex-engine` - Command orchestration
- `ettlex-store` - Persistence layer
- `ettlex-core` - Domain models
- `serde` - Serialization for IPC
- `tokio` - Async runtime

## Development Workflow

1. **Backend**: Implement commands in `ettlex-tauri`
2. **Frontend**: Call commands from SvelteKit components
3. **Test**: Unit test commands, integration test UI flows
4. **Build**: `cargo tauri build` for production

## Future Work

- [ ] Implement repository management commands
- [ ] Implement seed import commands
- [ ] Implement tree navigation commands
- [ ] Implement rendering commands
- [ ] Implement snapshot commands
- [ ] Implement validation commands
- [ ] Add real-time progress events
- [ ] Add file system watchers
- [ ] Support drag-and-drop seed import
- [ ] Add system tray integration
