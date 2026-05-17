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
# Self-contained invariant: every header must compile standalone (one TU
# per header). On any clang parse error the script aborts — see
# process_one's fail-fast block — because clang's degraded AST silently
# substitutes placeholder types for unresolved templates and would yield
# a manifest that varies with the parse environment's include path.
# scripts/test_emit_manifest_fail_fast.sh is the regression TC; pre-push
# Stage 1c invokes it.
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

# Source-repo third_party deps the embed package does not vendor but the
# manifest emit -I's into (quickjs for scripting/, cpp-httplib for
# events/HttpResponseUtils.h). Verifying them upfront fails fast with a
# concrete fix-path when:
#   1. The git submodule has not been initialised (`git submodule update
#      --init --recursive`), or
#   2. A future refactor breaks SCE_ROOT propagation into the xargs
#      worker shell — that bug silently fell through to the system
#      include path on hosts where the headers happened to be installed
#      (committer's machine: yes; ubuntu-latest CI: no), masking the
#      defect locally and surfacing it only at PR-merge time.
# Cheap directory probe, runs once before parallel emit kicks off.
for dep_dir in "${SCE_ROOT}/third_party/quickjs" \
               "${SCE_ROOT}/third_party/cpp-httplib"; do
    if [[ ! -d "${dep_dir}" ]]; then
        echo "ERROR: required third_party include root missing: ${dep_dir}" >&2
        echo "       The manifest emitter -I's into sce/third_party/ for" >&2
        echo "       headers the embed package does not vendor (quickjs," >&2
        echo "       cpp-httplib). Initialise submodules with:" >&2
        echo "         git submodule update --init --recursive" >&2
        exit 1
    fi
done

# Default to 4 parallel jobs; clang -ast-dump is memory-heavy (one TU per
# header pulling the full transitive closure), so capping avoids OOM on
# many-core machines. Override via JOBS=N to tune for the host.
JOBS="${JOBS:-4}"

echo "Scanning ${#HEADERS[@]} headers with ${CLANG} (JOBS=${JOBS})..."

TMP_WORK_DIR="$(mktemp -d --suffix=.embed-manifest)"
TMP_LINES="$(mktemp --suffix=.jsonl)"
trap 'rm -rf "${TMP_WORK_DIR}"; rm -f "${TMP_LINES}"' EXIT

# Each header is parsed standalone (one TU per header) and must compile
# cleanly — see process_one's fail-fast block below. Aborting on clang
# parse error keeps the manifest a pure function of the source headers
# rather than the parse environment's system include path.
#
# Workers write per-header outputs into TMP_WORK_DIR so xargs -P >1 has no
# shared-file race; the merge below appends in deterministic HEADERS order
# so byte-output is identical to the prior serial implementation.
process_one() {
    local include_dir="$1"
    local embed_dir="$2"
    local clang_bin="$3"
    local filter="$4"
    local work_dir="$5"
    local sce_root="$6"
    local header="$7"

    local rel="${header#${include_dir}/}"
    local key="${rel//\//__}"
    local out_lines="${work_dir}/${key}.jsonl"
    local clang_stderr
    clang_stderr="$(mktemp)"
    set +e
    # -I "${sce_root}/third_party/quickjs": embed/include/scripting/JSEngine.h
    # transitively #include "quickjs.h" but the embed package does not vendor
    # QuickJS (it's a consumer-provided scripting tier dep, see
    # package_embed.sh §3 comment). Without this -I the standalone parse of
    # JSEngine.h fails on hosts without QuickJS in their system include path
    # — historically that included GitHub Actions ubuntu-latest, where
    # clang's degraded AST produced placeholder types (int instead of
    # std::vector<...>) and yielded a manifest that drifted from the
    # committer's. The manifest emitter is a source-repo tool (lives in
    # scripts/ and reads SCE_ROOT), so reaching into sce/third_party/ here
    # is a deliberate coupling, not a layering break.
    # sce_root is passed in as a positional parameter rather than read from
    # the parent shell's SCE_ROOT — xargs spawns a fresh `bash -c` for each
    # worker, and that subshell does not inherit unexported variables, so an
    # ambient ${SCE_ROOT} would expand to empty and the -I would become
    # `-I /third_party/quickjs`. Locally that silently fell through to the
    # system include path (when clang already had quickjs/httplib resolved);
    # on CI it surfaced as "fatal error: 'quickjs.h' file not found".
    # -I "${sce_root}/third_party/cpp-httplib": embed/include/events/
    # HttpResponseUtils.h transitively `#include <httplib.h>`. Like
    # quickjs (see the comment above), httplib is a consumer-provided
    # HTTP client tier — embed does not vendor it. The manifest emitter
    # is a source-repo tool, so the -I into sce/third_party/ here is
    # deliberate (same precedent as the quickjs -I).
    # -H: clang writes the include tree to stderr (lines like
    # `. /path/to/header.h`). After a successful parse we scan that trace
    # for paths under /usr/local/include — see the pollution check below
    # the parse — so a host with a stale `make install`'d SCE at
    # /usr/local/include/ can no longer mask a missing entry in
    # SCE_BASE_INCLUDE_DIRS. Without this guard the local emit silently
    # falls through to /usr/local/include/<dir>/X.h when embed/include/
    # is missing <dir>, and CI (no /usr/local/include/SCE/) is the first
    # to notice. The check enforces identical include-resolution rules
    # locally and on CI.
    "${clang_bin}" \
        -H \
        -Xclang -ast-dump=json \
        -fsyntax-only \
        -std=c++17 \
        -x c++-header \
        -I "${include_dir}" \
        -I "${embed_dir}/third_party/nlohmann_json/include" \
        -I "${embed_dir}/third_party/pugixml/src" \
        -I "${sce_root}/third_party/quickjs" \
        -I "${sce_root}/third_party/cpp-httplib" \
        "${header}" 2>"${clang_stderr}" \
        | python3 "${filter}" "${include_dir}" > "${out_lines}"
    local pipe_status=("${PIPESTATUS[@]}")
    set -e
    if [[ "${pipe_status[1]}" != "0" ]]; then
        echo "ERROR: filter helper failed for ${header}" >&2
        rm -f "${clang_stderr}"
        return 1
    fi
    # Fail-fast on clang parse error rather than silently consuming the
    # degraded AST that clang still emits. The prior behaviour (record the
    # header in non_self_contained_headers and keep going) made the manifest
    # a function of the parse environment — locally-clean parses on a host
    # with quickjs.h on the system path produced a smaller, "correct" manifest
    # than CI parses without quickjs.h, where clang fell back to placeholder
    # types (int for std::vector<std::string>) and emitted ghost duplicate
    # symbols. See scripts/test_emit_manifest_fail_fast.sh for the regression
    # TC; pre-push Stage 1c gates against silent re-introduction.
    if [[ "${pipe_status[0]}" != "0" ]]; then
        # -H writes the include tree to the same stderr as parse errors;
        # strip those lines (leading "." dots) so the user sees only the
        # actual clang diagnostic, not 200 lines of include trace.
        grep -v -E '^\.+ ' "${clang_stderr}" >&2 || true
        rm -f "${clang_stderr}"
        echo "" >&2
        echo "ERROR: clang failed to parse standalone header: ${rel}" >&2
        echo "       The embed manifest requires every public header to be" >&2
        echo "       self-contained when compiled in isolation. Fix one of:" >&2
        echo "         1. Add the missing -I to scripts/emit_embed_manifest.sh" >&2
        echo "            (this script is the source-repo tool, so reaching" >&2
        echo "            into sce/third_party/ is allowed)" >&2
        echo "         2. Vendor the missing dep into embed/third_party/" >&2
        echo "            (only if it becomes part of the embed package)" >&2
        echo "         3. Make the header self-contained (forward-decl or" >&2
        echo "            move the include into the .cpp)" >&2
        return 1
    fi
    # Parse succeeded — check the -H trace for resolves that pollute the
    # manifest. clang's default search path includes /usr/local/include,
    # and a host with a stale `make install`'d SCE there will silently
    # resolve `#include "states/X.h"` (and friends) to
    # /usr/local/include/states/X.h when SCE_BASE_INCLUDE_DIRS is missing
    # `states`. That makes the local emit succeed on a packaging gap that
    # CI (clean ubuntu-latest, no /usr/local/include/SCE/) rejects. The
    # guard below makes local and CI agree: any include resolving under
    # /usr/local/include/ for an SCE-internal directory fails the parse
    # with the same fix-paths as a true clang parse error.
    if grep -qE '^\.+ /usr/local/include/' "${clang_stderr}"; then
        echo "ERROR: header ${rel} resolved through /usr/local/include/" >&2
        echo "       Your host has stale SCE/dependency headers at /usr/local/include," >&2
        echo "       which masks missing entries in SCE_BASE_INCLUDE_DIRS — the" >&2
        echo "       local emit succeeds via the system path fallback, then CI" >&2
        echo "       (no /usr/local/include/SCE/) fails the same parse. Resolve by:" >&2
        echo "         1. Adding the missing subdir to SCE_BASE_INCLUDE_DIRS in" >&2
        echo "            sce/sce_base_sources.cmake (so package_embed.sh ships it" >&2
        echo "            under embed/include/ and the include resolves locally), or" >&2
        echo "         2. sudo rm -rf the stale /usr/local/include/ SCE install" >&2
        echo "            (if you no longer need a system-wide build of SCE)." >&2
        echo "" >&2
        echo "       Offending include trace (from clang -H):" >&2
        grep -E '^\.+ /usr/local/include/' "${clang_stderr}" >&2
        rm -f "${clang_stderr}"
        return 1
    fi
    rm -f "${clang_stderr}"
}
export -f process_one

printf '%s\0' "${HEADERS[@]}" \
    | xargs -0 -n1 -P "${JOBS}" \
        bash -c 'process_one "$@"' _ \
            "${INCLUDE_DIR}" "${EMBED_DIR}" "${CLANG}" "${FILTER}" "${TMP_WORK_DIR}" "${SCE_ROOT}"

# Merge per-worker outputs in HEADERS order. The python merger below dedups
# and sorts anyway, so order is not load-bearing — but preserving it keeps
# the intermediate TMP_LINES diffable against the prior serial output.
for header in "${HEADERS[@]}"; do
    rel="${header#${INCLUDE_DIR}/}"
    key="${rel//\//__}"
    if [[ -s "${TMP_WORK_DIR}/${key}.jsonl" ]]; then
        cat "${TMP_WORK_DIR}/${key}.jsonl" >> "${TMP_LINES}"
    fi
done

# Collapse per-header lines into the final manifest with deterministic
# ordering; python handles unicode + sort_keys canonicalisation.
python3 - "${TMP_LINES}" "${MANIFEST}" "${EMBED_DIR}" "${CLANG}" << 'PY'
import json
import os
import subprocess
import sys

tmp_lines, manifest_path, embed_dir, clang_bin = sys.argv[1:5]

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

# Always [] by construction: process_one fails-fast on any non-self-contained
# header, so emit only reaches this point when every header parsed cleanly.
# The field is kept in the manifest for sce-embed-manifest.v1 schema
# stability — consumers checking schema compliance would break if we
# removed the key, and the [] value is unambiguous.
parse_errors = []

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
      "all headers self-contained)")
PY
