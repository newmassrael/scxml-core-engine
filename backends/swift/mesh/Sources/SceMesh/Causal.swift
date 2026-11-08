// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The causal route's delivery layer (docs/adr/0085, sections 3, 3.2, 3.3 and 4.2; SCE_MESH.md section
// 10.12): one copy's side of a causal broadcast group. The Swift twin of
// backends/rust/mesh/src/causal.rs, the reference.
//
// A change this copy makes is offered here and answered with a verdict: `CausalVerdict.accepted` with
// the dot the layer tagged it with, or `CausalVerdict.refused` when the log has no room for one more
// change not yet stable. An accepted change is released to the peers only once the host reports its
// journal record committed (`CausalLayer.committed`), so a number no peer has seen is the only one a
// crash can lose. A change that arrives is delivered once, and only after every change in its context;
// one that is not yet deliverable is held, and a gap is repaired by exchanging vectors, never skipped:
// there is no gap timeout. Every change sent or delivered is kept until the stable cut passes it.
//
// Every decision is a call into the generated standard rules (`sce:std/mesh`): whether a change is
// dropped, delivered or held (`CausalAdmit`), the merge of two vectors (`CausalRaise`), whether a
// vector lacks a change (`CausalLacks`) and the stable cut (`CausalCut`). What this file adds is the
// state those rules are applied to and the messages themselves while they are held or logged. It is
// generic over the message, which the router instantiates with an envelope's bytes, and transport-free,
// like the rest of the core.
//
// The author of a dot is a replica: one incarnation of one member, a UUID v7 the host draws when an
// incarnation begins and keeps in the copy's journal beside the highest number it used
// (`CausalStart`). A copy that cannot know its highest number begins a new incarnation, never one asked
// of its peers.

import SceForgeRuntime

/// The most entries a vector holds: the bound of a view, of the envelope's `causal_vector` and of the
/// rules' lists.
public let causalVectorBound = 64

/// A replica identity: one incarnation of one member of a group, a 16-byte UUID v7 unique within the
/// group and never used again, held as the rules hold it: its first eight bytes and its last eight,
/// big-endian (`sce:std/mesh/causal_entry`). Two identities order as their byte strings do.
public struct Replica: Hashable, Comparable, Sendable, CustomStringConvertible {
    public let hi: UInt64
    public let lo: UInt64

    public init(hi: UInt64, lo: UInt64) {
        self.hi = hi
        self.lo = lo
    }

    /// The replica whose identity is the 16 bytes `bytes`, and nil for any other length.
    public init?(bytes: [UInt8]) {
        guard bytes.count == 16 else {
            return nil
        }
        func half(_ from: Int) -> UInt64 {
            var value: UInt64 = 0
            for index in from..<(from + 8) {
                value = (value << 8) | UInt64(bytes[index])
            }
            return value
        }
        self.init(hi: half(0), lo: half(8))
    }

    /// The replica an entry names.
    public init(of entry: CausalEntryPayload) {
        self.init(hi: entry.hi, lo: entry.lo)
    }

    /// The identity's 16 bytes.
    public var bytes: [UInt8] {
        var out: [UInt8] = []
        out.reserveCapacity(16)
        for half in [hi, lo] {
            for shift in stride(from: 56, through: 0, by: -8) {
                out.append(UInt8(truncatingIfNeeded: half >> UInt64(shift)))
            }
        }
        return out
    }

    /// This replica's entry at `seq`.
    public func entry(_ seq: UInt64) -> CausalEntryPayload {
        CausalEntryPayload(hi: hi, lo: lo, seq: seq)
    }

    /// The identity as RFC 4122 text, as row 18 of SCE_MESH.md section 16.7 names it.
    public var description: String {
        uuidText(bytes)
    }

    public static func < (lhs: Replica, rhs: Replica) -> Bool {
        (lhs.hi, lhs.lo) < (rhs.hi, rhs.lo)
    }
}

/// The dot a change is tagged with: its author and its number, from 1 with no gap.
public struct Dot: Equatable, Sendable {
    public var replica: Replica
    public var seq: UInt64

    public init(replica: Replica, seq: UInt64) {
        self.replica = replica
        self.seq = seq
    }

    /// The dot an entry names.
    public init(of entry: CausalEntryPayload) {
        self.init(replica: Replica(of: entry), seq: entry.seq)
    }

    /// This dot as an entry of a vector.
    public var entry: CausalEntryPayload {
        replica.entry(seq)
    }
}

/// A causal binding's group as the deployment gives it: its name and the view of epoch 0, the only view
/// until view changes land (docs/adr/0085, section 3.3). Each entry of `view` names one instance as
/// `<device>/<machine>`.
public struct CausalGroup: Equatable, Sendable {
    public var name: String
    public var epoch: UInt64
    public var view: [String]

    public init(name: String, epoch: UInt64, view: [String]) {
        self.name = name
        self.epoch = epoch
        self.view = view
    }
}

/// What a tagged envelope or a `causalVector` carries: the view's epoch (envelope key 19) and a vector
/// (key 20) whose first entry is its sender's own. On a change that entry is the change's dot and the
/// rest its context; on a `causalVector` the vector is the sender's delivered vector.
public struct CausalTag: Equatable, Sendable {
    public var epoch: UInt64
    public var vector: [CausalEntryPayload]

    public init(epoch: UInt64, vector: [CausalEntryPayload]) {
        self.epoch = epoch
        self.vector = vector
    }
}

/// What the host reads from the copy's own journal when a copy starts: the replica identity of its
/// incarnation and its delivered vector, its own entry being the highest number it used.
public struct CausalJournalRecord: Equatable, Sendable {
    public var replica: Replica
    public var delivered: [CausalEntryPayload]

    public init(replica: Replica, delivered: [CausalEntryPayload]) {
        self.replica = replica
        self.delivered = delivered
    }
}

/// How an incarnation starts (docs/adr/0085, section 3.3.2).
public enum CausalStart: Equatable, Sendable {
    /// A normal restart: the copy's journal says who it is and what it delivered, and it continues
    /// after its highest number.
    case journal(CausalJournalRecord)

    /// A copy that cannot know its own highest number begins a new incarnation, with an identity the
    /// host has just drawn.
    case newIncarnation(Replica)

    /// The start of a copy whose journal gave `journal`, or none: a copy restored from an older save,
    /// or set up again from its seed, has no journal record of its own, and begins under `fresh`
    /// instead. It never asks its peers for its number: the change with the highest one may be held
    /// only by a member that is offline.
    public static func fromJournal(_ journal: CausalJournalRecord?, fresh: Replica) -> CausalStart {
        if let journal {
            return .journal(journal)
        }
        return .newIncarnation(fresh)
    }
}

/// The layer's answer to a change its copy offers (docs/adr/0093, section 3.1).
public enum CausalVerdict: Equatable, Sendable {
    /// Tagged with this dot; the host records it in the journal, then calls `CausalLayer.committed`.
    case accepted(Dot)

    /// Not tagged: it took no number, and no copy takes it in.
    case refused(CausalRefusal)
}

/// Why an offer was refused.
public enum CausalRefusal: Equatable, Sendable {
    /// The log holds as many changes not yet stable as the deployment allows; the list tells it as
    /// `noRoomForHistory`, and stability frees room.
    case noRoomForHistory

    /// The incarnation used every number a dot can carry.
    case numbersExhausted
}

/// Why something that arrived was not taken in.
public enum CausalRejected: Error, Equatable, Sendable {
    /// Tagged with another epoch than the current view's: raised, not delivered.
    case otherEpoch(epoch: UInt64)

    /// Tagged by a replica the view does not hold: raised, not delivered.
    case notInView(replica: Replica)

    /// A vector with no sender entry, or more entries than a view holds.
    case malformed

    /// A rule refused its input.
    case rule(AlgorithmError)
}

/// One copy's causal delivery layer for one group. Confined to one thread, as the router is.
public final class CausalLayer<Message> {
    /// A change in the log: its dot, the message as it was sent, and whether it has been released to
    /// the peers.
    private struct Logged {
        var dot: CausalEntryPayload
        var message: Message
        var released: Bool
    }

    /// A change held until it is deliverable: its tag and its message.
    private struct Held {
        var dot: CausalEntryPayload
        var context: [CausalEntryPayload]
        var message: Message
    }

    public let group: CausalGroup

    /// The replica identity this incarnation tags its changes with.
    public let replica: Replica

    /// What this copy delivered, its own entry among them.
    public private(set) var delivered: [CausalEntryPayload]

    /// The latest delivered vector known of each other member, by replica.
    private var known: [Replica: [CausalEntryPayload]] = [:]
    private var held: [Held] = []
    private var log: [Logged] = []

    /// The most changes the log may hold not yet stable.
    private let room: Int

    /// A layer for `group`, started as `start` says, whose log holds at most `room` changes not yet
    /// stable.
    public init(group: CausalGroup, start: CausalStart, room: Int) {
        self.group = group
        self.room = room
        switch start {
        case let .journal(record):
            replica = record.replica
            delivered = record.delivered
        case let .newIncarnation(fresh):
            replica = fresh
            delivered = []
        }
        // The copy's own entry is always present, at 0 before its first change, so its vector says who
        // sent it.
        let own = replica
        if !delivered.contains(where: { Replica(of: $0) == own }) {
            delivered.insert(own.entry(0), at: 0)
        }
    }

    /// How many changes are held until they are deliverable.
    public var heldCount: Int {
        held.count
    }

    /// How many changes the log keeps until the stable cut passes them.
    public var logCount: Int {
        log.count
    }

    /// The highest number this incarnation used.
    public var ownSeq: UInt64 {
        seen(delivered, replica)
    }

    /// Offer a change this copy made. When the log has room, the layer tags it with the next dot and its
    /// delivered vector, asks `build` for the message that carries that tag, counts the change delivered
    /// here and logs it; the message is released by `committed`. When the log has no room the change is
    /// refused and takes no number. When `build` answers nil, nothing is taken either, and the offer
    /// answers nil.
    public func offer(_ build: (CausalTag) -> Message?) throws(AlgorithmError) -> CausalVerdict? {
        if log.count >= room {
            return .refused(.noRoomForHistory)
        }
        let own = ownSeq
        if own == UInt64.max {
            return .refused(.numbersExhausted)
        }
        let seq = own + 1
        let dot = replica.entry(seq)
        guard let raised = raise(delivered, by: [dot]) else {
            throw AlgorithmError.capacityExceeded
        }
        let tag = tagOf(dot, raised)
        guard let message = build(tag) else {
            return nil
        }
        delivered = raised
        log.append(Logged(dot: dot, message: message, released: false))
        return .accepted(Dot(replica: replica, seq: seq))
    }

    /// The host committed the journal record of `dot`: every accepted change of this incarnation up to
    /// it may now leave, in the order it was accepted. These are the messages to send to the group.
    public func committed(_ dot: Dot) -> [Message] {
        guard dot.replica == replica else {
            return []
        }
        var out: [Message] = []
        for index in log.indices {
            let logged = log[index]
            if !logged.released && Replica(of: logged.dot) == replica && logged.dot.seq <= dot.seq {
                log[index].released = true
                out.append(logged.message)
            }
        }
        return out
    }

    /// A change that arrived, tagged `tag`: what it lets this copy deliver, in a causal order. A change
    /// delivered already is dropped, the next change of its author whose context is delivered is
    /// delivered with every held change that then follows, and any other is held. An echo of this
    /// copy's own change is dropped.
    public func receive(_ tag: CausalTag, _ message: Message) throws(CausalRejected) -> [Message] {
        let sender = try senderOf(tag)
        let author = Replica(of: sender)
        if author == replica {
            return []
        }
        try admitMember(author)
        try noteKnown(author, tag.vector)
        var out: [Message] = []
        switch try admission(sender, tag.vector) {
        case .drop:
            break
        case .hold:
            let already = held.contains { $0.dot == sender }
            // A hold past the log's room is let go: the change is still the sender's, and a later
            // vector exchange brings it again.
            if !already && held.count < room {
                held.append(Held(dot: sender, context: tag.vector, message: message))
            }
        case .deliver:
            try deliver(sender, message, into: &out)
            try drainHeld(into: &out)
            try prune()
        }
        return out
    }

    /// This copy's `causalVector`: the epoch and its delivered vector, its own entry first. Sent on a
    /// period and whenever a link comes back.
    public func summary() -> CausalTag {
        tagOf(replica.entry(ownSeq), delivered)
    }

    /// A member's `causalVector`: recorded as that member's latest known vector (its mark, which moves
    /// the stable cut), and answered with every released change of the log the vector lacks, in the
    /// order the log holds them (a causal order), as they were sent.
    public func receiveSummary(_ tag: CausalTag) throws(CausalRejected) -> [Message] {
        let sender = try senderOf(tag)
        let author = Replica(of: sender)
        if author == replica {
            return []
        }
        try admitMember(author)
        try noteKnown(author, tag.vector)
        let answer = log.filter { $0.released && CausalLacks.causalLacks(tag.vector, $0.dot) }.map(\.message)
        try prune()
        return answer
    }

    /// The stable cut: the pointwise minimum, over the view's members, of the latest delivered vector
    /// known from each, with a member not heard from at 0 (`CausalCut`). This copy's own vector comes
    /// first, then each other member's in the order of its replica.
    public func stableCut() throws(CausalRejected) -> [CausalEntryPayload] {
        var rows: [CausalKnownPayload] = []
        let others = known.sorted { $0.key < $1.key }.map { (holder: $0.key, vector: $0.value) }
        for (holder, vector) in [(holder: replica, vector: delivered)] + others {
            for entry in vector {
                rows.append(
                    CausalKnownPayload(
                        holderHi: holder.hi, holderLo: holder.lo, hi: entry.hi, lo: entry.lo, seq: entry.seq))
            }
        }
        let members = UInt32(clamping: group.view.count)
        let cut = CausalCut.causalCut(members, rows)
        if cut.count > causalVectorBound {
            throw CausalRejected.rule(.capacityExceeded)
        }
        return cut
    }

    /// The tag a message of this copy carries: the epoch, and `own` first, then every other entry of
    /// `vector`.
    private func tagOf(_ own: CausalEntryPayload, _ vector: [CausalEntryPayload]) -> CausalTag {
        let me = replica
        return CausalTag(epoch: group.epoch, vector: [own] + vector.filter { Replica(of: $0) != me })
    }

    /// The sender entry of a tag of this view, or why it is not one.
    private func senderOf(_ tag: CausalTag) throws(CausalRejected) -> CausalEntryPayload {
        if tag.epoch != group.epoch {
            throw CausalRejected.otherEpoch(epoch: tag.epoch)
        }
        guard tag.vector.count <= causalVectorBound, let first = tag.vector.first else {
            throw CausalRejected.malformed
        }
        return first
    }

    /// Hold `author` to the view: a member already heard from, or one more while the view has a member
    /// no replica holds yet. Until view changes land the view is epoch 0's, and each of its members is
    /// held by the first replica that presents itself for it; a replica past the view's size is a
    /// second incarnation of a member, which is not delivered.
    private func admitMember(_ author: Replica) throws(CausalRejected) {
        if known[author] != nil || 1 + known.count < group.view.count {
            return
        }
        throw CausalRejected.notInView(replica: author)
    }

    /// Raise the latest vector known of `author` by `vector`.
    private func noteKnown(_ author: Replica, _ vector: [CausalEntryPayload]) throws(CausalRejected) {
        guard let raised = raise(known[author] ?? [], by: vector) else {
            throw CausalRejected.rule(.capacityExceeded)
        }
        known[author] = raised
    }

    /// What `CausalAdmit` decides for the change `dot` with context `context` against this copy's
    /// delivered vector.
    private func admission(
        _ dot: CausalEntryPayload, _ context: [CausalEntryPayload]
    ) throws(CausalRejected) -> CausalAdmission {
        do {
            return try CausalAdmit.causalAdmit(delivered, dot, context)
        } catch {
            throw CausalRejected.rule(ruleFailure(error))
        }
    }

    /// Deliver the change `dot`: raise the delivered vector by it, log it and hand its message on.
    private func deliver(
        _ dot: CausalEntryPayload, _ message: Message, into out: inout [Message]
    ) throws(CausalRejected) {
        guard let raised = raise(delivered, by: [dot]) else {
            throw CausalRejected.rule(.capacityExceeded)
        }
        delivered = raised
        log.append(Logged(dot: dot, message: message, released: true))
        out.append(message)
    }

    /// Deliver every held change that is now deliverable, each after the ones its context names, and
    /// let go of every held change now delivered.
    private func drainHeld(into out: inout [Message]) throws(CausalRejected) {
        while true {
            var next: Int?
            var index = 0
            while index < held.count {
                switch try admission(held[index].dot, held[index].context) {
                case .drop:
                    held.remove(at: index)
                case .deliver:
                    next = index
                case .hold:
                    index += 1
                }
                if next != nil {
                    break
                }
            }
            guard let ready = next else {
                return
            }
            let change = held.remove(at: ready)
            try deliver(change.dot, change.message, into: &out)
        }
    }

    /// Drop every logged change the stable cut covers: it is delivered at every member of the view, so
    /// no repair can ask for it.
    private func prune() throws(CausalRejected) {
        let cut = try stableCut()
        log.removeAll { !CausalLacks.causalLacks(cut, $0.dot) }
    }
}

/// `vector` raised by `other` (`CausalRaise`), or nil when the result holds more entries than a vector
/// may.
private func raise(_ vector: [CausalEntryPayload], by other: [CausalEntryPayload]) -> [CausalEntryPayload]? {
    let raised = CausalRaise.causalRaise(vector, other)
    return raised.count > causalVectorBound ? nil : raised
}

/// The number `vector` gives `replica`, 0 when it names none.
private func seen(_ vector: [CausalEntryPayload], _ replica: Replica) -> UInt64 {
    vector.first { Replica(of: $0) == replica }?.seq ?? 0
}

/// The tag `envelope` carries in keys 19 (`causal_epoch`) and 20 (`causal_vector`), nil when it carries
/// neither, and `CausalRejected.malformed` when it carries one without the other.
public func causalTag(of envelope: Envelope) throws(CausalRejected) -> CausalTag? {
    switch (envelope.causal_epoch, envelope.causal_vector) {
    case (nil, nil):
        return nil
    case let (epoch?, vector?):
        return CausalTag(epoch: epoch, vector: vector)
    default:
        throw CausalRejected.malformed
    }
}

/// `envelope` with `tag` in keys 19 and 20; a vector longer than an envelope carries is
/// `CausalRejected.malformed`.
public func stamped(_ envelope: Envelope, with tag: CausalTag) throws(CausalRejected) -> Envelope {
    guard tag.vector.count <= causalVectorBound else {
        throw CausalRejected.malformed
    }
    var tagged = envelope
    tagged.causal_epoch = tag.epoch
    tagged.causal_vector = tag.vector
    return tagged
}
