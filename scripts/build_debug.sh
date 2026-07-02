#!/bin/bash
# Debug Build Script for SCXML Core Engine
# Development build with debug symbols and assertions

set -e  # Exit on any error

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${GREEN}==========================================${NC}"
echo -e "${GREEN}  SCXML Core Engine - Debug Build  ${NC}"
echo -e "${GREEN}==========================================${NC}"
echo ""

# Get absolute path to project root (parent directory of scripts/)
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Check if we're in the correct directory
if [[ ! -f "$PROJECT_ROOT/CMakeLists.txt" ]]; then
    echo -e "${RED}Error: CMakeLists.txt not found at $PROJECT_ROOT${NC}"
    exit 1
fi
echo -e "${GREEN}✅ Project root: $PROJECT_ROOT${NC}"
echo ""

# Build directory (absolute path)
BUILD_DIR="$PROJECT_ROOT/build_debug"
echo -e "${YELLOW}Build directory: $BUILD_DIR${NC}"
echo -e "${YELLOW}Build type: Debug (with debug symbols and assertions)${NC}"
echo ""

# Remove old build directory for clean build
if [ -d "$BUILD_DIR" ]; then
    echo -e "${YELLOW}Removing old build directory...${NC}"
    rm -rf "$BUILD_DIR"
fi

# System information
echo -e "${BLUE}System Information:${NC}"
echo -e "  Compiler: $(gcc --version | head -n 1)"
echo -e "  Platform: $(uname -sr)"
echo -e "  CPU cores: $(nproc)"
echo ""

# CMake configuration with explicit source and build directories
echo -e "${YELLOW}Configuring with CMake (Debug mode)...${NC}"
cmake -S "$PROJECT_ROOT" -B "$BUILD_DIR" \
    -DCMAKE_BUILD_TYPE=Debug \
    -DBUILD_TESTS=ON \
    -DCMAKE_EXPORT_COMPILE_COMMANDS=ON

echo ""
echo -e "${YELLOW}Building project with $(nproc) parallel jobs...${NC}"
echo ""

# Build with timing
START_TIME=$(date +%s)
cmake --build "$BUILD_DIR" --parallel $(nproc)
END_TIME=$(date +%s)
BUILD_TIME=$((END_TIME - START_TIME))

echo ""
echo -e "${GREEN}==========================================${NC}"
echo -e "${GREEN}   Debug Build Completed Successfully   ${NC}"
echo -e "${GREEN}==========================================${NC}"
echo ""
echo -e "${BLUE}Build Statistics:${NC}"
echo -e "  Build time: ${BUILD_TIME} seconds"
echo -e "  Build directory: $BUILD_DIR"
echo -e "  Debug features: -g -O0, assertions enabled, AOT_DEBUG enabled"
echo ""
echo -e "${YELLOW}Test executables:${NC}"
echo -e "  W3C tests: ./$BUILD_DIR/tests/w3c_test_cli"
echo -e "  All tests: cd $BUILD_DIR && ctest"
echo ""
echo -e "${GREEN}Debug build ready for development!${NC}"
