#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: cpp-suite.yml
#
# Every ctest case in the main build that the C11 arm does not run: the C++
# engine's own unit suites, the mesh transports, and the cross-device
# conformance drivers.
#
# Measured 2026-08-12, and the measurement is why this exists. `scripts/gate
# --all` was run with a logging shim in place of `ctest`, so every invocation
# recorded its working directory and its arguments. Twenty-eight gates passed
# and exactly three ctest runs happened: two from `w3c-c11` (`-N -L c11` then
# `-L c11`) and one from `forge-cpp` against its own scratch tree. Three of
# these binaries were additionally replaced with loggers that exec the real
# one, and none was executed. So of 382 registered cases, 223 ran and 159 —
# 114 of them `mesh_*`, including all 19 zenoh cases, plus twenty-four C++ unit
# suites — were run by nothing at all. Not by a gate, and not by CI either: no
# workflow configures the main tree, and the C++ conformance job builds the
# single `w3c_test_cli` target.
#
# `-L c11` and `-LE c11` are exact complements, so the pair of gates covers the
# registered set with no third list to keep current — that partition is
# asserted below and pinned by `sce-build/tests/ctest_lane_partition.rs`. The
# W3C C++ cases are not in it: they are not ctest cases at all, they are the
# 404 the `w3c-cpp` binary runs, so nothing here is a second spelling of a run
# another gate already makes.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

# A live fixture server makes 13 of these cases report Not Run, which is a
# smaller suite reported as a passing one. The count comparison at the end
# would catch it; this names the cause instead of the symptom.
sce_gate_requires_free_http_port

sce_main_build_dir
BUILD_DIR="$SCE_MAIN_BUILD_DIR"

# A killed run leaves its log temporary behind forever; the gate that creates
# them is the one that clears them.
sce_prune_ctest_temporaries "$BUILD_DIR"

# The default target, not a named one. These cases span the engine, the mesh
# transports and the fixtures, and a target list here would be the same
# list-shaped defect this gate exists to remove: a case whose target nobody
# added would be registered, skipped by the build, and then reported by ctest
# as a missing executable rather than as an untested one.
sce_gate_step "building the main tree"
cmake --build "$BUILD_DIR" --parallel "$(nproc)" >/dev/null \
    || sce_gate_fail "main tree build"

# The debug-info layout is a property of the binaries, not of CMakeLists.txt,
# and this is the only gate that has both a configured tree and the built
# executables in it.
#
# Both settings it checks are invisible to every other kind of check. A
# `-Wl,--gdb-index` that stops reaching the link — moved outside the
# `LLD_LINKER` guard, dropped by a toolchain that ignores it, or configured
# into a cache CI then restores — leaves the whole suite green while every
# executable regrows the 47 MB of pubname tables the flag exists to consume.
# Reading the option out of CMakeLists.txt would only confirm that someone
# wrote it down, which is the difference between a setting being declared and
# a setting being in effect, so the assertion is made against what came out.
#
# Judged on the largest executable rather than all of them: the property is
# per-link, one binary either carries the section or the flag did not reach
# any link, and readelf over 129 binaries would cost more than it tells.
sce_gate_step "debug-info layout of the emitted binaries"
biggest="$(find "$BUILD_DIR/tests" -maxdepth 1 -type f -executable -printf '%s\t%p\n' \
    | sort -rn | head -1 | cut -f2)"
[[ -n "$biggest" ]] \
    || sce_gate_fail "no test executable found under $BUILD_DIR/tests to judge"
sections="$(readelf -S --wide "$biggest")"
grep -q '\.gdb_index' <<<"$sections" \
    || sce_gate_fail "$biggest carries no .gdb_index — -Wl,--gdb-index did not reach the link. Reconfigure with a build type of Debug or RelWithDebInfo and lld available; without it every binary regrows ~47 MB of pubname tables."
if grep -qE '\.debug_gnu_pub(names|types)' <<<"$sections"; then
    sce_gate_fail "$biggest still carries pubname tables alongside .gdb_index — the linker emitted the index without consuming them, so the binary pays for both indexes."
fi
grep -q '\.debug_line' <<<"$sections" \
    || sce_gate_fail "$biggest carries no .debug_line — the tree was built without debug info at all, which is not what this lane is for."

count_registered() {
    ctest --test-dir "$BUILD_DIR" -N "$@" | grep -cE '^ *Test +#'
}

registered="$(count_registered -LE c11)"
c11="$(count_registered -L c11)"
total="$(count_registered)"

# The partition is the whole argument for selecting by label rather than by
# name. If the two halves ever stop adding up to the whole, a case belongs to
# neither gate and is back where the 159 were.
if (( registered + c11 != total )); then
    sce_gate_fail "the ctest partition is not total: $registered non-c11 + $c11 c11 != $total registered. A case in neither half is run by no gate — which is the state this gate was added to end."
fi

sce_gate_step "registered non-c11 tests: $registered (of $total)"

# Whether this machine could register the mesh half at all.
#
# The floor below asks "did the tree lose cases", and on a machine without the
# transport SDKs it was answering a different question. The mesh transport
# suites are registered only when `find_package` locates vsomeip3, zenohcxx or
# CycloneDDS, and the configure says so in as many words — `vsomeip3 not found
# — skipping mesh_someip_compile_test`. Measured 2026-08-14 on the build
# machine: 49 mesh cases absent for that reason, 129 non-c11 registered, and
# this gate reported FAILED — a verdict about the author's tree for a fact
# about the machine's installed packages. That is the distinction
# `sce_gate_cannot_run` exists for, and the one
# `gate_registry_contract`/`mnemosyne-cli` already paid for once.
#
# The `..._DIR` cache entries are read rather than the packages re-detected:
# they are what the SAME `find_package` calls that gate the tests left behind,
# so this cannot disagree with the configure the way a second detection would.
missing_transports=()
for _pkg in vsomeip3 zenohcxx CycloneDDS; do
    if grep -q "^${_pkg}_DIR:PATH=.*-NOTFOUND$" "$BUILD_DIR/CMakeCache.txt" 2>/dev/null; then
        missing_transports+=("$_pkg")
    fi
done

if (( ${#missing_transports[@]} > 0 )); then
    sce_gate_cannot_run "this machine has no ${missing_transports[*]}, so the mesh transport cases are not registered ($registered non-c11 here against a floor of 140) and the 'engine + mesh' half this gate is named for cannot be assembled. Nothing is claimed about the tree. Install the SDK(s), or run this gate where they are present — the local workstation registers 178."
fi

if (( registered < 140 )); then
    sce_gate_fail "only $registered non-c11 test(s) registered (expected at least 140) — the gate would report on a smaller set than its name claims"
fi

LOG="$(mktemp -d)"
sce_gate_on_exit "rm -rf '$LOG'"

# `--no-tests=error` covers the zero case even if the floor above is ever
# loosened; the count comparison below covers a partial run.
ctest --test-dir "$BUILD_DIR" -LE c11 \
      --output-on-failure --no-tests=error -j "$(nproc)" 2>&1 | tee "$LOG/ctest.log"
ctest_status="${PIPESTATUS[0]}"
(( ctest_status == 0 )) || sce_gate_fail "C++ ctest suite failed"

executed="$(grep -oE 'out of [0-9]+' "$LOG/ctest.log" | tail -1 | grep -oE '[0-9]+')"

# A DISABLED test is registered and deliberately not run — `ctest -N` counts it,
# the run's "out of N" does not. Comparing those two numbers directly therefore
# fails on a tree that merely has a disabled case, which is what the long-form
# `benchmark_*_full` cases are: registered so they can be asked for by name,
# disabled so a routine run does not spend minutes on them.
#
# Measured 2026-08-14: 178 registered, 171 run, 7 disabled, every one of the
# 171 green — reported as a failure. The check's intent is that nothing goes
# missing SILENTLY, so the partition, not the equality, is what to assert: run
# + disabled must account for every registered case, and the disabled ones are
# named rather than merely subtracted.
disabled_names="$(sed -n '/The following tests did not run:/,$p' "$LOG/ctest.log" \
    | grep -E '\(Disabled\)' || true)"
disabled="$(printf '%s' "$disabled_names" | grep -c . || true)"
if (( executed + disabled != registered )); then
    sce_gate_fail "ran ${executed:-0} and skipped $disabled disabled of $registered registered non-c11 test(s) — $((registered - executed - disabled)) neither ran nor declared themselves disabled"
fi

if (( disabled > 0 )); then
    sce_gate_step "$disabled disabled test(s) not run:"
    printf '      %s\n' $disabled_names >&2
fi
sce_gate_step "$executed C++ test(s) passed"
