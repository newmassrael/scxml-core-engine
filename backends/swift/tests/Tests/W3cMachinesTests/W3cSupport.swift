// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Written by hand: the one file of this directory that `sce-codegen generate-w3c -l swift` does not
// write. Each generated test (`Test<id>Tests.swift`) starts its machine through `w3cSettled` and
// reads where it stopped through `w3cIsState`, so what a W3C test means by "ran" and "passed" is
// said once here and not once per fixture.

import SceRuntime

/// The run every other backend gives a `simple` fixture: start the machine and let it tick, for at
/// most three seconds, until it reaches a final state. A machine that starts a child session or
/// sends to itself is only finished by the ticks. Throws what refused the start (docs/adr/0096).
func w3cSettled<Policy: StatePolicy>(_ policy: Policy) throws -> (engine: Engine<Policy>, completed: Bool) {
    let engine = Engine(policy: policy)
    try engine.initialize()
    let completed = engine.runUntilCompletion(timeout: .seconds(3), pollInterval: .milliseconds(10))
    return (engine, completed)
}

/// Whether the state a machine stopped in is the one `expected` names. The generator names the
/// state the way every backend's enum spells it (`Pass`), and a machine answers with the id as its
/// document writes it (`pass`, `Pass`, `done_state`), so the two are compared by their letters and
/// digits and not by their punctuation or case.
func w3cIsState(_ actual: String, named expected: String) -> Bool {
    func letters(_ text: String) -> String {
        String(text.lowercased().filter { $0.isLetter || $0.isNumber })
    }
    return letters(actual) == letters(expected)
}
