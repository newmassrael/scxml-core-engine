#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: genmc-queue.yml
#
# The C11 and C++ queues under GenMC: layer 3 of the queue kind's verification
# (SCE Protocol-Synthesis RFC §synth-5-P). GenMC explores every execution the C11
# memory model allows, where a stress run samples the few a machine gives.
#
# TWO CLAIMS, and the second is what makes the first worth reading:
#
#   1. Each harness in backends/c/forge-runtime/tests/genmc/ and
#      backends/cpp/forge-runtime/tests/genmc/ passes: no execution of the real
#      runtime loses, duplicates or tears an element.
#   2. Each MUTANT is caught. A mutant is the runtime's atomics with ONE ordering
#      weakened to relaxed, read through the same harness: for C the hosted atomics
#      (`backends/c/forge-runtime/tests/conformance/sce_atomic_host.c`), for C++ a
#      copy of the queue headers whose `std::memory_order_*` is rewritten. A check
#      that cannot tell a weakened ordering from the real one checks nothing, and
#      the first harness of this gate did exactly that: it reported "No errors were
#      detected" for the runtime with every ordering weakened to relaxed, for a
#      reason GenMC's own output made plain (measured 2026-10-10, below).
#
# WHAT THE FIRST HARNESS GOT WRONG, written down because each cost a measurement:
#
#   * `--unroll=4` is too small. The runtime's `ring_init` loops over the 2R
#     entries, GenMC cuts a loop at the bound and kills the execution, and the
#     main thread never reached `pthread_create`: the graph held one thread and
#     a KILL, and "No errors were detected" described an execution with no
#     concurrency in it. The bound here is `UNROLL`, and the gate checks the
#     graph for both thread-create events, so the verdict cannot rest on a
#     program that never started its threads.
#   * A variable-length `memcpy` is not a memory event to GenMC ("Cannot
#     promote non-constant-length mem intrinsic!"), so the slot an element is
#     written to was invisible. The harness writes the copy out word by word.
#
# THE C++ HARNESSES read the runtime as it ships, `std::atomic` and all, and not a
# model of it. GenMC's own <pthread.h> is the C one, so libstdc++'s thread layer
# (`gthr-default.h`, pulled in by <atomic> and <memory>) finds 20 names undeclared
# and clang stops (measured 2026-10-11: 3 types and 17 functions, none of which
# the queue calls). `genmc_cxx_shim.h`, force-included below, declares them and
# nothing else; `genmc_cxx_support.h`, which a harness includes, defines the three
# symbols the interpreter cannot resolve (`__dso_handle`, `__cxa_atexit`, and a
# word-by-word `memcpy`, for the reason above).
#
# Run on a host that has GenMC (`scripts/install_genmc.sh`); the pinned
# revision's install path derives from the pin, so a host with another
# revision has no install at all rather than one mistaken for current.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

GENMC_REV="$(sed -n 's/^GENMC_REV="\(.*\)"$/\1/p' scripts/install_genmc.sh)"
GENMC_ROOT="${GENMC_ROOT:-$HOME/.local/genmc/$GENMC_REV}"
GENMC="$GENMC_ROOT/bin/genmc"
[[ -x "$GENMC" ]] || sce_gate_fail "genmc $GENMC_REV is not installed at $GENMC; run scripts/install_genmc.sh on this host"

HARNESS_DIR=backends/c/forge-runtime/tests/genmc
HOST_ATOMICS=backends/c/forge-runtime/tests/conformance/sce_atomic_host.c
CXX_HARNESS_DIR=backends/cpp/forge-runtime/tests/genmc
CXX_INCLUDE_DIR=backends/cpp/forge-runtime/include
CXX_SHIM="$CXX_HARNESS_DIR/genmc_cxx_shim.h"
# The headers a C++ mutant rewrites; every other header is read as it is.
CXX_MUTATED_HEADERS=(queue.h queue_segmented.h)
UNROLL=20
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# The options every run shares. The model is RC11, GenMC's default, named so a
# change of default is not a change of what is checked.
GENMC_OPTIONS=(--rc11 "--unroll=$UNROLL" --disable-estimation --print-exec-graphs)
INCLUDES=(-I backends/c/forge-runtime/include -I "$GENMC_ROOT/include/genmc/runtime")
# C++17, and not the C++20 the runtime is built with. The two queue headers
# compile under both and write the same `std::memory_order_*` under both, and
# GenMC reads only what they write. Under C++20 <memory> pulls in <ostream> and so
# <stdlib.h>, which collides with GenMC's own `pthread_t` ("typedef redefinition
# with different types"), and <atomic> asks for `clockid_t` and
# `pthread_cond_clockwait`: declarations cannot fix a type clash (measured
# 2026-10-11, libstdc++ 14).
CXX_STANDARD=c++17
CXX_FLAGS=("-std=$CXX_STANDARD" -include "$CXX_SHIM" -I "$CXX_INCLUDE_DIR" -I "$GENMC_ROOT/include/genmc/runtime")

# One mutant per ordering the SCQ algorithm depends on, and one with all of
# them weakened. Each line is `name|sed program`; the program rewrites ONE
# ordering in the hosted atomics.
MUTANTS=(
    "load_acquire|s/__atomic_load_n(p0, __ATOMIC_ACQUIRE)/__atomic_load_n(p0, __ATOMIC_RELAXED)/"
    "load_seq_cst|s/__atomic_load_n(p0, __ATOMIC_SEQ_CST)/__atomic_load_n(p0, __ATOMIC_RELAXED)/"
    "store_seq_cst|s/__atomic_store_n(p0, p1, __ATOMIC_SEQ_CST)/__atomic_store_n(p0, p1, __ATOMIC_RELAXED)/"
    "store_release|s/__atomic_store_n(p0, p1, __ATOMIC_RELEASE)/__atomic_store_n(p0, p1, __ATOMIC_RELAXED)/"
    "cas|s/__ATOMIC_ACQ_REL, __ATOMIC_ACQUIRE)/__ATOMIC_RELAXED, __ATOMIC_RELAXED)/"
    "fetch_add|s/__atomic_fetch_add(p0, p1, __ATOMIC_ACQ_REL)/__atomic_fetch_add(p0, p1, __ATOMIC_RELAXED)/"
    "fetch_or|s/__atomic_fetch_or(p0, p1, __ATOMIC_ACQ_REL)/__atomic_fetch_or(p0, p1, __ATOMIC_RELAXED)/"
    # The one fence the segmented queues use (the hazard domain's handshake). It
    # is the only mutant that weakens it alone: `all` weakens it with every other
    # ordering, which a harness can catch for a reason that has nothing to do
    # with the fence. Measured 2026-10-11 on `hazard_keeps_a_named_node`: the
    # fence weakened alone is caught, so it is not redundant with the
    # sequentially consistent accesses around it.
    "fence|s/__atomic_thread_fence(__ATOMIC_SEQ_CST)/__atomic_thread_fence(__ATOMIC_RELAXED)/"
    # Orderings that may be redundant ONE AT A TIME because another provides the
    # hand-over, so each GROUP that could be the sole provider is weakened too.
    "threshold|s/__atomic_load_n(p0, __ATOMIC_SEQ_CST)/__atomic_load_n(p0, __ATOMIC_RELAXED)/;s/__atomic_store_n(p0, p1, __ATOMIC_SEQ_CST)/__atomic_store_n(p0, p1, __ATOMIC_RELAXED)/"
    "rmw|s/__ATOMIC_ACQ_REL, __ATOMIC_ACQUIRE)/__ATOMIC_RELAXED, __ATOMIC_RELAXED)/;s/__atomic_fetch_add(p0, p1, __ATOMIC_ACQ_REL)/__atomic_fetch_add(p0, p1, __ATOMIC_RELAXED)/;s/__atomic_fetch_or(p0, p1, __ATOMIC_ACQ_REL)/__atomic_fetch_or(p0, p1, __ATOMIC_RELAXED)/"
    "acquire_release|s/__atomic_load_n(p0, __ATOMIC_ACQUIRE)/__atomic_load_n(p0, __ATOMIC_RELAXED)/;s/__atomic_store_n(p0, p1, __ATOMIC_RELEASE)/__atomic_store_n(p0, p1, __ATOMIC_RELAXED)/"
    "all|s/__ATOMIC_ACQ_REL/__ATOMIC_RELAXED/g;s/__ATOMIC_ACQUIRE/__ATOMIC_RELAXED/g;s/__ATOMIC_RELEASE/__ATOMIC_RELAXED/g;s/__ATOMIC_SEQ_CST/__ATOMIC_RELAXED/g"
)

# The C++ runtime has no hosted-atomics layer to weaken: its orderings are the
# `std::memory_order_*` written at each call. A mutant therefore rewrites those in
# a copy of the queue headers. The names are not the C names, because the sets are
# not the same: `acquire` weakens every acquire the runtime writes, the loads and
# the failure order of a compare-and-swap together, where the C `load_acquire`
# weakens a family of functions and leaves the compare-and-swap alone.
CXX_MUTANTS=(
    "acquire|s/std::memory_order_acquire/std::memory_order_relaxed/g"
    "release|s/std::memory_order_release/std::memory_order_relaxed/g"
    "acq_rel|s/std::memory_order_acq_rel/std::memory_order_relaxed/g"
    "seq_cst|s/std::memory_order_seq_cst/std::memory_order_relaxed/g"
    "acquire_release|s/std::memory_order_acquire/std::memory_order_relaxed/g;s/std::memory_order_release/std::memory_order_relaxed/g"
    "all|s/std::memory_order_acq_rel/std::memory_order_relaxed/g;s/std::memory_order_acquire/std::memory_order_relaxed/g;s/std::memory_order_release/std::memory_order_relaxed/g;s/std::memory_order_seq_cst/std::memory_order_relaxed/g"
)

# Runs GenMC on `$1` into `$2` and leaves GenMC's exit status in `$GENMC_STATUS`.
# `$3` names the language, `c` or `cxx`; any further arguments are compiler flags
# placed BEFORE the language's own, which is how a C++ mutant's copy of the
# headers is found ahead of the real ones.
run_genmc() {
    local source="$1" log="$2" language="$3"
    shift 3
    GENMC_STATUS=0
    case "$language" in
        c)   "$GENMC" "${GENMC_OPTIONS[@]}" -- "$@" "${INCLUDES[@]}" "$source" >"$log" 2>&1 || GENMC_STATUS=$? ;;
        cxx) "$GENMC" "${GENMC_OPTIONS[@]}" -- "$@" "${CXX_FLAGS[@]}" "$source" >"$log" 2>&1 || GENMC_STATUS=$? ;;
        *)   sce_gate_fail "run_genmc: unknown language '$language'" ;;
    esac
}

# The name a harness is reported under: the C ones by their file's stem, as they
# always were, and the C++ ones with `.cpp`, so that a C++ harness of the same
# name as a C one (`spsc_slot_reuse`) is not mistaken for it, in the output or in
# the logs.
harness_name() {
    local name
    name="$(basename "$1")"
    case "$name" in
        *.cpp) printf '%s\n' "$name" ;;
        *)     printf '%s\n' "${name%.c}" ;;
    esac
}

# The verdict of a run that PASSED is only worth reading if the program ran its
# threads: the graph must hold a thread-create for each thread the harness makes.
require_threads() {
    local log="$1" want="$2" got
    got="$(grep -c 'THREAD_CREATE' "$log" || true)"
    (( got >= want )) || sce_gate_fail "$log: the explored graph has $got thread-create event(s), the harness makes $want: the verdict describes a program that never started its threads"
}

# The baseline of one harness: it passes, it started its threads, and GenMC said
# it found no error. `$1` is the harness, `$2` its language.
check_baseline() {
    local harness="$1" language="$2" name threads
    name="$(harness_name "$harness")"
    threads="$(sed -n 's/^\/\* THREADS: \([0-9]*\) \*\/$/\1/p' "$harness")"
    [[ -n "$threads" ]] || sce_gate_fail "$harness has no '/* THREADS: n */' line naming the threads it makes"

    run_genmc "$harness" "$WORK/$name.log" "$language"
    (( GENMC_STATUS == 0 )) || { cat "$WORK/$name.log"; sce_gate_fail "$name: GenMC found an error in the runtime (status $GENMC_STATUS)"; }
    require_threads "$WORK/$name.log" "$threads"
    grep -q 'No errors were detected' "$WORK/$name.log" || sce_gate_fail "$name: GenMC did not say it found no errors"
}

# What one mutant's run says, given that `run_genmc` has just left its status in
# `$GENMC_STATUS`. `$1` is the harness, `$2` the mutant's label, `$3` its log.
judge_mutant() {
    local harness="$1" label="$2" log="$3" name
    name="$(harness_name "$harness")"
    # ⚠ A MUTANT IS CAUGHT ONLY WHEN GENMC SAYS SO. A non-zero status is not
    # that: GenMC also exits non-zero when the program does not compile, and
    # this gate's first version read a mutant harness that could not find its
    # own host file (a path built twice) as "caught", for all seven mutants.
    # Measured 2026-10-10 by running one mutant by hand, which GenMC passed.
    # So the report must show GenMC got as far as exploring (`Transformation
    # complete`) and name an error it found. The error is looked for
    # ANYWHERE in the line, not at its start: GenMC writes it to stderr and
    # the graph to stdout, both into this one log, and a caught mutant
    # printed `(1, 13): Rna (, 0) [(1, 5)]Error: Non-atomic race!` on one
    # line (measured 2026-10-10), which `^Error` called "no error found".
    if (( GENMC_STATUS != 0 )); then
        grep -q 'Transformation complete' "$log" \
            || { cat "$log"; sce_gate_fail "$name: mutant '$label' did not even reach exploration; its failure says nothing about the runtime"; }
        grep -qE 'Error: ' "$log" \
            || { cat "$log"; sce_gate_fail "$name: mutant '$label' exited $GENMC_STATUS without reporting a memory-model error"; }
    fi
    if (( GENMC_STATUS == 0 )); then
        # Surviving is not always a hole in the harness: an ordering a
        # second one already provides is redundant for THIS scenario. The
        # gate asks only the mutants named `required` in the harness.
        if grep -q "^/\\* REQUIRED: .*\\b$label\\b" "$harness"; then
            sce_gate_fail "$name: mutant '$label' SURVIVED, and the harness says it must be caught"
        fi
        echo "  $name: mutant '$label' survived (the harness does not require it)"
    else
        echo "  $name: mutant '$label' caught"
    fi
}

checked=0

for harness in "$HARNESS_DIR"/*.c; do
    name="$(basename "$harness" .c)"
    check_baseline "$harness" c

    for mutant in "${MUTANTS[@]}"; do
        label="${mutant%%|*}"
        program="${mutant#*|}"
        sed "$program" "$HOST_ATOMICS" >"$WORK/host_$label.c"
        cmp -s "$HOST_ATOMICS" "$WORK/host_$label.c" && sce_gate_fail "mutant $label changed nothing in $HOST_ATOMICS: its sed program no longer matches the file"
        # `$WORK` is already absolute (`mktemp -d`), so it is the path as it is.
        sed "s#\"../conformance/sce_atomic_host.c\"#\"$WORK/host_$label.c\"#" "$harness" >"$WORK/${name}_$label.c"
        run_genmc "$WORK/${name}_$label.c" "$WORK/${name}_$label.log" c
        judge_mutant "$harness" "$label" "$WORK/${name}_$label.log"
    done
    checked=$((checked + 1))
done

(( checked > 0 )) || sce_gate_fail "no GenMC harness found under $HARNESS_DIR"

# The C++ harnesses, when there are any; the C ones above are required.
for harness in "$CXX_HARNESS_DIR"/*.cpp; do
    [[ -e "$harness" ]] || continue
    name="$(harness_name "$harness")"
    check_baseline "$harness" cxx

    for mutant in "${CXX_MUTANTS[@]}"; do
        label="${mutant%%|*}"
        program="${mutant#*|}"
        # A copy of the queue headers under `sce/forge/`, found ahead of the real
        # include directory, with the orderings rewritten. At least one of them
        # must have changed, or the program no longer matches what the runtime
        # writes and the mutant is a copy.
        mkdir -p "$WORK/inc_$label/sce/forge"
        changed=0
        for header in "${CXX_MUTATED_HEADERS[@]}"; do
            sed "$program" "$CXX_INCLUDE_DIR/sce/forge/$header" >"$WORK/inc_$label/sce/forge/$header"
            cmp -s "$CXX_INCLUDE_DIR/sce/forge/$header" "$WORK/inc_$label/sce/forge/$header" || changed=1
        done
        (( changed == 1 )) || sce_gate_fail "C++ mutant $label changed nothing in ${CXX_MUTATED_HEADERS[*]}: its sed program no longer matches the headers"
        run_genmc "$harness" "$WORK/${name}_$label.log" cxx -I "$WORK/inc_$label"
        judge_mutant "$harness" "$label" "$WORK/${name}_$label.log"
    done
    checked=$((checked + 1))
done

echo "genmc-queue: $checked harness(es) pass and every required mutant is caught"
