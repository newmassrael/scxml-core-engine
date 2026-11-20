#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Install the Bazel release SCE's Bazel channels are built against, from the
# release binary bazelbuild publishes, checked against the SHA-256 published
# beside it.
#
#     scripts/install_bazel.sh [<release>]
#
# The release is SCE_BAZEL_VERSION in scripts/lib/bazel_pins.sh. bx gives it as
# the argument, read from that line by `[platform.linux.tools.bazel]` in bx.toml;
# run by hand without one, the script reads the same line.
#
# WHY A DECLARED BAZEL (2026-11-21). The hosted lane takes bazelisk from the
# runner image, and no build machine had a Bazel at all, so a landing that
# reached `embed-vendor` failed its Bazel half on every machine ("no bazel
# (bazelisk) on PATH"). Installed here once per machine, the pinned release is
# the machine's, and scripts/smoke_bazel_consumer.sh finds it through
# `sce_bazel_select` (scripts/lib/bazel_pins.sh).
#
# The binary goes to BAZEL_ROOT/bin/bazel (default ~/.local/share/bazel/<release>,
# where the smoke looks for it).
#
# What it needs and does not install: curl and sha256sum, and access to
# github.com.
set -euo pipefail

# The SHA-256 of each binary this script may install, as published beside it
# (https://github.com/bazelbuild/bazel/releases/download/<release>/<file>.sha256;
# taken 2026-11-21). A release or an architecture with no line here is refused
# by name rather than downloaded unchecked: moving SCE_BAZEL_VERSION adds that
# release's lines here, taken the same way, in the same commit.
BAZEL_BINARY_SHA256=(
    "bazel-9.3.1-linux-x86_64 7f775b54bbec487efaf0f49ccda1d2ed02cd34f582edfdd6aeeec76e39ef585c"
    "bazel-9.3.1-linux-arm64 10ba9cb76d4ee748d274dcc2981992bb12b74a203dc41a9c2eceeb8f29359366"
)

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
version="${1:-}"
if [[ -z "$version" ]]; then
    # shellcheck disable=SC1091
    source "$REPO_ROOT/scripts/lib/bazel_pins.sh"
    version="$SCE_BAZEL_VERSION"
fi
if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "install_bazel: '$version' is not a Bazel release such as 9.3.1" >&2
    exit 2
fi

BAZEL_ROOT="${BAZEL_ROOT:-$HOME/.local/share/bazel/$version}"
BIN="$BAZEL_ROOT/bin/bazel"

if [[ -x "$BIN" ]] && "$BIN" --version 2>/dev/null | grep -Fx "bazel $version" >/dev/null; then
    echo "Bazel $version is already installed at $BIN"
    exit 0
fi

case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) platform="linux-x86_64" ;;
    Linux-aarch64) platform="linux-arm64" ;;
    *)
        echo "install_bazel: no Bazel binary is known for $(uname -s) $(uname -m)" >&2
        exit 1
        ;;
esac
file="bazel-$version-$platform"
sum=""
for entry in "${BAZEL_BINARY_SHA256[@]}"; do
    if [[ "${entry%% *}" == "$file" ]]; then
        sum="${entry#* }"
    fi
done
if [[ -z "$sum" ]]; then
    echo "install_bazel: no SHA-256 is recorded for $file; add its line to BAZEL_BINARY_SHA256" >&2
    exit 1
fi

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
curl --proto =https --tlsv1.2 -fsSL \
    "https://github.com/bazelbuild/bazel/releases/download/$version/$file" -o "$work/bazel"
if ! printf '%s  %s\n' "$sum" "$work/bazel" | sha256sum --check --status; then
    echo "install_bazel: $file does not have the recorded SHA-256 $sum" >&2
    exit 1
fi
mkdir -p "$BAZEL_ROOT/bin"
install -m 0755 "$work/bazel" "$BIN"

# The postcondition bx's check asks for: the binary runs and names the release.
if ! "$BIN" --version 2>/dev/null | grep -Fx "bazel $version" >/dev/null; then
    echo "install_bazel: $BIN does not report bazel $version" >&2
    exit 1
fi
echo "Bazel $version installed at $BIN"
