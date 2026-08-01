#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
#
# verify_embed_manifest.sh — drift guard for embed/MANIFEST.json.
#
# Re-emits the manifest in place and compares the API-surface fields
# against the checked-in file (label-only fields embed_version +
# clang_version are stripped before diff — see canon_manifest below).
# A mismatch means a sce/include/ header was edited without
# regenerating the manifest; fail and point at the regeneration
# command. The committed manifest is always restored before exit so
# the working tree stays clean for subsequent CI steps.
#
# On a fresh checkout (CI, or `git clean -fdx`) embed/include/ does
# not exist because /embed/** is gitignored except for MANIFEST.json;
# this script self-bootstraps by invoking package_embed.sh
# --skip-manifest before the emit step.
#
# Intended for CI; also runnable locally before sending a PR that
# touches public headers.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCE_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
EMBED_DIR="${SCE_ROOT}/embed"
INCLUDE_DIR="${EMBED_DIR}/include"
MANIFEST="${EMBED_DIR}/MANIFEST.json"

if [[ ! -f "${MANIFEST}" ]]; then
    echo "ERROR: ${MANIFEST} not found." >&2
    echo "       Run scripts/package_embed.sh (which regenerates the manifest)," >&2
    echo "       then commit the result." >&2
    exit 1
fi

# Snapshot the checked-in manifest, regenerate in place, compare, restore
# on mismatch so CI retry paths do not leave the tree dirty. Snapshot
# MUST happen before the bootstrap below, because package_embed.sh
# starts with `rm -rf "${OUTPUT_DIR}"` and would wipe the committed
# manifest along with the rest of embed/.
SNAPSHOT="$(mktemp --suffix=.json)"
trap 'rm -f "${SNAPSHOT}"' EXIT
cp "${MANIFEST}" "${SNAPSHOT}"

# Bootstrap: /embed/** is gitignored except for MANIFEST.json, so
# embed/include/ is a derived copy of the public headers. Run
# package_embed.sh with --skip-manifest so embed/include/ materialises
# but MANIFEST.json stays absent — the emit step below is then the sole
# writer of the comparison target, preserving this verifier's
# "snapshot vs freshly emitted" semantics.
#
# Unconditionally, not just when the directory is missing. Bootstrapping
# only on a fresh checkout makes this verifier report on whichever header
# snapshot the developer's embed/include/ happens to hold: add a public
# class, and a warm tree regenerates the manifest from the *old* copy, so
# the fresh side matches the committed side and the check passes. CI
# starts from an empty embed/ and therefore sees the real surface — the
# guard passes exactly where drift is introduced and fails only after the
# push. Re-materialising every run costs a couple of minutes and is what
# makes a local pass mean the same thing as a CI pass.
echo "Bootstrapping ${INCLUDE_DIR} via package_embed.sh --skip-manifest..." >&2
"${SCRIPT_DIR}/package_embed.sh" --skip-manifest >/dev/null

"${SCRIPT_DIR}/emit_embed_manifest.sh" >/dev/null

# Canonicalise both sides by stripping label-only fields before comparison:
#   embed_version — HEAD-derived; legitimately lags between regen commits.
#   clang_version — toolchain-derived; differs across machines/runners.
# Mirrors verify_embed_payload.sh's filter (lines 67-85): both verifiers
# share the "API surface vs label" axis and must agree on which fields
# count as drift signal vs noise. Without this, every push from a SHA
# that the committed manifest does not pin would fail spuriously even
# when the header surface is byte-identical.
canon_manifest() {
    python3 -c '
import json, sys
d = json.load(open(sys.argv[1]))
d.pop("embed_version", None)
d.pop("clang_version", None)
json.dump(d, sys.stdout, sort_keys=True, indent=2)
' "$1"
}

CANON_SNAPSHOT="$(mktemp --suffix=.committed.json)"
CANON_FRESH="$(mktemp --suffix=.fresh.json)"
trap 'rm -f "${SNAPSHOT}" "${CANON_SNAPSHOT}" "${CANON_FRESH}"' EXIT
canon_manifest "${SNAPSHOT}" > "${CANON_SNAPSHOT}"
canon_manifest "${MANIFEST}" > "${CANON_FRESH}"

if ! diff -q "${CANON_SNAPSHOT}" "${CANON_FRESH}" >/dev/null 2>&1; then
    echo "ERROR: embed/MANIFEST.json is stale." >&2
    echo "" >&2
    echo "The embed public-header surface has changed since the manifest" >&2
    echo "was last regenerated. Run:" >&2
    echo "" >&2
    echo "    scripts/package_embed.sh   # or scripts/emit_embed_manifest.sh" >&2
    echo "" >&2
    echo "commit the updated embed/MANIFEST.json, and retry." >&2
    echo "" >&2
    echo "--- diff (first 80 lines, excluding embed_version + clang_version) ---" >&2
    diff -u "${CANON_SNAPSHOT}" "${CANON_FRESH}" | head -n 80 >&2 || true
    # Restore the checked-in copy so subsequent CI steps see a clean tree.
    cp "${SNAPSHOT}" "${MANIFEST}"
    exit 1
fi

# Restore the checked-in copy: emit_embed_manifest.sh overwrote it
# with a label-bumped (embed_version/clang_version) variant that is
# API-equivalent but not byte-identical, and downstream CI steps
# (verify_embed_payload.sh) snapshot the committed copy themselves.
cp "${SNAPSHOT}" "${MANIFEST}"

echo "OK: embed/MANIFEST.json is up to date."
