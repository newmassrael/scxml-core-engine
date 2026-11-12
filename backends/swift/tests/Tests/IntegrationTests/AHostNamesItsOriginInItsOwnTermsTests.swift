// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10.1 (docs/adr/0090, test 3): a host names the origin of an event it delivers in its
// own terms. Swift AOT path.
//
// An ending of a host-run invocation whose origin is {x-sce-host, motor} reads back `motor` and
// `x-sce-host`. The address has no scheme, so it is no SCXML location, and the type is the host's:
// the engine neither wraps the one as `sce://scxml/motor` nor defaults the other to the SCXML
// processor's.
//
// Fixture: integration_resources/a_host_names_its_origin_in_its_own_terms/a_host_names_its_origin_in_its_own_terms.scxml
// (canonical, shared with the other channels), generated with `--host-invoker x-sce-host`.
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

/// The tokens of the starts the host was asked for, in order.
private final class Starts {
    var tokens: [UInt64] = []
}

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct AHostNamesItsOriginInItsOwnTerms {
    @Test func aHostNamesItsOriginInItsOwnTerms() throws {
        let hostType = "x-sce-host"
        let policy = AHostNamesItsOriginInItsOwnTermsPolicy()
        let engine = Engine(policy: policy)
        engine.setClock(ManualClock())
        let starts = Starts()
        // An invoker whose work outlives the start: the test ends it, naming the party it stands for.
        try engine.registerInvoker(hostType) { event in
            if case .start(let request) = event {
                starts.tokens.append(request.token)
            }
            return nil
        }
        try engine.initialize()
        engine.tick()
        let token = try #require(starts.tokens.first, "the host was not asked to run `job`")
        #expect(starts.tokens.count == 1, "the host was asked to run `job` \(starts.tokens.count) times")

        #expect(
            engine.completeHostInvoke(
                processorType: hostType, invokeID: "job", token: token, doneData: "{}",
                origin: EventOrigin(processor: hostType, address: "motor")),
            "the running invocation's completion was refused")
        engine.tick()

        #expect(engine.terminalState == .done, "the run must end in `done`; it stopped in \(engine.getCurrentState())")
        let wrong = mismatches([("hostNamed", 1)]) { policy.readScriptValue($0) }
        #expect(wrong.isEmpty, "the completion did not read back the address and the type the host named: \(wrong)")
    }
}
