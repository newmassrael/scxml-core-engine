# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The Rust toolchains the gates run on: the two pins, and where a declared
# toolchain is.
#
# EVERY TOOLCHAIN IS DECLARED TO bx, AND NO GATE INSTALLS ONE (2026-11-10). bx.toml
# has a `[platform.linux.tools.rust*]` table for each toolchain a gate uses: the
# release below, the nightly below, and the release each workspace package
# declares as its `rust-version`. `bx-provision <alias> --apply` installs each
# once per build machine with `scripts/install_rust.sh`, checks it, and links it
# at ~/.local/rust/<toolchain>; bx places a unit only on a machine that has the
# toolchains its `needed_by` names. Until then the gates ran `rustup toolchain
# install`, `rustup target add` and a sync of "stable" themselves, in the one
# account where a build machine runs the units of a set side by side, and rustup
# is not safe against that: on 2026-10-20 a build machine's `stable` was left
# with rustc and rust-std and no cargo, and a landing was refused over a target
# that one unit had and another had not yet added.
#
# Anywhere else the same install puts a toolchain at the same place:
# `scripts/install_rust.sh --declared <toolchain>` runs the install line of the
# table that declares it. A hosted runner is one job in one account, so a gate
# there installs what is missing that way (`sce_gate_require_rust` in
# scripts/gates/lib.sh); a developer's machine is told the command.
#
# Sourced; sets the two pins and defines functions only.

# The release every gate unit runs on: the plan writes it into each unit
# (`scripts/gate --rust <release>`), so it is part of the key bx keeps a unit's
# pass under, and `[platform.linux.tools.rust]` in bx.toml reads it from this
# line. Moving it is this one edit; the build machines then need the new
# release (`bx-provision <alias> --apply`) before a unit can be placed. The
# hosted lanes run `dtolnay/rust-toolchain@stable`, which moves on its own, so
# a lint a newer stable adds can turn a lane red before it reaches this pin.
SCE_RUST_RELEASE="1.99.1"

# The nightly, pinned by date, for what only a nightly does: the queue runtime
# under Miri and ThreadSanitizer (`forge-rust`, and the mutation casefiles that
# read `scripts/lib/queue_sanitizer.sh`), and the standard library rebuilt with
# atomics for the threaded emscripten tree (`scripts/lib/wasm_rust.sh`).
# `[platform.linux.tools.rust-nightly]` in bx.toml reads it from this line, and
# so does cmake/SCEBuildLowering.cmake, so it stays a plain `NAME="value"`
# assignment. One nightly for both: two pins of one date would be two tables at
# one place (~/.local/rust/<toolchain>), and a move of either re-runs both uses.
# The reasoning for the date is beside the commands that use it, in
# scripts/gates/forge-rust.sh and scripts/lib/wasm_rust.sh.
SCE_RUST_NIGHTLY="nightly-2026-10-08"

# Where a declared toolchain is: the `prefix_link` of its table in bx.toml,
# `~/.local/rust/<toolchain>`.
sce_rust_toolchain_dir() {
    printf '%s/.local/rust/%s\n' "$HOME" "$1"
}

# The bin directory of declared toolchain `$1`, on stdout, when it is there and
# its cargo runs; status 1 and nothing printed otherwise.
sce_rust_toolchain_bin() {
    local bin
    bin="$(sce_rust_toolchain_dir "$1")/bin"
    [[ -x "$bin/cargo" && -x "$bin/rustc" ]] || return 1
    "$bin/cargo" --version >/dev/null 2>&1 || return 1
    printf '%s\n' "$bin"
}

# The release a package's `rust-version` names: `1.87` is `1.87.0`, the oldest
# compiler the declaration admits, which is the one that holds it to its word.
sce_rust_msrv_release() {
    if [[ "$1" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        printf '%s\n' "$1"
    elif [[ "$1" =~ ^[0-9]+\.[0-9]+$ ]]; then
        printf '%s.0\n' "$1"
    else
        return 1
    fi
}

# Whether this is a hosted runner: one job in one account, whose toolchain is the
# lane's own setup, and the one place a gate may install what is missing.
sce_rust_on_hosted_runner() {
    [[ "${GITHUB_ACTIONS:-}" == true ]]
}

# The Rust toolchains bx.toml declares, one per line as `<table> <version>
# <install>` separated by the unit separator (0x1f): every
# `[platform.linux.tools.<table>]` whose install runs scripts/install_rust.sh,
# its version written out or read from its `version_file` with its
# `version_pattern`, as bx reads it. `$1` is the repository root.
#
# The table is read by its lines, as bx's shell reads it: each value on one line,
# a quoted string or an array, and only the strings are read here.
sce_rust_declared_toolchains() {
    local root="$1" name version file pattern install
    while IFS=$'\x1f' read -r name version file pattern install; do
        if [[ -z "$version" && -n "$file" && -n "$pattern" ]]; then
            version="$(sed -nE "s|${pattern}.*|\\1|p" "$root/$file" | head -n 1)"
        fi
        [[ -n "$version" ]] || continue
        printf '%s\x1f%s\x1f%s\n' "$name" "$version" "$install"
    done < <(awk '
        function value(line) {
            sub(/^[a-z_]+ = /, "", line)
            return substr(line, 2, length(line) - 2)
        }
        function flush() {
            if (name != "" && install ~ /scripts\/install_rust\.sh/)
                printf "%s\037%s\037%s\037%s\037%s\n", name, version, file, pattern, install
        }
        /^\[/ {
            flush()
            name = ""; version = ""; file = ""; pattern = ""; install = ""
            if ($0 ~ /^\[platform\.linux\.tools\.[A-Za-z0-9_-]+\]$/)
                name = substr($0, 23, length($0) - 23)
            next
        }
        name == "" { next }
        /^version = /         { version = value($0) }
        /^version_file = /    { file = value($0) }
        /^version_pattern = / { pattern = value($0) }
        /^install = /         { install = value($0) }
        END { flush() }
    ' "$root/bx.toml")
}
