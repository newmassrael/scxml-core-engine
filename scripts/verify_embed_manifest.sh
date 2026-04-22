#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
#
# verify_embed_manifest.sh — drift guard for embed/MANIFEST.json.
#
# Re-emits the manifest into a temporary location and byte-compares it
# against the checked-in file. A mismatch means a sce/include/ header
# was edited without regenerating the manifest — fail and point at the
# regeneration command.
#
# Intended for CI; also runnable locally before sending a PR that
# touches public headers.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCE_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
EMBED_DIR="${SCE_ROOT}/embed"
MANIFEST="${EMBED_DIR}/MANIFEST.json"

if [[ ! -f "${MANIFEST}" ]]; then
    echo "ERROR: ${MANIFEST} not found." >&2
    echo "       Run scripts/package_embed.sh (which regenerates the manifest)," >&2
    echo "       then commit the result." >&2
    exit 1
fi

# Snapshot the checked-in manifest, regenerate in place, compare, restore
# on mismatch so CI retry paths do not leave the tree dirty.
SNAPSHOT="$(mktemp --suffix=.json)"
trap 'rm -f "${SNAPSHOT}"' EXIT
cp "${MANIFEST}" "${SNAPSHOT}"

"${SCRIPT_DIR}/emit_embed_manifest.sh" >/dev/null

if ! diff -q "${SNAPSHOT}" "${MANIFEST}" >/dev/null 2>&1; then
    echo "ERROR: embed/MANIFEST.json is stale." >&2
    echo "" >&2
    echo "The embed public-header surface has changed since the manifest" >&2
    echo "was last regenerated. Run:" >&2
    echo "" >&2
    echo "    scripts/package_embed.sh   # or scripts/emit_embed_manifest.sh" >&2
    echo "" >&2
    echo "commit the updated embed/MANIFEST.json, and retry." >&2
    echo "" >&2
    echo "--- diff (first 80 lines) ---" >&2
    diff -u "${SNAPSHOT}" "${MANIFEST}" | head -n 80 >&2 || true
    # Restore the checked-in copy so subsequent CI steps see a clean tree.
    cp "${SNAPSHOT}" "${MANIFEST}"
    exit 1
fi

echo "OK: embed/MANIFEST.json is up to date."
