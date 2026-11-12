// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10 + B.2: a host's payload arrives as the value it was sent as. Swift AOT path.
//
// Fixture: integration_resources/event_data_arrives_as_sent/event_data_arrives_as_sent.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct EventDataArrivesAsSent {
    @Test func aHostsJsonPayloadIsAddressableAndItsTextStaysText() throws {
        // The fixture reads `_event.data` in its guards, so this is an ECMAScript-datamodel machine.
        let engine = Engine(policy: EventDataArrivesAsSentPolicy())
        try engine.initialize()

        // A JSON object, the shape an embedder has when it holds structured data and a state
        // machine to give it to.
        engine.processEventWithMeta(
            .payload, metadata: EventMetadata(data: #"{"milestone":"refined","turns":2}"#))

        #expect(
            engine.getCurrentState() != .mangled,
            "the host sent a JSON object and the guard `_event.data.milestone === 'refined' && _event.data.turns === 2` did not hold, so the payload did not arrive as an object with those properties"
        )
        #expect(
            engine.getCurrentState() == .heard,
            "the payload guard neither matched nor mismatched: the machine is not in `heard`")

        // Text that is not JSON. The same call, and it must NOT be parsed into something else:
        // `hold the line` is the value the document compares against, character for character.
        engine.processEventWithMeta(.note, metadata: EventMetadata(data: "hold the line"))

        #expect(
            engine.getCurrentState() != .garbled,
            "the host sent the text `hold the line` and `_event.data === 'hold the line'` did not hold, so a payload that is not JSON did not arrive as the string it was sent as"
        )

        // Text that happens to be a valid expression. The payload has three readings and none of
        // them is "evaluate it": a payload is what a host, a peer session or an HTTP sender put
        // there, and running it makes `_event.data` mean whatever the receiver's engine is
        // written in.
        engine.processEventWithMeta(.arith, metadata: EventMetadata(data: "2 + 3"))

        #expect(
            engine.getCurrentState() != .evaluated,
            "the host sent the text `2 + 3` and it arrived as 5: the payload was run rather than read"
        )
        #expect(
            engine.getCurrentState() == .documented,
            "the arithmetic-shaped payload neither matched nor mismatched")

        // The XML rung, reached through the EVENT path. The `<data>` path is
        // `xml_data_is_a_dom_tree`'s and the two are lowered on separate code in every backend.
        // Leading whitespace on purpose: the reading is chosen by the first NON-blank character,
        // and a pretty-printed document is the ordinary shape of one.
        engine.processEventWithMeta(
            .doc, metadata: EventMetadata(data: "\n  " + #"<books xmlns=""><book title="t1"/></books>"#))

        #expect(
            engine.getCurrentState() != .flattened,
            "the host sent a well-formed XML document and `_event.data.documentElement.nodeName === 'books'` did not hold, so the payload did not become the DOM structure the clause requires"
        )

        // The sentence that closes the clause. Every `error.*` message this repository raises
        // names the SCXML construct that failed, so every one of them has exactly this shape: it
        // opens like a document and is not one.
        engine.processEventWithMeta(
            .broken, metadata: EventMetadata(data: "<assign>  to  detail failed"))

        #expect(
            engine.getCurrentState() != .swallowed,
            "the host sent `<assign>  to  detail failed`, which opens with `<` and is not a valid XML document, so the closing MUST applies and the reading is the space-normalized string"
        )
        #expect(
            engine.getCurrentState() == .settled,
            "the malformed-XML payload neither matched nor mismatched")
    }
}
