#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Install one tool the formal judges pin (docs/adr/0146, D3): fetch it by curl,
# admit what was fetched by the sha256 its pin names, and unpack it.
#
#     install_tool.sh <tool> <place> [<scratch>]
#
# <tool> is tlc, java, verus, apalache, storm, tlaps, kani or cbmc. <place> is
# where the tool goes: the jar's path for TLC, a directory for the others. It
# must be absent or an
# empty directory, as a build machine's tool store gives it; a place that holds
# anything is refused rather than written into. The tool is fetched and put
# together in a directory of its own under <scratch>, which is removed when the
# script ends, and moved to <place> last, so what is at <place> is a whole
# installation or nothing. Without <scratch>, that directory is made beside
# <place>, so that the move is one rename on one file system.
#
# The pins are the toolchain.json beside this script. This is the one procedure
# that installs them: `sce-codegen formal install` carries this script and the
# pins compiled in, and runs it beside the pin it installs, and bx.toml's tool
# tables run it from the tree, with the script and the pins as their install
# inputs. The transport is not trusted; the digest is.
#
# Verus, Apalache, Storm, TLAPS, Kani and CBMC are given a program `bin/<tool>`
# that answers `--version` by itself, so a check of an installation starts no
# JVM and no Isabelle, and asks nothing of the Rust release Verus or Kani runs
# on. The Java runtime's own `bin/java` answers it.
#
# What it needs and does not install: curl, sha256sum, tar, unzip (Verus and
# Apalache), dpkg-deb (Storm and CBMC), make and perl (TLAPS, whose installer
# builds its Isabelle heaps), and access to the hosts the pins name. CBMC runs
# gcc to preprocess C, as its package's dependencies say. The Rust release Kani
# runs on is installed by scripts/install_rust.sh, as the one Verus runs on is.
set -euo pipefail

die() {
    echo "install_tool: $*" >&2
    exit 1
}

if [[ $# -lt 2 || $# -gt 3 || -z "$2" ]]; then
    echo "usage: install_tool.sh <tlc|java|verus|apalache|storm|tlaps|kani|cbmc> <place> [<scratch>]" >&2
    exit 2
fi
tool="$1"
place="$2"
scratch_root="${3:-}"
case "$tool" in
    tlc | java | verus | apalache | storm | tlaps | kani | cbmc) ;;
    *)
        echo "install_tool: '$tool' is no tool the formal judges pin" >&2
        exit 2
        ;;
esac

pins="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/toolchain.json"
[[ -f "$pins" ]] || die "no pins at $pins"

# The tool's pin, the one object the pins hold under its name, and its fields.
# Each pin is an object of strings with no object inside it, which the unit
# tests of sce-build/src/formal/toolchain.rs hold the pins to.
pin="$(grep -oE "\"$tool\": *\\{[^}]*\\}" "$pins" || true)"
if [[ -z "$pin" || "$pin" == *$'\n'* ]]; then
    die "$pins holds no one pin of $tool"
fi
field() {
    sed -nE "s/.*\"$1\": *\"([^\"]*)\".*/\\1/p" <<<"$pin"
}
version="$(field version)"
url="$(field url)"
sha256="$(field sha256)"
[[ -n "$version" && -n "$url" ]] || die "the pin of $tool in $pins names no version or no url"
[[ "$sha256" =~ ^[0-9a-f]{64}$ ]] || die "the pin of $tool in $pins names no sha256 of 64 lowercase hex digits"

if [[ -L "$place" ]] || [[ -e "$place" && ! -d "$place" ]]; then
    die "$place is there already: a tool is installed into an absent place or an empty directory"
fi
if [[ -d "$place" && -n "$(ls -A "$place")" ]]; then
    die "$place is not empty: a tool is installed into an absent place or an empty directory"
fi

parent="$(dirname "$place")"
mkdir -p "$parent"
if [[ -n "$scratch_root" ]]; then
    mkdir -p "$scratch_root"
    scratch="$(mktemp -d "$scratch_root/install-$tool.XXXXXX")"
else
    scratch="$(mktemp -d "$parent/.install-$tool.XXXXXX")"
fi
trap 'rm -rf "$scratch"' EXIT

fetched="$scratch/fetched"
curl --fail --silent --show-error --location --retry 3 --output "$fetched" "$url" ||
    die "curl could not fetch $url"
actual="$(sha256sum "$fetched")"
actual="${actual%% *}"
[[ "$actual" == "$sha256" ]] ||
    die "$url is not the $tool this judge pins (sha256 $actual, pinned $sha256)"

# The program `bin/verus`: Verus run on the Rust release it was built with, at
# the place scripts/install_rust.sh puts every declared release, with no
# rustup, which Verus asks for its release unless told not to.
verus_wrapper() {
    local toolchain
    toolchain="$(field toolchain)"
    [[ -n "$toolchain" ]] || die "the pin of verus in $pins names no toolchain"
    cat <<EOF
#!/bin/sh
# Written by install_tool.sh: Verus $version on Rust $toolchain, with no rustup.
if [ "\$1" = --version ]; then
    echo "verus $version"
    exit 0
fi
rust="\$HOME/.local/rust/$toolchain"
VERUS_USE_RUSTUP=0 LD_LIBRARY_PATH="\$rust/lib" exec "\$(dirname "\$(readlink -f "\$0")")/../verus" --sysroot "\$rust" "\$@"
EOF
}

# The program `bin/apalache`: the release's own launcher, `bin/apalache-mc`,
# which sets the JVM's heap and collector as its authors run it.
apalache_wrapper() {
    cat <<EOF
#!/bin/sh
# Written by install_tool.sh: Apalache $version, run by its own launcher.
if [ "\$1" = --version ]; then
    echo "apalache $version"
    exit 0
fi
exec "\$(dirname "\$(readlink -f "\$0")")/apalache-mc" "\$@"
EOF
}

# The program `bin/storm`: the package's program run with the libraries the
# package holds, which it does not find by itself (its run path names
# /usr/local/lib, where it is not unpacked). The libraries it asks of the
# system are found where the system keeps them.
storm_wrapper() {
    cat <<EOF
#!/bin/sh
# Written by install_tool.sh: Storm $version, run with the libraries of its package.
if [ "\$1" = --version ]; then
    echo "storm $version"
    exit 0
fi
here="\$(dirname "\$(readlink -f "\$0")")/.."
LD_LIBRARY_PATH="\$here/usr/lib" exec "\$here/usr/bin/storm-$version" "\$@"
EOF
}

# The program `bin/tlaps`: the release's `bin/tlapm`, with the back ends of the
# release (Zenon, Isabelle, Z3, LS4) first on the search path, so a back end a
# machine has of its own is never the one a proof is checked by.
tlaps_wrapper() {
    cat <<EOF
#!/bin/sh
# Written by install_tool.sh: TLAPS $version, its tlapm run with the back ends of its release.
if [ "\$1" = --version ]; then
    echo "tlaps $version"
    exit 0
fi
here="\$(dirname "\$(readlink -f "\$0")")/.."
PATH="\$here/lib/tlaps/bin:\$PATH" exec "\$here/bin/tlapm" "\$@"
EOF
}

# The program `bin/cbmc`: the package's program, with the package's other
# programs (goto-cc, goto-instrument, goto-harness) first on the search path, so
# a step of a harness that calls one runs the release's and not a machine's.
cbmc_wrapper() {
    cat <<EOF
#!/bin/sh
# Written by install_tool.sh: CBMC $version, run with the programs of its package.
if [ "\$1" = --version ]; then
    echo "cbmc $version"
    exit 0
fi
here="\$(dirname "\$(readlink -f "\$0")")/.."
PATH="\$here/usr/bin:\$PATH" exec "\$here/usr/bin/cbmc" "\$@"
EOF
}

# The host Kani's release bundle is built for, which the bundle's
# `rust-toolchain-version` names after the Rust release.
KANI_HOST="x86_64-unknown-linux-gnu"

# The programs `bin/kani` and `bin/cargo-kani`: the bundle's `kani-driver`, run
# as the proxy `cargo install kani-verifier` installs runs it (its `exec`): the
# bundle's programs first on the search path, `RUSTUP_TOOLCHAIN` naming the
# Rust release the bundle was built against, and no rustup toolchain's `lib` on
# the loader's path, where a `librustc_driver` of the same name would be loaded
# in place of the one `kani-compiler` was linked with. That Rust release's
# `bin`, through the link `toolchain`, is on the search path next, so no rustup
# proxy is reached; `kani-driver` runs its cargo through the same link. The
# driver takes a first argument `kani` as a call through cargo, so
# `bin/cargo-kani` gives it one when cargo has not.
kani_wrapper() {
    local toolchain="$1" mode="$2" through_cargo=""
    if [[ "$mode" == cargo ]]; then
        # shellcheck disable=SC2016 # expanded by the program written, not here
        through_cargo='[ "$1" = kani ] || set -- kani "$@"'
    fi
    cat <<EOF
#!/bin/sh
# Written by install_tool.sh: Kani $version on Rust $toolchain, with no rustup.
if [ "\$1" = --version ]; then
    echo "kani $version"
    exit 0
fi
$through_cargo
here="\$(dirname "\$(readlink -f "\$0")")/.."
kept=
saved="\$IFS"
IFS=:
set -f
for dir in \$LD_LIBRARY_PATH; do
    case "\$dir" in
        '' | */toolchains/*/lib | */toolchains/*/lib/) ;;
        *) kept="\${kept:+\$kept:}\$dir" ;;
    esac
done
set +f
IFS="\$saved"
if [ -n "\$kept" ]; then
    LD_LIBRARY_PATH="\$kept"
    export LD_LIBRARY_PATH
else
    unset LD_LIBRARY_PATH
fi
PATH="\$here/bin:\$here/toolchain/bin:\$PATH" RUSTUP_TOOLCHAIN="$toolchain-$KANI_HOST" exec "\$here/bin/kani-driver" "\$@"
EOF
}

unpacked="$scratch/unpacked"
mkdir "$unpacked"
case "$tool" in
    tlc)
        built="$fetched"
        ;;
    java)
        # Eclipse Temurin's runtime, which unpacks as jdk-<version>-jre/.
        tar -xzf "$fetched" -C "$unpacked" || die "tar could not unpack $url"
        built="$unpacked/jdk-$version-jre"
        [[ -f "$built/bin/java" ]] || die "$url holds no jdk-$version-jre/bin/java"
        ;;
    verus)
        # The release's x86-linux archive; unzip keeps each file's mode.
        unzip -q "$fetched" -d "$unpacked" || die "unzip could not unpack $url"
        built="$unpacked/verus-x86-linux"
        [[ -f "$built/verus" ]] || die "$url holds no verus-x86-linux/verus"
        mkdir -p "$built/bin"
        verus_wrapper >"$built/bin/verus"
        chmod 0755 "$built/bin/verus"
        ;;
    apalache)
        # The release's archive, and the standard module Apalache taken out of
        # its jar into modules/, where TLC reads it for a model that extends it.
        tar -xzf "$fetched" -C "$unpacked" || die "tar could not unpack $url"
        built="$unpacked/apalache-$version"
        for needed in bin/apalache-mc lib/apalache.jar; do
            [[ -f "$built/$needed" ]] || die "$url holds no apalache-$version/$needed"
        done
        mkdir -p "$built/modules"
        module="tla2sany/StandardModules/Apalache.tla"
        unzip -q -j -o "$built/lib/apalache.jar" "$module" -d "$built/modules" ||
            die "$url: lib/apalache.jar holds no $module"
        [[ -f "$built/modules/Apalache.tla" ]] || die "$url: lib/apalache.jar holds no $module"
        apalache_wrapper >"$built/bin/apalache"
        chmod 0755 "$built/bin/apalache"
        ;;
    storm)
        # The Debian package, unpacked and never installed: its usr/ tree is
        # kept, and what it asks of the system it does not get by being unpacked.
        dpkg-deb -x "$fetched" "$unpacked" || die "dpkg-deb could not unpack $url"
        [[ -f "$unpacked/usr/bin/storm-$version" ]] || die "$url holds no usr/bin/storm-$version"
        built="$scratch/storm"
        mkdir -p "$built/bin"
        mv "$unpacked/usr" "$built/usr"
        storm_wrapper >"$built/bin/storm"
        chmod 0755 "$built/bin/storm"
        ;;
    tlaps)
        # The release's installer, a program that carries the tree it unpacks,
        # run as its authors run it: it unpacks bin/tlapm and lib/tlaps/ into
        # the directory -d names, builds Isabelle's heaps of Pure and TLA+ there
        # and checks examples/cantor/Cantor1.tla with them, and ends 0 only if
        # every step did.
        built="$(cd "$scratch" && pwd)/tlaps"
        chmod 0755 "$fetched"
        "$fetched" -d "$built" >"$scratch/installer.log" 2>&1 ||
            die "the installer of $url did not install it: $(tail -n 20 "$scratch/installer.log")"
        [[ -f "$built/bin/tlapm" ]] || die "the installer of $url put no bin/tlapm"
        compgen -G "$built/lib/tlaps/heaps/*/TLA+" >/dev/null ||
            die "the installer of $url built no heap of TLA+ under lib/tlaps/heaps"
        # It writes the directory it was given into Isabelle's settings as the
        # place of the heaps it built. That line is written again relative to
        # Isabelle's own directory, lib/tlaps/Isabelle, so the heaps are found
        # where the tree is after it is moved to <place>.
        settings="$built/lib/tlaps/Isabelle/etc/settings"
        written="ISABELLE_HOME_USER=$built/lib/tlaps"
        grep -qxF "$written" "$settings" ||
            die "$settings does not name the heaps at $built/lib/tlaps by the line its installer writes"
        awk -v written="$written" '
            $0 == written { print "ISABELLE_HOME_USER=\"$ISABELLE_HOME/..\""; next }
            { print }
        ' "$settings" >"$settings.relocated"
        mv "$settings.relocated" "$settings"
        tlaps_wrapper >"$built/bin/tlaps"
        chmod 0755 "$built/bin/tlaps"
        ;;
    kani)
        # The release bundle, unpacked as `cargo kani setup --use-local-bundle`
        # unpacks it, and the link `toolchain` it makes beside the bundle's
        # programs, to the Rust release the bundle was built against: here the
        # one scripts/install_rust.sh puts at ~/.local/rust/<release>, never a
        # rustup's. `kani-compiler` loads that release's compiler libraries
        # through the link, and `kani-driver` runs its cargo.
        toolchain="$(field toolchain)"
        [[ -n "$toolchain" ]] || die "the pin of kani in $pins names no toolchain"
        [[ -n "${HOME:-}" ]] || die "HOME is not set, and the Rust release Kani runs on is kept under it"
        tar -xzf "$fetched" -C "$unpacked" || die "tar could not unpack $url"
        built="$unpacked/kani-$version"
        for needed in bin/kani-driver bin/kani-compiler bin/cbmc rust-toolchain-version; do
            [[ -f "$built/$needed" ]] || die "$url holds no kani-$version/$needed"
        done
        bundled="$(<"$built/rust-toolchain-version")"
        [[ "$bundled" == "$toolchain-$KANI_HOST" ]] ||
            die "$url was built against Rust $bundled, and its pin names $toolchain"
        ln -s "$HOME/.local/rust/$toolchain" "$built/toolchain"
        kani_wrapper "$toolchain" standalone >"$built/bin/kani"
        kani_wrapper "$toolchain" cargo >"$built/bin/cargo-kani"
        chmod 0755 "$built/bin/kani" "$built/bin/cargo-kani"
        ;;
    cbmc)
        # The release's package for Ubuntu 24.04 on x64, unpacked and never
        # installed, as Storm's is: its usr/ tree is kept. Its programs ask the
        # system for the C and C++ runtimes alone.
        dpkg-deb -x "$fetched" "$unpacked" || die "dpkg-deb could not unpack $url"
        [[ -f "$unpacked/usr/bin/cbmc" ]] || die "$url holds no usr/bin/cbmc"
        built="$scratch/cbmc"
        mkdir -p "$built/bin"
        mv "$unpacked/usr" "$built/usr"
        cbmc_wrapper >"$built/bin/cbmc"
        chmod 0755 "$built/bin/cbmc"
        ;;
esac

if [[ -d "$place" ]]; then
    rmdir "$place"
fi
mv -T "$built" "$place"
echo "$tool $version fetched, admitted by its sha256, at $place"
