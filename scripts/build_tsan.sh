#!/bin/bash
# ThreadSanitizer build for SCXML Core Engine — host-native, no container.
#
# TSAN is a compiler instrumentation (`-fsanitize=thread`); it needs only a
# TSAN-capable gcc/clang, which every supported toolchain provides. The tests
# skip the HTTP paths that are incompatible with TSAN via
# SCE::Test::Utils::isThreadSanitizerBuild() (keyed on the compiler's TSAN
# macro), so no environment flag is required.

set -euo pipefail

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_DIR="$PROJECT_ROOT/build_tsan"

echo -e "${GREEN}=== SCXML Core Engine — ThreadSanitizer build (native) ===${NC}"

# glibc's nscd DNS cache has had TSAN-hostile TLS races (glibc #16743). It is
# absent by default on modern distros; warn only if it is actually running.
if pgrep -x nscd >/dev/null 2>&1; then
    echo -e "${YELLOW}Warning: nscd is running. If TSAN reports TLS races in getaddrinfo,${NC}"
    echo -e "${YELLOW}  disable it (sudo systemctl stop nscd) or force files-based resolution${NC}"
    echo -e "${YELLOW}  in /etc/nsswitch.conf (hosts: files dns).${NC}"
fi

# ignore_noninstrumented_modules silences reports inside uninstrumented system
# libraries; halt_on_error stops at the first race for an actionable trace.
export TSAN_OPTIONS="${TSAN_OPTIONS:-ignore_noninstrumented_modules=1:halt_on_error=1}"
echo -e "${YELLOW}TSAN_OPTIONS=${TSAN_OPTIONS}${NC}"

rm -rf "$BUILD_DIR"
cmake -S "$PROJECT_ROOT" -B "$BUILD_DIR" -DCMAKE_BUILD_TYPE=Debug -DENABLE_TSAN=ON
cmake --build "$BUILD_DIR" -j"$(nproc)"

echo -e "${GREEN}=== TSAN build complete ===${NC}"
echo "Build directory: $BUILD_DIR"
echo "Run a single test:  (cd $BUILD_DIR/tests && env SPDLOG_LEVEL=off ./w3c_test_cli 144)"
echo "Run the suite:      (cd $BUILD_DIR && ctest --output-on-failure)"
echo "Note: HTTP (BasicHTTP) W3C tests are skipped under TSAN by design."
