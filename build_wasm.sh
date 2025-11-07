#!/bin/bash
# WASM Build Script for SCXML Core Engine
# Uses Emscripten to compile C++ code to WebAssembly

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}==========================================${NC}"
echo -e "${GREEN}  SCXML Core Engine - WASM Build  ${NC}"
echo -e "${GREEN}==========================================${NC}"
echo ""

# Auto-source emsdk if needed
if ! command -v emcc &> /dev/null; then
    echo -e "${YELLOW}Emscripten not found, attempting to source emsdk...${NC}"

    # Use EMSDK_PATH environment variable or default to ~/emsdk
    EMSDK_PATH="${EMSDK_PATH:-$HOME/emsdk}"
    EMSDK_ENV="$EMSDK_PATH/emsdk_env.sh"

    if [ -f "$EMSDK_ENV" ]; then
        echo -e "${YELLOW}Sourcing: $EMSDK_ENV${NC}"
        source "$EMSDK_ENV"

        # Check again after sourcing
        if ! command -v emcc &> /dev/null; then
            echo -e "${RED}Error: Emscripten still not found after sourcing emsdk!${NC}"
            echo "Please check your emsdk installation at: $EMSDK_PATH"
            exit 1
        fi
        echo -e "${GREEN}Successfully activated Emscripten SDK${NC}"
    else
        echo -e "${RED}Error: emsdk_env.sh not found at: $EMSDK_ENV${NC}"
        echo "Please either:"
        echo "  1. Set EMSDK_PATH environment variable: export EMSDK_PATH=/path/to/emsdk"
        echo "  2. Install emsdk at default location: ~/emsdk"
        echo "  3. Manually source emsdk: source /path/to/emsdk/emsdk_env.sh"
        exit 1
    fi
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
