// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Child sessions", and §scxml-6.4 (docs/adr/0098) on the Swift
// channel: a `datamodel="sce-static"` child declares its variables with the values its invoke
// handed, each in place of its own initial value, so a variable declared after a handed one reads
// the handed value.
//
// `worker` is handed `start = 7` and declares `copied = start` after it; it ends the moment
// `copied` is 7. A child that wrote the handed value over `start` after declaring both holds
// `copied = 0`, never ends, and leaves the parent in `working`.
//
// Fixture: tests/integration/a_static_child_declares_its_data_with_a_handed_value.scxml
//
// Regeneration: scripts/regen_static_datamodel_swift.sh

import SceRuntime
import Testing

@testable import StaticDatamodelMachines

@Suite struct AStaticChildDeclaresItsDataWithAHandedValue {
    @Test func aVariableDeclaredAfterAHandedOneReadsTheHandedValue() throws {
        let engine = Engine(policy: AStaticChildDeclaresItsDataWithAHandedValuePolicy())
        engine.setClock(ManualClock())
        try engine.initialize()
        // Let the child start, run and end, and its parent take `done.invoke`.
        for _ in 0..<5 where !engine.isInFinalState {
            engine.tick()
        }
        #expect(
            engine.terminalState == .passed,
            "`worker` declares `copied = start` after `start`, which its invoke handed 7, so it ends: the parent ended in \(String(describing: engine.terminalState))"
        )
    }
}
