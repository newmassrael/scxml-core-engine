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

# Layout and routing
cp tools/web/constraint-solver.js "$DEST_DIR/"
cp tools/web/constraint-solver-worker.js "$DEST_DIR/"
cp tools/web/edge-direction-utils.js "$DEST_DIR/"
cp tools/web/routing-state.js "$DEST_DIR/"

# Visualizer manager
cp tools/web/visualizer-manager.js "$DEST_DIR/"

# Transition Layout Optimizer Modules
mkdir -p "$DEST_DIR/optimizer"
cp tools/web/optimizer/snap-calculator.js "$DEST_DIR/optimizer/"
cp tools/web/optimizer/path-utils.js "$DEST_DIR/optimizer/"
cp tools/web/optimizer/csp-solver.js "$DEST_DIR/optimizer/"
cp tools/web/optimizer/optimizer-core.js "$DEST_DIR/optimizer/"

# SCXML Visualizer Modules
mkdir -p "$DEST_DIR/visualizer"
cp tools/web/visualizer/node-builder.js "$DEST_DIR/visualizer/"
cp tools/web/visualizer/link-builder.js "$DEST_DIR/visualizer/"
cp tools/web/visualizer/layout-manager.js "$DEST_DIR/visualizer/"
cp tools/web/visualizer/path-calculator.js "$DEST_DIR/visualizer/"
cp tools/web/visualizer/renderer.js "$DEST_DIR/visualizer/"
cp tools/web/visualizer/interaction-handler.js "$DEST_DIR/visualizer/"
cp tools/web/visualizer/visualizer-core.js "$DEST_DIR/visualizer/"

# Execution Controller Modules
mkdir -p "$DEST_DIR/controller"
cp tools/web/controller/formatters.js "$DEST_DIR/controller/"
cp tools/web/controller/ui-updater.js "$DEST_DIR/controller/"
cp tools/web/controller/control-handler.js "$DEST_DIR/controller/"
cp tools/web/controller/breadcrumb-manager.js "$DEST_DIR/controller/"
cp tools/web/controller/controller-core.js "$DEST_DIR/controller/"

# Main entry point
cp tools/web/main.js "$DEST_DIR/"

# External libraries
cp tools/web/d3.v7.min.js "$DEST_DIR/"

# Configuration and data
cp tools/web/spec_references.json "$DEST_DIR/"

echo "Visualizer files deployed successfully to $DEST_DIR"

# List deployed JavaScript files for verification
echo ""
echo "Deployed JavaScript modules:"
echo "  Core files:"
ls -1 "$DEST_DIR"/*.js 2>/dev/null | sed 's/.*\//    - /' || echo "    (none)"
echo "  Optimizer modules:"
ls -1 "$DEST_DIR/optimizer"/*.js 2>/dev/null | sed 's/.*\//    - /' || echo "    (none)"
echo "  Visualizer modules:"
ls -1 "$DEST_DIR/visualizer"/*.js 2>/dev/null | sed 's/.*\//    - /' || echo "    (none)"
echo "  Controller modules:"
ls -1 "$DEST_DIR/controller"/*.js 2>/dev/null | sed 's/.*\//    - /' || echo "    (none)"
