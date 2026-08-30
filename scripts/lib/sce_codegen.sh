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

# Pin the `generated-at` header stamp for every generator run this shell
# makes, unless the caller already chose a value.
#
# `docs/SCE_CODEGEN_DETERMINISM.md` §1 names `SOURCE_DATE_EPOCH=0` as half of
# what makes "regenerate and expect no diff" true, and
# `committed_trees_carry_a_pinned_generated_at` rejects a committed file whose
# stamp is anything else. Only `regen_all_committed_trees.sh` exported it, so
# all 115 per-stem `regen_<stem>*.sh` scripts stamped wall-clock: regenerating
# one fixture — the normal shape of a round that changes one fixture — put a
# dirty header on every file it touched and the drift gate rejected the push.
# Measured 2026-08-21, and it had cost a push cycle the day before.
#
# Here rather than in each script, because this file is the one thing all 115
# already source, and a 115-line change is the same list-shaped defect: the
# next regen script written would be the one that forgot.
#
# The default is only a default: `SOURCE_DATE_EPOCH=1234 scripts/regen_x.sh`
# still stamps 1234, so a caller that wants a real time can still say so.
export SOURCE_DATE_EPOCH="${SOURCE_DATE_EPOCH:-0}"

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
# holds one *or when the one it holds was built from other sources*.
# Build output goes to stderr so the path stays the only thing on stdout
# and `$(...)` capture stays usable.
#
# "Or when the one it holds was built from other sources" is the whole
# difference between a regeneration and a re-stamp. This helper used to
# hand back whatever binary existed, so `regen_all_committed_trees.sh`
# refreshed the W3C trees with a generator predating the very edit being
# regenerated for — and then rebuilt, mid-script, for a later phase that
# happened to go through cargo, leaving one commit holding trees from two
# different generators. The binary carries a witness for exactly this
# question and `verify-generator` reads it (`cmake/SCEFindCodegen.cmake`
# already asks it at configure time); asking it here costs one process
# and makes "regenerate and expect no diff" true of a tree whose sources
# just moved.
# Wrap a resolved binary so every `generate` it is asked for names a
# script-engine language, and print the wrapper's path.
#
# `generate-integration` dispatches to 38 per-stem regen scripts, and each one
# spells its own `sce-codegen generate` invocation — some of them twice, for a
# synth-invoke child. Threading a `--script-engine` flag through all of them
# would put ONE decision in 38 places, which is the drift this repository has
# already paid for in this exact seam. They share one thing, and it is the
# binary this file resolves for them, so that is where the selection goes.
#
# ⚠ It rewrites `generate` and nothing else. `verify-generator` — which this
# file itself calls, and `cmake/SCEFindCodegen.cmake` calls at configure time —
# must reach the real binary untouched, and so must `generate-w3c`, which
# carries the flag as a first-class argument already.
#
# ⚠⚠ A caller that named the language itself WINS. Appending a second
# `--script-engine` would leave which one applies to clap's last-wins rule,
# which is a decision nobody wrote down; a script that already made the choice
# keeps it.
# Which backend's emissions the script-engine selection applies to.
#
# ⚠ REFUSED when unset rather than defaulted to `kotlin`. A default would make
# the pairing invisible at exactly the site that has to state it, and the
# script this scope exists for emits three backends from one file — so "which
# one did I mean" is a question with a wrong answer available.
sce_codegen_shim_backend() {
    if [[ -z "${SCE_SCRIPT_ENGINE_FOR:-}" ]]; then
        echo "error: SCE_SCRIPT_ENGINE=$SCE_SCRIPT_ENGINE was set without" >&2
        echo "  SCE_SCRIPT_ENGINE_FOR naming the backend it applies to. A regen" >&2
        echo "  script may emit several backends from one file and only one of" >&2
        echo "  them can take that selection." >&2
        return 1
    fi
    printf '%s\n' "$SCE_SCRIPT_ENGINE_FOR"
}

# ⚠⚠⚠ SCOPED TO ONE BACKEND LANGUAGE, and that is not caution either.
# Measured 2026-08-30: `scripts/regen_native_action.sh` emits the Rust, Go AND
# Kotlin trees from a single script, so a shim that rewrote every `generate`
# handed Rust `--script-engine ecmascript` and the generator refused —
# *"backend 'rust' cannot emit for --script-engine ecmascript"*. The selection
# belongs to the emissions for ONE backend, exactly as
# `SCE_KOTLIN_GENERATED_ROOT` does, so the scope is read from the `-l` the
# caller already spells rather than guessed.
sce_codegen_shim() {
    local real="$1" engine="$2" root="$3" backend="$4"
    local shim="$root/target/sce-codegen-shim-$backend-$engine"
    cat >"$shim" <<SHIM
#!/usr/bin/env bash
set -euo pipefail
if [ "\${1:-}" = "generate" ]; then
    scoped=0
    previous=""
    for arg in "\$@"; do
        if [ "\$arg" = "--script-engine" ]; then
            exec "$real" "\$@"
        fi
        if [ "\$previous" = "-l" ] || [ "\$previous" = "--language" ]; then
            if [ "\$arg" = "$backend" ]; then
                scoped=1
            fi
        fi
        previous="\$arg"
    done
    if [ "\$scoped" = "1" ]; then
        shift
        exec "$real" generate "\$@" --script-engine "$engine"
    fi
fi
exec "$real" "\$@"
SHIM
    chmod +x "$shim"
    printf '%s\n' "$shim"
}

sce_codegen_require() {
    local root="${1:-$(git rev-parse --show-toplevel)}"
    local path
    if path="$(sce_codegen_path "$root")" \
        && "$path" verify-generator --root "$root" >/dev/null 2>&1; then
        if [[ -n "${SCE_SCRIPT_ENGINE:-}" ]]; then
            sce_codegen_shim "$path" "$SCE_SCRIPT_ENGINE" "$root" \
                "$(sce_codegen_shim_backend)"
            return 0
        fi
        printf '%s\n' "$path"
        return 0
    fi
    # The build below produces the debug binary only, and `sce_codegen_path`
    # searches debug first and release second — so a release binary left by
    # an older tree would be found *after* this build and never used. It
    # would be used, though, if this build failed to produce anything, and
    # on a CI runner with a restored cache it is the stale one. Dropping it
    # first is what the workflows that spell this build out do; a caller
    # that reaches the binary through this helper gets the same guarantee
    # rather than inheriting the risk the workflow step was written to
    # avoid.
    sce_codegen_drop_stale_release "$root"
    (cd "$root" && cargo build --bin sce-codegen --features cli -p sce-build) >&2
    if path="$(sce_codegen_path "$root")"; then
        # Both exits shim, and that is not tidiness: the branch above is taken
        # on a warm tree and this one on a cold one, so a selection honoured by
        # only one of them would depend on whether `target/` happened to hold a
        # current binary.
        if [[ -n "${SCE_SCRIPT_ENGINE:-}" ]]; then
            sce_codegen_shim "$path" "$SCE_SCRIPT_ENGINE" "$root" \
                "$(sce_codegen_shim_backend)"
            return 0
        fi
        printf '%s\n' "$path"
        return 0
    fi
    echo "error: sce-codegen not found under $root/target/{debug,release} and" >&2
    echo "  the build produced no binary. Build it with:" >&2
    echo "  cargo build --bin sce-codegen --features cli -p sce-build" >&2
    return 1
}

# Delete a release binary that the build about to run will not produce.
#
# Call this before a step that builds the debug profile alone while a
# restored cache may still hold `target` from an earlier run. Release is
# second in the search order above, so a stale release binary cannot
# outrank a fresh debug one — but a debug-only build that finds no debug
# binary yet leaves release as the only candidate, and every locator
# would then hand out a binary predating the checkout. That is how CI
# once ran a generator older than a filter its own templates use and
# failed with "unknown filter: unsupported" against a self-consistent
# source tree.
#
# It lives here because this file is one of the four locators allowed to
# know that `target/release` can hold a binary at all. Spelling the path
# into each workflow instead is what `codegen_binary_resolution.rs`
# forbids, and for the reason the header describes: a build-layout
# detail repeated across files moves at some of them and not the rest.
sce_codegen_drop_stale_release() {
    local root="${1:-$(git rev-parse --show-toplevel)}"
    rm -f "$root/target/release/sce-codegen"
}
