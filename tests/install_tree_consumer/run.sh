#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# run.sh — Drive the install-tree consumer smoke test end to end.
#
# Run by `scripts/gate install-consumer` (install-consumer.yml in CI), which
# configures and builds a scratch umbrella tree first, and by ctest (label
# install-consumer) from an umbrella build configured with
# -DSCE_BUILD_INSTALL_CONSUMER_TEST=ON:
#   1. install the umbrella build to a fresh temp prefix
#   2. configure + build each probe against that prefix via
#      find_package(SCE COMPONENTS <tier>): base, scripting and runtime
#      include each tier's headers; generated generates a statechart with
#      the installed SCECodegen.cmake and sce-codegen and runs it
#   3. clean up the prefix
#
# A step that fails prints its own output, so the cause is in the log; a
# passing step stays quiet.
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

# Run a command with its output captured in $1, and print that output when
# the command fails. `set -e` would otherwise end the script with nothing but
# the exit status to read.
run_logged() {
    local log="$1"
    shift
    if ! "$@" >"${log}" 2>&1; then
        echo "[install-consumer] FAILED: $*" >&2
        tail -n 80 "${log}" >&2
        return 1
    fi
}

run_logged "${PREFIX}/install.log" \
    cmake --install "${BUILD_DIR}" --prefix "${PREFIX}"

for tier in base scripting runtime generated; do
    PROBE_BUILD="${PREFIX}/probe-${tier}"
    echo "[install-consumer] ${tier}"
    run_logged "${PREFIX}/configure-${tier}.log" \
        cmake -S "${SCRIPT_DIR}" -B "${PROBE_BUILD}" \
            -DCMAKE_PREFIX_PATH="${PREFIX}" \
            -DSCE_PROBE_TIER="${tier}" \
            -DSCE_PROBE_PREFIX="${PREFIX}" \
            -DCMAKE_BUILD_TYPE=Release
    run_logged "${PREFIX}/build-${tier}.log" \
        cmake --build "${PROBE_BUILD}" --target "probe_${tier}"
    "${PROBE_BUILD}/probe_${tier}"
done

echo "[install-consumer] PASS (base + scripting + runtime + generated)"
