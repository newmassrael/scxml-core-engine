// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.3: a `<data>` whose id is a keyword somewhere, or no identifier anywhere, is its own
// variable and holds its own value. Swift AOT path.
//
// The Kotlin test of this stem reads each variable through a generated typed reader (`box()`,
// `object()`, `screenRules()`), and what it pins is how `reader_names.rs` spells or refuses a
// reader. The Swift policy generates no per-variable reader, and `reader_names.rs` scans no Swift
// runtime type for a collision (it says Swift has none yet), so there is no reader name here to
// escape. What the Swift backend does own is the half underneath: every id the document declares
// is a variable of the session under that very id, a neighbour's value never lands in it, and an
// assignment moves the variable it names and no other. Each id is read as `_G["<id>"]`, the one
// spelling that names `screen-rules` and `a-b`, which no expression can.
//
// Fixture: integration_resources/typed_reader_names/typed_reader_names.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

private func started() -> (policy: TypedReaderNamesPolicy, engine: Engine<TypedReaderNamesPolicy>) {
    let policy = TypedReaderNamesPolicy()
    let engine = Engine(policy: policy)
    startRecordingARefusal(engine)
    return (policy, engine)
}

/// Every variable of the document and the value the run should hold for it, in document order.
private func variables(box: Int64, object: Int64, pass: Int64, aB: Int64) -> [(
    name: String, want: Int64
)] {
    [
        ("box", box), ("object", object), ("pass", pass), ("screen-rules", 4), ("auto", 5),
        ("self", 6), ("new", 7), ("start", 8), ("t", 9), ("a_b", aB), ("a-b", 11),
    ]
}

private func wrong(
    _ expected: [(name: String, want: Int64)], in policy: TypedReaderNamesPolicy
) -> [String] {
    mismatches(expected) { policy.readScriptValue("_G[\"\($0)\"]") }
}

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct TypedReaderNames {
    /// Each id holds the value its own `<data>` was declared with, and `a_b` and `a-b`, which fold to
    /// one spelling in some backends, are two variables here.
    @Test func aVariableHoldsItsOwnValue() {
        let (policy, _) = started()
        let observed = wrong(variables(box: 1, object: 2, pass: 3, aB: 10), in: policy)
        #expect(observed.isEmpty, "observed: \(observed)")
    }

    /// And the value it holds now: `bump` adds 10 to four of them, and leaves `screen-rules`, which
    /// no ECMAScript expression can name, and every other id alone.
    @Test func aVariableHoldsTheLiveValue() {
        let (policy, engine) = started()
        engine.processEvent(.bump)
        let observed = wrong(variables(box: 11, object: 12, pass: 13, aB: 20), in: policy)
        #expect(observed.isEmpty, "observed: \(observed)")
    }
}
