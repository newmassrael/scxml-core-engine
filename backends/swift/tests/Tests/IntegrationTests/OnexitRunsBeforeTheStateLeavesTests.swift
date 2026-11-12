// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix D exitStates: a state is still active while its own `<onexit>` runs. Swift
// AOT path.
//
// Fixture: integration_resources/onexit_runs_before_the_state_leaves/onexit_runs_before_the_state_leaves.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct OnexitRunsBeforeTheStateLeaves {
    @Test func aStateIsStillActiveWhileItsOwnOnexitRuns() throws {
        // The handlers record with `<assign>`, so this is an ECMAScript-datamodel machine.
        let policy = OnexitRunsBeforeTheStateLeavesPolicy()
        let engine = Engine(policy: policy)
        try engine.initialize()
        #expect(engine.getCurrentState() == .inner, "the run has to start inside `inner`")

        engine.processEvent(.leave)

        // The document checks its clauses in document order and lands each in a `<final>` of its
        // own, so the state reported here names which one broke; the records say what the handler
        // actually saw.
        let records = ["selfInInner", "parentInInner", "selfInOuter", "childInOuter", "exits"]
            .map { "\($0)=\(String(describing: policy.readScriptValue($0)))" }
            .joined(separator: " ") + "; wanted 1 / 1 / 1 / 0 / 2"
        #expect(
            engine.getCurrentState() == .settled,
            "(\(records)) `leave` did not carry the machine to `settled`: `failExits` is a handler that did not run, `failSelfInInner` / `failSelfInOuter` a state already out of the configuration during its own `<onexit>`, `failParentInInner` the parent leaving before its child's `<onexit>`, `failChildInOuter` the child still active during its parent's `<onexit>`"
        )
    }
}
