#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: clippy-check.yml
#
# Every workspace package declares the oldest Rust it builds with, and builds
# with it.
#
# `rust-version` is a claim made to whoever depends on the package: cargo
# refuses an older toolchain with that number, and clippy measures its
# `incompatible_msrv` and `manual_*` lints against it. A package that declares
# none is judged against whatever toolchain lints it. That is how `sce-build`
# came to need Rust 1.87 with nothing saying so: clippy on a current stable
# asked for `is_multiple_of` (`manual_is_multiple_of`), the code took it, and a
# consumer pinned to 1.85 met E0658 in five places (reported 2026-10-20). A
# declared number nothing builds with goes stale the same way: the same day the
# workspace's 1.75 could not even be read by Cargo 1.75, because a dependency's
# manifest is in the 2024 edition.
#
# So each package is built with exactly the toolchain it declares. Cargo
# itself refuses a dependency that declares a newer Rust, and rustc refuses the
# package's own use of a newer API. What is built is what a dependent gets by
# default: the library and binaries with default features, against the
# committed lockfile, for the runner's target. Not every feature at once: what
# a declared number promises a dependent is the build it gets by default, and
# an opt-out such as `sce-rust-runtime`'s `no_macrostep_diagnostics` describes
# a build no dependent gets unasked. A dependency only another target compiles is not
# checked here; reading the lockfile instead would have to evaluate every
# target's `cfg` and each package's own features, and the first attempt at it
# (2026-10-20) charged `sce-forge-runtime` for `loom`, which only its
# `cfg(loom)` test build compiles. Lints are capped to warnings: an older
# rustc warns differently from the one the workspace's `-D warnings` policy is
# held to by the clippy gate, and the claim checked here is that the package
# compiles.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

command -v rustup >/dev/null 2>&1 \
    || sce_gate_cannot_run "rustup is not on PATH; each declared toolchain is installed through it"

# `version<TAB>package` for every workspace member, `-` for one that declares
# nothing.
declarations="$(cargo metadata --no-deps --format-version 1 | python3 -I -c '
import json, sys
metadata = json.load(sys.stdin)
members = set(metadata["workspace_members"])
for package in metadata["packages"]:
    if package["id"] in members:
        print("%s\t%s" % (package.get("rust_version") or "-", package["name"]))
')" || sce_gate_fail "cargo metadata did not list the workspace"
[[ -n "$declarations" ]] || sce_gate_fail "cargo metadata listed no workspace member"

undeclared="$(awk -F'\t' '$1 == "-" { print $2 }' <<<"$declarations" | sort)"
if [[ -n "$undeclared" ]]; then
    sce_gate_fail "these packages declare no rust-version, so nothing says which Rust they need: $(tr '\n' ' ' <<<"$undeclared")"
fi

for version in $(cut -f1 <<<"$declarations" | sort -uV); do
    packages=()
    while IFS=$'\t' read -r declared name; do
        [[ "$declared" == "$version" ]] && packages+=(-p "$name")
    done <<<"$declarations"
    sce_gate_step "Rust $version: ${packages[*]}"
    rustup toolchain install "$version" --profile minimal --no-self-update >/dev/null \
        || sce_gate_cannot_run "rustup could not install Rust $version"
    RUSTFLAGS="--cap-lints warn" cargo "+$version" check --locked --lib --bins \
        --target-dir "target/rust-version/$version" "${packages[@]}" \
        || sce_gate_fail "a package that declares rust-version $version does not build with Rust $version"
done
