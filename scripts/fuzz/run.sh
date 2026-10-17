#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Run the coverage-guided fuzzer (libFuzzer, through cargo-fuzz) on what the trust, relay and group key
# code of a shared list read (backends/rust/assurance/fuzz). The targets are those the seeded tests of
# backends/rust/assurance run; here an input that reaches code no earlier input reached is kept and
# changed further, so that a night finds what a seeded run of a push cannot.
#
#   scripts/fuzz/run.sh [--seconds N] [target ...]
#
# With no target, every target runs for N seconds each (default 60). Each starts from what the target
# reads when it is true (exported by the `export_seed_corpus` example). What the fuzzer keeps is in
# ${SCE_FUZZ_HOME:-~/.cache/sce-fuzz}/<target>, so a later run continues from it; an input that made a
# target panic is left in the crate's fuzz/artifacts/<target>/ and the script exits 1.
#
# Needs a nightly toolchain and `cargo install cargo-fuzz --locked`.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
crate="$root/backends/rust/assurance"
home="${SCE_FUZZ_HOME:-$HOME/.cache/sce-fuzz}"
seconds=60
targets=()

while [[ $# -gt 0 ]]; do
    case "$1" in
        --seconds)
            seconds="$2"
            shift 2
            ;;
        *)
            targets+=("$1")
            shift
            ;;
    esac
done

if ! command -v cargo-fuzz >/dev/null 2>&1; then
    echo "run.sh: cargo-fuzz is not installed (cargo install cargo-fuzz --locked)" >&2
    exit 2
fi

if [[ ${#targets[@]} -eq 0 ]]; then
    for file in "$crate"/fuzz/fuzz_targets/*.rs; do
        name="$(basename "$file" .rs)"
        targets+=("${name//_/-}")
    done
fi

seeds="$(mktemp -d)"
trap 'rm -rf "$seeds"' EXIT
(cd "$crate" && cargo run --quiet --example export_seed_corpus -- "$seeds" >/dev/null)

status=0
for name in "${targets[@]}"; do
    file="${name//-/_}"
    corpus="$home/$name"
    mkdir -p "$corpus"
    cp -n "$seeds/$name"/* "$corpus"/
    echo "== $name: ${seconds}s from $(find "$corpus" -type f | wc -l) inputs"
    if ! (cd "$crate" && cargo +nightly fuzz run "$file" "$corpus" -- \
        -max_total_time="$seconds" -rss_limit_mb=4096 -print_final_stats=1); then
        echo "run.sh: $name found an input that makes it fail; see $crate/fuzz/artifacts/$file/" >&2
        status=1
    fi
done
exit "$status"
