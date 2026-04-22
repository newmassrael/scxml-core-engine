#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
#
# emit_embed_manifest.sh — generate embed/MANIFEST.json from embed/include/**/*.h.
#
# Each public header is parsed by clang into a JSON AST, then filtered to
# the declarations whose defining file lives under embed/include/. The
# merged, deduplicated, sorted result is the embed API surface manifest.
#
# Consumers vendoring embed/ compare the old vs new MANIFEST.json via
# `diff` to spot upstream API changes before re-sync; see docs/EMBED_MANIFEST.md.
#
# Usage:
#   scripts/emit_embed_manifest.sh [-d EMBED_DIR]
#   EMBED_DIR defaults to ./embed/ relative to the repo root.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCE_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
EMBED_DIR="${SCE_ROOT}/embed"
CLANG="${CLANG:-clang++-19}"

while [[ $# -gt 0 ]]; do
    case "$1" in
        -d|--embed-dir)
            EMBED_DIR="$2"
            shift 2
            ;;
        -h|--help)
            echo "Usage: $0 [-d EMBED_DIR]"
            echo "  -d DIR   Embed package root (default: ./embed/)"
            echo ""
            echo "Environment:"
            echo "  CLANG    C++ compiler with AST dump support (default: clang++-19)"
            exit 0
            ;;
        *)
            echo "ERROR: unknown argument: $1" >&2
            exit 2
            ;;
    esac
done

if ! command -v "${CLANG}" >/dev/null 2>&1; then
    echo "ERROR: ${CLANG} not on PATH. Install clang-19 or override via CLANG=..." >&2
    exit 1
fi

INCLUDE_DIR="${EMBED_DIR}/include"
MANIFEST="${EMBED_DIR}/MANIFEST.json"
FILTER="${SCRIPT_DIR}/_embed_manifest_filter.py"

if [[ ! -d "${INCLUDE_DIR}" ]]; then
    echo "ERROR: ${INCLUDE_DIR} not found. Run scripts/package_embed.sh first." >&2
    exit 1
fi
if [[ ! -f "${FILTER}" ]]; then
    echo "ERROR: filter helper ${FILTER} missing." >&2
    exit 1
fi

# Headers gated on optional dependencies the embed packager does not
# pull by default. Their declarations enter MANIFEST.json only when the
# consumer rebuilds with those deps in tree; excluding them keeps the
# core-API manifest reproducible without a --with-spdlog knob here.
SKIP_BASENAMES=(
    "SpdlogBackend.h"          # requires spdlog headers (see --with-spdlog)
    "EmscriptenFetchClient.h"  # Emscripten-only; not part of host API
)

should_skip() {
    local h="$1"
    local pat
    for pat in "${SKIP_BASENAMES[@]}"; do
        if [[ "${h##*/}" == "${pat}" ]]; then
            return 0
        fi
    done
    return 1
}

HEADERS=()
while IFS= read -r header; do
    if should_skip "${header}"; then
        continue
    fi
    HEADERS+=("${header}")
done < <(find "${INCLUDE_DIR}" -type f -name '*.h' | LC_ALL=C sort)

if [[ ${#HEADERS[@]} -eq 0 ]]; then
    echo "ERROR: no headers found under ${INCLUDE_DIR}" >&2
    exit 1
fi

echo "Scanning ${#HEADERS[@]} headers with ${CLANG}..."

TMP_LINES="$(mktemp --suffix=.jsonl)"
TMP_PARSE_ERRORS="$(mktemp --suffix=.txt)"
trap 'rm -f "${TMP_LINES}" "${TMP_PARSE_ERRORS}"' EXIT

CLANG_ARGS=(
    -Xclang -ast-dump=json
    -fsyntax-only
    -std=c++17
    -x c++-header
    -I "${INCLUDE_DIR}"
    -I "${EMBED_DIR}/third_party/nlohmann_json/include"
    -I "${EMBED_DIR}/third_party/pugixml/src"
)

# clang emits a complete AST even when a header is not self-contained
# (missing transitive include, forward-decl collision, etc.), so the
# manifest still captures symbols from partially-parsed inputs. The
# headers that produced diagnostics are recorded verbatim in the manifest
# so consumers can tell which entries may have degraded member layouts.
for header in "${HEADERS[@]}"; do
    CLANG_STDERR="$(mktemp)"
    set +e
    "${CLANG}" "${CLANG_ARGS[@]}" "${header}" 2>"${CLANG_STDERR}" \
        | python3 "${FILTER}" "${INCLUDE_DIR}" >> "${TMP_LINES}"
    pipe_status=("${PIPESTATUS[@]}")
    set -e
    if [[ "${pipe_status[1]}" != "0" ]]; then
        echo "ERROR: filter helper failed for ${header}" >&2
        rm -f "${CLANG_STDERR}"
        exit 1
    fi
    if [[ "${pipe_status[0]}" != "0" ]]; then
        rel="${header#${INCLUDE_DIR}/}"
        printf '%s\n' "${rel}" >> "${TMP_PARSE_ERRORS}"
    fi
    rm -f "${CLANG_STDERR}"
done

# Collapse per-header lines into the final manifest with deterministic
# ordering; python handles unicode + sort_keys canonicalisation.
python3 - "${TMP_LINES}" "${MANIFEST}" "${EMBED_DIR}" "${CLANG}" "${TMP_PARSE_ERRORS}" << 'PY'
import json
import os
import subprocess
import sys

tmp_lines, manifest_path, embed_dir, clang_bin, tmp_parse_errors = sys.argv[1:6]

lines = []
with open(tmp_lines, encoding="utf-8") as fh:
    for line in fh:
        line = line.strip()
        if line:
            lines.append(line)

# Each line is already canonical JSON (sort_keys=True). String-level
# dedup + sort yields the same ordering as parse+sort-by-tuple but
# without relying on dict ordering.
unique_sorted = sorted(set(lines))
symbols = [json.loads(s) for s in unique_sorted]

parse_errors = []
if os.path.exists(tmp_parse_errors):
    with open(tmp_parse_errors, encoding="utf-8") as fh:
        parse_errors = sorted({line.strip() for line in fh if line.strip()})

# embed_version: reuse embed/VERSION if packaged, else fall back to "unknown".
version_path = os.path.join(embed_dir, "VERSION")
embed_version = "unknown"
if os.path.exists(version_path):
    with open(version_path, encoding="utf-8") as fh:
        embed_version = fh.read().strip() or "unknown"

# clang_version: include the compiler identity so consumers can tell whether
# a manifest diff reflects API drift or a toolchain upgrade.
try:
    cv = subprocess.run(
        [clang_bin, "--version"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout.splitlines()[0].strip()
except Exception:
    cv = "unknown"

manifest = {
    "schema": "sce-embed-manifest.v1",
    "embed_version": embed_version,
    "clang_version": cv,
    "symbol_count": len(symbols),
    "non_self_contained_headers": parse_errors,
    "symbols": symbols,
}

with open(manifest_path, "w", encoding="utf-8") as fh:
    json.dump(manifest, fh, indent=2, sort_keys=True, ensure_ascii=False)
    fh.write("\n")

print(f"Wrote {manifest_path} ({len(symbols)} symbols, "
      f"{len(parse_errors)} header(s) not self-contained)")
PY
