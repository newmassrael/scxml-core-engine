// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceRuntime
import Testing

@testable import StaticDatamodelMachines

// A Swift machine sends a composed value by its name (docs/adr/0086, section 5.2), run
// with no script engine, as the Rust, Kotlin, Go, Python and C++ machines do
// (backends/go/tests/integration/static_datamodel/a_composed_value_is_sent_by_its_name_test.go).
// The committed machine `static_composed_send_sm.swift` is generated from
// `sce-build/tests/fixtures/static_datamodel/static_composed_send.scxml`
// (regen: `scripts/regen_static_datamodel_swift.sh`).
//
// The first delivery of `batch.sent` is forwarded to the machine itself: four lists and an
// optional list named as the parts of the payload they are, and a list variable the
// machine took the payload's `late` into. The delivery that comes back is lifted as any
// delivery is (section 4), so it is refused whole if anything written is outside its
// type; it is not, and every value read from it at depths 1 to 3 is what the first
// delivery held.

@Test func aDeliveryForwardedByItsNamesComesBackWhole() throws {
    let policy = StaticComposedSendPolicy()
    let engine = Engine(policy: policy)
    engine.setClock(ManualClock())
    try engine.initialize()
    // Every list full, the second item's `tags` holding three, an element of `maybe`
    // absent and `spare` present (`composedBatch`, AComposedPayloadIsReadAtAnyDepthTests).
    engine.raiseExternalByName("batch.sent", eventData: composedBatch(4, 3, 4))
    engine.step()
    // The one refusal is the list sent short of its field's lower count (docs/adr/0086,
    // section 15.6): sent empty, refused whole and never delivered; sent with its one
    // element, delivered, and forwarded once more with its scalar `hops`, a scalar field
    // of a payload that holds a list.
    #expect(policy.refusals == 1)
    #expect(policy.countedDeliveries == 2)
    #expect(policy.countedSum == 7)
    #expect(policy.countedHops == 3)
    // The forwarded delivery came back on the internal queue.
    #expect(policy.deliveries == 2)
    // Depth 1: the lengths of a list of records and a list of enum values.
    #expect(policy.itemCount == 3)
    #expect(policy.levelCount == 2)
    // Depth 2: each item's list, and a field of an element read by its index.
    #expect(policy.tagCount == 11)
    #expect(policy.idSum == 6)
    // A list of lists, walked at both depths.
    #expect(policy.cellSum == 10)
    // An element that may be absent, present.
    #expect(policy.firstMaybe == 7)
    // The list variable, sent by its name.
    #expect(policy.lateSum == 11)
    #expect(policy.late == [5, 6])
}
