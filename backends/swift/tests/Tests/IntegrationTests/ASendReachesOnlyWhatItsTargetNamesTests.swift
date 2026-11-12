// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2 + 6.4 + C.1: a `<send>` reaches what its target names, carries its payload there,
// and a target that names nothing reachable is reported. Swift AOT path.
//
// Fixture: integration_resources/a_send_reaches_only_what_its_target_names/a_send_reaches_only_what_its_target_names.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct ASendReachesOnlyWhatItsTargetNames {
    /// W3C SCXML 6.2.4: this document sends to `#_parent`, so a host that runs machines as roots and
    /// asks for the refusal gets it, and nothing starts. The plain `initialize` below runs the same
    /// machine; the refusal is opt-in.
    @Test func aRootStartOfAMachineThatNeedsAParentIsRefused() {
        let engine = Engine(policy: ASendReachesOnlyWhatItsTargetNamesPolicy())
        #expect(engine.rootStartRefusal() == .needsParent)
        #expect(throws: RootStartRefusal.needsParent) { try engine.initializeAsRoot() }
        #expect(engine.getActiveStates().isEmpty, "a refused root start must enter nothing")
        #expect(
            RootStartRefusal.needsParent.description
                == "the machine sends to #_parent and was started with no parent session")
    }

    @Test func aSendReachesOnlyWhatItsTargetNames() throws {
        let policy = ASendReachesOnlyWhatItsTargetNamesPolicy()
        let engine = try initializedOnVirtualTime(policy)
        _ = driveOnVirtualTime(engine)

        #expect(
            engine.isInFinalState,
            "the machine never completed (parked in \(engine.getCurrentState()))")
        #expect(engine.terminalState == .done, "the run must end in `done`")
        let wrong = mismatches([
            ("execErrors", 1), ("commErrors", 5), ("afterRefused", 0), ("afterNobody", 0),
            ("afterStranger", 0), ("afterOrphan", 0), ("afterOrphanExpr", 0),
            ("afterOrphanExprLater", 0), ("bareArrived", 1), ("pongOk", 1),
        ]) { policy.readScriptValue($0) }
        #expect(wrong.isEmpty, "observed: \(wrong)")
    }
}
