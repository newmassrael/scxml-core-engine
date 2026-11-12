// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceRuntime
import Testing

@testable import StaticDatamodelMachines

// A Swift machine reads a composed payload at any depth (docs/adr/0086, section 6), run
// with no script engine, as the Rust, Kotlin, Go, Python and C++ machines do
// (backends/go/tests/integration/static_datamodel/a_composed_payload_is_read_at_any_depth_test.go).
// The committed machine `static_composed_payload_sm.swift` is generated from
// `sce-build/tests/fixtures/static_datamodel/static_composed_payload.scxml`
// (regen: `scripts/regen_static_datamodel_swift.sh`).
//
// A delivery whose every list is full is lifted into the machine's own types, and each
// read at depths 1 to 3 is what a host reads back. A delivery one past a bound at depth 2
// is refused whole (section 4.2): none of the content runs, and `error.execution` is
// raised once. A value a host injects through the seam is held to every bound its type
// declares, at any depth, before anything is raised (section 5.4): within them it is read
// as any delivery, and one past a bound is thrown as the refusal that names its path, and
// raises nothing.

/// One item of a batch as a delivery carries it, its `tags` holding `tags` texts.
private func composedItem(id: Int, tags: Int) -> String {
    let words = (0..<tags).map { "\"tag\($0)\"" }.joined(separator: ",")
    return #"{"id":\#(id),"amount":"9007199254740993","label":"abcdefgh","level":"high","due":null,"blob":"wxyz","tags":[\#(words)]}"#
}

/// A delivery of `batch.sent` whose every list is full but the items' `tags`, item n
/// holding `tags[n - 1]` texts: what the machines that read and send a composed payload
/// are each handed.
func composedBatch(_ tags: Int...) -> String {
    let items = tags.enumerated().map { composedItem(id: $0.offset + 1, tags: $0.element) }
    return #"{"items":["# + items.joined(separator: ",")
        + #"],"grid":[[1,2],[3,4]],"levels":["low","critical"],"maybe":[7,null],"spare":[1,2],"note":"abcd","late":[5,6]}"#
}

/// A started static_composed_payload machine, and its engine.
private func startedComposedPayload() -> (StaticComposedPayloadPolicy, Engine<StaticComposedPayloadPolicy>) {
    let policy = StaticComposedPayloadPolicy()
    let engine = Engine(policy: policy)
    engine.setClock(ManualClock())
    startRecordingARefusal(engine)
    return (policy, engine)
}

/// A static_composed_payload machine that took one delivery of `batch.sent` carrying
/// `data`.
private func deliveredComposedPayload(_ data: String) -> StaticComposedPayloadPolicy {
    let (policy, engine) = startedComposedPayload()
    engine.raiseExternalByName("batch.sent", eventData: data)
    engine.step()
    return policy
}

@Test func aFullDeliveryIsReadAtEveryDepth() {
    let policy = deliveredComposedPayload(composedBatch(4, 4, 4))
    #expect(policy.refusals == 0)
    // Depth 1: the length of a list of the payload.
    #expect(policy.itemCount == 3)
    // Depth 2: a loop over each item's list, and a field of an element read through
    // its index.
    #expect(policy.tagCount == 12)
    #expect(policy.wordCount == 12)
    #expect(policy.idSum == 6)
    // A list of lists, walked at both depths and read by two indices.
    #expect(policy.cellSum == 10)
    #expect(policy.cell == 3)
    // Depth 3: the length of the first item's `tags`.
    #expect(policy.firstTags == 4)
    // An element that may be absent, present.
    #expect(policy.firstMaybe == 7)
    // A list taken whole into a list of a larger bound.
    #expect(policy.late == [5, 6])
}

@Test func aListPastItsBoundAtDepthTwoIsRefusedWhole() {
    let policy = deliveredComposedPayload(composedBatch(4, 5, 4))
    #expect(policy.refusals == 1)
    #expect(policy.itemCount == 0)
    #expect(policy.late.isEmpty)
}

/// An item a host builds, its `tags` holding the texts given.
private func composedHostItem(_ id: UInt32, _ tags: String...) -> StaticComposedPayloadItemRecord {
    StaticComposedPayloadItemRecord(
        id: id, amount: 2, label: "abc", level: .low, due: nil, blob: Array("wx".utf8), tags: tags)
}

/// A started machine whose seam was handed `batch.sent` with `late` and `items`, after it
/// ran, and what the seam threw, if it threw.
private func injectedComposedPayload(
    late: [UInt8], items: [StaticComposedPayloadItemRecord]
) -> (StaticComposedPayloadPolicy, BoundExceeded?) {
    let (policy, engine) = startedComposedPayload()
    var refused: BoundExceeded?
    do {
        try engine.raiseBatchSent(
            items: items, grid: [[1, 2]], levels: [.low], maybe: [9, nil], spare: nil, note: "ab",
            late: late)
    } catch let refusal as BoundExceeded {
        refused = refusal
    } catch {
        Issue.record("the seam threw what is no refusal of a bound: \(error)")
    }
    engine.step()
    return (policy, refused)
}

@Test func aValueAHostInjectsWithinItsBoundsIsReadAsAnyDelivery() {
    let (policy, refused) = injectedComposedPayload(late: [5, 6], items: [composedHostItem(4, "t0", "t1")])
    #expect(refused == nil)
    #expect(policy.refusals == 0)
    #expect(policy.itemCount == 1)
    #expect(policy.tagCount == 2)
    #expect(policy.idSum == 4)
    #expect(policy.cellSum == 3)
    #expect(policy.firstMaybe == 9)
    #expect(policy.late == [5, 6])
}

// `late` is a list<uint8>[2]: three elements are no value of its type, and the seam
// refuses them before anything is raised.
@Test func aListAHostInjectsOnePastItsBoundIsRefusedBeforeItIsRaised() {
    let (policy, refused) = injectedComposedPayload(late: [5, 6, 7], items: [composedHostItem(4, "t0")])
    #expect(refused?.reason == "'late' holds 3 elements, past the 2 its type declares")
    #expect(policy.itemCount == 0)
    #expect(policy.refusals == 0)
}

// `tags` is a list<string[8]>[4] inside each item: an item of five tags is refused at the
// path of the list in the element, and a tag of nine bytes at its own.
@Test func aRecordAHostBuildsOnePastAnInnerBoundIsRefusedAtItsPath() {
    let (policy, refused) = injectedComposedPayload(
        late: [5], items: [composedHostItem(1, "a", "b", "c", "d", "e")])
    #expect(refused?.reason == "'items/0/tags' holds 5 elements, past the 4 its type declares")
    #expect(policy.itemCount == 0)
    let (_, nine) = injectedComposedPayload(late: [5], items: [composedHostItem(1, "ninebytes")])
    #expect(nine?.reason == "'items/0/tags/0' is 9 bytes, past the 8 its type declares")
}
