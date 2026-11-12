// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2.4 + C.2: a delay is a property of the send, not of the processor it names. A
// BasicHTTP `<send delay=...>` is POSTed when the delay has elapsed, not at once and not never, and
// `<cancel>` reaches it while it waits. Swift AOT path.
//
// The host stands in for the HTTP transport with a callback that keeps what it was handed, and
// advances a manual clock, so what the document did is read off the requests and when.
//
// Fixture: integration_resources/a_delayed_http_send_is_posted_when_due/a_delayed_http_send_is_posted_when_due.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct ADelayedHttpSendIsPostedWhenDue {
    @Test func aDelayedHttpSendIsPostedWhenDue() throws {
        let engine = Engine(policy: ADelayedHttpSendIsPostedWhenDuePolicy())
        var posted: [HttpSendRequest] = []
        engine.setHTTPSendCallback { request in
            posted.append(request)
            return nil
        }
        engine.setClock(ManualClock())
        try engine.initialize()
        engine.tick()

        // A zero wait, written or evaluated, is no deferral: the POST is made before
        // initialize() returns, with no tick to bring it out.
        #expect(
            posted.map(\.eventName) == ["now", "zero", "zeroexpr"],
            "the undelayed send and the two zero-delay sends are POSTed at once")

        engine.advanceTimeMs(99)
        engine.tick()
        #expect(
            posted.map(\.eventName) == ["now", "zero", "zeroexpr"],
            "a send delayed 100ms is not POSTed at 99ms")

        engine.advanceTimeMs(1)
        engine.tick()
        #expect(
            posted.map(\.eventName) == ["now", "zero", "zeroexpr", "later"],
            "it is POSTed when the delay has elapsed, and the cancelled one never is")
        #expect(posted.count == 4)
        if posted.count == 4 {
            #expect(posted[3].target == "http://127.0.0.1:18081/later")
            #expect(posted[3].sendID == "later")
        }

        engine.advanceTimeMs(100)
        engine.tick()
        #expect(posted.map(\.eventName) == ["now", "zero", "zeroexpr", "later", "dynamic"])
        #expect(posted.count == 5)
        if posted.count == 5 {
            #expect(
                posted[4].target == "http://127.0.0.1:18081/dynamic",
                "a targetexpr is read when the send is made")
            #expect(posted[4].params["k"] == ["v"])
        }

        engine.advanceTimeMs(100)
        engine.tick()
        #expect(engine.terminalState == .done, "the run must end in `done`")
        #expect(posted.map(\.eventName) == ["now", "zero", "zeroexpr", "later", "dynamic"])
    }
}
