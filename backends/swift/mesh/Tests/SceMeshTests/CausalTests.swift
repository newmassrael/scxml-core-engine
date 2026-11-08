// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE-VERIFIES: mesh-10.12
//
// The causal delivery layer, case for case with the Rust core's causal tests (docs/adr/0085, section
// 9.2). Each names the change to the rule, or to the layer, that turns it red.

import SceMesh
import Testing

/// A small deterministic generator for the worlds of the causal tests (xorshift64).
struct CausalWorldRng {
    var state: UInt64

    mutating func step() -> UInt64 {
        state ^= state << 13
        state ^= state >> 7
        state ^= state << 17
        return state
    }

    mutating func below(_ n: Int) -> Int {
        Int(step() % UInt64(n))
    }

    mutating func chance(_ percent: UInt64) -> Bool {
        step() % 100 < percent
    }
}

/// The replica whose 16 bytes are all `n`.
func causalReplica(_ n: UInt8) -> Replica {
    let half = UInt64(n) &* 0x0101_0101_0101_0101
    return Replica(hi: half, lo: half)
}

/// A message as the layer tests carry it: the tag it was sent with and a name.
private struct Msg: Equatable {
    var tag: CausalTag
    var name: UInt32
}

/// What arrives in the layer world: a change, or the `causalVector` of the copy `from`.
private enum Packet {
    case change(Msg)
    case vector(from: Int, tag: CausalTag)
}

private let threeView = CausalGroup(name: "people", epoch: 0, view: ["phone/list", "laptop/list", "tablet/list"])
private let twoView = CausalGroup(name: "pair", epoch: 0, view: ["phone/list", "laptop/list"])

private func layer(_ group: CausalGroup, _ n: UInt8, _ room: Int) -> CausalLayer<Msg> {
    CausalLayer(group: group, start: .newIncarnation(causalReplica(n)), room: room)
}

/// Offer the change `name` and commit its journal record: the message that leaves for the group.
private func make(_ at: CausalLayer<Msg>, _ name: UInt32) throws -> Msg {
    let verdict = try at.offer { tag in Msg(tag: tag, name: name) }
    guard case let .accepted(dot)? = verdict else {
        throw Unexpected(description: "expected the change accepted, got \(String(describing: verdict))")
    }
    let sent = at.committed(dot)
    guard sent.count == 1, let message = sent.first else {
        throw Unexpected(description: "expected one change released per commit, got \(sent.count)")
    }
    return message
}

/// Give `message` to `at`: the names it delivers, in order.
private func give(_ at: CausalLayer<Msg>, _ message: Msg) throws -> [UInt32] {
    try at.receive(message.tag, message).map(\.name)
}

/// Give `to` the `causalVector` of `from`: what `to` answers with.
@discardableResult
private func exchange(_ from: CausalLayer<Msg>, _ to: CausalLayer<Msg>) throws -> [Msg] {
    try to.receiveSummary(from.summary())
}

private func column(_ at: CausalLayer<Msg>, _ of: Replica) -> UInt64 {
    at.delivered.first { Replica(of: $0) == of }?.seq ?? 0
}

/// Why `body` was rejected, or nil when it was not.
private func rejection<R>(_ body: () throws(CausalRejected) -> R) -> CausalRejected? {
    do throws(CausalRejected) {
        _ = try body()
        return nil
    } catch {
        return error
    }
}

/// Three copies on links that lose, duplicate and reorder, and what each delivered.
private final class LayerWorld {
    var copies: [CausalLayer<Msg>]
    var delivered: [[Msg]] = [[], [], []]
    var made: [UInt32] = [0, 0, 0]
    var wire: [(to: Int, packet: Packet)] = []
    var rng: CausalWorldRng

    init(seed: UInt64) {
        copies = (1...3).map { layer(threeView, UInt8($0), 1000) }
        rng = CausalWorldRng(state: seed)
    }

    func send(_ to: Int, _ packet: Packet, lossy: Bool) {
        if lossy && rng.chance(30) {
            return
        }
        if case let .change(message) = packet, lossy && rng.chance(10) {
            wire.append((to: to, packet: .change(message)))
        }
        wire.append((to: to, packet: packet))
    }

    func deliverOne(lossy: Bool) throws {
        let index = rng.below(wire.count)
        let (to, packet) = wire.remove(at: index)
        switch packet {
        case let .change(message):
            let out = try copies[to].receive(message.tag, message)
            delivered[to].append(contentsOf: out)
        case let .vector(from, tag):
            for message in try copies[to].receiveSummary(tag) {
                send(from, .change(message), lossy: lossy)
            }
        }
    }

    func announce(_ from: Int, lossy: Bool) {
        let tag = copies[from].summary()
        for to in 0..<3 where to != from {
            send(to, .vector(from: from, tag: tag), lossy: lossy)
        }
    }
}

@Suite struct CausalLayerTests {
    /// Drop. Red if `dot.seq <= seen` becomes `<` in causal_admit: the change given again is then
    /// held, not dropped.
    @Test func aChangeGivenAgainIsDroppedAndNotHeld() throws {
        let p = layer(threeView, 1, 16)
        let q = layer(threeView, 2, 16)
        let c1 = try make(p, 1)
        #expect(try give(q, c1) == [1])
        #expect(try give(q, c1) == [])
        #expect(q.heldCount == 0)
        #expect(column(q, causalReplica(1)) == 1)
    }

    /// Deliver. Red if the delivered vector is not raised by the change delivered: the author's next
    /// change is then held.
    @Test func theNextChangeIsDeliveredOnceAndRaisesItsAuthorsColumn() throws {
        let p = layer(threeView, 1, 16)
        let q = layer(threeView, 2, 16)
        let c1 = try make(p, 1)
        let c2 = try make(p, 2)
        #expect(try give(q, c1) == [1])
        #expect(column(q, causalReplica(1)) == 1)
        #expect(try give(q, c2) == [2])
        #expect(column(q, causalReplica(1)) == 2)
    }

    /// Hold, a gap of the author. Red if `dot.seq === seen + 1` is weakened to `dot.seq > seen` in
    /// causal_admit: the change past the gap is then delivered first.
    @Test func aChangePastAGapOfItsAuthorIsHeldUntilTheGapFills() throws {
        let p = layer(threeView, 1, 16)
        let q = layer(threeView, 2, 16)
        let c1 = try make(p, 1)
        let c2 = try make(p, 2)
        #expect(try give(q, c2) == [])
        #expect(q.heldCount == 1)
        #expect(try give(q, c1) == [1, 2])
        #expect(q.heldCount == 0)
    }

    /// Hold, an uncovered context. Red if the check `C[b] <= D[b]` is removed from causal_admit: the
    /// change is then delivered before the change of another author it depends on.
    @Test func aChangeWhoseContextIsNotDeliveredIsHeldUntilItIs() throws {
        let p = layer(threeView, 1, 16)
        let q = layer(threeView, 2, 16)
        let r = layer(threeView, 3, 16)
        let c1 = try make(p, 1)
        #expect(try give(r, c1) == [1])
        let d1 = try make(r, 10)
        #expect(try give(q, d1) == [])
        #expect(try give(q, c1) == [1, 10])
    }

    /// No gap timeout. Red if anything other than the change before it releases a held change, as a
    /// gap timeout that fast-forwards the author's column would.
    @Test func aHeldChangeStaysHeldHoweverManyVectorsPass() throws {
        let p = layer(threeView, 1, 16)
        let q = layer(threeView, 2, 16)
        let r = layer(threeView, 3, 16)
        let c1 = try make(p, 1)
        let c2 = try make(p, 2)
        #expect(try give(q, c2) == [])
        for _ in 0..<1000 {
            try exchange(r, q)
            try exchange(q, r)
        }
        #expect(q.heldCount == 1)
        #expect(column(q, causalReplica(1)) == 0)
        #expect(try give(q, c1) == [1, 2])
    }

    /// Repair by vector. Red if what a vector lacks is computed as empty (causal_lacks answers false):
    /// the lost change is then never sent again.
    @Test func aLostChangeIsReceivedAfterOneVectorExchange() throws {
        let p = layer(threeView, 1, 16)
        let q = layer(threeView, 2, 16)
        _ = try make(p, 1)
        let answer = try exchange(q, p)
        #expect(answer.count == 1)
        let repaired = try #require(answer.first)
        #expect(try give(q, repaired) == [1])
    }

    /// The stable cut. Red if the minimum becomes a maximum in causal_cut (the cut is then {p:2}), or
    /// if a member whose vector is unknown is left out (the cut is then not empty before the third
    /// member is heard from).
    @Test func theStableCutIsTheMinimumOverTheViewWithAnUnknownMemberAtZero() throws {
        let p = layer(threeView, 1, 16)
        let q = layer(threeView, 2, 16)
        let r = layer(threeView, 3, 16)
        let c1 = try make(p, 1)
        let c2 = try make(p, 2)
        _ = try give(q, c1)
        _ = try give(q, c2)
        try exchange(q, p)
        #expect(try p.stableCut() == [])
        _ = try give(r, c1)
        try exchange(r, p)
        #expect(try p.stableCut() == [causalReplica(1).entry(1)])
    }

    /// The mark. Red if a received vector is not recorded as its member's latest (`noteKnown` skipped
    /// in `receiveSummary`): members that never wrote then never move the cut, and the log keeps the
    /// change.
    @Test func membersThatNeverWroteMoveTheCutByTheirVectorsAlone() throws {
        let p = layer(threeView, 1, 16)
        let q = layer(threeView, 2, 16)
        let r = layer(threeView, 3, 16)
        let c1 = try make(p, 1)
        _ = try give(q, c1)
        _ = try give(r, c1)
        #expect(p.logCount == 1)
        try exchange(q, p)
        try exchange(r, p)
        #expect(p.logCount == 0)
    }

    /// Retention. Red if `prune` drops at the copy's own delivered vector instead of the cut: the change
    /// a member still lacks is then gone, and its repair is not answered.
    @Test func theLogDropsAChangeAtTheCutAndAnswersARepairBeforeIt() throws {
        let p = layer(threeView, 1, 16)
        let q = layer(threeView, 2, 16)
        let r = layer(threeView, 3, 16)
        let c1 = try make(p, 1)
        let c2 = try make(p, 2)
        _ = try give(q, c1)
        _ = try give(q, c2)
        _ = try give(r, c1)
        try exchange(q, p)
        let answer = try exchange(r, p)
        #expect(p.logCount == 1, "c1 is stable, c2 is not")
        #expect(answer.count == 1)
        let repaired = try #require(answer.first)
        #expect(try give(r, repaired) == [2])
    }

    /// The verdict. Red if `offer` takes the number before it checks the log's room: the next accepted
    /// change then leaves a gap, and the peer holds it.
    @Test func anOfferPastTheRoomIsRefusedAndTakesNoNumber() throws {
        let p = layer(twoView, 1, 1)
        let q = layer(twoView, 2, 16)
        let c1 = try make(p, 1)
        let refused = try p.offer { tag in Msg(tag: tag, name: 99) }
        #expect(refused == CausalVerdict.refused(.noRoomForHistory))
        #expect(p.ownSeq == 1)
        #expect(try give(q, c1) == [1])
        try exchange(q, p)
        #expect(p.logCount == 0, "stability frees room")
        let c2 = try make(p, 2)
        #expect(c2.tag.vector.first == causalReplica(1).entry(2))
        #expect(try give(q, c2) == [2])
    }

    /// Write-ahead. Red if `offer` logs the change as released when it tags it: a repair asked before
    /// the journal commit is then answered with it.
    @Test func anAcceptedChangeLeavesOnlyOnceItsJournalRecordIsCommitted() throws {
        let p = layer(threeView, 1, 16)
        let q = layer(threeView, 2, 16)
        let verdict = try p.offer { tag in Msg(tag: tag, name: 1) }
        guard case let .accepted(dot)? = verdict else {
            throw Unexpected(description: "expected the change accepted, got \(String(describing: verdict))")
        }
        #expect(dot.seq == 1)
        #expect(try exchange(q, p).isEmpty)
        #expect(p.committed(dot).count == 1)
        #expect(p.committed(dot).isEmpty, "a change is released once")
        #expect(try exchange(q, p).count == 1)
    }

    /// Incarnation. Red if a copy without its journal reuses its previous identity, or continues from a
    /// number any peer reports: its next change then carries the dot of the change an offline member
    /// holds.
    @Test func aCopyWithoutItsJournalBeginsANewIncarnation() throws {
        let p = layer(threeView, 1, 16)
        let q = layer(threeView, 2, 16)
        let c1 = try make(p, 1)
        let c2 = try make(p, 2)
        _ = try give(q, c1)
        // c2 reached only the tablet, which is offline now. The phone is restored from a save taken
        // after c1: it cannot know its highest. A copy whose journal is its own continues from it.
        let record = CausalJournalRecord(replica: causalReplica(1), delivered: [causalReplica(1).entry(2)])
        #expect(CausalStart.fromJournal(record, fresh: causalReplica(9)) == .journal(record))
        let resumed = CausalLayer<Msg>(group: threeView, start: .journal(record), room: 16)
        #expect(try make(resumed, 4).tag.vector.first == causalReplica(1).entry(3))
        let restored = CausalLayer<Msg>(
            group: threeView, start: .fromJournal(nil, fresh: causalReplica(9)), room: 16)
        #expect(restored.replica == causalReplica(9))
        let next = try make(restored, 3)
        let dots = Set([c1, c2, next].map { message -> [UInt64] in
            let dot = message.tag.vector[0]
            return [dot.hi, dot.lo, dot.seq]
        })
        #expect(dots.count == 3, "no dot names two contents")
        #expect(next.tag.vector.first == causalReplica(9).entry(1))
        // Until view changes land, the laptop, which knows the phone's first incarnation and the
        // tablet, does not deliver the second.
        let r = layer(threeView, 3, 16)
        try exchange(r, q)
        let rejected = rejection { () throws(CausalRejected) -> [Msg] in try q.receive(next.tag, next) }
        #expect(rejected == CausalRejected.notInView(replica: causalReplica(9)))
    }

    /// The view. Red if the epoch check in `senderOf` is removed: a change of another epoch is then
    /// delivered.
    @Test func aChangeOfAnotherEpochIsRejectedAndNotDelivered() throws {
        let p = layer(threeView, 1, 16)
        let q = layer(threeView, 2, 16)
        var other = try make(p, 1)
        other.tag.epoch = 1
        let change = other
        let onChange = rejection { () throws(CausalRejected) -> [Msg] in try q.receive(change.tag, change) }
        #expect(onChange == CausalRejected.otherEpoch(epoch: 1))
        let vector = CausalTag(epoch: 1, vector: [causalReplica(1).entry(0)])
        let onVector = rejection { () throws(CausalRejected) -> [Msg] in try q.receiveSummary(vector) }
        #expect(onVector == CausalRejected.otherEpoch(epoch: 1))
        #expect(column(q, causalReplica(1)) == 0)
    }

    private func plainEnvelope() -> Envelope {
        var envelope = Envelope()
        envelope.id = [UInt8](repeating: 7, count: 16)
        envelope.source = "list"
        envelope.event_type = ""
        envelope.pattern = .causalVector
        envelope.datacontenttype = PayloadCodec.none
        return envelope
    }

    /// The wire. A tag is written as keys 19 and 20, the highest keys, so it ends the map: the epoch,
    /// then an array of one map per entry, its keys as text in RFC 8949 order (`hi`, `lo`, `seq`), the
    /// same bytes the Rust core's golden tail holds. Red if a core numbers either key otherwise, or
    /// writes an entry's fields in another order.
    @Test func aTagIsWrittenAsKeys19And20AndReadBack() throws {
        let tag = CausalTag(epoch: 0, vector: [CausalEntryPayload(hi: 1, lo: 2, seq: 3)])
        let bytes = try encodeEnvelope(stamped(plainEnvelope(), with: tag))
        let h = UInt8(ascii: "h")
        let i = UInt8(ascii: "i")
        let l = UInt8(ascii: "l")
        let o = UInt8(ascii: "o")
        let s = UInt8(ascii: "s")
        let e = UInt8(ascii: "e")
        let q = UInt8(ascii: "q")
        let tail: [UInt8] = [
            0x13, 0x00,  // key 19, epoch 0
            0x14, 0x81,  // key 20, an array of one
            0xA3,  // a map of three
            0x62, h, i, 0x01,  // "hi": 1
            0x62, l, o, 0x02,  // "lo": 2
            0x63, s, e, q, 0x03,  // "seq": 3
        ]
        #expect(Array(bytes.suffix(tail.count)) == tail, "\(bytes)")
        #expect(try causalTag(of: decodeEnvelope(bytes)) == tag)
        #expect(try causalTag(of: plainEnvelope()) == nil)
        var half = plainEnvelope()
        half.causal_epoch = 0
        let halfTag = half
        let rejected = rejection { () throws(CausalRejected) -> CausalTag? in try causalTag(of: halfTag) }
        #expect(rejected == CausalRejected.malformed)
    }

    /// The world. Three copies on links that lose, duplicate and reorder deliver every change exactly
    /// once, in an order that is a linear extension of the contexts, with no gap skipped, and their
    /// logs drop to the cut once the links are quiet. Red if the hold branch of causal_admit delivers
    /// (the order check fails), or if repair is removed (`receiveSummary` answers nothing: a lost change
    /// then never arrives).
    @Test func threeCopiesOnLossyReorderingLinksDeliverEveryChangeOnceInCausalOrder() throws {
        let changesEach: UInt32 = 20
        for seed: UInt64 in [0x5EED_0001, 0x5EED_0002, 0x5EED_0003] {
            let world = LayerWorld(seed: seed)
            // The busy part: changes made, delivered out of order, lost and repeated, with vectors
            // exchanged now and then.
            while world.made.contains(where: { $0 < changesEach }) || !world.wire.isEmpty {
                switch world.rng.below(10) {
                case 0...2:
                    let at = world.rng.below(3)
                    if world.made[at] < changesEach {
                        world.made[at] += 1
                        let message = try make(world.copies[at], UInt32(at) * 1000 + world.made[at])
                        world.delivered[at].append(message)
                        for to in 0..<3 where to != at {
                            world.send(to, .change(message), lossy: true)
                        }
                    }
                case 3:
                    world.announce(world.rng.below(3), lossy: true)
                default:
                    if !world.wire.isEmpty {
                        try world.deliverOne(lossy: true)
                    }
                }
            }
            // The quiet part: links that lose nothing, a vector from each copy per round, until every
            // copy holds every change.
            for _ in 0..<20 {
                for from in 0..<3 {
                    world.announce(from, lossy: false)
                }
                while !world.wire.isEmpty {
                    try world.deliverOne(lossy: false)
                }
            }
            for (at, order) in world.delivered.enumerated() {
                #expect(order.count == 3 * Int(changesEach), "seed \(seed): copy \(at) delivers every change")
                var columns: [Replica: UInt64] = [:]
                for message in order {
                    let dot = message.tag.vector[0]
                    let author = Replica(of: dot)
                    let before = columns[author] ?? 0
                    #expect(
                        dot.seq == before + 1,
                        "seed \(seed): copy \(at) delivers \(message.name) once and skips no gap")
                    for needed in message.tag.vector.dropFirst() {
                        let have = columns[Replica(of: needed)] ?? 0
                        #expect(
                            needed.seq <= have,
                            "seed \(seed): copy \(at) delivers \(message.name) before its context")
                    }
                    columns[author] = dot.seq
                }
                #expect(world.copies[at].heldCount == 0, "seed \(seed): nothing held")
                #expect(world.copies[at].logCount == 0, "seed \(seed): the log is at the cut")
            }
        }
    }
}
