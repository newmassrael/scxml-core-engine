// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The router core: one machine's Mesh endpoint, joining the send half (`Outbound`) and the receive
// half (`Inbound`) to the engine's host-processor surface. The Swift twin of backends/kotlin/mesh
// Router.kt and backends/rust/mesh/src/router.rs.
//
// Sans-IO: every call answers with `Effect`s and the host performs them. It owns the transport, the
// clock and the randomness an envelope id needs, the shape SCE_MESH.md §mesh-6.4 gives a transport:
// the transport moves bytes, the core decides.

import SceForgeRuntime
import SceRuntime

/// What deployment says about one peer this machine talks to.
public struct PeerConfig: Equatable, Sendable {
    /// The binding's transport kind (`"wss"`, `"custom_tcp"`, ...): the `transport` column of its
    /// §mesh-16.7 rows.
    public var transport: String

    /// deploy.yaml's `outbound_buffer:` section, or nil when the machine declares none and every send
    /// is dispatched directly (§mesh-10.10).
    public var buffer: OutboundBuffer?

    /// deploy.yaml's `retry` block, if the binding has one (§mesh-10.10).
    public var retry: RetryPolicy?

    /// Whether envelopes TO this peer carry a `sequence_no`: `ordering: required` on a transport that
    /// does not order (§mesh-10.6.3).
    public var stampSequence: Bool

    /// How envelopes FROM this peer are delivered (§mesh-10.5, §mesh-10.6).
    public var delivery: Delivery

    /// The machines whose reply may answer a request sent to this peer: the binding's `reply_from:`,
    /// or the peer alone (§mesh-14.6). Never empty.
    public var responders: [String]

    /// deploy.yaml's binding-level request deadline, which a request to this peer takes when it
    /// carries no `_mesh_deadline_ms`; nil lets it wait for its reply indefinitely (§mesh-9.5).
    public var deadlineMs: Int64?

    /// The events this machine sends to the peer as replies (`service.response.*`, §mesh-8.1). One
    /// goes out as `rpcReply` with the invokeid of the request being handled, which is how the
    /// requester matches it (§mesh-10.7); every other send is `fireForget`.
    public var replyEvents: [String]

    /// The causal group this binding is the route of (docs/adr/0085, section 3.2), or nil for the
    /// per-source rules. A causal peer's changes go through its causal layer (`Router.startCausal`),
    /// which then replaces the duplicate window and the sequence buffer for it.
    public var causal: CausalGroup?

    public init(
        transport: String, buffer: OutboundBuffer?, retry: RetryPolicy?, stampSequence: Bool,
        delivery: Delivery, responders: [String], deadlineMs: Int64?, replyEvents: [String],
        causal: CausalGroup? = nil
    ) {
        self.transport = transport
        self.buffer = buffer
        self.retry = retry
        self.stampSequence = stampSequence
        self.delivery = delivery
        self.responders = responders
        self.deadlineMs = deadlineMs
        self.replyEvents = replyEvents
        self.causal = causal
    }
}

/// One thing the host must do.
public enum Effect: Equatable, Sendable {
    /// Hand `bytes` to the transport bound to `peer`.
    case transmit(peer: String, bytes: [UInt8])

    /// Raise `event` on the engine's external queue with `data` as `_event.data`; `source` is the
    /// machine that sent it, `sendID` the id its `<send>` carried (§mesh-10.7: the envelope's
    /// `subject`), and `invokeID` the request's wire invokeid as RFC 4122 text when the envelope is an
    /// `rpcRequest`: the one field that comes back on the reply (§mesh-10.7).
    case deliver(event: String, data: String, source: String, sendID: String?, invokeID: String?)

    /// Raise `error.communication` with this §mesh-16.7 row; `peer` names the binding it is about,
    /// when there is one.
    case raise(peer: String?, signal: Signal)

    /// End the start `token` of the SCXML invocation `invokeID` with `done.invoke.<invokeID>`: `data`
    /// is the reply's payload text and `source` the machine that answered, whose `mesh://<source>` is
    /// the event's `_event.origin` (§mesh-9.5).
    case complete(invokeID: String, token: UInt64, data: String, source: String)

    /// End it with `error.invoke.<invokeID>` instead: `data` is the §mesh-10.7.1 `errorName: "invoke"`
    /// object, and `source` the machine that answered, nil when the requester's own deadline ended it.
    case fail(invokeID: String, token: UInt64, data: String, source: String?)

    /// A change the document sent to the causal peer `peer` was accepted and tagged with `dot`
    /// (docs/adr/0085, section 3, "offer and verdict"). The host records it with its dot in the copy's
    /// journal, then calls `Router.committed`; nothing of it leaves before.
    case accepted(peer: String, dot: Dot)

    /// A change the document sent to the causal peer `peer` was refused: it took no number and no copy
    /// takes it in. The host tells the copy, which tells `noRoomForHistory`.
    case refused(peer: String, refusal: CausalRefusal)

    /// The causal peer `peer` is owed this copy's `causalVector`, its link having come back or its
    /// period having passed. The host draws an envelope id and calls `Router.announce`.
    case announceDue(peer: String)
}

/// What became of a request `Router.invoke` was handed.
public enum Invoked: Equatable, Sendable {
    /// It is on its way (or queued for its peer); its answer arrives later as `Effect.complete` or
    /// `Effect.fail`.
    case started([Effect])

    /// It could not reach the wire, so the invocation never starts: the host refuses it with this
    /// `_event.data`, and the engine raises `error.execution` (§mesh-9.5's pre-envelope tier).
    case refused(String)

    /// Only the host can act on it (see `RouterError`).
    case failed(RouterError)
}

/// Why the router could not do what the host asked. None of these is a §mesh-16.7 row: each is the
/// host's to act on.
public enum RouterError: Error, Equatable, Sendable {
    /// The host handed `Router.send` a target that is not a Mesh peer reference (see `meshPeer`).
    case notMeshTarget(String)

    /// The host named a peer it never bound with `Router.addPeer`.
    case unknownPeer(String)

    /// A standard rule refused its input: a sequence at the `UInt64` maximum, or a clock that cannot
    /// be subtracted.
    case rule(AlgorithmError)

    /// The envelope could not be written within the bounds its document declares.
    case encode

    /// A Mesh request reached `Router.invoke` without a reserved param the build always gives it.
    case missingParam(String)

    /// A reserved param's value is not what the build writes there.
    case badParam(name: String, value: String)

    /// An event arrived on a link that may not deliver it, and was dropped before the machine was
    /// told (`Endpoint.forbidEvents`).
    case eventForbidden(peer: String, event: String)

    /// An event arrived that the host's `EventAdmission` did not admit, and was dropped before the
    /// machine was told; `reason` is its word for it.
    case eventNotAdmitted(peer: String, event: String, reason: String)

    /// An event the machine sent was not sent: the host's `Sealer` said it must not leave; `reason`
    /// is its word for it.
    case eventNotSealed(target: String, event: String, reason: String)

    /// The host asked for the causal layer of a peer whose binding is not causal.
    case notCausal(String)

    /// A causal peer was used before the host started its causal layer (`Router.startCausal`).
    case causalNotStarted(String)
}

/// A router call's answer: the effects to perform, or why there are none.
public enum Routed: Equatable, Sendable {
    case done([Effect])
    case refused(RouterError)
}

/// One machine's Mesh endpoint, with the receive side's dedup window and gap timeout (deploy.yaml
/// `window_size`, `gap_timeout_ms`).
public final class Router {
    private final class BoundPeer {
        let config: PeerConfig
        let outbound: Outbound
        var nextSequence: UInt64 = 1

        /// The causal layer of a causal peer, once the host started it.
        var causal: Causal?

        init(_ config: PeerConfig) {
            self.config = config
            self.outbound = Outbound(buffer: config.buffer)
        }
    }

    /// A causal peer's layer and when it last announced its vector.
    private final class Causal {
        let layer: CausalLayer<[UInt8]>
        let announceEveryMs: Int64
        var announcedMs: Int64?

        init(layer: CausalLayer<[UInt8]>, announceEveryMs: Int64) {
            self.layer = layer
            self.announceEveryMs = announceEveryMs
        }
    }

    public let machine: String
    private let inbound: Inbound
    private var peers: [String: BoundPeer] = [:]
    private let requests = Correlation()

    public init(machine: String, dedupWindow: UInt32, gapTimeoutMs: Int64) {
        self.machine = machine
        self.inbound = Inbound(dedupWindow: dedupWindow, gapTimeoutMs: gapTimeoutMs)
    }

    /// Bind `peer`: the name a document writes as `#peer`.
    public func addPeer(_ peer: String, config: PeerConfig) {
        peers[peer] = BoundPeer(config)
    }

    /// Start the causal layer of the causal peer `peer` (docs/adr/0085, sections 3.2 and 3.3): `start`
    /// is the incarnation the host read from the copy's journal, or a new one; `room` the most changes
    /// the layer may hold not yet stable; `announceEveryMs` the period of its `causalVector`. A send to a
    /// causal peer before this is refused.
    public func startCausal(
        _ peer: String, start: CausalStart, room: Int, announceEveryMs: Int64
    ) throws(RouterError) {
        guard let bound = peers[peer] else {
            throw .unknownPeer(peer)
        }
        guard let group = bound.config.causal else {
            throw .notCausal(peer)
        }
        // §mesh-10.12, what a host does: the peer's layer begins from the incarnation the host read
        // from the copy's journal, or a new one.
        bound.causal = Causal(
            layer: CausalLayer(group: group, start: start, room: room), announceEveryMs: announceEveryMs)
    }

    /// The causal layer of `peer`, for the host to read (its replica, its delivered vector, what it
    /// holds).
    public func causalLayer(_ peer: String) -> CausalLayer<[UInt8]>? {
        peers[peer]?.causal?.layer
    }

    /// The host committed the journal record of `dot`, a change of the causal peer `peer`: what it
    /// releases goes to that peer's send half.
    public func committed(_ peer: String, dot: Dot, nowMs: Int64) -> Routed {
        do throws(RouterError) {
            let (bound, causal) = try causalPeer(peer)
            return .done(causal.layer.committed(dot).flatMap { dispatch(peer, bound, $0, nowMs: nowMs) })
        } catch {
            return .refused(error)
        }
    }

    /// Send this copy's `causalVector` to the causal peer `peer` (pattern 23: the epoch and its
    /// delivered vector, keys 19 and 20). `id` is the envelope id the host drew.
    public func announce(_ peer: String, id: [UInt8], nowMs: Int64) -> Routed {
        do throws(RouterError) {
            let (bound, causal) = try causalPeer(peer)
            let tag = causal.layer.summary()
            causal.announcedMs = nowMs
            var envelope = Envelope()
            envelope.id = id
            envelope.source = machine
            envelope.event_type = ""
            envelope.pattern = .causalVector
            envelope.datacontenttype = PayloadCodec.none
            guard let tagged = try? stamped(envelope, with: tag),
                let bytes = encodeToBytes({ sink in try tagged.encode(into: &sink) })
            else {
                throw .encode
            }
            return .done(dispatch(peer, bound, bytes, nowMs: nowMs))
        } catch {
            return .refused(error)
        }
    }

    /// The bound causal peer `name` and its started layer, or why there is none.
    private func causalPeer(_ name: String) throws(RouterError) -> (BoundPeer, Causal) {
        guard let bound = peers[name] else {
            throw .unknownPeer(name)
        }
        guard bound.config.causal != nil else {
            throw .notCausal(name)
        }
        guard let causal = bound.causal else {
            throw .causalNotStarted(name)
        }
        return (bound, causal)
    }

    /// The retry policy the host applies when a transmission to `peer` fails.
    public func retryPolicy(_ peer: String) -> RetryPolicy? {
        peers[peer]?.config.retry
    }

    /// The binding a raised row names: the peer an `Effect.raise` carries, with the transport the
    /// deployment bound it to. A peer the deployment never bound is named with no transport.
    public func binding(_ peer: String?) -> Binding? {
        peer.map { Binding(peer: $0, transport: peers[$0]?.config.transport) }
    }

    /// Perform a `<send target="#peer">` the engine handed the host. `id` is the envelope id the host
    /// drew (a UUID v7), `nowMs` its monotonic clock. A Mesh target this router has no binding for is
    /// unreachable, and says so as §mesh-16.7 row 1.
    public func send(_ request: HostSendRequest, id: [UInt8], nowMs: Int64) -> Routed {
        guard let peerName = meshPeer(request.target) else {
            return .refused(.notMeshTarget(request.target))
        }
        guard let peer = peers[peerName] else {
            return .done([.raise(peer: peerName, signal: .transportUnavailable)])
        }
        // §mesh-8.1, §mesh-10.7: a reply the deployment names for this peer goes out as `rpcReply`,
        // carrying the invokeid of the request the document was handling when it sent it: the one
        // field that makes the round trip. A reply sent with no request current carries none, and the
        // requester delivers it as the event it names.
        let reply = peer.config.replyEvents.contains(request.eventName)
        var envelope = Envelope()
        envelope.id = id
        envelope.source = machine
        envelope.event_type = request.eventName
        envelope.pattern = reply ? .rpcReply : .fireForget
        envelope.datacontenttype = payloadCodec(request.eventData)
        envelope.data = Array(request.eventData.utf8)
        // §mesh-10.7: the receiver's `_event.sendid` is the envelope's `subject`.
        envelope.subject = request.sendID.isEmpty ? nil : request.sendID
        envelope.invoke_id = reply ? parseUUIDText(request.invokeID) : nil
        envelope.rpc_status = reply ? RpcStatus.ok : nil
        if peer.config.causal != nil {
            return offer(peerName, peer, envelope)
        }
        guard let effects = transmit(peerName, peer, envelope, nowMs: nowMs) else {
            return .refused(.encode)
        }
        return .done(effects)
    }

    /// Start an `<invoke type="sce:mesh-rpc">` the engine handed the host (§mesh-9.5): write its
    /// `RpcRequest` and keep it until its reply, its deadline or its cancellation ends it.
    ///
    /// `wireID` is the `invoke_id` the host minted for this invocation and `id` the envelope's own id:
    /// two UUID v7s, so correlation and dedup never share a key. `nowMs` is the host's monotonic
    /// clock, which the deadline is kept against, and `nowUnixMs` its wall clock, which the wire's
    /// `deadline_unix_ms` is written in.
    ///
    /// A target that names no Mesh peer, or one this router has no binding for, cannot reach the
    /// wire: the invocation is `Invoked.refused`.
    public func invoke(
        _ request: HostInvokeRequest, wireID: [UInt8], id: [UInt8], nowMs: Int64, nowUnixMs: Int64
    ) -> Invoked {
        guard let peerName = meshPeer(request.src) else {
            return .refused(srcNotFoundData("'\(request.src)' names no Mesh peer"))
        }
        guard let peer = peers[peerName] else {
            return .refused(srcNotFoundData("no binding for '#\(peerName)'"))
        }
        guard let event = request.params[meshEventParam]?.first else {
            return .failed(.missingParam(meshEventParam))
        }
        // §mesh-9.5 deadline precedence: the invoke's own param, else the binding's, else none.
        let deadlineMs: Int64?
        if let text = request.params[meshDeadlineParam]?.first {
            // Read as the Rust core reads it (any u64), and held as an Int64, beyond which no
            // deadline is distinguishable from none.
            guard let parsed = UInt64(text) else {
                return .failed(.badParam(name: meshDeadlineParam, value: text))
            }
            deadlineMs = Int64(clamping: parsed)
        } else {
            deadlineMs = peer.config.deadlineMs
        }
        var envelope = Envelope()
        envelope.id = id
        envelope.source = machine
        envelope.event_type = event
        envelope.pattern = .rpcRequest
        envelope.datacontenttype = payloadCodec(request.eventData)
        envelope.data = Array(request.eventData.utf8)
        envelope.invoke_id = wireID
        envelope.deadline_unix_ms = deadlineMs.map { UInt64(saturatingAdd(nowUnixMs, $0)) }
        guard let effects = transmit(peerName, peer, envelope, nowMs: nowMs) else {
            return .failed(.encode)
        }
        requests.register(
            wireID: wireID,
            Pending(
                invokeID: request.invokeID,
                token: request.token,
                target: peerName,
                responders: peer.config.responders,
                expiresMs: deadlineMs.map { saturatingAdd(nowMs, $0) }
            ))
        return .started(effects)
    }

    /// The state that started the start `token` of `invokeID` exited: stop waiting for its answer.
    /// Nothing goes on the wire, and an answer that arrives later is dropped (§mesh-9.5, `<cancel>`).
    public func cancelInvoke(invokeID: String, token: UInt64) {
        requests.cancel(invokeID: invokeID, token: token)
    }

    /// Write `envelope` for `peerName` and hand it to that peer's send half: what `send` and `invoke`
    /// share once they know what to say. Nil when it could not be written.
    private func transmit(_ peerName: String, _ peer: BoundPeer, _ envelope: Envelope, nowMs: Int64) -> [Effect]? {
        var envelope = envelope
        envelope.sequence_no = peer.config.stampSequence ? peer.nextSequence : nil
        guard let bytes = encodeToBytes({ sink in try envelope.encode(into: &sink) }) else {
            return nil
        }
        // §mesh-10.6.3: the counter advances once per envelope written, and has no wrap guard.
        if peer.config.stampSequence {
            peer.nextSequence &+= 1
        }
        return dispatch(peerName, peer, bytes, nowMs: nowMs)
    }

    /// Hand written envelope `bytes` to `peer`'s send half.
    private func dispatch(_ peerName: String, _ peer: BoundPeer, _ bytes: [UInt8], nowMs: Int64) -> [Effect] {
        switch peer.outbound.admit(bytes, nowMs: nowMs) {
        case let .send(bytes): return [.transmit(peer: peerName, bytes: bytes)]
        case .queued: return []
        case let .dropped(signal): return [.raise(peer: peerName, signal: signal)]
        }
    }

    /// Offer `envelope` to the causal layer of `peer` (docs/adr/0085, section 3, "offer and verdict"):
    /// the verdict is the host's to act on, and nothing leaves before the host commits an accepted
    /// change's journal record.
    private func offer(_ peerName: String, _ peer: BoundPeer, _ envelope: Envelope) -> Routed {
        guard let causal = peer.causal else {
            return .refused(.causalNotStarted(peerName))
        }
        let verdict: CausalVerdict?
        do {
            verdict = try causal.layer.offer { tag in
                guard let tagged = try? stamped(envelope, with: tag) else {
                    return nil
                }
                return encodeToBytes({ sink in try tagged.encode(into: &sink) })
            }
        } catch {
            return .refused(.rule(ruleFailure(error)))
        }
        switch verdict {
        case nil:
            return .refused(.encode)
        case let .accepted(dot)?:
            return .done([.accepted(peer: peerName, dot: dot)])
        case let .refused(refusal)?:
            return .done([.refused(peer: peerName, refusal: refusal)])
        }
    }

    /// `peer`'s transport became ready: release what waited for it.
    public func peerReady(_ peer: String, nowMs: Int64) -> Routed {
        guard let bound = peers[peer] else {
            return .refused(.unknownPeer(peer))
        }
        do {
            let drained = try bound.outbound.markReady(nowMs: nowMs)
            // docs/adr/0085, section 3.2: a link that comes back carries this copy's vector at once, so
            // what was lost while it was down is repaired without waiting for the period.
            let announce: [Effect] = bound.causal != nil ? [.announceDue(peer: peer)] : []
            return .done(
                drained.send.map { .transmit(peer: peer, bytes: $0) }
                    + drained.signals.map { .raise(peer: peer, signal: $0) } + announce)
        } catch {
            return .refused(.rule(ruleFailure(error)))
        }
    }

    /// `peer`'s transport stopped being ready.
    public func peerNotReady(_ peer: String) -> Routed {
        guard let bound = peers[peer] else {
            return .refused(.unknownPeer(peer))
        }
        // A request waiting on the link can no longer be answered over it: it is forgotten, as a
        // cancel forgets it, and each is §mesh-16.7 row 5, the peer-down path the C++ core takes for
        // the same requests.
        let lost = requests.forgetSentTo(peer).map {
            Effect.raise(peer: peer, signal: .invokeChildLost(invokeID: $0, target: peer))
        }
        let edge = bound.outbound.markNotReady().map { [Effect.raise(peer: peer, signal: $0)] } ?? []
        return .done(edge + lost)
    }

    /// Envelope `bytes` a transport received from `peer`, at `nowMs`. An envelope that cannot be read
    /// is §mesh-16.7 row 4 and one an ordered binding cannot place is row 11, so both are effects; a
    /// rule's refusal is a limit of this receiver, which no row names, so it is returned.
    public func receive(peer: String, bytes: [UInt8], nowMs: Int64) -> Routed {
        guard let bound = peers[peer] else {
            return .refused(.unknownPeer(peer))
        }
        if bound.config.causal != nil {
            return receiveCausal(peer, bound, bytes, nowMs: nowMs)
        }
        let delivery = bound.config.delivery
        // §mesh-14.6: a reply is checked against its request's responder set on arrival, while the
        // binding it came in on is still known: an ordered binding may hold it and release it on a
        // later tick. One from outside the set is refused before the dedup window or the ordering hold
        // sees it, so it takes no slot in either.
        if let signal = undeclaredReply(bytes, peer: peer) {
            return .done([.raise(peer: peer, signal: signal)])
        }
        switch inbound.admit(bytes, delivery: delivery, nowMs: nowMs) {
        case let .admitted(outcome):
            return .done(effectsOf(outcome, peer: peer))
        case let .refused(error):
            switch error {
            // The bytes are the envelope's CBOR (§mesh-7.5); nothing in them can be trusted to name a
            // source.
            case .malformed: return .done([.raise(peer: peer, signal: .envelopeCorrupt(source: nil, codec: "cbor"))])
            case .unstamped: return .done([.raise(peer: peer, signal: .missingSequence(source: peer))])
            case let .rule(failure): return .refused(.rule(failure))
            }
        }
    }

    /// End every ordering gap that has waited out its timeout by `nowMs`, then every request whose
    /// deadline has passed: each ends in `error.invoke` with `deadlineExceeded`, the same shape a
    /// peer's `DeadlineExceeded` reply takes (§mesh-10.7.1).
    public func tick(nowMs: Int64) -> Routed {
        let gaps: [Effect]
        switch inbound.tick(nowMs: nowMs) {
        case let .admitted(outcome):
            gaps = effectsOf(outcome, peer: nil)
        case let .refused(error):
            switch error {
            case let .rule(failure): return .refused(.rule(failure))
            // A tick reads no bytes and stamps nothing.
            case .malformed, .unstamped: preconditionFailure("Inbound.tick decodes no envelope")
            }
        }
        let expired = requests.expire(nowMs: nowMs).map { entry in
            Effect.fail(
                invokeID: entry.pending.invokeID,
                token: entry.pending.token,
                data: invokeErrorData(status: .deadlineExceeded, detail: nil, source: nil, wireIDText: entry.wireID),
                source: nil
            )
        }
        // docs/adr/0085, section 3.2: each causal peer is owed this copy's vector on its period, which
        // repairs a loss no link edge reports. Peers are visited in name order, as the Rust core visits
        // them.
        let announce = peers.keys.sorted().compactMap { name -> Effect? in
            guard let causal = peers[name]?.causal else {
                return nil
            }
            if let at = causal.announcedMs, saturatingSubtract(nowMs, at) < causal.announceEveryMs {
                return nil
            }
            return .announceDue(peer: name)
        }
        return .done(gaps + expired + announce)
    }

    /// Envelope `bytes` the causal peer `peer` delivered: a change goes through the causal layer, which
    /// delivers it with whatever it releases, holds it or drops it; a `causalVector` is recorded as its
    /// sender's mark and answered with every change of the log it lacks. An envelope of another epoch,
    /// or by a replica the view does not hold, is raised as §mesh-16.7 row 18, `CAUSAL_VIEW_MISMATCH`,
    /// and is not delivered (docs/adr/0085, section 3.2).
    private func receiveCausal(_ peer: String, _ bound: BoundPeer, _ bytes: [UInt8], nowMs: Int64) -> Routed {
        // §mesh-10.12: every envelope of a causal peer reaches its layer before the engine; a
        // `causalVector` is a repair and a mark, and never reaches an engine.
        var cursor = SceCursor(bytes)
        guard let envelope = try? Envelope.decode(&cursor) else {
            return .done([.raise(peer: peer, signal: .envelopeCorrupt(source: nil, codec: "cbor"))])
        }
        let corrupt = Effect.raise(peer: peer, signal: .envelopeCorrupt(source: envelope.source, codec: "cbor"))
        // An envelope on a causal route carries its tag; one without it, or with half of it, cannot be
        // placed, and is not delivered.
        guard let tag = try? causalTag(of: envelope) else {
            return .done([corrupt])
        }
        guard let causal = bound.causal else {
            return .refused(.causalNotStarted(peer))
        }
        var delivered: [[UInt8]] = []
        var answer: [[UInt8]] = []
        do throws(CausalRejected) {
            if envelope.pattern == .causalVector {
                answer = try causal.layer.receiveSummary(tag)
            } else {
                delivered = try causal.layer.receive(tag, bytes)
            }
        } catch {
            switch error {
            case let .otherEpoch(epoch):
                let replica = tag.vector.first.map { Replica(of: $0) }
                return .done([.raise(peer: peer, signal: viewMismatch(envelope.source, epoch, replica))])
            case let .notInView(replica):
                return .done([.raise(peer: peer, signal: viewMismatch(envelope.source, tag.epoch, replica))])
            case .malformed:
                return .done([corrupt])
            case let .rule(failure):
                return .refused(.rule(failure))
            }
        }
        var effects = answer.flatMap { dispatch(peer, bound, $0, nowMs: nowMs) }
        for change in delivered {
            // These bytes decoded on arrival, and decoding is a function of the bytes alone.
            var reader = SceCursor(change)
            if let decoded = try? Envelope.decode(&reader) {
                effects.append(deliver(decoded, peer: peer))
            }
        }
        return .done(effects)
    }

    /// Row 14 for `bytes`, if they are a reply to a live request that arrived on a binding outside its
    /// responder set. Bytes that do not decode are left for the receive half to report.
    private func undeclaredReply(_ bytes: [UInt8], peer: String) -> Signal? {
        var cursor = SceCursor(bytes)
        guard let envelope = try? Envelope.decode(&cursor), let wireID = replyTo(envelope) else {
            return nil
        }
        switch requests.check(wireID: wireID, peer: peer) {
        case .undeclared: return .rpcReplyFromUndeclaredPeer(source: envelope.source, invokeID: uuidText(wireID))
        case .admitted, .unknown: return nil
        }
    }

    /// What the receive half released, as effects, in release order, followed by the rows it observed.
    private func effectsOf(_ outcome: Outcome, peer: String?) -> [Effect] {
        let released = outcome.released.compactMap { envelope -> Effect? in
            // A causal route's control message never reaches an engine (docs/adr/0085, section
            // 3.2); one on a binding that is not causal is no message of its route, and is dropped.
            if envelope.pattern == .causalVector {
                return nil
            }
            // A reply's responder was checked on arrival.
            if let wireID = replyTo(envelope) {
                return answer(envelope, wireID: wireID, peer: peer)
            }
            return deliver(envelope, peer: peer)
        }
        return released + outcome.signals.map { .raise(peer: peer, signal: $0) }
    }

    /// How a released reply ends the request `wireID` (§mesh-9.5), or nil when no request is waiting
    /// on it any more: it was answered, cancelled or expired, and a late answer is dropped.
    ///
    /// `ok`, or no status at all, which the requester reads as `ok` as the C++ core does, completes it
    /// with the reply's payload; any other status fails it. An `ok` whose payload the engine cannot be
    /// handed is §mesh-16.7 row 4 and ends nothing: the request stays waiting for an answer it can
    /// use, or for its deadline.
    private func answer(_ envelope: Envelope, wireID: [UInt8], peer: String?) -> Effect? {
        let status = envelope.rpc_status ?? RpcStatus.ok
        if status == .ok {
            switch payloadText(envelope) {
            case let .unreadable(codec):
                return requests.isWaiting(wireID: wireID)
                    ? .raise(peer: peer, signal: .envelopeCorrupt(source: envelope.source, codec: codec)) : nil
            case let .text(text):
                guard let pending = requests.retire(wireID: wireID) else { return nil }
                return .complete(invokeID: pending.invokeID, token: pending.token, data: text, source: envelope.source)
            }
        }
        guard let pending = requests.retire(wireID: wireID) else { return nil }
        return .fail(
            invokeID: pending.invokeID,
            token: pending.token,
            data: invokeErrorData(
                status: status, detail: envelope.rpc_error_message, source: envelope.source,
                wireIDText: uuidText(wireID)),
            source: envelope.source
        )
    }
}

/// The wire id of the request `envelope` answers, when it is a reply that carries one. A reply to a
/// request this router never sent, or one already retired, is then dropped rather than delivered:
/// this core sends every request with an `invoke_id`, so nothing else can be waiting for it.
private func replyTo(_ envelope: Envelope) -> [UInt8]? {
    guard envelope.pattern == .rpcReply, let id = envelope.invoke_id, id.count == 16 else {
        return nil
    }
    return id
}

/// The codec a payload of the sending engine's own `_event.data` text travels in: JSON, or nothing
/// when there is none: the rule the C++ core applies to a child's donedata (ChildSessionAdapter.h).
private func payloadCodec(_ eventData: String) -> PayloadCodec {
    eventData.isEmpty ? PayloadCodec.none : .json
}

/// An envelope's payload as the text the engine is handed, or the codec that makes it unreadable.
private enum PayloadText {
    case text(String)
    case unreadable(codec: String)
}

/// A JSON or empty payload is the `_event.data` text the sender's engine wrote; a payload in another
/// codec is bytes the engine's text surface cannot be handed, named by that codec so §mesh-16.7 row 4
/// can say which.
private func payloadText(_ envelope: Envelope) -> PayloadText {
    switch envelope.datacontenttype {
    case .none:
        return .text("")
    case .json:
        if let text = String(validating: envelope.data, as: UTF8.self) {
            return .text(text)
        }
        return .unreadable(codec: "json")
    case .cbor: return .unreadable(codec: "cbor")
    case .typed: return .unreadable(codec: "typed")
    case .raw: return .unreadable(codec: "raw")
    }
}

/// `a` + `b` for non-negative `b`, held at `Int64.max` rather than wrapping.
private func saturatingAdd(_ a: Int64, _ b: Int64) -> Int64 {
    a > Int64.max - b ? Int64.max : a + b
}

/// `a` - `b`, held at the nearer end of `Int64` rather than wrapping.
private func saturatingSubtract(_ a: Int64, _ b: Int64) -> Int64 {
    let (difference, overflow) = a.subtractingReportingOverflow(b)
    if !overflow {
        return difference
    }
    return b < 0 ? Int64.max : Int64.min
}

/// The row an envelope of another epoch, or of a replica the view does not hold, raises.
private func viewMismatch(_ source: String, _ epoch: UInt64, _ replica: Replica?) -> Signal {
    .causalViewMismatch(source: source, epoch: epoch, replica: replica?.description ?? "")
}

/// The event an admitted envelope raises. A JSON or empty payload is the `_event.data` text the
/// sender's engine wrote; a payload in another codec is bytes the engine's text surface cannot be
/// handed, so it is §mesh-16.7 row 4 naming that codec. `peer` is the binding it arrived on, when a
/// receipt rather than a tick released it.
private func deliver(_ envelope: Envelope, peer: String?) -> Effect {
    switch payloadText(envelope) {
    case let .text(text):
        // §mesh-10.7: `_event.invokeid` of an inbound request is its wire invokeid, as RFC 4122 text.
        var invokeID: String?
        if envelope.pattern == .rpcRequest, let id = envelope.invoke_id, id.count == 16 {
            invokeID = uuidText(id)
        }
        return .deliver(
            event: envelope.event_type, data: text, source: envelope.source, sendID: envelope.subject,
            invokeID: invokeID)
    case let .unreadable(codec):
        return .raise(peer: peer, signal: .envelopeCorrupt(source: envelope.source, codec: codec))
    }
}
