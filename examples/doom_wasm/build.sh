#!/bin/bash
# SPDX-License-Identifier: MIT
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
#
# DOOM WASM with SCE State Machines - Build Script
#
# This script builds DOOM with SCE state machine integration
# using CMake and Emscripten.
#
# Prerequisites:
#   - Emscripten SDK (emsdk) installed and activated
#   - CMake 3.16+
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BUILD_DIR="$SCRIPT_DIR/build"
PROJECT_ROOT="$SCRIPT_DIR/../.."

echo "============================================"
echo "  DOOM WASM + SCE State Machines Builder"
echo "============================================"
echo ""

# Check for Emscripten
if ! command -v emcc &> /dev/null; then
    echo "Error: Emscripten (emcc) not found in PATH"
    echo "Please install and activate emsdk first:"
    echo "  source /path/to/emsdk/emsdk_env.sh"
    exit 1
fi

echo "Using Emscripten: $(emcc --version | head -1)"
echo ""

# Create build directory
mkdir -p "$BUILD_DIR"
cd "$BUILD_DIR"

# Configure with CMake
echo "Configuring with CMake..."
emcmake cmake "$SCRIPT_DIR" \
    -DCMAKE_BUILD_TYPE=Release \
    -DSCE_ROOT="$PROJECT_ROOT"

# Build
echo "Building..."
emmake make -j$(nproc)

echo ""
echo "============================================"
echo "  Build Complete!"
echo "============================================"
echo ""
echo "Output files:"
echo "  $BUILD_DIR/doom_sce.html"
echo "  $BUILD_DIR/doom_sce.js"
echo "  $BUILD_DIR/doom_sce.wasm"
echo "  $BUILD_DIR/doom_sce.data"
echo "  $BUILD_DIR/index.html"
echo ""
echo "To run locally:"
echo "  cd $PROJECT_ROOT && ./start-server.sh"
echo "  Open http://localhost:8000/visualizer/doom/"
