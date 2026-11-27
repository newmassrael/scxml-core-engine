#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Install the pinned TypeScript compiler `wasm32-scenarios` holds the page surface's declaration
# to (docs/adr/0117, D7): a page in TypeScript that raises an event its machine does not take must
# not compile, and which compiler says so is part of the verdict. A verdict that moves with the
# compiler is not one the tree decides.
#
# The unit is the VERSION: `TYPESCRIPT_REV` below is the one home of the pin, and the install path
# derives from it. Raise it in a commit of its own, with the gate's run on the new version in front
# of the person raising it. 5.9 is the last line of the compiler written in TypeScript itself: one
# archive of JavaScript that runs on the Node the lane already has. The releases from 7 on are a
# native compiler shipped as one package per platform, which this script would have to choose
# among.
#
# The npm registry publishes the SHA-512 of each archive (its `dist.integrity`); the SHA-256
# written here is that of the archive whose SHA-512 matched the registry's, read when the pin was
# set (2026-11-28), and a download that does not match it is refused before it is unpacked. The
# postcondition runs the compiler and reads its version.
#
# What it needs and does not install: curl, tar, gzip, sha256sum and Node.
#
# The build machines get it from `[platform.linux.tools.typescript]` in bx.toml, which runs this
# script with TYPESCRIPT_ROOT set to bx's store entry. The gate looks for the package at the path
# derived here.
set -euo pipefail

# The pin: a TypeScript release, and the SHA-256 of its archive on the npm registry.
TYPESCRIPT_REV="5.9.3"
TYPESCRIPT_SHA256="10e108c9cf7d5f2879053dff18515fb405abf2ccef63eaaf017d9c571687a1d3"

TYPESCRIPT_ROOT="${TYPESCRIPT_ROOT:-$HOME/.local/share/typescript/$TYPESCRIPT_REV}"
TSC="$TYPESCRIPT_ROOT/package/bin/tsc"

installed() {
    [[ -f "$TSC" ]] && [[ "$(node "$TSC" --version 2>/dev/null)" == "Version $TYPESCRIPT_REV" ]]
}

if installed; then
    echo "TypeScript $TYPESCRIPT_REV is already installed at $TYPESCRIPT_ROOT"
    exit 0
fi

for tool in curl tar gzip sha256sum node; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "install_typescript: '$tool' is not installed; see the header of this script" >&2
        exit 2
    }
done

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
url="https://registry.npmjs.org/typescript/-/typescript-$TYPESCRIPT_REV.tgz"
echo "installing TypeScript $TYPESCRIPT_REV in $TYPESCRIPT_ROOT"
curl -fsSL --retry 3 --retry-all-errors -o "$WORK/typescript.tgz" "$url"
echo "$TYPESCRIPT_SHA256  $WORK/typescript.tgz" | sha256sum -c --quiet - || {
    echo "install_typescript: the download of $TYPESCRIPT_REV does not match its pinned SHA-256" >&2
    exit 1
}
mkdir -p "$TYPESCRIPT_ROOT"
# The archive holds one directory, `package`, as every npm archive does.
tar -xzf "$WORK/typescript.tgz" -C "$TYPESCRIPT_ROOT"
# The compiler is run by its path as well as through Node, by bx's check of the tool.
chmod 0755 "$TSC"

# The postcondition, asserted rather than assumed: the compiler runs on this machine's Node and
# names the pinned release.
installed || {
    echo "install_typescript: $TSC does not run here or does not report $TYPESCRIPT_REV" >&2
    exit 1
}
echo "TypeScript $TYPESCRIPT_REV installed at $TYPESCRIPT_ROOT"
