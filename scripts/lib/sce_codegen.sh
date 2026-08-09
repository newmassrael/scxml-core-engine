#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Single source of truth for locating the sce-codegen binary from shell.
# Source it, then call `sce_codegen_require`:
#
#   REPO_ROOT="$(git rev-parse --show-toplevel)"
#   source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
#   CODEGEN="$(sce_codegen_require "$REPO_ROOT")"
#
# Search order is debug first, release second — the same order
# cmake/SCEFindCodegen.cmake, the Gradle builds and the Python harness
# use. Debug leads because that is the profile every build path in this
# repository now produces: the generator's cost is process start-up and
# I/O rather than optimisation, so a release build only compiles the
# dependency tree a second time instead of sharing the one clippy and
# the test suite already produced. Release stays in the search path so a
# tree still holding an older release build keeps working, and it is
# looked at second so a stale binary cannot outrank a fresh one.
#
# Every consumer resolves both profiles rather than naming one, because
# naming one is what broke: the profile is a build-layout detail that
# was spelled out independently at ~100 sites across five languages,
# and moving it moved only some of them. `codegen_binary_resolution.rs`
# fails if a profile-specific path reappears outside the four locators.

# Print the path of an existing sce-codegen binary, or return 1.
sce_codegen_path() {
    local root="${1:-$(git rev-parse --show-toplevel)}"
    local candidate
    for candidate in "$root/target/debug/sce-codegen" \
                     "$root/target/release/sce-codegen"; do
        if [[ -x "$candidate" ]]; then
            printf '%s\n' "$candidate"
            return 0
        fi
    done
    return 1
}

# Print the path of the sce-codegen binary, building it when no profile
# holds one. Build output goes to stderr so the path stays the only
# thing on stdout and `$(...)` capture stays usable.
sce_codegen_require() {
    local root="${1:-$(git rev-parse --show-toplevel)}"
    local path
    if path="$(sce_codegen_path "$root")"; then
        printf '%s\n' "$path"
        return 0
    fi
    (cd "$root" && cargo build --bin sce-codegen --features cli -p sce-build) >&2
    if path="$(sce_codegen_path "$root")"; then
        printf '%s\n' "$path"
        return 0
    fi
    echo "error: sce-codegen not found under $root/target/{debug,release} and" >&2
    echo "  the build produced no binary. Build it with:" >&2
    echo "  cargo build --bin sce-codegen --features cli -p sce-build" >&2
    return 1
}
