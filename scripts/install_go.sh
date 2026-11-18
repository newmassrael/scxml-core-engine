#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Install the Go release this repository's Go modules declare, from the official
# release tarball, checked against the SHA-256 go.dev publishes for it.
#
#     scripts/install_go.sh [<release>]
#
# The release is the `go` line of backends/go/forge-runtime/go.mod
# (`SCE_GO_VERSION` in scripts/lib/go_toolchain.sh says why that line). bx gives
# it as the argument, read from that line by `[platform.linux.tools.go]` in
# bx.toml; run by hand without one, the script reads the same line through the
# same library.
#
# WHY A DECLARED GO (2026-11-19). The build machines carry the distribution's Go
# 1.22.2 and the modules declare 1.23.0, so every Go run there relied on
# GOTOOLCHAIN=auto fetching go1.23.0 from the network at run time; a machine that
# cannot fetch it failed every Go run. Installed here once per machine, the
# release is the machine's, and the runs use it with GOTOOLCHAIN=local.
#
# The tarball unpacks to GO_ROOT (default ~/.local/share/go/<release>, where the
# runs look for it), which becomes its GOROOT. Beside its `bin/` the script
# writes `sce-go-version`, what bx runs to tell the toolchain is installed: bx
# asks `<path> --version`, and `go --version` prints its usage, not its release.
#
# What it needs and does not install: curl, tar and sha256sum, and access to
# go.dev.
set -euo pipefail

# The SHA-256 of each archive this script may install, as go.dev publishes it
# (https://go.dev/dl/?mode=json&include=all, the `sha256` of the file; taken
# 2026-11-19). A release or an architecture with no line here is refused by name
# rather than downloaded unchecked: moving the `go` line of the modules' go.mod
# files to another release adds that release's line here, taken the same way, in
# the same commit. The registry's self-test holds this list to the go.mod line.
GO_ARCHIVE_SHA256=(
    "go1.23.0.linux-amd64.tar.gz 905a297f19ead44780548933e0ff1a1b86e8327bb459e92f9c0012569f76f5e3"
)

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
version="${1:-}"
if [[ -z "$version" ]]; then
    # shellcheck disable=SC1091
    source "$REPO_ROOT/scripts/lib/go_toolchain.sh"
    version="$SCE_GO_VERSION"
    [[ -n "$version" ]] || {
        echo "install_go: backends/go/forge-runtime/go.mod states no \`go x.y.z\` line, and no release was given" >&2
        exit 2
    }
fi
if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "install_go: '$version' is not a Go release such as 1.23.0" >&2
    exit 2
fi

GO_ROOT="${GO_ROOT:-$HOME/.local/share/go/$version}"
PROBE="$GO_ROOT/sce-go-version"

if [[ -x "$PROBE" ]] && "$PROBE" --version 2>/dev/null | grep -F "go version go$version " >/dev/null; then
    echo "Go $version is already installed at $GO_ROOT"
    exit 0
fi

case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) platform="linux-amd64" ;;
    Linux-aarch64) platform="linux-arm64" ;;
    *)
        echo "install_go: no Go archive is known for $(uname -s) $(uname -m)" >&2
        exit 1
        ;;
esac
archive="go$version.$platform.tar.gz"
sum=""
for entry in "${GO_ARCHIVE_SHA256[@]}"; do
    if [[ "${entry%% *}" == "$archive" ]]; then
        sum="${entry#* }"
    fi
done
if [[ -z "$sum" ]]; then
    echo "install_go: no SHA-256 is pinned for $archive; take it from https://go.dev/dl/?mode=json&include=all and add it to GO_ARCHIVE_SHA256 in this script" >&2
    exit 1
fi

for tool in curl tar sha256sum; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "install_go: '$tool' is not installed; see the header of this script" >&2
        exit 2
    }
done

# A directory that holds anything is not overlaid: two releases' files in one
# tree would be neither.
if [[ -d "$GO_ROOT" ]] && [[ -n "$(ls -A "$GO_ROOT")" ]]; then
    echo "install_go: $GO_ROOT is not empty and holds no Go $version; remove it and run this again" >&2
    exit 1
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
echo "installing Go $version ($archive) in $GO_ROOT"
curl -fsSL --retry 3 --retry-all-errors -o "$WORK/$archive" "https://go.dev/dl/$archive"
echo "$sum  $WORK/$archive" | sha256sum -c --quiet - || {
    echo "install_go: the download of $archive does not match its pinned SHA-256; nothing was unpacked" >&2
    exit 1
}
mkdir -p "$GO_ROOT"
tar -xzf "$WORK/$archive" -C "$GO_ROOT" --strip-components=1

cat >"$PROBE" <<'EOF'
#!/bin/sh
# What bx runs to tell this Go is installed: `<path> --version` must name the
# release, and `go` names it only to `go version`. GOTOOLCHAIN=local so the answer
# is this binary's own, never a toolchain it would switch to.
root="$(dirname "$(readlink -f "$0")")"
GOTOOLCHAIN=local exec "$root/bin/go" version
EOF
chmod +x "$PROBE"

# The postcondition, asserted rather than assumed: the binary runs on this
# machine and names the release.
"$PROBE" --version | grep -F "go version go$version " >/dev/null || {
    echo "install_go: $GO_ROOT/bin/go does not run here, or names another release than $version" >&2
    exit 1
}
"$PROBE" --version
echo "Go $version installed at $GO_ROOT"
