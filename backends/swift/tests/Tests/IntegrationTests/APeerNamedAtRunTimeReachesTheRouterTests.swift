// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE_MESH.md §mesh-19: a `targetexpr` that evaluates to a Mesh peer reaches the host's Mesh router.
// Swift AOT path.
//
// The peer is named only once the expression is evaluated, so the machine chooses the act on the
// evaluated target: a peer is handed to the router the host registered (`registerMeshRouter`), as a
// literal `#peer` is, and with no router registered it is a send to a host processor nobody serves,
// one `error.execution`. The non-peer target is this session's either way.
//
// Fixture: integration_resources/a_peer_named_at_run_time_reaches_the_router/
// a_peer_named_at_run_time_reaches_the_router.scxml (canonical, shared with the other channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

/// The requests the Mesh router was handed, in order.
private final class Routed {
    var requests: [HostSendRequest] = []
}

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct APeerNamedAtRunTimeReachesTheRouter {
    /// With a router registered, the peer send is the router's: processor `sce:mesh`, the evaluated
    /// target, the event. The non-peer target is still this session's.
    @Test func aPeerNamedAtRunTimeIsSentToTheRouter() throws {
        let policy = APeerNamedAtRunTimeReachesTheRouterPolicy()
        let engine = Engine(policy: policy)
        let routed = Routed()
        engine.registerMeshRouter { request in
            routed.requests.append(request)
            return []
        }
        try engine.initialize()
        engine.tick()

        #expect(routed.requests.count == 1, "exactly the peer send reaches the router: \(routed.requests)")
        #expect(routed.requests.first?.processorType == meshProcessorType)
        #expect(routed.requests.first?.target == "#hmi")
        #expect(routed.requests.first?.eventName == "ping")
        let wrong = mismatches([("refused", 0), ("looped", 1)]) { policy.readScriptValue($0) }
        #expect(wrong.isEmpty, "observed: \(wrong)")
        #expect(engine.terminalState == .done)
    }

    /// With no router registered, the peer send is a send to a host processor nobody serves: one
    /// error.execution, and the non-peer target is unaffected.
    @Test func aPeerWithNoRouterRegisteredIsErrorExecution() throws {
        let policy = APeerNamedAtRunTimeReachesTheRouterPolicy()
        let engine = Engine(policy: policy)
        try engine.initialize()
        engine.tick()

        let wrong = mismatches([("refused", 1), ("looped", 1)]) { policy.readScriptValue($0) }
        #expect(wrong.isEmpty, "observed: \(wrong)")
        #expect(engine.terminalState == .done)
    }
}
