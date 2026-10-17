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
echo "check.sh: the models hold with the rules the code applies and fail without them"
