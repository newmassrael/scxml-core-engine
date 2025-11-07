#!/bin/bash
# TSAN Build Script for Reactive State Machine
# Uses Docker with ThreadSanitizer for race condition detection

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}==========================================${NC}"
echo -e "${GREEN}  Reactive State Machine - TSAN Build  ${NC}"
echo -e "${GREEN}==========================================${NC}"
echo ""

# Get host UID/GID for permission matching
HOST_UID=$(id -u)
HOST_GID=$(id -g)
echo -e "${YELLOW}Host User: UID=$HOST_UID, GID=$HOST_GID${NC}"
echo ""

# Project root directory
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Check if Docker image exists
if ! docker image inspect rsm-tsan-env:latest &> /dev/null; then
    echo -e "${YELLOW}TSAN Docker Image Not Found${NC}"
    echo ""
    echo "Building lightweight Docker image with TSAN..."
    echo ""
    echo "This will:"
    echo "  1. Download Ubuntu 24.04 base image (GCC 13.3 with <format> support)"
    echo "  2. Install build tools and dependencies"
    echo "  3. Configure nscd workarounds for TSAN"
    echo ""
    echo "Estimated time: 5-10 minutes"
    echo "Disk space required: ~1.5GB"
    echo ""

    read -p "Continue with build? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Aborted."
        exit 1
    fi

    echo ""
    echo -e "${GREEN}Starting Docker build...${NC}"
    docker build -f "$PROJECT_ROOT/docker_tsan/Dockerfile.tsan" \
                 -t rsm-tsan-env:latest \
                 "$PROJECT_ROOT/docker_tsan" 2>&1 | tee "$PROJECT_ROOT/docker_tsan/docker-tsan-build.log"

    if [ $? -ne 0 ]; then
        echo ""
        echo -e "${RED}Docker image build failed!${NC}"
        exit 1
    fi

    echo ""
    echo -e "${GREEN}Docker image build complete!${NC}"
    echo ""
fi

# Build directory
BUILD_DIR="build_tsan"
echo -e "${GREEN}Building project with TSAN in Docker...${NC}"
echo -e "${YELLOW}Build directory: $BUILD_DIR${NC}"
echo ""

# Run Docker container with:
# - Host UID/GID for permission matching
# - Project root mounted to /workspace
# - Removed after exit (--rm)
# - SYS_PTRACE capability for gdb/TSAN
docker run --rm \
    --cap-add=SYS_PTRACE \
    --security-opt seccomp=unconfined \
    -e HOST_UID=$HOST_UID \
    -e HOST_GID=$HOST_GID \
    -v "$PROJECT_ROOT:/workspace" \
    -w /workspace \
    rsm-tsan-env:latest \
    /bin/bash -c "
        set -e

        # Create group and user matching host permissions
        groupadd -g \$HOST_GID -o hostgroup 2>/dev/null || true
        useradd -u \$HOST_UID -g \$HOST_GID -o -m hostuser 2>/dev/null || true

        # Switch to host user for build (files will have correct permissions)
        su - hostuser -c '
            set -e  # Exit on error
            cd /workspace

            echo \"Configuring CMake with TSAN (Debug build)...\"
            echo \"\"

            # Remove and recreate build directory
            rm -rf $BUILD_DIR
            mkdir -p $BUILD_DIR
            cd $BUILD_DIR

            # CMake configure (TSAN auto-enabled via IN_DOCKER_TSAN env var)
            cmake -DCMAKE_BUILD_TYPE=Debug ..

            echo \"\"
            echo \"Building with make (using all cores)...\"
            make -j\$(nproc)

            echo \"\"
            echo \"Build complete!\"
            echo \"\"
            echo \"Build artifacts in: $BUILD_DIR\"
            echo \"Run tests with: cd $BUILD_DIR && ctest --output-on-failure\"
            echo \"Or specific test: cd $BUILD_DIR/tests && ./w3c_test_cli 201\"
        '
    "

if [ $? -eq 0 ]; then
    echo ""
    echo -e "${GREEN}==========================================${NC}"
    echo -e "${GREEN}      TSAN Build Completed Successfully      ${NC}"
    echo -e "${GREEN}==========================================${NC}"
    echo ""
    echo -e "${YELLOW}Build directory:${NC} $BUILD_DIR"
    echo -e "${YELLOW}Test executable:${NC} $BUILD_DIR/tests/w3c_test_cli"
    echo ""
    echo -e "${YELLOW}Entering interactive shell in Docker container...${NC}"
    echo -e "${YELLOW}Working directory:${NC} /workspace/$BUILD_DIR/tests"
    echo ""
    echo -e "${YELLOW}Example commands:${NC}"
    echo "  env SPDLOG_LEVEL=off ./w3c_test_cli 144"
    echo "  env SPDLOG_LEVEL=off ./w3c_test_cli 411"
    echo "  cd .. && ctest --output-on-failure"
    echo ""

    # Enter interactive shell in Docker container
    docker run --rm -it \
        --cap-add=SYS_PTRACE \
        --security-opt seccomp=unconfined \
        -v "$PROJECT_ROOT:/workspace" \
        -w /workspace/$BUILD_DIR/tests \
        rsm-tsan-env:latest \
        /bin/bash
else
    echo ""
    echo -e "${RED}==========================================${NC}"
    echo -e "${RED}         TSAN Build Failed         ${NC}"
    echo -e "${RED}==========================================${NC}"
    exit 1
fi
