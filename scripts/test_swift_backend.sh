#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate the Swift backend's committed machines and run its four SwiftPM
# packages: the statechart runtime (backends/swift/runtime), the forge runtime
# (backends/swift/forge-runtime), the reference host of the standard host features
# (backends/swift/host-features) and the generated machines with the tests that drive
# them (backends/swift/tests).
#
# Like `./gradlew test` for Kotlin, it writes: the machines under
# backends/swift/tests/Sources are rewritten from the generator before they are
# compiled, so what is tested is what the templates write today and not what was
# last committed. Run on a build machine (`bx -- scripts/test_swift_backend.sh`), the
# rewritten files stay there, and `bx --action regen` is what brings them back.
#
# Usage (from repo root):
#   scripts/test_swift_backend.sh [package ...]    # runtime, forge-runtime, host-features, tests, w3c,
#                                                  # ecma262-lowered
#
# `w3c` and `ecma262-lowered` are not among the four packages: each writes machines into
# `backends/swift/tests` and runs its own tests alone, so neither is in the default list.
# `scripts/gate w3c-swift` runs the first, `scripts/gate ecma262-lowered-swift` the second, and
# `scripts/gate swift-backend` runs the four.
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing) and a
#   Swift toolchain (`swift` on PATH, or ~/.local/share/swift/<version>/bin/swift).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

# `swift` is a link into a toolchain whose own tools (`swiftc`, `swift-build`) sit
# beside the file it points at, and a toolchain installed by hand is not on PATH.
if ! command -v swift > /dev/null 2>&1; then
    swift_link="$(ls "$HOME"/.local/share/swift/*/bin/swift 2> /dev/null | LC_ALL=C sort | tail -n 1)"
    if [ -z "$swift_link" ]; then
        echo "error: no Swift toolchain on PATH or under ~/.local/share/swift" >&2
        exit 1
    fi
    PATH="$(dirname "$(readlink -f "$swift_link")"):$PATH"
    export PATH
fi

if [ "$#" -gt 0 ]; then
    PACKAGES=("$@")
else
    PACKAGES=(runtime forge-runtime host-features tests)
fi

for package in "${PACKAGES[@]}"; do
    case "$package" in
        w3c)
            # The W3C suite is a question of its own (`scripts/gate w3c-swift`): the machines
            # `generate-w3c -l swift` writes from the W3C documents, run against the engine. It
            # is written here and not committed, and `tests` below neither writes nor runs it.
            scripts/regen_swift_w3c.sh
            echo "== swift test: backends/swift/tests (W3C suite)"
            (cd backends/swift/tests && swift test --filter W3cMachinesTests)
            ;;
        ecma262-lowered)
            # The ECMA-262 artifact is a question of its own (`scripts/gate ecma262-lowered-swift`):
            # one machine that asks every case of the shared table, compiled and run. Written here
            # and not committed, and `tests` below neither writes nor runs it.
            scripts/regen_swift_ecma262_lowered.sh
            echo "== swift test: backends/swift/tests (ECMA-262 lowered artifact)"
            (cd backends/swift/tests && swift test --filter Ecma262LoweredTests)
            ;;
        tests)
            scripts/regen_swift_statecharts.sh
            scripts/regen_static_datamodel_swift.sh
            scripts/regen_swift_ecmascript.sh
            echo "== swift test: backends/swift/tests"
            # A tree another lane left on this machine is built with the rest, and not run here.
            (cd backends/swift/tests && swift test --skip W3cMachinesTests --skip Ecma262LoweredTests)
            ;;
        *)
            echo "== swift test: backends/swift/$package"
            (cd "backends/swift/$package" && swift test)
            ;;
    esac
done
