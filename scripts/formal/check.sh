#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Check the TLA+ model of the shared list's convergence rule (docs/formal/SharedListConvergence.tla)
# with TLC, whole (every reachable state of each configuration):
#
#   * SharedListConvergence.cfg, _fork.cfg, _both.cfg: the rule as the code applies it (a host drops
#     what a later entry or a conflict puts out), for a device taken off below what a host admitted, a
#     device that signs two operations at one number, and both. TLC must find no violation.
#   * SharedListConvergence_without_eviction.cfg: the rule as it was before the model was checked against
#     the code. TLC must find the divergence. A model that cannot fail proves nothing, so this is the
#     check that the model is able to say no.
#   * SharedListConflicts.cfg: a device whose operations conflict (two at one number, a write whose time
#     goes back), with the rule the code has since an independent review: a host holds every operation
#     that holds under its log and is told what is below the lowest conflict, so that two hosts given the
#     same operations in any order are told the same. TLC must find no violation.
#   * SharedListConflicts_arrival.cfg: the rule before that review (a conflicting operation is refused
#     at the door and the device frozen from there). TLC must find the divergence the review found.
#   * BroadcastLog.cfg, BroadcastLog_r1.cfg (docs/formal/BroadcastLog.tla, RFC §synth-5-S, ADR 0062): the bounded log's
#     protocol, retention 2 and 1, one producer and two readers. TLC must find no violation.
#   * BroadcastLog_without_extra_slot.cfg, BroadcastLog_without_revalidation.cfg: the ring with only R
#     slots, and the read without its second stamp load. TLC must find an answer the specification does
#     not give (a Lagged that skipped nothing; an Item made of two entries).
#   * BroadcastLog_deferred.cfg: a reader opened by reader_deferred() (its first read binds its cursor to the
#     oldest retained entry of the head that read loads) beside an ordinary one. TLC must find no violation,
#     of the existing invariants or of DeferredStart (its start is that entry, and what it was told it lost
#     lies after it) and FirstLagIsOvertaken (its first read answers Lagged only if the head had moved).
#     BroadcastLog_deferred_without_extra_slot.cfg: the same reader over R slots. TLC must find a first
#     read told of a loss that no publish had caused.
#   * BroadcastLogWrapped.cfg, _r3.cfg, _oldest.cfg (docs/formal/BroadcastLogWrapped.tla, ADR 0062
#     "Stamp width"): the narrow row, whose 3-bit stamp wraps, with one reader that opens at the head, one
#     whose log keeps as many slots as half the stamp space, and one that opens at the oldest entry, each
#     under the wrap bound. TLC must find no violation of AnswerRefines (order, wholeness, exact gaps),
#     CursorNotPastHead, LostIsNonNegative, SlotIndexContinuous or OpenRefines.
#     BroadcastLogWrapped_deferred.cfg: a reader that opens by its first read (the two loads of opening at
#     the oldest are the start of that read), under the same bound, which here also holds the log to K
#     publishes past the reader's first load. TLC must find no violation, DeferredStart and
#     FirstLagIsOvertaken among the invariants.
#     BroadcastLogWrapped_deferred_without_bound.cfg: the same without the bound. TLC must find an Empty
#     answered for a head that is a whole stamp space ahead, between the two loads of the open.
#   * BroadcastLogWrapped_without_bound.cfg: the same log whose reader may be held for as long as it
#     likes. TLC must find the entry of a later lap handed as the one the reader wanted.
#     BroadcastLogWrapped_non_divisor.cfg: a slot count (3) that does not divide the stamp space (8). TLC
#     must find a reader whose cursor, known only modulo the stamp space, names another slot than the
#     publisher's. Together they are the check that the bound and the divisor are needed.
#
# Needs Java 11 or later. TLC is fetched once into ~/.cache/sce-formal and checked against a pinned
# SHA-256, or taken from $TLA2TOOLS.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../docs/formal" && pwd)"
jar="${TLA2TOOLS:-$HOME/.cache/sce-formal/tla2tools.jar}"
url="https://github.com/tlaplus/tlaplus/releases/download/v1.8.0/tla2tools.jar"
pinned="15d103b7e5617780bcc904c0af12f1ad242e2b345c0301136c5be4588f17edac"

if [[ ! -f "$jar" ]]; then
    mkdir -p "$(dirname "$jar")"
    curl -fsSL -o "$jar" "$url"
fi
if [[ -z "${TLA2TOOLS:-}" ]]; then
    actual="$(sha256sum "$jar" | cut -d' ' -f1)"
    if [[ "$actual" != "$pinned" ]]; then
        echo "check.sh: $jar is not the TLC release this script pins ($actual)" >&2
        exit 2
    fi
fi

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
cp "$here"/*.tla "$here"/*.cfg "$work"/
cd "$work"

tlc() {
    java -XX:+UseParallelGC -cp "$jar" tlc2.TLC -workers auto -config "$1" "${2:-SharedListConvergence}.tla"
}

for config in SharedListConvergence SharedListConvergence_fork SharedListConvergence_both; do
    echo "== $config: the rule as the code applies it, no violation expected"
    if ! tlc "$config.cfg"; then
        echo "check.sh: TLC found a violation of the rule the code applies ($config)" >&2
        exit 1
    fi
done

echo "== the rule without the dropping: a violation expected"
set +e
tlc SharedListConvergence_without_eviction.cfg > without.log 2>&1
status=$?
set -e
tail -n 25 without.log
if [[ "$status" -ne 12 ]]; then
    echo "check.sh: expected TLC to report an invariant violation (exit 12), got $status" >&2
    exit 1
fi
echo "== conflicts of a device (two at a number, a time that goes back): the set rule, no violation expected"
if ! tlc SharedListConflicts.cfg SharedListConflicts; then
    echo "check.sh: TLC found a violation of the rule the code applies to conflicts" >&2
    exit 1
fi

echo "== conflicts of a device refused at the door: a violation expected"
set +e
tlc SharedListConflicts_arrival.cfg SharedListConflicts > arrival.log 2>&1
status=$?
set -e
tail -n 25 arrival.log
if [[ "$status" -ne 12 ]]; then
    echo "check.sh: expected TLC to report an invariant violation (exit 12), got $status" >&2
    exit 1
fi
for config in BroadcastLog BroadcastLog_r1 BroadcastLog_deferred; do
    echo "== $config: the bounded log of ADR 0062 as designed, no violation expected"
    if ! tlc "$config.cfg" BroadcastLog; then
        echo "check.sh: TLC found a violation of the bounded log's protocol ($config)" >&2
        exit 1
    fi
done

for config in BroadcastLog_without_extra_slot BroadcastLog_without_revalidation BroadcastLog_deferred_without_extra_slot; do
    echo "== $config: a design the ADR rejects, a violation expected"
    set +e
    tlc "$config.cfg" BroadcastLog > "$config.log" 2>&1
    status=$?
    set -e
    tail -n 25 "$config.log"
    if [[ "$status" -ne 12 ]]; then
        echo "check.sh: expected TLC to report an invariant violation (exit 12) for $config, got $status" >&2
        exit 1
    fi
done
for config in BroadcastLogWrapped BroadcastLogWrapped_r3 BroadcastLogWrapped_oldest BroadcastLogWrapped_deferred; do
    echo "== $config: the narrow bounded log under the wrap bound, no violation expected"
    if ! tlc "$config.cfg" BroadcastLogWrapped; then
        echo "check.sh: TLC found a violation of the narrow log's protocol ($config)" >&2
        exit 1
    fi
done

for config in BroadcastLogWrapped_without_bound BroadcastLogWrapped_non_divisor BroadcastLogWrapped_deferred_without_bound; do
    echo "== $config: a narrow log the ADR refuses, a violation expected"
    set +e
    tlc "$config.cfg" BroadcastLogWrapped > "$config.log" 2>&1
    status=$?
    set -e
    tail -n 25 "$config.log"
    if [[ "$status" -ne 12 ]]; then
        echo "check.sh: expected TLC to report an invariant violation (exit 12) for $config, got $status" >&2
        exit 1
    fi
done
echo "check.sh: the models hold with the rules the code applies and fail without them"
