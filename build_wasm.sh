#!/bin/bash
# WASM Build Script for Reactive State Machine
# Uses Emscripten to compile C++ code to WebAssembly

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}==========================================${NC}"
echo -e "${GREEN}  Reactive State Machine - WASM Build  ${NC}"
echo -e "${GREEN}==========================================${NC}"
echo ""

# Check if emscripten is installed
if ! command -v emcc &> /dev/null; then
    echo -e "${RED}Error: Emscripten not found!${NC}"
    echo "Please install Emscripten SDK from: https://emscripten.org/docs/getting_started/downloads.html"
    echo "Or activate emsdk: source /path/to/emsdk/emsdk_env.sh"
    exit 1
fi

# Print emscripten version
echo -e "${YELLOW}Emscripten version:${NC}"
emcc --version | head -1
echo ""

# Create WASM build directory
BUILD_DIR="build_wasm"
if [ -d "$BUILD_DIR" ]; then
    echo -e "${YELLOW}Removing existing WASM build directory...${NC}"
    rm -rf "$BUILD_DIR"
fi

mkdir -p "$BUILD_DIR"
cd "$BUILD_DIR"

echo -e "${GREEN}Configuring CMake with Emscripten...${NC}"
emcmake cmake .. \
    -DCMAKE_BUILD_TYPE=Release \
    -DEMSCRIPTEN=ON \
    -DBUILD_TESTS=ON \
    -DBUILD_EXAMPLES=OFF \
    -DBUILD_BENCHMARKS=OFF

echo ""
echo -e "${GREEN}Building RSM for WASM...${NC}"
emmake make -j$(nproc)

if [ $? -eq 0 ]; then
    echo ""
    echo -e "${GREEN}==========================================${NC}"
    echo -e "${GREEN}       WASM Build Completed Successfully       ${NC}"
    echo -e "${GREEN}==========================================${NC}"
    echo ""
    echo -e "${YELLOW}Build artifacts:${NC}"
    ls -lh rsm/librsm_unified.a 2>/dev/null || ls -lh librsm_unified.a
    echo ""
    echo -e "${YELLOW}Test executables:${NC}"
    find tests -maxdepth 1 -type f -executable 2>/dev/null | head -5 || echo "No test executables found"
    echo ""
    echo -e "${YELLOW}Next steps:${NC}"
    echo "1. Run tests with: node <test_executable>.js"
    echo "2. Link librsm_unified.a with your WASM application"
    echo "3. Use emcc to create final .wasm output"
    echo ""
    echo -e "${GREEN}Example link command:${NC}"
    echo "emcc -o rsm.js librsm_unified.a -s WASM=1 -s EXPORTED_FUNCTIONS='[...]'"
else
    echo ""
    echo -e "${RED}==========================================${NC}"
    echo -e "${RED}         WASM Build Failed         ${NC}"
    echo -e "${RED}==========================================${NC}"
    exit 1
fi
