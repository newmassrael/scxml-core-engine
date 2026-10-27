#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# smoke_bazel_consumer.sh — build and run a Bazel consumer through both of SCE's
# Bazel channels, with Bzlmod and the pinned Bazel (scripts/lib/bazel_pins.sh):
#
#   embed      a freshly packaged embed/ tree, taken as module `sce` with
#              `local_path_override`, as a vendoring consumer does;
#   src_tree   this checkout, through scripts/sce_repo.bzl and `use_repo_rule`.
#
# The consumer is tests/bazel_consumer/app. Bazel 9 is the pin because it is the
# major that refuses what older ones accepted (it removed the native C++ rules),
# and Bazel 7 and 8 accept the same files. Every step's output goes to a log
# that is printed when the step fails.
#
# Exit codes: 0 both channels built and the probe exited 0; 1 a channel failed;
# 3 no `bazel` (bazelisk) on PATH, so nothing could be measured.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCE_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
source "${SCE_ROOT}/scripts/lib/bazel_pins.sh"

if ! command -v bazel >/dev/null 2>&1; then
    echo "smoke_bazel_consumer: no bazel (bazelisk) on PATH — nothing measured" >&2
    exit 3
fi

WORK="$(mktemp -d)"
trap 'rm -rf "${WORK}"' EXIT

run_logged() {
    local what="$1" log="$2"
    shift 2
    if ! "$@" >"${log}" 2>&1; then
        echo "smoke_bazel_consumer: ${what} FAILED; its output:" >&2
        cat "${log}" >&2
        return 1
    fi
}

echo "Packaging embed/ into a scratch tree..."
run_logged "package_embed.sh" "${WORK}/package.log" \
    bash "${SCE_ROOT}/scripts/package_embed.sh" -o "${WORK}/embed" --skip-manifest

make_consumer() {
    local dir="$1"
    mkdir -p "${dir}/app"
    cp "${SCE_ROOT}/tests/bazel_consumer/app/BUILD.bazel" "${SCE_ROOT}/tests/bazel_consumer/app/probe.cpp" \
        "${dir}/app/"
    : > "${dir}/BUILD.bazel"
    echo "${SCE_BAZEL_VERSION}" > "${dir}/.bazelversion"
}

make_consumer "${WORK}/consumer_embed"
cat > "${WORK}/consumer_embed/MODULE.bazel" <<EOF
module(name = "consumer_embed")

bazel_dep(name = "rules_cc", version = "${SCE_BAZEL_RULES_CC_VERSION}")
bazel_dep(name = "sce")

local_path_override(
    module_name = "sce",
    path = "${WORK}/embed",
)
EOF

make_consumer "${WORK}/consumer_src_tree"
cp "${SCE_ROOT}/scripts/sce_repo.bzl" "${WORK}/consumer_src_tree/"
cat > "${WORK}/consumer_src_tree/MODULE.bazel" <<EOF
module(name = "consumer_src_tree")

bazel_dep(name = "rules_cc", version = "${SCE_BAZEL_RULES_CC_VERSION}")

sce_local_repository = use_repo_rule("//:sce_repo.bzl", "sce_local_repository")

sce_local_repository(
    name = "sce",
    path = "${SCE_ROOT}",
)
EOF

failures=0
for consumer in consumer_embed consumer_src_tree; do
    echo "Bazel ${SCE_BAZEL_VERSION}: ${consumer} — build and run //app:probe"
    if ! (cd "${WORK}/${consumer}" && run_logged "${consumer}" "${WORK}/${consumer}.log" bazel run //app:probe); then
        failures=$((failures + 1))
    fi
    (cd "${WORK}/${consumer}" && bazel shutdown >/dev/null 2>&1) || true
done

if [ "${failures}" -ne 0 ]; then
    echo "smoke_bazel_consumer: ${failures} channel(s) failed" >&2
    exit 1
fi
echo "smoke_bazel_consumer: PASS (embed + src_tree, Bazel ${SCE_BAZEL_VERSION})"
