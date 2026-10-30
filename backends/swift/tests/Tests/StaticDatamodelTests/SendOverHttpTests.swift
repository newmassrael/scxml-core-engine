// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceRuntime
import Testing

@testable import StaticDatamodelMachines

// §scxml-C-2: the `<param>`s of a BasicHTTP `<send>` of a `datamodel="sce-static"` machine
// are read from its own fields when the send runs, and cross as the text a form carries
// (docs/adr/0005, decision 4). The request is observed where the engine hands it to its
// transport, so no listener is involved. There is no scenario file: a scenario states what
// a machine's fields hold and not what it sent over a wire, so each backend holds the pairs
// with a test of its own.
//
// Fixture: sce-build/tests/fixtures/static_datamodel/static_send_http.scxml

/// What the transport was handed, oldest first.
private final class Transport {
    var posted: [HttpSendRequest] = []
}

/// A machine whose transport keeps what it was handed.
private func httpMachine() -> (StaticSendHttpPolicy, Engine<StaticSendHttpPolicy>, Transport) {
    let policy = StaticSendHttpPolicy()
    let engine = Engine(policy: policy)
    engine.setClock(ManualClock())
    let transport = Transport()
    engine.setHTTPSendCallback { request in
        transport.posted.append(request)
        return nil
    }
    engine.initialize()
    return (policy, engine, transport)
}

private func raise(_ engine: Engine<StaticSendHttpPolicy>, _ name: String) {
    engine.raiseExternalByName(name, eventData: "")
    engine.step()
}

@Test func aSendOverHttpCarriesTheTextTheFieldsHoldWhenItRuns() {
    let (_, engine, transport) = httpMachine()
    raise(engine, "bump")
    raise(engine, "go")

    #expect(transport.posted.count == 1)
    guard let request = transport.posted.first else {
        return
    }
    #expect(request.target == "http://example.invalid/hook")
    #expect(request.eventName == "note")
    // No <content>, so the body is the pairs.
    #expect(request.content.isEmpty)
    // Each value is the text it spells: a number without a `.0`, a truth value as its word, a
    // text without the quotes JSON would wrap it in.
    #expect(
        request.params == [
            "count": ["4"], "ready": ["true"], "label": ["busy"], "twice": ["8"], "delta": ["-5"],
            "ratio": ["1.5"],
        ]
    )
}

@Test func thePairsAreTheFieldsAsTheyStandAndNotACopyFromStartUp() {
    let (_, engine, transport) = httpMachine()
    raise(engine, "go")

    #expect(transport.posted.count == 1)
    // Without `bump` the fields hold their initial values, and the request is read when the
    // send runs.
    #expect(
        transport.posted.first?.params == [
            "count": ["3"], "ready": ["false"], "label": ["idle"], "twice": ["6"], "delta": ["-5"],
            "ratio": ["1.5"],
        ]
    )
}

@Test func aParamThatCannotBeReadIsLeftOutAndTheRequestStillGoes() {
    let (policy, engine, transport) = httpMachine()
    raise(engine, "bump")
    raise(engine, "boom")

    // The request goes with the pair that could be read: `big` is `count * 2000000000`, which
    // a 32-bit field cannot hold, so its pair is left out and not carried as a zero.
    #expect(transport.posted.count == 1)
    #expect(transport.posted.first?.params == ["count": ["4"]])
    // §scxml-5.7.1: the failed pair is reported as error.execution once.
    #expect(policy.errors == 1)
}
