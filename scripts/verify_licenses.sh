#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
#
# verify_licenses.sh — drift guard for sce/sce_licenses.cmake.
#
# LGPL-2.1 §1 (SCE itself) and MIT §1 (vendored third_party deps) both
# require the licence text to ride along with the distribution. The
# SSOT in sce/sce_licenses.cmake is shared by CMakeLists.txt install()
# rules and scripts/package_embed.sh; this verifier closes the drift
# loop those two consumers cannot detect by themselves.
#
# Three invariants — exit 1 on any failure:
#
#   1. SSOT-stale
#      Every entry in sce/sce_licenses.cmake resolves to a real file.
#      Catches "renamed upstream LICENSE without updating SSOT".
#
#   2. LICENSE-orphan
#      Every standalone LICENSE/COPYING file under third_party/<dep>/
#      is referenced by the SSOT. Catches "vendored a new dep with its
#      LICENSE but forgot to extend sce_licenses.cmake" — the silent
#      compliance violation that motivated this script.
#
#   3. dep-uncovered  /  notice-drift
#      Every third_party/<dep>/ that ships source is either covered by
#      a SSOT-listed standalone LICENSE (Invariant 2 territory) or is
#      registered in EMBEDDED_NOTICE_DEPS below with a header path +
#      grep pattern that confirms the upstream SPDX/copyright block is
#      still where it was when the dep was vendored. If upstream moves
#      the notice on a dep upgrade we want a CI failure, not a silent
#      MIT §1 violation.
#
# Intended for CI; runs in well under a second.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCE_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
SSOT="${SCE_ROOT}/sce/sce_licenses.cmake"

if [[ ! -f "${SSOT}" ]]; then
    echo "ERROR: ${SSOT} not found" >&2
    exit 1
fi

# Deps whose MIT/BSD notice rides embedded in source headers rather
# than as a standalone LICENSE file in the upstream tarball.
#
# Format (tilde-separated to allow ":" inside grep patterns):
#   <dep-dir-name>~~<header-path-relative-to-SCE_ROOT>~~<grep -E -i pattern>
#
# Matched case-insensitively. Patterns are deliberately broad enough to
# survive upstream year/version bumps but narrow enough to flag a
# header that lost its copyright block entirely.
EMBEDDED_NOTICE_DEPS=(
    "nlohmann_json~~third_party/nlohmann_json/include/nlohmann/json.hpp~~SPDX-License-Identifier: MIT"
    "lua~~third_party/lua/src/lua.h~~Lua\\.org.*PUC-Rio"
)

# ============================================================================
# Parse SSOT — every set(SCE_LICENSE_FILES_*) block flattens to one
# source-relative path per non-blank, non-comment line.
# ============================================================================
parse_ssot_paths() {
    sed -n '/set(SCE_LICENSE_FILES_/,/^)/{
        s/^[[:space:]]*//
        /^set(/d
        /^)/d
        /^#/d
        /^$/d
        p
    }' "${SSOT}"
}

mapfile -t SSOT_PATHS < <(parse_ssot_paths)

if [[ ${#SSOT_PATHS[@]} -eq 0 ]]; then
    echo "ERROR: parsed zero entries from ${SSOT}" >&2
    exit 1
fi

errors=0

# ============================================================================
# Invariant 1: every SSOT entry resolves to a real file.
# ============================================================================
for f in "${SSOT_PATHS[@]}"; do
    if [[ ! -f "${SCE_ROOT}/${f}" ]]; then
        echo "ERROR [SSOT-stale]: SSOT lists ${f} but file is missing on disk" >&2
        errors=$((errors + 1))
    fi
done

# ============================================================================
# Invariant 2: every standalone LICENSE under third_party/ is in SSOT.
# ============================================================================
declare -A SSOT_SET
for f in "${SSOT_PATHS[@]}"; do
    SSOT_SET["${f}"]=1
done

while IFS= read -r abs; do
    rel="${abs#${SCE_ROOT}/}"
    if [[ -z "${SSOT_SET[${rel}]:-}" ]]; then
        echo "ERROR [LICENSE-orphan]: ${rel} exists on disk but is missing from sce/sce_licenses.cmake" >&2
        errors=$((errors + 1))
    fi
done < <(find "${SCE_ROOT}/third_party" -maxdepth 2 -type f \
    \( -iname "LICENSE" -o -iname "LICENSE.*" \
       -o -iname "COPYING" -o -iname "COPYING.*" \) \
    | LC_ALL=C sort)

# ============================================================================
# Invariant 3a: notice-drift — every embedded-notice header still
# matches its registered pattern.
# ============================================================================
declare -A COVERED_BY_NOTICE
for entry in "${EMBEDDED_NOTICE_DEPS[@]}"; do
    dep="${entry%%~~*}"
    rest="${entry#*~~}"
    header="${rest%%~~*}"
    pattern="${rest#*~~}"
    COVERED_BY_NOTICE["${dep}"]=1
    if [[ ! -f "${SCE_ROOT}/${header}" ]]; then
        echo "ERROR [notice-stale]: ${dep} embedded-notice header ${header} is missing" >&2
        errors=$((errors + 1))
        continue
    fi
    if ! grep -i -E -q "${pattern}" "${SCE_ROOT}/${header}"; then
        echo "ERROR [notice-drift]: ${dep} embedded notice no longer matches /${pattern}/i in ${header}" >&2
        echo "                        (upstream may have moved or removed the SPDX/copyright block)" >&2
        errors=$((errors + 1))
    fi
done

# ============================================================================
# Invariant 3b: dep-uncovered — every third_party/<dep>/ ships under
# either a SSOT LICENSE entry or an embedded-notice registration.
# ============================================================================
declare -A COVERED_BY_FILE
for f in "${SSOT_PATHS[@]}"; do
    if [[ "${f}" == third_party/*/* ]]; then
        dep_path="${f#third_party/}"
        dep="${dep_path%%/*}"
        COVERED_BY_FILE["${dep}"]=1
    fi
done

while IFS= read -r abs; do
    dep="$(basename "${abs}")"
    if [[ -n "${COVERED_BY_FILE[${dep}]:-}" || -n "${COVERED_BY_NOTICE[${dep}]:-}" ]]; then
        continue
    fi
    echo "ERROR [dep-uncovered]: third_party/${dep}/ ships in the build but has no SSOT entry and no embedded-notice registration" >&2
    echo "                       Add a LICENSE path to sce/sce_licenses.cmake," >&2
    echo "                       or register an EMBEDDED_NOTICE_DEPS tuple in $(basename "${BASH_SOURCE[0]}")." >&2
    errors=$((errors + 1))
done < <(find "${SCE_ROOT}/third_party" -mindepth 1 -maxdepth 1 -type d \
    | LC_ALL=C sort)

# ============================================================================
if (( errors > 0 )); then
    echo "" >&2
    echo "License drift detected (${errors} error(s)). See sce/sce_licenses.cmake header for the SSOT pattern." >&2
    exit 1
fi

echo "OK: ${#SSOT_PATHS[@]} licence entries, ${#EMBEDDED_NOTICE_DEPS[@]} embedded-notice entries verified."
