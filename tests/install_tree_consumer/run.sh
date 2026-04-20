#!/usr/bin/env bash
# run.sh — Drive the install-tree consumer smoke test end to end.
#
# Invoked by ctest (label install-consumer) from the umbrella build:
#   1. install the umbrella build to a fresh temp prefix
#   2. configure + build each probe (base, scripting, runtime) against that
#      prefix via find_package(SCE COMPONENTS <tier>)
#   3. clean up the prefix on success
#
# Usage: run.sh <umbrella_source_dir> <umbrella_build_dir>

set -euo pipefail

if [[ $# -ne 2 ]]; then
    echo "usage: $0 <source_dir> <build_dir>" >&2
    exit 2
fi

SOURCE_DIR="$1"
BUILD_DIR="$2"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

PREFIX="$(mktemp -d -t sce-install-consumer.XXXXXX)"
trap 'rm -rf "${PREFIX}"' EXIT

echo "[install-consumer] prefix : ${PREFIX}"
echo "[install-consumer] source : ${SOURCE_DIR}"
echo "[install-consumer] build  : ${BUILD_DIR}"

cmake --install "${BUILD_DIR}" --prefix "${PREFIX}" >/dev/null

for tier in base scripting runtime; do
    PROBE_BUILD="${PREFIX}/probe-${tier}"
    echo "[install-consumer] ${tier}"
    cmake -S "${SCRIPT_DIR}" -B "${PROBE_BUILD}" \
        -DCMAKE_PREFIX_PATH="${PREFIX}" \
        -DSCE_PROBE_TIER="${tier}" \
        -DCMAKE_BUILD_TYPE=Release >/dev/null
    cmake --build "${PROBE_BUILD}" --target "probe_${tier}" >/dev/null
    "${PROBE_BUILD}/probe_${tier}"
done

echo "[install-consumer] PASS (base + scripting + runtime)"
