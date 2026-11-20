#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Install the Arm GNU Toolchain release the `c11-mcu` gate compiles, links and
# weighs its Cortex-M4 images with (docs/adr/0110, D4), from Arm's own tarball,
# checked against the SHA-256 Arm publishes for it.
#
#     scripts/install_arm_gnu_toolchain.sh
#
# WHY A PINNED TARBALL (2026-11-21). The gate holds each machine's `.text`,
# `.rodata`, `.data` and `.bss` to backends/c/mcu/footprint.budget, and those
# figures are the compiler's: another release lays out and inlines differently,
# and the gate would call that a change of the tree. The distribution's
# gcc-arm-none-eabi is whatever release the distribution carries (Ubuntu 22.04
# ships 10.3, 24.04 a rebuild of 13.2.1), and apt can replace the file behind a
# path while the name stays the same. Arm's tarball is one set of bytes for a
# release, so the release the budget was measured with is the one every machine
# and the hosted lane run.
#
# The unit is the RELEASE: `ARM_GNU_TOOLCHAIN_REV` below is the one home of the
# pin. bx.toml's `[platform.linux.tools.arm-none-eabi-gcc]` reads it from this
# line, and scripts/gates/c11-mcu.sh reads it from the same line to find the
# toolchain. Raising it is a commit of its own that adds the new release's
# digests here and measures the budget again with it.
#
# The tarball unpacks to ARM_GNU_TOOLCHAIN_ROOT (default
# ~/.local/share/arm-gnu-toolchain/<release>, where the gate looks for it). It
# holds the compiler, binutils (`arm-none-eabi-size` among them), newlib's
# headers and the libgcc of every multilib. Run again over an installed
# release, the script does nothing.
#
# What it needs and does not install: curl, tar, xz and sha256sum, and access to
# developer.arm.com.
set -euo pipefail

ARM_GNU_TOOLCHAIN_REV="13.2.rel1"

# The SHA-256 of each archive this script may install, as Arm publishes it in
# the `.sha256asc` beside the archive (taken 2026-11-21; the x86_64 archive was
# also downloaded and its digest computed). A host architecture with no line
# here is refused by name rather than downloaded unchecked; raising the release
# replaces these lines, taken the same way, in the same commit.
ARM_GNU_TOOLCHAIN_SHA256=(
    "arm-gnu-toolchain-13.2.rel1-x86_64-arm-none-eabi.tar.xz 6cd1bbc1d9ae57312bcd169ae283153a9572bd6a8e4eeae2fedfbc33b115fdbb"
    "arm-gnu-toolchain-13.2.rel1-aarch64-arm-none-eabi.tar.xz 8fd8b4a0a8d44ab2e195ccfbeef42223dfb3ede29d80f14dcf2183c34b8d199a"
)

ARM_GNU_TOOLCHAIN_ROOT="${ARM_GNU_TOOLCHAIN_ROOT:-$HOME/.local/share/arm-gnu-toolchain/$ARM_GNU_TOOLCHAIN_REV}"
GCC="$ARM_GNU_TOOLCHAIN_ROOT/bin/arm-none-eabi-gcc"
# What `arm-none-eabi-gcc --version` names Arm's build of the release by. A
# distribution's build of the same compiler does not say it, so this tells the
# two apart.
BUILD_NAME="Arm GNU Toolchain $ARM_GNU_TOOLCHAIN_REV"

if [[ -x "$GCC" ]] && "$GCC" --version 2>/dev/null | grep -F "$BUILD_NAME" >/dev/null; then
    echo "$BUILD_NAME is already installed at $ARM_GNU_TOOLCHAIN_ROOT"
    exit 0
fi

case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) host="x86_64" ;;
    Linux-aarch64) host="aarch64" ;;
    *)
        echo "install_arm_gnu_toolchain: no Arm GNU Toolchain archive is pinned for $(uname -s) $(uname -m)" >&2
        exit 1
        ;;
esac
archive="arm-gnu-toolchain-$ARM_GNU_TOOLCHAIN_REV-$host-arm-none-eabi.tar.xz"
sum=""
for entry in "${ARM_GNU_TOOLCHAIN_SHA256[@]}"; do
    if [[ "${entry%% *}" == "$archive" ]]; then
        sum="${entry#* }"
    fi
done
if [[ -z "$sum" ]]; then
    echo "install_arm_gnu_toolchain: no SHA-256 is pinned for $archive; take it from the .sha256asc Arm publishes beside it and add it to ARM_GNU_TOOLCHAIN_SHA256 in this script" >&2
    exit 1
fi

for tool in curl tar xz sha256sum; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "install_arm_gnu_toolchain: '$tool' is not installed; see the header of this script" >&2
        exit 2
    }
done

# A directory that holds anything is not overlaid: two releases' files in one
# tree would be neither.
if [[ -d "$ARM_GNU_TOOLCHAIN_ROOT" ]] && [[ -n "$(ls -A "$ARM_GNU_TOOLCHAIN_ROOT")" ]]; then
    echo "install_arm_gnu_toolchain: $ARM_GNU_TOOLCHAIN_ROOT is not empty and holds no $BUILD_NAME; remove it and run this again" >&2
    exit 1
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
url="https://developer.arm.com/-/media/Files/downloads/gnu/$ARM_GNU_TOOLCHAIN_REV/binrel/$archive"
echo "installing $BUILD_NAME ($archive) in $ARM_GNU_TOOLCHAIN_ROOT"
curl -fsSL --retry 3 --retry-all-errors -o "$WORK/$archive" "$url"
echo "$sum  $WORK/$archive" | sha256sum -c --quiet - || {
    echo "install_arm_gnu_toolchain: the download of $archive does not match its pinned SHA-256; nothing was unpacked" >&2
    exit 1
}
mkdir -p "$ARM_GNU_TOOLCHAIN_ROOT"
tar -xJf "$WORK/$archive" -C "$ARM_GNU_TOOLCHAIN_ROOT" --strip-components=1

# The postcondition, asserted rather than assumed: the compiler runs on this
# machine and names Arm's build of the release, and the size reader the gate
# weighs with is beside it.
"$GCC" --version | grep -F "$BUILD_NAME" >/dev/null || {
    echo "install_arm_gnu_toolchain: $GCC does not run here, or names another build than $BUILD_NAME" >&2
    exit 1
}
[[ -x "$ARM_GNU_TOOLCHAIN_ROOT/bin/arm-none-eabi-size" ]] || {
    echo "install_arm_gnu_toolchain: $ARM_GNU_TOOLCHAIN_ROOT holds no bin/arm-none-eabi-size" >&2
    exit 1
}
"$GCC" --version | sed -n '1p'
echo "$BUILD_NAME installed at $ARM_GNU_TOOLCHAIN_ROOT"
