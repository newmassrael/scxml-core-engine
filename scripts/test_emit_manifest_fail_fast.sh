#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
#
# test_emit_manifest_fail_fast.sh — regression TC for emit_embed_manifest.sh.
#
# The bug (caught on GitHub Actions ubuntu-latest, May 2026):
#   emit_embed_manifest.sh recorded clang parse errors in
#   non_self_contained_headers but still consumed the degraded AST that
#   clang emits in that case. Degraded AST replaces unresolved template
#   types (e.g. std::vector<std::string>) with placeholder types (int),
#   producing a manifest that is a function of the parse environment's
#   system include path rather than the embed source tree. On the
#   committer's host (QuickJS in /usr/local/include) the manifest looked
#   clean; on CI (no QuickJS) 13 headers degraded and 7 ghost duplicate
#   symbols appeared, failing verify_embed_manifest.sh.
#
# The fix: emit_embed_manifest.sh now exits non-zero on any clang parse
# error. This TC proves the fail-fast still fires by:
#   1. Building a minimal embed/-shaped scratch tree.
#   2. Planting one header that #include "definitely_does_not_exist.h".
#   3. Running emit_embed_manifest.sh against it.
#   4. Asserting the script exits non-zero AND emits the expected
#      "clang failed to parse standalone header" diagnostic.
#
# If a future refactor silently re-introduces the swallow-errors pattern,
# this TC fails and pre-push Stage 1c blocks the regression.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EMIT="${SCRIPT_DIR}/emit_embed_manifest.sh"

if [[ ! -x "${EMIT}" ]]; then
    echo "FAIL: ${EMIT} not found or not executable" >&2
    exit 1
fi

# Skip silently if clang-19 is not installed — same policy as the script
# under test (which exit 1's on missing clang). The TC's job is to verify
# fail-fast SEMANTICS, not to bootstrap a toolchain.
CLANG="${CLANG:-clang++-19}"
if ! command -v "${CLANG}" >/dev/null 2>&1; then
    echo "SKIP: ${CLANG} not on PATH — TC requires clang for AST dump" >&2
    exit 0
fi

WORK="$(mktemp -d --suffix=.emit-fail-fast-tc)"
trap 'rm -rf "${WORK}"' EXIT

# Minimal embed/-shaped tree. We do NOT mirror the full embed payload;
# emit_embed_manifest.sh only needs include/ and (optionally) third_party/
# include dirs that match the -I flags it hardcodes. Empty third_party
# subdirs would still satisfy the directory probes if any existed; the
# script today probes only INCLUDE_DIR, so we provide that plus the
# nlohmann/pugixml/quickjs include paths the script -I's.
mkdir -p "${WORK}/include"
mkdir -p "${WORK}/third_party/nlohmann_json/include/nlohmann"
mkdir -p "${WORK}/third_party/pugixml/src"
# nlohmann/json.hpp and pugixml.hpp are referenced as -I roots but our
# synthetic header does not include them, so empty dirs are sufficient.

# The probe header: #includes a name that resolves nowhere. clang will
# emit a fatal error and exit non-zero. The fix under test must propagate
# that exit through to the script's overall exit code.
PROBE_HEADER="${WORK}/include/_FailFastProbe.h"
cat > "${PROBE_HEADER}" <<'EOF'
// Synthetic header for test_emit_manifest_fail_fast.sh.
// The include below resolves nowhere; clang exits non-zero.
#pragma once
#include "definitely_does_not_exist_anywhere_ever.h"

namespace SCE {
struct FailFastProbe {
    int x;
};
}
EOF

# Run emit against the scratch tree. Capture stdout + stderr + exit code
# separately so we can assert both the exit semantics and the diagnostic
# content. Use `|| true` to prevent set -e from aborting on the expected
# non-zero exit; we want to inspect $? deliberately.
EMIT_STDOUT="${WORK}/emit.out"
EMIT_STDERR="${WORK}/emit.err"
set +e
"${EMIT}" -d "${WORK}" >"${EMIT_STDOUT}" 2>"${EMIT_STDERR}"
EMIT_RC=$?
set -e

# Assertion 1: non-zero exit. The whole point of the fix.
if [[ "${EMIT_RC}" == "0" ]]; then
    echo "FAIL: emit_embed_manifest.sh exited 0 on broken header" >&2
    echo "      The fail-fast guard regressed. emit must abort on" >&2
    echo "      clang parse error rather than silently degrade." >&2
    echo "----- stdout -----" >&2
    cat "${EMIT_STDOUT}" >&2
    echo "----- stderr -----" >&2
    cat "${EMIT_STDERR}" >&2
    exit 1
fi

# Assertion 2: diagnostic content. We pin a fragment of the error message
# so a future refactor that "fails fast" with a generic error (or a clang
# crash message that omits our guidance) still trips this TC. Pinning the
# whole message would be brittle; pinning the load-bearing phrase is not.
if ! grep -q 'clang failed to parse standalone header' "${EMIT_STDERR}"; then
    echo "FAIL: emit exited non-zero but missing expected diagnostic." >&2
    echo "      Expected substring: 'clang failed to parse standalone header'" >&2
    echo "      The error message is part of the developer-facing fix-path;" >&2
    echo "      losing it forces the dev to read clang's raw output to" >&2
    echo "      figure out what the manifest emitter wanted." >&2
    echo "----- stderr -----" >&2
    cat "${EMIT_STDERR}" >&2
    exit 1
fi

# Assertion 3: manifest was NOT written. Fail-fast must leave no
# half-baked artifact for downstream consumers (e.g. verify_embed_manifest.sh
# would otherwise diff against a partial manifest and report a confusing
# "stale" rather than the actual parse failure upstream).
if [[ -f "${WORK}/MANIFEST.json" ]]; then
    echo "FAIL: emit wrote MANIFEST.json despite parse failure." >&2
    echo "      Fail-fast must abort before the python merger writes" >&2
    echo "      anything to disk." >&2
    exit 1
fi

echo "OK: emit_embed_manifest.sh fail-fast TC passed."
echo "    (exit=${EMIT_RC}, diagnostic present, no manifest written)"
