// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML C.2 + 6.2.3 `<send namelist>` over BasicHTTP. Swift AOT path.
//
// Two claims the IRP corpus states and cannot measure:
//
//   the namelist reaches the form   test518 is titled "namelist values get encoded as POST
//     parameters" and passes as soon as the event comes back, whatever it carried.
//
//   an unreadable item reports and discards   W3C SCXML 5.9.2 puts `error.execution` on the internal
//     queue; 6.2.3 discards the message. `<param>`'s per-item exception (5.7.1) has no counterpart
//     for namelist anywhere in the specification.
//
// The fixture sends to `_ioprocessors['basichttp'].location`, the address the host declares for its
// BasicHTTP listener (`basicHTTPAccessURI`), and reads the form back out of the echoed event. The
// host stands in for that listener: the callback it registers keeps the request it was handed and
// answers with the form as the data of the event of the same name, as the harness server does. A
// manual clock stands in for the waits, so the two timeouts that give each phase its verdict pass
// without the test waiting for them.
//
// The two claims reach distinct final states, so a failure names which one broke.
//
// Fixture: integration_resources/send_namelist_over_http/send_namelist_over_http.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import Foundation
import IntegrationMachines
import SceLua
import SceRuntime
import Testing

/// The form of a request as the data of the event that echoes it: each name with its value, or
/// with the list of its values when it repeats.
private func echoedForm(_ params: [String: [String]]) -> String {
    var object: [String: Any] = [:]
    for (name, values) in params {
        object[name] = values.count == 1 ? values[0] : values
    }
    guard let data = try? JSONSerialization.data(withJSONObject: object, options: [.sortedKeys]) else {
        return "{}"
    }
    return String(decoding: data, as: UTF8.self)
}

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct SendNamelistOverHttp {
    private static let accessURI = "http://127.0.0.1:18080/test"

    @Test func theNamelistReachesTheFormAndABrokenItemDiscardsTheMessage() throws {
        let policy = SendNamelistOverHttpPolicy()
        policy.basicHTTPAccessURI = Self.accessURI
        let engine = Engine(policy: policy)
        var posted: [HttpSendRequest] = []
        engine.setHTTPSendCallback { request in
            posted.append(request)
            return HttpSendResponse(eventName: request.eventName, eventData: echoedForm(request.params))
        }
        engine.setClock(ManualClock())
        try engine.initialize()
        _ = driveOnVirtualTime(engine, budgetMs: 10_000)

        switch engine.terminalState {
        case .pass:
            break
        case .failNamelistNeverArrived:
            Issue.record(
                "the BasicHTTP send never came back at all: the listener did not answer, which is a different failure from posting the wrong form"
            )
        case .failNamelistNotPosted:
            Issue.record(
                "`mapped` arrived without `Var1` in its data: W3C SCXML C.2 requires a namelist's variable names and values to be mapped to HTTP POST parameters"
            )
        case .failMessageNotDiscarded:
            Issue.record(
                "`shouldNotArrive` was delivered: W3C SCXML 6.2.3 discards the message when the evaluation of a `<send>`'s arguments produces an error, and `<param>`'s per-item rule (5.7.1) does not reach a namelist item"
            )
        case .failNoNamelistError:
            Issue.record(
                "no `error.execution` preceded the timeout: W3C SCXML 5.9.2 requires it when a location expression yields no valid location, and the wire the send would have crossed does not change the answer"
            )
        default:
            Issue.record(
                "ended in \(String(describing: engine.terminalState)), parked in \(engine.getCurrentState())")
        }

        // What crossed the wire: the namelist's one variable to the address the host declared, and
        // nothing for the send whose item is unreadable.
        #expect(posted.map(\.eventName) == ["mapped"], "only the first send is POSTed")
        #expect(posted.first?.target == Self.accessURI)
        #expect(posted.first?.params["Var1"] == ["2"])
    }
}
