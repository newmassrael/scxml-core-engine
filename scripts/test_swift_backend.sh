#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate the Swift backend's committed machines and run its SwiftPM
# packages: the statechart runtime (backends/swift/runtime), the forge runtime
# (backends/swift/forge-runtime), the reference host of the standard host features
# (backends/swift/host-features), the generated machines with the tests that drive
# them (backends/swift/tests), and the host Mesh core (backends/swift/mesh). The forge runtime
# is run by the `forge` arm, with the numerical conformance tests of the generated units, and
# not by the default list.
#
# Like `./gradlew test` for Kotlin, it writes: the machines under
# backends/swift/tests/Sources are rewritten from the generator before they are
# compiled, so what is tested is what the templates write today and not what was
# last committed. Run on a build machine (`bx -- scripts/test_swift_backend.sh`), the
# rewritten files stay there, and `bx --action regen-swift-tests` is what brings
# them back (`regen-mesh-swift` for the Mesh core's standard documents).
#
# Usage (from repo root):
#   scripts/test_swift_backend.sh [package ...]    # runtime, forge-runtime, host-features, tests,
#                                                  # transition-outputs, feature-conformance, mesh,
#                                                  # forge, w3c, ecma262-lowered
#
# `mesh` is the package `backends/swift/mesh`, whose standard documents
# `backends/swift/mesh/generate.sh` writes first: the router core a host puts between an engine and
# its transports (SCE_MESH.md section 6.4), held to the Kotlin and Rust cores' tests. It is among the
# default list.
#
# `mesh-wss` is the package `backends/swift/mesh-wss`: the WebSocket binding's SwiftNIO client for that
# core, run against the Rust core over one real socket. It is among the default list.
#
# `feature-conformance` is the package `backends/swift/feature-conformance`, whose machines
# `scripts/regen_swift_feature_conformance.sh` writes first: the machine for every build of
# a product line, held to each build's own (docs/adr/0057 §10). It is among the default list.
#
# `transition-outputs` is not a package: it is the two programs `sce-build`'s equivalence tests
# build in Swift, the transition table a machine carries held to `sce-codegen transition-table`
# and the transition function compiled alone held to its producer
# (docs/SCE_TRANSITION_TABLE.md, docs/SCE_TRANSITION_FUNCTION.md). Those tests skip where no
# Swift toolchain is found; here they run on the toolchain this script puts on PATH, under
# `SCE_REQUIRE_TOOLS`, so a skip is a failure. It is among the default list.
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
    PACKAGES=(runtime host-features tests transition-outputs feature-conformance mesh mesh-wss)
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
        feature-conformance)
            # docs/adr/0057 §10: the machine for every build of tests/product_line/session.scxml,
            # compiled under each feature set's `.define`s, held to the machine that set
            # generates alone through every run of up to four events.
            scripts/regen_swift_feature_conformance.sh
            echo "== swift test: backends/swift/feature-conformance"
            (cd backends/swift/feature-conformance && swift test)
            # A build the feature model does not admit (a feature on without the one it
            # requires) stops at the model's `#error`, which names the edge. The compiler
            # fails either way, since the machine imports a module this invocation does not
            # name; the verdict is that it fails on the refusal.
            echo "== swiftc: the machine for every build refuses a build that breaks an edge"
            refusal="feature \`session-unicast-accept\` requires \`session-unicast\`"
            refused="$(swiftc -typecheck -D SCE_FEATURE_SESSION_UNICAST_ACCEPT \
                backends/swift/feature-conformance/Sources/LiftedNone/session_sm.swift 2>&1 || true)"
            if grep -qF "$refusal" <<< "$refused"; then
                echo "feature-conformance: the build that breaks an edge is refused by the model's #error"
            else
                echo "error: swiftc did not refuse a build with session-unicast-accept and without session-unicast" >&2
                exit 1
            fi
            ;;
        mesh)
            # The host Mesh core (`backends/swift/mesh`, SCE_MESH.md section 6.4): the standard Mesh
            # documents written from `stdlib/mesh` first, so what is compiled is what the templates
            # write today, then the router core that applies them and its tests.
            backends/swift/mesh/generate.sh
            echo "== swift test: backends/swift/mesh"
            (cd backends/swift/mesh && swift test)
            ;;
        mesh-wss)
            # The WebSocket binding's client (`backends/swift/mesh-wss`, SCE_MESH.md section 18) over
            # SwiftNIO, which SwiftPM resolves from the exact versions its manifest names. Its
            # loopback test dials the Rust core's server machine (`wss_peer`), built here from the
            # current sources, so the Swift and Rust cores are run against each other on every run
            # and not against a binary somebody left behind; the generated peer table both sides
            # build their routers from is written first.
            backends/swift/mesh-wss/generate.sh
            cargo build -p sce-rust-mesh --features wss --example wss_peer
            echo "== swift test: backends/swift/mesh-wss"
            (cd backends/swift/mesh-wss \
                && SCE_MESH_WSS_PEER="$REPO_ROOT/${CARGO_TARGET_DIR:-target}/debug/examples/wss_peer" swift test)
            ;;
        transition-outputs)
            # Each name is matched exactly in both test binaries, so each binary runs the one
            # test it holds; `scripts/gates/swift-backend.sh` reads both verdicts by name, so a
            # renamed test is a red and not a run of nothing. `cli` is the feature the two
            # targets require (`sce-build/Cargo.toml`).
            echo "== cargo test: the Swift transition table and transition function"
            SCE_REQUIRE_TOOLS=1 cargo test -p sce-build --features cli \
                --test the_emitted_transition_table_equals_the_export \
                --test the_emitted_transition_function_equals_the_producer \
                -- --exact \
                the_swift_table_compiles_and_holds_the_export \
                the_swift_function_compiles_alone_and_answers_as_the_producer
            ;;
        *)
            echo "== swift test: backends/swift/$package"
            (cd "backends/swift/$package" && swift test)
            ;;
    esac
done
