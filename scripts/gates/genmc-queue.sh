#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: genmc-queue.yml
#
# The C11 queue under GenMC: layer 3 of the queue kind's verification (SCE
# Protocol-Synthesis RFC §synth-5-P). GenMC explores every execution the C11
# memory model allows, where a stress run samples the few a machine gives.
#
# TWO CLAIMS, and the second is what makes the first worth reading:
#
#   1. Each harness in backends/c/forge-runtime/tests/genmc/ passes: no execution
#      of the real runtime loses, duplicates or tears an element.
#   2. Each MUTANT is caught. A mutant is the hosted atomics
#      (`backends/c/forge-runtime/tests/conformance/sce_atomic_host.c`) with ONE
#      ordering weakened to relaxed, read through the same harness. A check that
#      cannot tell a weakened ordering from the real one checks nothing, and the
#      first harness of this gate did exactly that: it reported "No errors were
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
UNROLL=20
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# The options every run shares. The model is RC11, GenMC's default, named so a
# change of default is not a change of what is checked.
GENMC_OPTIONS=(--rc11 "--unroll=$UNROLL" --disable-estimation --print-exec-graphs)
INCLUDES=(-I backends/c/forge-runtime/include -I "$GENMC_ROOT/include/genmc/runtime")

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

# Runs GenMC on `$1` into `$2` and leaves GenMC's exit status in `$GENMC_STATUS`.
run_genmc() {
    GENMC_STATUS=0
    "$GENMC" "${GENMC_OPTIONS[@]}" -- "${INCLUDES[@]}" "$1" >"$2" 2>&1 || GENMC_STATUS=$?
}

# The verdict of a run that PASSED is only worth reading if the program ran its
# threads: the graph must hold a thread-create for each thread the harness makes.
require_threads() {
    local log="$1" want="$2" got
    got="$(grep -c 'THREAD_CREATE' "$log" || true)"
    (( got >= want )) || sce_gate_fail "$log: the explored graph has $got thread-create event(s), the harness makes $want: the verdict describes a program that never started its threads"
}

checked=0
for harness in "$HARNESS_DIR"/*.c; do
    name="$(basename "$harness" .c)"
    threads="$(sed -n 's/^\/\* THREADS: \([0-9]*\) \*\/$/\1/p' "$harness")"
    [[ -n "$threads" ]] || sce_gate_fail "$harness has no '/* THREADS: n */' line naming the threads it makes"

    run_genmc "$harness" "$WORK/$name.log"
    (( GENMC_STATUS == 0 )) || { cat "$WORK/$name.log"; sce_gate_fail "$name: GenMC found an error in the runtime (status $GENMC_STATUS)"; }
    require_threads "$WORK/$name.log" "$threads"
    grep -q 'No errors were detected' "$WORK/$name.log" || sce_gate_fail "$name: GenMC did not say it found no errors"

    for mutant in "${MUTANTS[@]}"; do
        label="${mutant%%|*}"
        program="${mutant#*|}"
        sed "$program" "$HOST_ATOMICS" >"$WORK/host_$label.c"
        cmp -s "$HOST_ATOMICS" "$WORK/host_$label.c" && sce_gate_fail "mutant $label changed nothing in $HOST_ATOMICS: its sed program no longer matches the file"
        # `$WORK` is already absolute (`mktemp -d`), so it is the path as it is.
        sed "s#\"../conformance/sce_atomic_host.c\"#\"$WORK/host_$label.c\"#" "$harness" >"$WORK/${name}_$label.c"
        run_genmc "$WORK/${name}_$label.c" "$WORK/${name}_$label.log"
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
            grep -q 'Transformation complete' "$WORK/${name}_$label.log" \
                || { cat "$WORK/${name}_$label.log"; sce_gate_fail "$name: mutant '$label' did not even reach exploration; its failure says nothing about the runtime"; }
            grep -qE 'Error: ' "$WORK/${name}_$label.log" \
                || { cat "$WORK/${name}_$label.log"; sce_gate_fail "$name: mutant '$label' exited $GENMC_STATUS without reporting a memory-model error"; }
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
    done
    checked=$((checked + 1))
done

(( checked > 0 )) || sce_gate_fail "no GenMC harness found under $HARNESS_DIR"
echo "genmc-queue: $checked harness(es) pass and every required mutant is caught"
