// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE_MESH.md §mesh-19: a `targetexpr` that evaluates to a Mesh peer reaches the host's Mesh router.
// Swift AOT path.
//
// Only the half that needs no router is driven here. The Swift runtime has no Mesh router to
// register (the Mesh row of `a_backend_holds_what_the_other_backends_hold` is still open for
// Swift), so the Kotlin test that registers one and reads the routed request has nothing to call.
// What Swift does hold is the other half of the same document: with no router, a send to a peer is
// a send to a host processor nobody serves, one `error.execution`, and the non-peer target is
// still this session's. When Swift gains a router, the router half is added to this suite.
//
// Fixture: integration_resources/a_peer_named_at_run_time_reaches_the_router/
// a_peer_named_at_run_time_reaches_the_router.scxml (canonical, shared with the other channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct APeerNamedAtRunTimeReachesTheRouter {
    @Test func aPeerWithNoRouterRegisteredIsErrorExecution() {
        let policy = APeerNamedAtRunTimeReachesTheRouterPolicy()
        let engine = Engine(policy: policy)
        engine.initialize()
        engine.tick()

        let wrong = mismatches([("refused", 1), ("looped", 1)]) { policy.readScriptValue($0) }
        #expect(wrong.isEmpty, "observed: \(wrong)")
        #expect(engine.terminalState == .done)
    }
}
