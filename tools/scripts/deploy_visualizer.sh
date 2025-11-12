#!/bin/bash
# SCXML Visualizer Deployment Script
# Single Source of Truth for visualizer file deployment
#
# Usage:
#   ./deploy_visualizer.sh <destination_dir> <wasm_file_path> <js_file_path>
#
# Arguments:
#   destination_dir: Target directory for deployment (e.g., web_dist, gh-pages/visualizer)
#   wasm_file_path: Path to visualizer.wasm (e.g., tools/web/visualizer.wasm)
#   js_file_path: Path to visualizer.js (e.g., tools/web/visualizer.js)

set -e  # Exit on error

if [ $# -ne 3 ]; then
    echo "Error: Invalid number of arguments"
    echo "Usage: $0 <destination_dir> <wasm_file_path> <js_file_path>"
    exit 1
fi

DEST_DIR="$1"
WASM_FILE="$2"
JS_FILE="$3"

echo "Deploying SCXML Visualizer..."
echo "  Destination: $DEST_DIR"
echo "  WASM: $WASM_FILE"
echo "  JS: $JS_FILE"

# Create destination directory
mkdir -p "$DEST_DIR"

# Copy WASM files (compiled outputs)
cp "$WASM_FILE" "$DEST_DIR/"
cp "$JS_FILE" "$DEST_DIR/"

# Copy HTML entry point
cp tools/web/visualizer.html "$DEST_DIR/"

# Copy CSS files
cp tools/web/visualizer.css "$DEST_DIR/"
cp tools/web/primer.css "$DEST_DIR/"

# Copy JavaScript modules (order matters for documentation clarity)
# Shared utilities (DRY Principle)
cp tools/web/utils.js "$DEST_DIR/"

# Core visualizer
cp tools/web/scxml-visualizer.js "$DEST_DIR/"

# Layout and routing
cp tools/web/constraint-solver.js "$DEST_DIR/"
cp tools/web/constraint-solver-worker.js "$DEST_DIR/"
cp tools/web/transition-layout-optimizer.js "$DEST_DIR/"
cp tools/web/edge-direction-utils.js "$DEST_DIR/"
cp tools/web/routing-state.js "$DEST_DIR/"

# Controllers and managers
cp tools/web/execution-controller.js "$DEST_DIR/"
cp tools/web/visualizer-manager.js "$DEST_DIR/"

# Main entry point
cp tools/web/main.js "$DEST_DIR/"

# External libraries
cp tools/web/d3.v7.min.js "$DEST_DIR/"

# Configuration and data
cp tools/web/spec_references.json "$DEST_DIR/"

echo "✅ Visualizer files deployed to $DEST_DIR"

# List deployed JavaScript files for verification
echo ""
echo "Deployed JavaScript modules:"
ls -1 "$DEST_DIR"/*.js | sed 's/.*\//  - /'
