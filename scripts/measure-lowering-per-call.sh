#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The per-call price of the two ECMAScript->Lua paths, on ONE host.
#
# NOT A GATE. Nothing here asserts a bound and nothing returns non-zero for
# a slow number: this machine is shared between sessions and the same 21
# gates have been measured at 529s and at 1161s, so a timing assertion would
# be a flake generator. What this buys is that the figures quoted in
# docs/SCE_LUA_TRANSLATION_SEAM.md have a command behind them. The first
# version of that measurement lived in /tmp and was deleted when the round
# ended, leaving three numbers nobody could re-derive.
#
# WHY ONE SCRIPT AND NOT TWO COMMANDS. A cross-machine comparison is not a
# comparison. The first attempt at this measurement timed the C++ side
# locally and the Rust side on the build machine, and the result had to be
# thrown away. Both halves run here, in sequence, in one load window, and
# the census line says which host so a reader can tell two runs apart.
#
# WHICH NUMBER IS THE COMPARISON. The rewriter memoises inside itself; the
# frontend does not. So the honest pairing is COLD against the frontend's
# steady state, and the warm column exists to attribute the gap to the memo
# rather than to the algorithm. Reading the warm number as "the cost" is the
# mistake this axis has walked into three times.
#
# Usage:  scripts/measure-lowering-per-call.sh [build-dir]
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_DIR="${1:-$REPO_ROOT/build}"
BENCH="$BUILD_DIR/tests/benchmarks/benchmark_ecma_lowering_per_call"

cd "$REPO_ROOT"

# How much of this machine was actually free. The census header carries it
# because the figures below are only comparable between runs taken under
# similar load — the same 21 gates have been measured here at 529s and at
# 1161s. It is READ FROM THE OWNER rather than derived from `/proc/loadavg`
# here: that arithmetic has one home, and a second spelling of it is a second
# answer (`build_jobs_has_one_owner` refuses one, and refused this file).
# shellcheck source=scripts/lib/sce_build_jobs.sh
source "$REPO_ROOT/scripts/lib/sce_build_jobs.sh"

# The population both halves must agree on. Read from the file rather than
# hard-coded, because a table that grows and a benchmark that does not is
# exactly the drift this line exists to refuse.
POPULATION="$(python3 -c 'import json,sys; print(len(json.load(open(sys.argv[1]))["cases"]))' \
    "$REPO_ROOT/tests/ecmascript/ecma262_semantics.json")"

if [[ ! -x "$BENCH" ]]; then
    echo "measure-lowering-per-call: $BENCH is not built." >&2
    echo "  cmake --build $BUILD_DIR --target benchmark_ecma_lowering_per_call" >&2
    exit 2
fi

RESULTS="$(mktemp -d)"
trap 'rm -rf "$RESULTS"' EXIT

echo "==> host $(hostname), $(nproc) core(s), $(sce_build_jobs_value) free of them"

# --- C++: the run-time rewriter, cold and warm -----------------------------
"$BENCH" --benchmark_format=json --benchmark_min_time=1s > "$RESULTS/cpp.json"

# --- Rust: the build-time frontend ----------------------------------------
cargo run --quiet --release --example lowering_per_call -p sce-build > "$RESULTS/rust.txt"

python3 - "$RESULTS/cpp.json" "$RESULTS/rust.txt" "$POPULATION" <<'PY'
import json, re, sys

cpp_path, rust_path, population = sys.argv[1], sys.argv[2], int(sys.argv[3])

cpp = json.load(open(cpp_path))
per_call = {}
for b in cpp["benchmarks"]:
    # items_per_second counts CALLS, because each fixture reports the whole
    # table as its item count. Deriving ns/call from it rather than from
    # real_time keeps the arithmetic in one place and makes a fixture that
    # forgot SetItemsProcessed fail loudly instead of reporting the table's
    # cost as one call's.
    items = b.get("items_per_second")
    if not items:
        raise SystemExit(f"{b['name']}: no items_per_second — the fixture did not "
                         f"call SetItemsProcessed, so a per-CALL figure cannot be derived")
    per_call[b["name"]] = 1e9 / items

rust = open(rust_path).read()
m = re.search(r"LoweringPerCall census: (.*)", rust)
if not m:
    raise SystemExit("the Rust half printed no census line")
fields = dict(kv.split("=", 1) for kv in m.group(1).split() if "=" in kv)

rust_population = int(fields["population"])
if rust_population != population:
    raise SystemExit(f"the two halves read different populations: rust={rust_population} "
                     f"table={population} — they are not measuring the same thing")

cold = per_call["BM_RewriteCold"]
warm = per_call["BM_RewriteWarm"]
frontend = float(fields["steady_ns_per_call"])
frontend_first = float(fields["first_pass_ns_per_call"])

print()
print(f"LoweringPerCall census: population={population} "
      f"frontend_steady_ns={frontend:.0f} frontend_first_pass_ns={frontend_first:.0f} "
      f"rewriter_cold_ns={cold:.0f} rewriter_warm_ns={warm:.0f} "
      f"cold_ratio={cold / frontend:.2f} memo_speedup={cold / warm:.1f}")
print()
print(f"  build-time frontend, steady        {frontend:8.0f} ns/call   (no cache)")
print(f"  build-time frontend, first pass    {frontend_first:8.0f} ns/call   (warm-up, not a memo)")
print(f"  run-time rewriter, COLD            {cold:8.0f} ns/call   (fresh instance per call)")
print(f"  run-time rewriter, warm            {warm:8.0f} ns/call   (its own memo answering)")
print()
print(f"  The comparison is COLD against steady: {cold / frontend:.2f}x.")
print(f"  The warm column is the memo, worth {cold / warm:.0f}x — and the")
print(f"  frontend has none, which is what the document has to price.")
PY
