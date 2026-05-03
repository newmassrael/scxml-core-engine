#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
#
# verify_embed_payload.sh — drift guard for embed/MANIFEST.json vs sce/.
#
# embed/ is almost entirely gitignored — `.gitignore` carries
# `/embed/**` + `!/embed/MANIFEST.json`, so the manifest is the sole
# embed-side artifact downstream consumers can compare across SCE
# versions to see which symbols moved. That makes MANIFEST.json the
# operational "release marker" of the embed channel; if it lags sce/
# the channel is stale even though git looks clean.
#
# The existing verify_embed_manifest.sh re-emits the manifest from the
# *current on-disk* embed/include/ tree, which assumes someone ran
# package_embed.sh first. On a fresh checkout (CI) embed/include/
# does not exist, and on a developer machine it can be arbitrarily
# stale; either way an embed-internal manifest check passes silently
# while sce/ has moved on. This verifier closes that upstream half by
# running package_embed.sh end-to-end (sce/ → embed/include/ →
# manifest emit) into a tmp directory, then comparing the freshly
# emitted MANIFEST.json against the committed one.
#
# Sister verifiers (orthogonal scopes):
#   verify_embed_manifest.sh — embed/MANIFEST.json ↔ embed/include/
#                              (embed-internal self-consistency, fast).
#   verify_licenses.sh       — sce/sce_licenses.cmake covers every
#                              third_party LICENSE / embedded notice.
#   verify_embed_payload.sh  — sce/ ↔ embed/MANIFEST.json end-to-end
#                              (this script; gates the upstream lag).
#
# Cost: 5-7 minutes wall-clock on a typical 4-core machine (clang AST
# emit dominates; emit_embed_manifest.sh already parallelises with
# JOBS=4). For sub-second local checks verify_embed_manifest.sh
# remains the faster path; reach for this verifier in CI or before
# tagging a release.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCE_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
EMBED_DIR="${SCE_ROOT}/embed"
COMMITTED_MANIFEST="${EMBED_DIR}/MANIFEST.json"

if [[ ! -f "${COMMITTED_MANIFEST}" ]]; then
    echo "ERROR: ${COMMITTED_MANIFEST} not found." >&2
    echo "       This is the only embed/ artifact tracked in git;" >&2
    echo "       run scripts/package_embed.sh and commit it first." >&2
    exit 1
fi

TMP_FRESH="$(mktemp -d --suffix=.embed-payload-check)"
trap 'rm -rf "${TMP_FRESH}"' EXIT

# Run package_embed.sh into a tmp directory so the committed
# embed/MANIFEST.json is never overwritten — this verifier is
# idempotent and safe to re-run on the same tree.
"${SCRIPT_DIR}/package_embed.sh" -o "${TMP_FRESH}" >/dev/null

if [[ ! -f "${TMP_FRESH}/MANIFEST.json" ]]; then
    echo "ERROR: package_embed.sh did not emit MANIFEST.json." >&2
    echo "       Likely cause: clang++-19 not on PATH." >&2
    echo "       Install clang-19 or set CLANG=<path> and retry." >&2
    exit 1
fi

# Compare API-surface fields only — strip the two label-only fields
# (embed_version: HEAD-derived; clang_version: toolchain-derived) so
# the verifier fires exclusively on actual symbol/header drift, not on
# every src commit that bumps HEAD nor on cross-machine toolchain
# differences. Relaxed mode by design: the embed_version label can
# legitimately lag a few commits as long as the API surface it
# describes is still accurate. Strict-mode (forcing a regen commit
# per src commit) would generate ~2× commit churn for zero
# correctness gain — see the "stale embed window" discussion in the
# tc8-harness feedback thread.
canon_manifest() {
    python3 -c '
import json, sys
d = json.load(open(sys.argv[1]))
d.pop("embed_version", None)
d.pop("clang_version", None)
json.dump(d, sys.stdout, sort_keys=True, indent=2)
' "$1"
}

CANON_COMMITTED="$(mktemp --suffix=.committed.json)"
CANON_FRESH="$(mktemp --suffix=.fresh.json)"
trap 'rm -rf "${TMP_FRESH}" "${CANON_COMMITTED}" "${CANON_FRESH}"' EXIT
canon_manifest "${COMMITTED_MANIFEST}" > "${CANON_COMMITTED}"
canon_manifest "${TMP_FRESH}/MANIFEST.json" > "${CANON_FRESH}"

if ! diff -q "${CANON_COMMITTED}" "${CANON_FRESH}" >/dev/null 2>&1; then
    echo "ERROR: embed/MANIFEST.json API surface is stale relative to sce/." >&2
    echo "" >&2
    echo "Running package_embed.sh against the current sce/ source produces a" >&2
    echo "MANIFEST.json whose symbol/header set differs from the committed copy." >&2
    echo "Downstream consumers re-vendoring at this commit would see an API" >&2
    echo "shape that does not match what the embed marker claims." >&2
    echo "" >&2
    echo "Resolve by regenerating + committing the manifest:" >&2
    echo "" >&2
    echo "    scripts/package_embed.sh" >&2
    echo "    git add embed/MANIFEST.json" >&2
    echo "    git commit -m \"chore: Regenerate embed/MANIFEST.json after \$(git rev-parse --short HEAD)\"" >&2
    echo "" >&2
    echo "--- diff (first 80 lines, excluding embed_version + clang_version) ---" >&2
    diff -u "${CANON_COMMITTED}" "${CANON_FRESH}" 2>/dev/null | head -n 80 >&2 || true
    exit 1
fi

echo "OK: embed/MANIFEST.json API surface matches what package_embed.sh produces from current sce/."
