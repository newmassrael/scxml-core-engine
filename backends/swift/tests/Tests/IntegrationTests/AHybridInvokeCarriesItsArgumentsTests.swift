// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4.1 + 6.4.3: an `<invoke>` whose child is named by an expression carries its
// arguments exactly as one whose child is fixed does. Swift AOT path.
//
// The two candidates disagree on purpose: `keeper` declares `seed`, `fromLoc` and `emptyLoc`, `bare`
// declares `nowhere` and nothing else, and the value always names `keeper`. A backend that filtered
// by the wrong candidate's declarations, or by all of them, hands `keeper` a `nowhere` it never
// declared.
//
// Fixture: integration_resources/a_hybrid_invoke_carries_its_arguments/a_hybrid_invoke_carries_its_arguments.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct AHybridInvokeCarriesItsArguments {
    @Test func eachArgumentReachesOnlyTheChildThatDeclaresIt() throws {
        let policy = AHybridInvokeCarriesItsArgumentsPolicy()
        let engine = try initializedOnVirtualTime(policy)
        _ = driveOnVirtualTime(engine)

        #expect(
            engine.isInFinalState,
            "the machine never completed (parked in \(engine.getCurrentState())); refusedPhase parks when its invoke raised nothing"
        )
        #expect(
            engine.terminalState == .done,
            "ended in \(String(describing: engine.terminalState)): FailWrongChild means bare ran; FailRefusedChildStarted means an unreadable namelist still started a child"
        )
        let wrong = mismatches([
            ("errors", 2), ("started", 2), ("paramsOk", 1), ("namelistOk", 1),
        ]) { policy.readScriptValue($0) }
        #expect(wrong.isEmpty, "observed: \(wrong)")
    }
}
