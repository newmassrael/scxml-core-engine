#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
# SCE_BAZEL_SODIUM_PREFIX, when set, names an installed libsodium (`include/`
# and `lib/`, and `bin/` for a DLL: vcpkg's layout). The src_tree consumer then
# names it to sce_local_repository (`sodium`) and also builds and runs
# //app:cose_probe, through @sce//:sce_host_cose. Unset, the COSE probe is not
# built, which this script says.
#
# It runs where bash does, a Windows machine's Git Bash among them: a path
# written into a MODULE.bazel is given in the form the native Bazel reads.
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

# A path as Bazel reads it. On Windows the shell's /c/... form names nothing to
# the native Bazel, which takes C:/...; cygpath, which Git Bash and MSYS2 carry,
# converts it. Elsewhere the path is already the native one.
native_path() {
    if command -v cygpath >/dev/null 2>&1; then
        cygpath -m "$1"
    else
        printf '%s\n' "$1"
    fi
}

# The BUILD file of a repository over the libsodium installed under $1: a
# cc_import of its library in the form the installation holds it (a DLL with
# its import library, a static library, or a shared object) and a cc_library
# that carries its headers. A static libsodium on Windows needs SODIUM_STATIC
# stated to its headers. Status 1 when $1 holds none of these forms.
sodium_build_file() {
    local prefix="$1" import defines=""
    if [[ -f "${prefix}/bin/libsodium.dll" && -f "${prefix}/lib/libsodium.lib" ]]; then
        import='interface_library = "lib/libsodium.lib", shared_library = "bin/libsodium.dll"'
    elif [[ -f "${prefix}/lib/libsodium.lib" ]]; then
        import='static_library = "lib/libsodium.lib"'
        defines='"SODIUM_STATIC=1"'
    elif [[ -f "${prefix}/lib/libsodium.so" ]]; then
        import='shared_library = "lib/libsodium.so"'
    elif [[ -f "${prefix}/lib/libsodium.a" ]]; then
        import='static_library = "lib/libsodium.a"'
    else
        return 1
    fi
    cat <<EOF
load("@rules_cc//cc:defs.bzl", "cc_import", "cc_library")

cc_import(
    name = "sodium_import",
    ${import},
)

cc_library(
    name = "sodium",
    hdrs = glob(["include/sodium.h", "include/sodium/**/*.h"]),
    includes = ["include"],
    defines = [${defines}],
    deps = [":sodium_import"],
    visibility = ["//visibility:public"],
)
EOF
}

echo "Packaging embed/ into a scratch tree..."
run_logged "package_embed.sh" "${WORK}/package.log" \
    bash "${SCE_ROOT}/scripts/package_embed.sh" -o "${WORK}/embed" --skip-manifest

make_consumer() {
    local dir="$1"
    mkdir -p "${dir}/app"
    cp "${SCE_ROOT}/tests/bazel_consumer/app/BUILD.bazel" "${SCE_ROOT}/tests/bazel_consumer/app/probe.cpp" \
        "${SCE_ROOT}/tests/bazel_consumer/app/cose_probe.cpp" "${dir}/app/"
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
    path = "$(native_path "${WORK}/embed")",
)
EOF

# The libsodium the src_tree consumer names, when one is given.
SODIUM_REPOSITORY=""
SODIUM_ATTRIBUTE=""
if [[ -n "${SCE_BAZEL_SODIUM_PREFIX:-}" ]]; then
    if ! sodium_build="$(sodium_build_file "${SCE_BAZEL_SODIUM_PREFIX}")"; then
        echo "smoke_bazel_consumer: SCE_BAZEL_SODIUM_PREFIX=${SCE_BAZEL_SODIUM_PREFIX} holds no libsodium" \
            "(lib/libsodium.lib, lib/libsodium.so or lib/libsodium.a)" >&2
        exit 1
    fi
    SODIUM_REPOSITORY="
new_local_repository = use_repo_rule(\"@bazel_tools//tools/build_defs/repo:local.bzl\", \"new_local_repository\")

new_local_repository(
    name = \"libsodium\",
    path = \"$(native_path "${SCE_BAZEL_SODIUM_PREFIX}")\",
    build_file_content = \"\"\"
${sodium_build}
\"\"\",
)
"
    SODIUM_ATTRIBUTE='    sodium = "@libsodium//:sodium",'
fi

make_consumer "${WORK}/consumer_src_tree"
cp "${SCE_ROOT}/scripts/sce_repo.bzl" "${WORK}/consumer_src_tree/"
cat > "${WORK}/consumer_src_tree/MODULE.bazel" <<EOF
module(name = "consumer_src_tree")

bazel_dep(name = "rules_cc", version = "${SCE_BAZEL_RULES_CC_VERSION}")
${SODIUM_REPOSITORY}
sce_local_repository = use_repo_rule("//:sce_repo.bzl", "sce_local_repository")

sce_local_repository(
    name = "sce",
    path = "$(native_path "${SCE_ROOT}")",
${SODIUM_ATTRIBUTE}
)
EOF

failures=0
# A Bazel label is handed to Bazel with the shell's rewriting of a native
# program's arguments turned off for that one call: Git Bash and MSYS2 read
# `//app:probe` as a path and pass `/app:probe`, which Bazel refuses as a
# package name. Only this call: the other native programs the script runs
# (the Windows Python of package_embed.sh among them) are handed `/c/...`
# paths and need that rewriting. Elsewhere the two variables name nothing.
run_probe() {
    local consumer="$1" target="$2"
    echo "Bazel ${SCE_BAZEL_VERSION}: ${consumer} — build and run ${target}"
    if ! (cd "${WORK}/${consumer}" &&
        run_logged "${consumer} ${target}" "${WORK}/${consumer}.log" \
            env MSYS_NO_PATHCONV=1 MSYS2_ARG_CONV_EXCL='*' bazel run "${target}"); then
        failures=$((failures + 1))
    fi
}
for consumer in consumer_embed consumer_src_tree; do
    run_probe "${consumer}" //app:probe
    if [[ "${consumer}" == consumer_src_tree ]]; then
        if [[ -n "${SODIUM_ATTRIBUTE}" ]]; then
            run_probe "${consumer}" //app:cose_probe
        else
            echo "Bazel ${SCE_BAZEL_VERSION}: ${consumer} — //app:cose_probe not built: SCE_BAZEL_SODIUM_PREFIX names no libsodium"
        fi
    fi
    (cd "${WORK}/${consumer}" && bazel shutdown >/dev/null 2>&1) || true
done

if [ "${failures}" -ne 0 ]; then
    echo "smoke_bazel_consumer: ${failures} channel(s) failed" >&2
    exit 1
fi
echo "smoke_bazel_consumer: PASS (embed + src_tree, Bazel ${SCE_BAZEL_VERSION})"
