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

# Get absolute path to project root (parent directory of build_scripts/)
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Check if we're in the correct directory
if [[ ! -f "$PROJECT_ROOT/CMakeLists.txt" ]]; then
    echo -e "${RED}Error: CMakeLists.txt not found at $PROJECT_ROOT${NC}"
    exit 1
fi
echo -e "${GREEN}✅ Project root: $PROJECT_ROOT${NC}"
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

# Build directory (absolute path)
BUILD_DIR="$PROJECT_ROOT/build_wasm"
echo -e "${YELLOW}Build directory: $BUILD_DIR${NC}"
echo ""

if [ -d "$BUILD_DIR" ]; then
    echo -e "${YELLOW}Removing existing WASM build directory...${NC}"
    rm -rf "$BUILD_DIR"
fi

# Start timing
START_TIME=$(date +%s)

echo -e "${GREEN}Configuring CMake with Emscripten...${NC}"
emcmake cmake -S "$PROJECT_ROOT" -B "$BUILD_DIR" \
    -DCMAKE_BUILD_TYPE=Release \
    -DEMSCRIPTEN=ON \
    -DBUILD_TESTS=ON \
    -DBUILD_EXAMPLES=OFF \
    -DBUILD_BENCHMARKS=OFF

echo ""
echo -e "${GREEN}Building SCE for WASM...${NC}"
cmake --build "$BUILD_DIR" --parallel $(nproc)

if [ $? -eq 0 ]; then
    # Calculate build time
    END_TIME=$(date +%s)
    BUILD_TIME=$((END_TIME - START_TIME))

    echo ""
    echo -e "${GREEN}==========================================${NC}"
    echo -e "${GREEN}       WASM Build Completed Successfully       ${NC}"
    echo -e "${GREEN}==========================================${NC}"
    echo ""
    echo -e "${BLUE}Build Statistics:${NC}"
    echo -e "  Build time: ${BUILD_TIME} seconds"
    echo -e "  Build directory: $BUILD_DIR"
    echo ""

    # Visualizer deployment is now handled by CMake target (deploy-visualizer-local)
    # with proper dependency on visualizer build target
    if [ -f "$BUILD_DIR/tests/visualizer.js" ] && [ -f "$BUILD_DIR/tests/visualizer.wasm" ]; then
        echo -e "${GREEN}✓ Visualizer built successfully${NC}"
        echo -e "${YELLOW}  WASM files: $BUILD_DIR/tests/visualizer.{js,wasm}${NC}"
        echo ""
    fi

    echo -e "${YELLOW}Build artifacts:${NC}"
    ls -lh "$BUILD_DIR/sce/libsce_unified.a" 2>/dev/null || ls -lh "$BUILD_DIR/libsce_unified.a" 2>/dev/null || echo "libsce_unified.a not found"
    echo ""
    echo -e "${YELLOW}Test executables:${NC}"
    find "$BUILD_DIR/tests" -maxdepth 1 -type f -executable 2>/dev/null | head -5 || echo "No test executables found"
    echo ""
    echo -e "${YELLOW}Next steps:${NC}"
    echo "1. Run tests with: node <test_executable>.js"
    echo "2. Link libsce_unified.a with your WASM application"
    echo "3. Use emcc to create final .wasm output"
    echo ""
    echo -e "${GREEN}Example link command:${NC}"
    echo "emcc -o sce.js libsce_unified.a -s WASM=1 -s EXPORTED_FUNCTIONS='[...]'"
else
    echo ""
    echo -e "${RED}==========================================${NC}"
    echo -e "${RED}         WASM Build Failed         ${NC}"
    echo -e "${RED}==========================================${NC}"
    exit 1
fi
