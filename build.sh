#!/bin/bash
# SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
#
# Main Build Script for SCXML Core Engine
# Provides interactive menu or command-line selection for different build types

set -e  # Exit on any error

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo -e "${GREEN}==========================================${NC}"
echo -e "${GREEN}   SCXML Core Engine - Build Menu   ${NC}"
echo -e "${GREEN}==========================================${NC}"
echo ""

# Function to show usage
show_usage() {
    echo -e "${CYAN}Usage:${NC}"
    echo "  $0 [build_type]"
    echo ""
    echo -e "${CYAN}Build types:${NC}"
    echo "  debug      - Debug build with symbols and assertions"
    echo "  release    - Release build with optimizations"
    echo "  tsan       - ThreadSanitizer build in Docker"
    echo "  wasm       - WebAssembly build"
    echo "  all        - Run all builds sequentially"
    echo ""
    echo -e "${CYAN}Examples:${NC}"
    echo "  $0           # Interactive menu"
    echo "  $0 debug     # Direct debug build"
    echo "  $0 release   # Direct release build"
    echo "  $0 all       # Build everything"
    echo ""
}

# Function to run a build script
run_build() {
    local build_type=$1
    local script_name=$2
    local script_path="$SCRIPT_DIR/build_scripts/$script_name"

    echo ""
    echo -e "${YELLOW}========================================${NC}"
    echo -e "${YELLOW}Starting $build_type build...${NC}"
    echo -e "${YELLOW}========================================${NC}"
    echo ""

    if [ -f "$script_path" ]; then
        if "$script_path"; then
            echo ""
            echo -e "${GREEN}✅ $build_type build completed!${NC}"
            return 0
        else
            echo ""
            echo -e "${RED}❌ $build_type build failed!${NC}"
            return 1
        fi
    else
        echo -e "${RED}Error: $script_name not found at $script_path${NC}"
        return 1
    fi
}

# Function to run all builds sequentially
run_all_builds() {
    local failed=0
    local total=4

    echo -e "${BLUE}Running all builds...${NC}"
    echo ""

    run_build "Debug" "build_debug.sh" || ((failed++))
    run_build "Release" "build_release.sh" || ((failed++))
    run_build "TSAN" "build_tsan.sh" || ((failed++))
    run_build "WASM" "build_wasm.sh" || ((failed++))

    echo ""
    echo -e "${BLUE}==========================================${NC}"
    if [ $failed -eq 0 ]; then
        echo -e "${GREEN}   All builds completed successfully!   ${NC}"
        echo -e "${BLUE}==========================================${NC}"
        return 0
    else
        local succeeded=$((total - failed))
        echo -e "${YELLOW}   Build Summary: $succeeded/$total succeeded   ${NC}"
        echo -e "${BLUE}==========================================${NC}"
        return 1
    fi
}

# Function to show interactive menu
show_menu() {
    echo -e "${CYAN}Select build type:${NC}"
    echo ""
    echo "  1) Debug      - Development build (build_debug/)"
    echo "  2) Release    - Production build (build_release/)"
    echo "  3) TSAN       - ThreadSanitizer build (build_tsan/)"
    echo "  4) WASM       - WebAssembly build (build_wasm/)"
    echo "  5) All        - Run all builds"
    echo "  6) Help       - Show usage information"
    echo "  q) Quit"
    echo ""
    read -p "Enter selection [1-6, q]: " choice

    case $choice in
        1)
            run_build "Debug" "build_debug.sh"
            ;;
        2)
            run_build "Release" "build_release.sh"
            ;;
        3)
            run_build "TSAN" "build_tsan.sh"
            ;;
        4)
            run_build "WASM" "build_wasm.sh"
            ;;
        5)
            run_all_builds
            ;;
        6)
            echo ""
            show_usage
            ;;
        q|Q)
            echo ""
            echo "Exiting..."
            exit 0
            ;;
        *)
            echo ""
            echo -e "${RED}Invalid selection. Please try again.${NC}"
            echo ""
            show_menu
            ;;
    esac
}

# Main logic
if [ $# -eq 0 ]; then
    # No arguments - show interactive menu
    show_menu
else
    # Command line argument provided
    BUILD_TYPE=$1

    case $BUILD_TYPE in
        debug)
            run_build "Debug" "build_debug.sh"
            ;;
        release)
            run_build "Release" "build_release.sh"
            ;;
        tsan)
            run_build "TSAN" "build_tsan.sh"
            ;;
        wasm)
            run_build "WASM" "build_wasm.sh"
            ;;
        all)
            run_all_builds
            exit $?
            ;;
        help|--help|-h)
            show_usage
            ;;
        *)
            echo -e "${RED}Error: Unknown build type '$BUILD_TYPE'${NC}"
            echo ""
            show_usage
            exit 1
            ;;
    esac
fi
