#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate the Swift backend's committed machines and run its four SwiftPM
# packages: the statechart runtime (backends/swift/runtime), the forge runtime
# (backends/swift/forge-runtime), the reference host of the standard host features
# (backends/swift/host-features) and the generated machines with the tests that drive
# them (backends/swift/tests). The forge runtime is run by the `forge` arm, with the numerical
# conformance tests of the generated units, and not by the default list.
#
# Like `./gradlew test` for Kotlin, it writes: the machines under
# backends/swift/tests/Sources are rewritten from the generator before they are
# compiled, so what is tested is what the templates write today and not what was
# last committed. Run on a build machine (`bx -- scripts/test_swift_backend.sh`), the
# rewritten files stay there, and `bx --action regen` is what brings them back.
#
# Usage (from repo root):
#   scripts/test_swift_backend.sh [package ...]    # runtime, forge-runtime, host-features, tests, forge,
#                                                  # w3c, ecma262-lowered
#
# `forge`, `w3c` and `ecma262-lowered` are not among the default list: each is a question of its
# own with its own gate. `forge` runs the forge runtime and the numerical conformance tests,
# `w3c` and `ecma262-lowered` write machines into `backends/swift/tests` and run their own tests
# alone. `scripts/gate forge-swift` runs the first, `scripts/gate w3c-swift` the second,
# `scripts/gate ecma262-lowered-swift` the third, and `scripts/gate swift-backend` runs the
# default list.
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
    PACKAGES=(runtime host-features tests)
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
        forge)
            # The Forge conformance question (`scripts/gate forge-swift`): the forge runtime's own
            # tests, then every unit the generator writes for a forge document held to the numerical
            # oracle. The units are written by the sweep `tests` below also runs, and the tests that
            # replay them are the ones named `...MatchesReference`; `tests` skips those, so each
            # question is judged by one gate.
            echo "== swift test: backends/swift/forge-runtime"
            (cd backends/swift/forge-runtime && swift test)
            scripts/regen_static_datamodel_swift.sh
            echo "== swift test: backends/swift/tests (numerical conformance)"
            (cd backends/swift/tests && swift test --filter MatchesReference)
            ;;
        tests)
            scripts/regen_swift_statecharts.sh
            scripts/regen_static_datamodel_swift.sh
            scripts/regen_swift_ecmascript.sh
            scripts/regen_swift_integration.sh
            echo "== swift test: backends/swift/tests"
            # A tree another lane left on this machine is built with the rest, and not run here;
            # the numerical conformance tests are the Forge lane's (`forge` above).
            (cd backends/swift/tests && swift test --skip W3cMachinesTests --skip Ecma262LoweredTests --skip MatchesReference)
            ;;
        *)
            echo "== swift test: backends/swift/$package"
            (cd "backends/swift/$package" && swift test)
            ;;
    esac
done
