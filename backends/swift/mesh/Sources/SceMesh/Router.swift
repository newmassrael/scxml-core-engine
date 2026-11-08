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

    public init(
        transport: String, buffer: OutboundBuffer?, retry: RetryPolicy?, stampSequence: Bool,
        delivery: Delivery, responders: [String], deadlineMs: Int64?, replyEvents: [String]
    ) {
        self.transport = transport
        self.buffer = buffer
        self.retry = retry
        self.stampSequence = stampSequence
        self.delivery = delivery
        self.responders = responders
        self.deadlineMs = deadlineMs
        self.replyEvents = replyEvents
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

        init(_ config: PeerConfig) {
            self.config = config
            self.outbound = Outbound(buffer: config.buffer)
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
        switch peer.outbound.admit(bytes, nowMs: nowMs) {
        case let .send(bytes): return [.transmit(peer: peerName, bytes: bytes)]
        case .queued: return []
        case let .dropped(signal): return [.raise(peer: peerName, signal: signal)]
        }
    }

    /// `peer`'s transport became ready: release what waited for it.
    public func peerReady(_ peer: String, nowMs: Int64) -> Routed {
        guard let bound = peers[peer] else {
            return .refused(.unknownPeer(peer))
        }
        do {
            let drained = try bound.outbound.markReady(nowMs: nowMs)
            return .done(
                drained.send.map { .transmit(peer: peer, bytes: $0) }
                    + drained.signals.map { .raise(peer: peer, signal: $0) })
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
        guard let delivery = peers[peer]?.config.delivery else {
            return .refused(.unknownPeer(peer))
        }
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
        return .done(gaps + expired)
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
            // 3.2). This core carries no causal route yet (the build refuses `delivery: causal` for
            // Swift), so one that arrives is dropped.
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
