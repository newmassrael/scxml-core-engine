// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 4.9: an error ends the block it was raised in, whichever element raised it, and no
// other block. Swift AOT path.
//
// Fixture: integration_resources/an_error_ends_the_block_it_was_raised_in/an_error_ends_the_block_it_was_raised_in.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct AnErrorEndsTheBlockItWasRaisedIn {
    /// The control for the refused root start: this document never sends to `#_parent`, so a
    /// root-start policy lets it start, and the checked start runs it as `initialize` would.
    @Test func aRootStartOfAMachineThatNeedsNoParentRuns() throws {
        let engine = Engine(policy: AnErrorEndsTheBlockItWasRaisedInPolicy())
        #expect(engine.rootStartRefusal() == nil)
        try engine.initializeAsRoot()
        #expect(
            !engine.getActiveStates().isEmpty,
            "a root start that is not refused must enter the machine")
    }

    @Test func eachErrorEndsOnlyItsOwnBlock() throws {
        let policy = AnErrorEndsTheBlockItWasRaisedInPolicy()
        let engine = Engine(policy: policy)
        try engine.initialize()
        for event in [AnErrorEndsTheBlockItWasRaisedInEvent.t, .finish] {
            engine.processEvent(event)
        }

        #expect(engine.terminalState == .done, "`finish` must carry the run to `done`")
        let wrong = mismatches([
            ("errors", 10), ("afterAssign", 0), ("afterScript", 0), ("afterLog", 0),
            ("afterCancel", 0), ("afterIfInner", 0), ("afterIf", 0), ("afterSingle", 0),
            ("afterTrans", 0), ("initRan", 1), ("pairs", 4), ("sum", 90), ("ifThen", 0),
            ("ifElse", 1), ("afterIfCond", 0), ("elseifThen", 0), ("elseifElse", 1),
            ("afterElseifCond", 0), ("afterNestedIf", 0), ("afterOuterIf", 0),
        ]) { policy.readScriptValue($0) }
        #expect(wrong.isEmpty, "observed: \(wrong)")
    }
}
