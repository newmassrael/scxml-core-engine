// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The host half of a Swift Mesh endpoint: the `Router` core put to work with a transport, a clock and
// a source of randomness, and joined to an engine. The Swift twin of backends/kotlin/mesh Endpoint.kt
// and backends/rust/mesh/src/endpoint.rs.
//
// The router decides and this performs. A transmission that fails is retried on the binding's
// schedule (§mesh-10.10) or given up as its §mesh-16.7 row; an event the router releases, and every
// row it raises, becomes an `EngineEvent` carrying the `_event` fields the C++ core gives the same
// event (sce/include/mesh/MeshDispatch.h), so a document cannot tell which core it runs beside; a Mesh
// request the router ends becomes the call that ends its invocation (§mesh-9.5). What the engine must
// be told is queued rather than handed back from the send handler: the engine raises a handler's
// answer with no origin, where the C++ core raises a row as an envelope from its own machine, and one
// queue keeps an invocation's end in order with the events around it.
//
// Confined to one thread: the one that steps the engine. The send handler already runs there; a
// transport that receives on its own thread hands the bytes to that one (the engine's inbox, a
// dispatch queue) rather than calling `Endpoint.receive` itself. Confinement instead of a lock keeps
// the core in plain code and its order the engine's order.

import SceForgeRuntime
import SceRuntime

/// Why a transport declined a transmission.
public struct TransportFailure: Equatable, Sendable {
    /// Whether sending the same bytes again could succeed; a terminal failure is given up at once.
    public var retryable: Bool

    /// The transport's own words, carried as the row's `transport_error`.
    public var message: String?

    public init(retryable: Bool, message: String?) {
        self.retryable = retryable
        self.message = message
    }
}

/// Moves envelope bytes to a peer. Bytes that arrive go to `Endpoint.receive`.
public protocol Transport: AnyObject {
    /// Nil when the bytes were handed on; why not, otherwise.
    func transmit(peer: String, bytes: [UInt8]) -> TransportFailure?
}

/// What the core leaves to the host: the time, and the randomness an envelope id and a retry's jitter
/// need.
public protocol Environment: AnyObject {
    /// A monotonic clock, in milliseconds.
    func nowMs() -> Int64

    /// The wall clock, in milliseconds since the Unix epoch: what a request's `deadline_unix_ms` is
    /// written in (§mesh-9.5).
    func nowUnixMs() -> Int64

    /// A fresh UUID v7 (§mesh-7.5): an envelope's id, or the wire `invoke_id` a request is correlated
    /// by.
    func envelopeID() -> [UInt8]

    /// A non-negative number drawn uniformly, for a retry's jitter.
    func jitterDraw() -> Int64
}

/// An event for the engine's external queue, by name, with its `_event`.
public struct EngineEvent {
    public var name: String
    public var metadata: EventMetadata
}

/// One thing the host must tell its engine, in the order the endpoint came to know it.
public enum EngineCall {
    /// Raise this event on the external queue.
    case raise(EngineEvent)

    /// End the start `token` of the Mesh request `invokeID`, with `done.invoke`, or with
    /// `error.invoke` when `failed`, carrying `data` as `_event.data`. `source` is the machine that
    /// answered, nil for a deadline the requester reached itself.
    case endInvoke(invokeID: String, token: UInt64, failed: Bool, data: String, source: String?)

    /// What the runtime applies: the call the engine's own `applyMeshCall` takes.
    public var meshCall: MeshCall {
        switch self {
        case let .raise(event):
            .raise(name: event.name, metadata: event.metadata)
        case let .endInvoke(invokeID, token, failed, data, source):
            .endInvoke(invokeID: invokeID, token: token, failed: failed, data: data, source: source)
        }
    }
}

/// What an `EventAdmission` answers for an event that arrived.
public enum Admit: Equatable, Sendable {
    /// The event may reach the machine: as it arrived when `data` is nil, and with `data` in place of
    /// what arrived otherwise.
    case allow(data: String? = nil)

    /// The event may not reach the machine; `reason` is told to the host.
    case refuse(reason: String)
}

/// What the host asks of an event that arrived before the machine is told of it: the link it came on
/// (which the host authenticated), its name, and its data as the sender wrote it. It is where a rule
/// that needs the event's content is kept, which a machine of the statically typed data model cannot
/// keep for itself: it reads no `_event` but the payload, so it cannot tell who sent an event or check
/// a signature the payload carries.
public protocol EventAdmission: AnyObject {
    func admit(peer: String, event: String, data: String) -> Admit
}

/// What a `Sealer` answers for an event the machine sent.
public enum Sealed: Equatable, Sendable {
    /// The event leaves as it is when `data` is nil, and with `data` in place of what the machine
    /// sent otherwise.
    case leave(data: String? = nil)

    /// The event must not leave at all (a write the host cannot sign or seal as the list requires);
    /// `reason` is told to the host.
    case withhold(reason: String)
}

/// What the host writes into an event as it leaves, after the machine sent it and before it is put in
/// an envelope: the sending half of an `EventAdmission`. One it withholds is dropped, told to the host
/// as `RouterError.eventNotSealed`, and nothing else is sent in its place.
public protocol Sealer: AnyObject {
    func seal(target: String, event: String, data: String) -> Sealed
}

/// The verdict of a causal peer's layer on a change the machine sent to it (docs/adr/0085, section 3,
/// "offer and verdict"). An accepted change is recorded with its dot in the copy's journal, and then
/// `Endpoint.committed` releases it; a refused one took no number and no copy takes it in.
public struct CausalPeerVerdict: Equatable, Sendable {
    public var peer: String
    public var verdict: CausalVerdict

    public init(peer: String, verdict: CausalVerdict) {
        self.peer = peer
        self.verdict = verdict
    }
}

/// One machine's Mesh endpoint on a host.
public final class Endpoint {
    private struct Retry {
        var peer: String
        var bytes: [UInt8]
        var attempts: Attempts
        var dueMs: Int64
    }

    private let router: Router
    private let transport: any Transport
    private let environment: any Environment
    private var retries: [Retry] = []
    private var toEngine: [EngineCall] = []
    private var hostErrors: [RouterError] = []
    private var forbidden: [String: Set<String>] = [:]
    private var admission: (any EventAdmission)?
    private var sealer: (any Sealer)?

    /// The verdicts of the causal layers on the changes the machine sent, in the order they were given
    /// (`takeVerdicts`).
    private var verdicts: [CausalPeerVerdict] = []

    public init(router: Router, transport: any Transport, environment: any Environment) {
        self.router = router
        self.transport = transport
        self.environment = environment
    }

    /// Start the causal layer of the causal peer `peer` (see `Router.startCausal`).
    public func startCausal(
        _ peer: String, start: CausalStart, room: Int, announceEveryMs: Int64
    ) throws(RouterError) {
        try router.startCausal(peer, start: start, room: room, announceEveryMs: announceEveryMs)
    }

    /// The verdicts of the causal layers on what the machine sent, in order. The host records each
    /// accepted change in the copy's journal, calls `committed` once the record is committed, and tells
    /// the copy of each refusal.
    public func takeVerdicts() -> [CausalPeerVerdict] {
        defer { verdicts.removeAll() }
        return verdicts
    }

    /// The host committed the journal record of `dot`, a change sent to the causal peer `peer`: it, and
    /// every accepted change before it, leaves.
    public func committed(peer: String, dot: Dot) {
        apply(router.committed(peer, dot: dot, nowMs: environment.nowMs()))
    }

    /// The events the link `peer` may not deliver to the machine: what arrives on it under one of
    /// these names is dropped before the machine is told, and the host is told instead
    /// (`RouterError.eventForbidden`). An empty collection lifts it.
    ///
    /// The link is what the host authenticated when it accepted the connection, and the envelope is
    /// whatever the peer wrote, its `source` among it: so what a link may say is decided here, where
    /// the link is known, and not by a machine that is told who sent an event by the event.
    public func forbidEvents(from peer: String, _ events: some Collection<String>) {
        if events.isEmpty {
            forbidden.removeValue(forKey: peer)
        } else {
            forbidden[peer] = Set(events)
        }
    }

    /// Put `admission` in front of the machine: every event that arrives and is not already forbidden
    /// by its link is shown to it, and one it does not admit is dropped and told to the host as
    /// `RouterError.eventNotAdmitted`. Replaces the one set before.
    public func setAdmission(_ admission: any EventAdmission) {
        self.admission = admission
    }

    /// Put `sealer` behind the machine: what it sends leaves with the data the sealer answers with.
    /// Replaces the one set before.
    public func setSealer(_ sealer: any Sealer) {
        self.sealer = sealer
    }

    /// Perform a `<send type="sce:mesh">` the engine handed its host.
    public func send(_ request: HostSendRequest) {
        let id = environment.envelopeID()
        var leaving = request
        if let sealer {
            switch sealer.seal(target: request.target, event: request.eventName, data: request.eventData) {
            case let .leave(data):
                if let data {
                    leaving.eventData = data
                }
            case let .withhold(reason):
                hostErrors.append(.eventNotSealed(target: request.target, event: request.eventName, reason: reason))
                return
            }
        }
        apply(router.send(leaving, id: id, nowMs: environment.nowMs()))
    }

    /// Start an `<invoke type="sce:mesh-rpc">` the engine handed its host, answering the engine as a
    /// host invoker does: with nothing when the request is on its way (its end is queued later), or
    /// with a refusal when it cannot reach the wire (§mesh-9.5's pre-envelope tier).
    public func invoke(_ request: HostInvokeRequest) -> HostInvokeResponse {
        let wireID = environment.envelopeID()
        let id = environment.envelopeID()
        switch router.invoke(
            request, wireID: wireID, id: id, nowMs: environment.nowMs(), nowUnixMs: environment.nowUnixMs())
        {
        case let .started(effects):
            apply(.done(effects))
            return HostInvokeResponse()
        case let .refused(data):
            return HostInvokeResponse(refusal: data)
        case let .failed(error):
            // Only the host can act on it; the document is told the request could not start rather
            // than left waiting on one that never left.
            hostErrors.append(error)
            return HostInvokeResponse(refusal: "\(error)")
        }
    }

    /// The state that started the start `token` of the Mesh request `invokeID` exited.
    public func cancelInvoke(invokeID: String, token: UInt64) {
        router.cancelInvoke(invokeID: invokeID, token: token)
    }

    /// Envelope `bytes` the transport received from `peer`.
    public func receive(peer: String, bytes: [UInt8]) {
        switch router.receive(peer: peer, bytes: bytes, nowMs: environment.nowMs()) {
        case let .done(effects): apply(.done(screen(peer: peer, effects)))
        case let routed: apply(routed)
        }
    }

    /// What a link may deliver, and what the host admits, applied to the events a receive released.
    private func screen(peer: String, _ effects: [Effect]) -> [Effect] {
        let refused = forbidden[peer]
        return effects.compactMap { effect -> Effect? in
            guard case let .deliver(event, data, source, sendID, invokeID) = effect else {
                return effect
            }
            if refused?.contains(event) == true {
                hostErrors.append(.eventForbidden(peer: peer, event: event))
                return nil
            }
            guard let admission else {
                return effect
            }
            switch admission.admit(peer: peer, event: event, data: data) {
            case let .allow(replacement):
                return .deliver(
                    event: event, data: replacement ?? data, source: source, sendID: sendID, invokeID: invokeID)
            case let .refuse(reason):
                hostErrors.append(.eventNotAdmitted(peer: peer, event: event, reason: reason))
                return nil
            }
        }
    }

    /// `peer`'s transport became ready.
    public func peerReady(_ peer: String) {
        apply(router.peerReady(peer, nowMs: environment.nowMs()))
    }

    /// `peer`'s transport stopped being ready.
    public func peerNotReady(_ peer: String) {
        apply(router.peerNotReady(peer))
    }

    /// End the ordering gaps that have waited out their timeout, and send again what has waited out
    /// its backoff. The host calls this on its own schedule; nothing else makes time pass for the core.
    public func tick() {
        let now = environment.nowMs()
        apply(router.tick(nowMs: now))
        let due = retries.filter { $0.dueMs <= now }
        retries.removeAll { $0.dueMs <= now }
        for retry in due {
            transmit(retry.peer, retry.bytes, retry.attempts)
        }
    }

    /// What the engine must now be told, in the order it arose.
    public func takeCalls() -> [EngineCall] {
        defer { toEngine.removeAll() }
        return toEngine
    }

    /// What went wrong that only the host can act on. None of these reaches the document (§mesh-16.7,
    /// "Scope of synthesis").
    public func takeHostErrors() -> [RouterError] {
        defer { hostErrors.removeAll() }
        return hostErrors
    }

    private func apply(_ routed: Routed) {
        let effects: [Effect]
        switch routed {
        case let .refused(error):
            hostErrors.append(error)
            return
        case let .done(done):
            effects = done
        }
        for effect in effects {
            switch effect {
            case let .transmit(peer, bytes):
                transmit(peer, bytes, Attempts())
            case let .deliver(event, data, source, sendID, invokeID):
                // §mesh-10.7: an inbound request's invokeid, which a reply sent while it is handled
                // carries back.
                var metadata = meshEventMetadata(data: data, source: source, sendID: sendID ?? "")
                metadata.invokeID = invokeID ?? ""
                toEngine.append(.raise(EngineEvent(name: event, metadata: metadata)))
            case let .raise(peer, signal):
                raise(peer, signal)
            case let .complete(invokeID, token, data, source):
                toEngine.append(.endInvoke(invokeID: invokeID, token: token, failed: false, data: data, source: source))
            case let .fail(invokeID, token, data, source):
                toEngine.append(.endInvoke(invokeID: invokeID, token: token, failed: true, data: data, source: source))
            case let .accepted(peer, dot):
                verdicts.append(CausalPeerVerdict(peer: peer, verdict: .accepted(dot)))
            case let .refused(peer, refusal):
                verdicts.append(CausalPeerVerdict(peer: peer, verdict: .refused(refusal)))
            case let .announceDue(peer):
                let id = environment.envelopeID()
                apply(router.announce(peer, id: id, nowMs: environment.nowMs()))
            }
        }
    }

    /// A transmission the transport accepted and then could not complete: an envelope still queued on
    /// a link that closed (SCE_MESH.md §mesh-18.3). It is that envelope's first failed send: sent again
    /// on the binding's schedule, or given up as its row.
    public func transmitFailed(peer: String, bytes: [UInt8], failure: TransportFailure) {
        afterFailure(peer, bytes, Attempts(), failure)
    }

    /// Transmit `bytes` to `peer`; a failure waits out the binding's backoff or is given up as its row.
    private func transmit(_ peer: String, _ bytes: [UInt8], _ attempts: Attempts) {
        guard let failure = transport.transmit(peer: peer, bytes: bytes) else {
            return
        }
        afterFailure(peer, bytes, attempts, failure)
    }

    private func afterFailure(_ peer: String, _ bytes: [UInt8], _ attempts: Attempts, _ failure: TransportFailure) {
        let decided: AfterFailure
        do {
            decided = try attempts.afterFailure(
                policy: router.retryPolicy(peer), retryable: failure.retryable, transportError: failure.message,
                draw: environment.jitterDraw())
        } catch {
            hostErrors.append(.rule(ruleFailure(error)))
            return
        }
        switch decided {
        case let .retryIn(waitMs):
            retries.append(Retry(peer: peer, bytes: bytes, attempts: attempts, dueMs: environment.nowMs() + waitMs))
        case let .giveUp(signal):
            raise(peer, signal)
        }
    }

    /// Queue the `error.communication` a row raises, as the C++ core raises it: an envelope from its
    /// own machine.
    private func raise(_ peer: String?, _ signal: Signal) {
        let data = signal.eventData(binding: router.binding(peer))
        toEngine.append(
            .raise(
                EngineEvent(
                    name: "error.communication",
                    metadata: meshEventMetadata(data: data, source: router.machine, sendID: ""))))
    }
}

/// Tell `engine` what `calls` say, in order. An event name the machine does not declare is dropped, as
/// the engine drops any such event; an invocation's end the engine no longer waits for (its state
/// exited, or it was started again) is refused by the engine's own token check.
///
/// An answer carries `_event.origin` `mesh://<source>` and origin type `sce:mesh-rpc` (§mesh-9.5); a
/// deadline the requester reached itself has no peer to name, and carries neither. Applied by the
/// engine's own `applyMeshCall`, the body a child session's calls go through.
public func apply<Policy: StatePolicy>(_ calls: [EngineCall], to engine: Engine<Policy>) {
    for call in calls {
        engine.applyMeshCall(call.meshCall)
    }
}

/// Serve `<send type="sce:mesh">` and `<invoke type="sce:mesh-rpc">` on `engine` with `endpoint`. The
/// send handler answers the engine with nothing, and a request that starts is answered with nothing
/// too: what either produces for the document is queued on the endpoint, and the host applies it with
/// `apply(_:to:)` after the step that sent it.
public func register<Policy: StatePolicy>(_ endpoint: Endpoint, on engine: Engine<Policy>) {
    engine.registerMeshRouter { request in
        endpoint.send(request)
        return []
    }
    engine.registerMeshRpcInvoker { event in
        switch event {
        case let .start(request):
            return endpoint.invoke(request)
        case let .cancel(cancel):
            endpoint.cancelInvoke(invokeID: cancel.invokeID, token: cancel.token)
            return nil
        }
    }
}
