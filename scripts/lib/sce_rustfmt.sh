#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Format the Rust files a regeneration script just wrote, and only those.
#
# rustfmt is part of the committed form of the generated trees —
# `backends/rust/tests` is a workspace member, `cargo fmt --all` reformats
# it, and `fmt-check.yml` requires that state — so every regen script has to
# end in a format pass. Thirteen of them ended in `cargo fmt -p
# sce-rust-tests`, which formats the WHOLE package, and that is a coupling
# rather than a convenience: rustfmt resolves `mod` declarations, so a file
# missing anywhere under the package aborts the format step of a script that
# has nothing to do with it. Measured: deleting one integration artifact
# made `regen_autoforward_dequeue_point.sh` fail with
# "failed to resolve mod `autoforward_done_invoke__sce_synth_invoke__inv_watch_sm`"
# — a tree it does not own — and the script that WOULD have restored that
# file never ran, because the umbrella stops at the first failure.
#
# Scoping the pass to the directory a script just wrote removes the
# dependency in both directions and drops twelve redundant whole-package
# formats from an umbrella run. The umbrella still ends with the
# whole-package pass, so nothing loses coverage.
#
# The edition is read from the workspace manifest rather than typed here:
# `cargo fmt` derives it from the manifest, and a standalone rustfmt that
# guesses a different one produces different bytes. Measured on this tree,
# the two agree exactly — 654 W3C artifacts and 39 integration artifacts
# reformat to zero diff — which is what makes the substitution safe.

sce_rustfmt_edition() {
    local root="${1:-$(git rev-parse --show-toplevel)}"
    local edition
    edition="$(sed -n 's/^edition[[:space:]]*=[[:space:]]*"\([0-9]*\)".*/\1/p' \
        "$root/Cargo.toml" | head -1)"
    if [[ -z "$edition" ]]; then
        echo "sce_rustfmt: no edition in $root/Cargo.toml" >&2
        return 1
    fi
    printf '%s\n' "$edition"
}

# Format every `.rs` directly inside DIR. Not recursive: a regen script
# writes one directory, and formatting below it would reach trees it does
# not own — the behaviour this exists to stop.
sce_rustfmt_dir() {
    local dir="$1"
    local root="${2:-$(git rev-parse --show-toplevel)}"
    local edition
    edition="$(sce_rustfmt_edition "$root")" || return 1
    mapfile -t files < <(find "$dir" -maxdepth 1 -name '*.rs' -type f | sort)
    if (( ${#files[@]} == 0 )); then
        echo "sce_rustfmt: no .rs files in $dir" >&2
        return 1
    fi
    rustfmt --edition "$edition" --quiet "${files[@]}"
}
