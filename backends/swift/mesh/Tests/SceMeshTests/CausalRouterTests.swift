// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE-VERIFIES: mesh-10.12
//
// The causal route through the router (docs/adr/0085, sections 3.2 and 9.2), case for case with the
// Rust core's router tests of the route. Each names the change that turns it red.

import SceMesh
import Testing

/// A causal group of three, every copy one machine `list` on its own device; the binding `#people` is
/// the group's route. Unbuffered, so a release is transmitted at once.
private let people = PeerConfig(
    transport: "wss",
    buffer: nil,
    retry: nil,
    stampSequence: false,
    delivery: Delivery(dedup: false, ordered: false),
    responders: ["people"],
    deadlineMs: nil,
    replyEvents: [],
    causal: CausalGroup(name: "people", epoch: 0, view: ["phone/phone_list", "laptop/laptop_list", "tablet/tablet_list"])
)

/// The copy of the list whose replica is `causalReplica(n)`, started.
private func copy(_ n: UInt8) throws -> Router {
    let router = Router(machine: "list", dedupWindow: 8, gapTimeoutMs: 50)
    router.addPeer("people", config: people)
    try router.startCausal("people", start: .newIncarnation(causalReplica(n)), room: 64, announceEveryMs: 100)
    return router
}

/// The single dot an offer was accepted with.
private func accepted(_ routed: Routed) throws -> Dot {
    let effects = try done(routed)
    guard effects.count == 1, case let .accepted(peer, dot) = effects[0], peer == "people" else {
        throw Unexpected(description: "expected one accepted change, got \(effects)")
    }
    return dot
}

/// What was transmitted to `#people`, when that is all that was done.
private func sent(_ routed: Routed) throws -> [[UInt8]] {
    try done(routed).map { effect in
        guard case let .transmit(peer, bytes) = effect, peer == "people" else {
            throw Unexpected(description: "expected only transmissions, got \(effect)")
        }
        return bytes
    }
}

/// The events delivered, when that is all that was done.
private func taken(_ routed: Routed) throws -> [String] {
    try done(routed).map { effect in
        guard case let .deliver(event, _, _, _, _) = effect else {
            throw Unexpected(description: "expected only deliveries, got \(effect)")
        }
        return event
    }
}

/// Why `body` was refused, or nil when it was not.
private func refusal(_ body: () throws(RouterError) -> Void) -> RouterError? {
    do throws(RouterError) {
        try body()
        return nil
    } catch {
        return error
    }
}

/// An envelope from `list` of the event `edit`, tagged `tag` when one is given.
private func edit(_ tag: CausalTag?) throws -> [UInt8] {
    var envelope = Envelope()
    envelope.id = envelopeID(5)
    envelope.source = "list"
    envelope.event_type = "edit"
    envelope.pattern = .fireForget
    envelope.datacontenttype = PayloadCodec.none
    guard let tag else {
        return try encodeEnvelope(envelope)
    }
    return try encodeEnvelope(stamped(envelope, with: tag))
}

/// Three copies of the list, the bytes in flight to each, and what each has taken in: per author, how
/// many of its changes.
private final class RouterWorld {
    var copies: [Router]
    var wire: [(to: Int, bytes: [UInt8])] = []
    var seen: [[UInt32]] = [[0, 0, 0], [0, 0, 0], [0, 0, 0]]
    var rng: CausalWorldRng
    var now: Int64 = 0
    var lossy = true

    init(seed: UInt64) throws {
        copies = try (1...3).map { try copy(UInt8($0)) }
        rng = CausalWorldRng(state: seed)
    }

    private func drawID() -> [UInt8] {
        [UInt8](repeating: UInt8(truncatingIfNeeded: rng.step()), count: 16)
    }

    /// Act on what copy `at` was told to do, as its host would.
    func act(_ at: Int, _ routed: Routed) throws {
        for effect in try done(routed) {
            switch effect {
            case let .transmit(_, bytes):
                for to in 0..<3 where to != at {
                    if lossy && rng.chance(30) {
                        continue
                    }
                    if lossy && rng.chance(10) {
                        wire.append((to: to, bytes: bytes))
                    }
                    wire.append((to: to, bytes: bytes))
                }
            case let .accepted(peer, dot):
                try act(at, copies[at].committed(peer, dot: dot, nowMs: now))
            case let .announceDue(peer):
                try act(at, copies[at].announce(peer, id: drawID(), nowMs: now))
            case let .deliver(_, data, _, _, _):
                try take(at, data)
            default:
                throw Unexpected(description: "copy \(at): unexpected \(effect)")
            }
        }
    }

    /// Copy `at` takes in the change `data` names: `<author> <n> <seen of author 0> <of 1> <of 2>`, what
    /// its author had taken in when it made it. It must be the author's next change, and everything its
    /// author had taken in must be taken in here already.
    func take(_ at: Int, _ data: String) throws {
        let fields = data.split(separator: " ").compactMap { UInt32($0) }
        guard fields.count == 5 else {
            throw Unexpected(description: "copy \(at) takes in \(data), which names no change")
        }
        let author = Int(fields[0])
        guard fields[1] == seen[at][author] + 1 else {
            throw Unexpected(description: "copy \(at) takes in \(data) more than once or past a gap")
        }
        for other in 0..<3 where other != author {
            guard seen[at][other] >= fields[2 + other] else {
                throw Unexpected(description: "copy \(at) takes in \(data) before a change it depends on")
            }
        }
        seen[at][author] = fields[1]
    }

    /// Copy `at` makes its next change.
    func make(_ at: Int) throws {
        let n = seen[at][at] + 1
        let data = "\(at) \(n) \(seen[at][0]) \(seen[at][1]) \(seen[at][2])"
        seen[at][at] = n
        try act(at, copies[at].send(sendRequest("#people", "edit", data), id: drawID(), nowMs: now))
    }

    func deliverOne() throws {
        let index = rng.below(wire.count)
        let (to, bytes) = wire.remove(at: index)
        try act(to, copies[to].receive(peer: "people", bytes: bytes, nowMs: now))
    }
}

@Suite struct CausalRouterTests {
    /// Write-ahead. Red if `Router.send` transmits a causal change when the layer tags it (dispatches
    /// rather than offers): the send then leaves before the journal commit.
    @Test func aCausalSendLeavesOnlyOnceItsJournalRecordIsCommitted() throws {
        let phone = try copy(1)
        let laptop = try copy(2)
        let dot = try accepted(phone.send(sendRequest("#people", "edit", #"{"n":1}"#), id: envelopeID(9), nowMs: 0))
        #expect(dot.seq == 1)
        let released = try sent(phone.committed("people", dot: dot, nowMs: 0))
        #expect(released.count == 1)
        let change = try #require(released.first)
        #expect(try taken(laptop.receive(peer: "people", bytes: change, nowMs: 0)) == ["edit"])
        // The same change again is dropped by its dot, not delivered twice.
        #expect(try done(laptop.receive(peer: "people", bytes: change, nowMs: 0)).isEmpty)
    }

    /// A causal peer the host has not started is the host's error, and so is starting, committing to or
    /// announcing on a peer that is not causal. Red if `causalPeer` lets a plain peer through.
    @Test func aCausalPeerIsUsedOnlyOnceStarted() throws {
        let router = Router(machine: "list", dedupWindow: 8, gapTimeoutMs: 50)
        router.addPeer("people", config: people)
        var plain = people
        plain.causal = nil
        router.addPeer("hmi", config: plain)
        #expect(
            router.send(sendRequest("#people", "edit", ""), id: envelopeID(1), nowMs: 0)
                == Routed.refused(.causalNotStarted("people")))
        #expect(
            router.committed("people", dot: Dot(replica: causalReplica(1), seq: 1), nowMs: 0)
                == Routed.refused(.causalNotStarted("people")))
        let notCausal = refusal { () throws(RouterError) -> Void in
            try router.startCausal("hmi", start: .newIncarnation(causalReplica(1)), room: 8, announceEveryMs: 100)
        }
        #expect(notCausal == RouterError.notCausal("hmi"))
        #expect(router.announce("hmi", id: envelopeID(2), nowMs: 0) == Routed.refused(.notCausal("hmi")))
        let started = router.causalLayer("people") != nil
        #expect(!started, "a causal peer has no layer until the host starts it")
    }

    /// Repair, on a link that comes back. Red if `peerReady` asks for no vector on a causal peer: the
    /// lost change then waits for the period.
    @Test func aLinkThatComesBackSendsAVectorAndRepairsWhatWasLost() throws {
        let phone = try copy(1)
        let laptop = try copy(2)
        let dot = try accepted(phone.send(sendRequest("#people", "edit", ""), id: envelopeID(9), nowMs: 0))
        _ = try sent(phone.committed("people", dot: dot, nowMs: 0))
        let ready = try done(laptop.peerReady("people", nowMs: 10))
        #expect(ready.contains(Effect.announceDue(peer: "people")), "\(ready)")
        let vector = try sent(laptop.announce("people", id: envelopeID(3), nowMs: 10))
        #expect(vector.count == 1)
        let summary = try #require(vector.first)
        let answer = try sent(phone.receive(peer: "people", bytes: summary, nowMs: 10))
        #expect(answer.count == 1)
        let repaired = try #require(answer.first)
        #expect(try taken(laptop.receive(peer: "people", bytes: repaired, nowMs: 10)) == ["edit"])
    }

    /// The period. Red if the tick never asks for a vector, or asks before its period has passed: a
    /// member that lost a change and whose links never drop is then never repaired, or floods them.
    @Test func aCausalPeerIsOwedAVectorOnItsPeriod() throws {
        let phone = try copy(1)
        func due(_ routed: Routed) throws -> Bool {
            try done(routed).contains(Effect.announceDue(peer: "people"))
        }
        #expect(try due(phone.tick(nowMs: 0)))
        _ = try done(phone.announce("people", id: envelopeID(3), nowMs: 0))
        #expect(try !due(phone.tick(nowMs: 99)))
        #expect(try due(phone.tick(nowMs: 100)))
    }

    /// The view. Red if the epoch check is removed: a change tagged with another epoch is then delivered
    /// rather than raised as row 18.
    @Test func aChangeOfAnotherViewIsRaisedAndNotDelivered() throws {
        let laptop = try copy(2)
        let bytes = try edit(CausalTag(epoch: 1, vector: [causalReplica(1).entry(1)]))
        #expect(
            try done(laptop.receive(peer: "people", bytes: bytes, nowMs: 0))
                == [
                    Effect.raise(
                        peer: "people",
                        signal: .causalViewMismatch(
                            source: "list", epoch: 1, replica: uuidText([UInt8](repeating: 1, count: 16))))
                ])
    }

    /// A second incarnation of a member, until view changes land. Red if `admitMember` admits a replica
    /// past the view's size: the change of the copy restored from an older save is then delivered.
    @Test func aReplicaPastTheViewIsRaisedAndNotDelivered() throws {
        let phone = try copy(1)
        let laptop = try copy(2)
        let tablet = try copy(3)
        for vector in try sent(phone.announce("people", id: envelopeID(3), nowMs: 0))
            + sent(tablet.announce("people", id: envelopeID(4), nowMs: 0))
        {
            _ = try done(laptop.receive(peer: "people", bytes: vector, nowMs: 0))
        }
        let bytes = try edit(CausalTag(epoch: 0, vector: [causalReplica(9).entry(1)]))
        #expect(
            try done(laptop.receive(peer: "people", bytes: bytes, nowMs: 0))
                == [
                    Effect.raise(
                        peer: "people",
                        signal: .causalViewMismatch(
                            source: "list", epoch: 0, replica: uuidText([UInt8](repeating: 9, count: 16))))
                ])
    }

    /// An envelope on a causal route without its tag cannot be placed. Red if `receiveCausal` hands an
    /// untagged envelope to the plain rules: it is then delivered with no place in the order.
    @Test func anUntaggedEnvelopeOnACausalRouteIsCorrupt() throws {
        let laptop = try copy(2)
        #expect(
            try done(laptop.receive(peer: "people", bytes: edit(nil), nowMs: 0))
                == [Effect.raise(peer: "people", signal: .envelopeCorrupt(source: "list", codec: "cbor"))])
    }

    /// The world. Three copies on links that lose, duplicate and reorder take in every change exactly
    /// once, in an order that is a linear extension of what each change's author had taken in, with no
    /// gap skipped, and their logs drop to the stable cut once the links are quiet. Red if the hold
    /// branch of causal_admit delivers, or if repair is removed (`receiveCausal` sends no answer to a
    /// `causalVector`: a lost change then never arrives).
    @Test func threeCopiesThroughTheirRoutersTakeInEveryChangeOnceInCausalOrder() throws {
        let changesEach: UInt32 = 15
        for seed: UInt64 in [0x0BAD_5EED, 0x0CA5_CADE, 0x00C0_FFEE] {
            let world = try RouterWorld(seed: seed)
            while (0..<3).contains(where: { world.seen[$0][$0] < changesEach }) || !world.wire.isEmpty {
                world.now += 1
                switch world.rng.below(10) {
                case 0...2:
                    let at = world.rng.below(3)
                    if world.seen[at][at] < changesEach {
                        try world.make(at)
                    }
                case 3:
                    let at = world.rng.below(3)
                    try world.act(at, world.copies[at].tick(nowMs: world.now))
                default:
                    if !world.wire.isEmpty {
                        try world.deliverOne()
                    }
                }
            }
            world.lossy = false
            for _ in 0..<20 {
                world.now += 1
                for at in 0..<3 {
                    let id = [UInt8](repeating: UInt8(truncatingIfNeeded: world.rng.step()), count: 16)
                    try world.act(at, world.copies[at].announce("people", id: id, nowMs: world.now))
                }
                while !world.wire.isEmpty {
                    try world.deliverOne()
                }
            }
            for (at, seen) in world.seen.enumerated() {
                #expect(
                    seen == [changesEach, changesEach, changesEach], "seed \(seed): copy \(at) takes in every change")
                let layer = try #require(world.copies[at].causalLayer("people"))
                #expect(layer.heldCount == 0, "seed \(seed): nothing held")
                #expect(layer.logCount == 0, "seed \(seed): the log is at the cut")
            }
        }
    }
}
