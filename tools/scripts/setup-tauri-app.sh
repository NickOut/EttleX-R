#!/bin/bash
# File: ~/Development/tools/scripts/setup-tauri-app.sh

set -e

PROJECT_NAME=$1
FRONTEND_FRAMEWORK=${2:-"sveltekit"}  # sveltekit, react, or vanilla

if [ -z "$PROJECT_NAME" ]; then
    echo "Usage: $0 <project-name> [sveltekit|react|vanilla]"
    exit 1
fi

cd ~/Development/rust-workspace/applications

echo "🚀 Creating Tauri application: $PROJECT_NAME"

if [ "$FRONTEND_FRAMEWORK" = "sveltekit" ]; then
    # Create SvelteKit + Tauri project
    pnpm create svelte@latest "$PROJECT_NAME"
    cd "$PROJECT_NAME"

    # Add Tauri
    pnpm add -D @tauri-apps/cli @tauri-apps/api
    pnpm tauri init

    # Configure SvelteKit for Tauri
    cat > vite.config.js << EOF
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [sveltekit()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**"]
    }
  }
});
EOF

elif [ "$FRONTEND_FRAMEWORK" = "react" ]; then
    # Create React + TypeScript + Tauri project
    pnpm create tauri-app --template react-ts "$PROJECT_NAME"
    cd "$PROJECT_NAME"

else
    # Create vanilla Tauri project
    pnpm create tauri-app "$PROJECT_NAME"
    cd "$PROJECT_NAME"
fi

# Add common dependencies to Rust backend
cd src-tauri
cat >> Cargo.toml << EOF

# Additional dependencies for knowledge management
[dependencies.additional]
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "sqlite"] }
qdrant-client = "1.7"
ort = "1.16"  # ONNX Runtime
redis = "0.24"
rdkafka = "0.36"
tracing = "0.1"
tracing-subscriber = "0.3"
anyhow = "1.0"
uuid = { version = "1.0", features = ["v4"] }
EOF

cd ..

# Setup basic project structure
mkdir -p {tests/e2e,docs,scripts}

# Create development scripts
cat > scripts/dev.sh << 'EOF'
#!/bin/bash
echo "🔄 Starting Tauri development mode..."
pnpm tauri dev
EOF

cat > scripts/build.sh << 'EOF'
#!/bin/bash
echo "🏗️ Building Tauri application..."
pnpm tauri build
EOF

chmod +x scripts/*.sh

# Git setup
cat > .gitignore << EOF
node_modules/
/target
/dist
/.svelte-kit
/build
.DS_Store
.env
.env.local
.env.production.local
src-tauri/target/
*.log
EOF

echo "✅ Tauri application '$PROJECT_NAME' created successfully!"
echo "📁 Location: $(pwd)"
echo "🚀 Run: cd $PROJECT_NAME && pnpm install && pnpm tauri dev"