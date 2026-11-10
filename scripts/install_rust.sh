#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Install one Rust toolchain, at a pinned release and with the components and
# targets asked for, into a directory of its own.
#
#   scripts/install_rust.sh <toolchain> <prefix> [-c <component>]... [-t <target>]...
#   scripts/install_rust.sh --declared <toolchain>
#
# The first form is the install of the `[platform.linux.tools.rust*]` tables of
# bx.toml: `bx-provision <alias> --apply` copies this script to the machine
# (`install_inputs`) and runs the table's install line, which sets RUSTUP_HOME and
# CARGO_HOME to directories under the install's scratch and hands it the store
# entry as <prefix>. bx links the entry at ~/.local/rust/<toolchain>, where the
# gates look (scripts/lib/rust_toolchains.sh).
#
# The second form is the same install anywhere else: a developer's machine, or a
# hosted runner whose gate found a toolchain missing. It reads the install line of
# the table that declares <toolchain>, so the components and targets are the ones
# the build machines get, and runs it with homes of its own and the prefix the
# gates look at. A toolchain already there is left alone.
#
# WHY A HOME OF ITS OWN. The toolchain is installed by a rustup that lives under
# the install's scratch and nowhere else, and only the toolchain's directory is
# kept. rustup's own home is shared by everything an account runs, and is not
# safe against two installs at once: on 2026-10-20 a build machine's `stable`,
# which the gates had synced in parallel, was left with rustc and rust-std and no
# cargo. A toolchain copied out of a private home is one directory nothing else
# writes to, and its rustc finds its standard library beside itself, so it runs
# with no rustup at all.
#
# rustup itself is pinned by release and digest, the way the build machines'
# other tools are, so the install fetches nothing a digest does not check before
# it runs: rustup-init for x86_64 Linux built against musl, which runs on any
# glibc. The toolchain it installs is the host's, x86_64-unknown-linux-gnu, the
# one platform bx.toml declares.
#
# What it needs and does not install: curl, sha256sum, and network access to
# static.rust-lang.org.
set -euo pipefail

RUSTUP_INIT_REV="1.29.1"
RUSTUP_INIT_TRIPLE="x86_64-unknown-linux-musl"
RUSTUP_INIT_SHA256="331228566cc931f32cd684f9bebc5956a5e5d7394c8d5de595f196a658ab98e6"
HOST="x86_64-unknown-linux-gnu"

refuse() {
    echo "install_rust: $1" >&2
    exit 2
}

# --declared <toolchain>: the install line of the table that declares it, run
# with a prefix beside the place the gates look and moved there when it has
# ended, so that a failed install leaves nothing there.
if [[ "${1:-}" == "--declared" ]]; then
    [[ $# -eq 2 && -n "$2" ]] || refuse "--declared takes one toolchain, e.g. 1.99.1 or nightly-2026-10-08"
    want="$2"
    root="$(git rev-parse --show-toplevel)"
    # shellcheck source=lib/rust_toolchains.sh
    source "$root/scripts/lib/rust_toolchains.sh"
    if sce_rust_toolchain_bin "$want" >/dev/null; then
        echo "install_rust: Rust $want is already installed at $(sce_rust_toolchain_dir "$want")"
        exit 0
    fi
    final="$(sce_rust_toolchain_dir "$want")"
    if [[ -e "$final" || -L "$final" ]]; then
        refuse "$final is there and runs no cargo; it is left as it is (remove it to install again)"
    fi
    matches="$(sce_rust_declared_toolchains "$root" | awk -F'\037' -v want="$want" '$2 == want')"
    [[ -n "$matches" ]] || refuse "bx.toml declares no Rust toolchain $want (a [platform.linux.tools.rust*] table whose install runs this script)"
    [[ "$matches" != *$'\n'* ]] || refuse "bx.toml declares Rust $want in more than one table"
    IFS=$'\x1f' read -r table _ install <<<"$matches"

    scratch="$(mktemp -d)"
    prefix="$final.installing.$$"
    trap 'rm -rf "$scratch" "$prefix"' EXIT
    mkdir -p "$prefix"
    cmd="${install//\{version\}/$want}"
    cmd="${cmd//\{prefix\}/$prefix}"
    cmd="${cmd//\{scratch\}/$scratch}"
    echo "install_rust: [platform.linux.tools.$table] of bx.toml: $cmd"
    (cd "$root" && bash -c "$cmd" </dev/null)
    mv -T "$prefix" "$final"
    trap 'rm -rf "$scratch"' EXIT
    sce_rust_toolchain_bin "$want" >/dev/null \
        || refuse "the install ended and $final/bin/cargo does not run"
    echo "install_rust: Rust $want is installed at $final"
    exit 0
fi

[[ $# -ge 2 ]] || refuse "usage: $0 <toolchain> <prefix> [-c <component>]... [-t <target>]... | --declared <toolchain>"
toolchain="$1"
prefix="$2"
shift 2

[[ "$toolchain" =~ ^([0-9]+\.[0-9]+\.[0-9]+|nightly-[0-9]{4}-[0-9]{2}-[0-9]{2})$ ]] \
    || refuse "the toolchain is a release (1.99.1) or a dated nightly (nightly-2026-10-08), never a channel that moves: got '$toolchain'"
[[ -d "$prefix" ]] || refuse "the prefix $prefix is not a directory"
[[ -n "${RUSTUP_HOME:-}" && -n "${CARGO_HOME:-}" ]] \
    || refuse "RUSTUP_HOME and CARGO_HOME must name the install's own directories, never the account's"
[[ "$(uname -s) $(uname -m)" == "Linux x86_64" ]] \
    || refuse "rustup-init is pinned for x86_64 Linux, and this is $(uname -s) $(uname -m)"
for tool in curl sha256sum; do
    command -v "$tool" >/dev/null 2>&1 || refuse "'$tool' is not installed; see the header of this script"
done

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

init="$work/rustup-init"
curl -fsSL --retry 3 --retry-all-errors -o "$init" \
    "https://static.rust-lang.org/rustup/archive/$RUSTUP_INIT_REV/$RUSTUP_INIT_TRIPLE/rustup-init"
echo "$RUSTUP_INIT_SHA256  $init" >"$init.sha256"
sha256sum -c --quiet "$init.sha256" \
    || refuse "rustup-init $RUSTUP_INIT_REV does not have the SHA-256 this script pins"
chmod +x "$init"

# `--default-toolchain none`: the private rustup holds the one toolchain named
# below and nothing else. The path check is skipped because the account's own
# Rust on PATH is not what this installs or touches.
RUSTUP_INIT_SKIP_PATH_CHECK=yes "$init" -y --no-modify-path --profile minimal --default-toolchain none
"$CARGO_HOME/bin/rustup" toolchain install "$toolchain" --profile minimal --no-self-update "$@"

installed="$RUSTUP_HOME/toolchains/$toolchain-$HOST"
[[ -x "$installed/bin/cargo" ]] || refuse "rustup installed no $installed/bin/cargo"
cp -a "$installed/." "$prefix/"
"$prefix/bin/cargo" --version
