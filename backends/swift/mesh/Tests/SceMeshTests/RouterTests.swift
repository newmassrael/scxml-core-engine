// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The router core, case for case with the Kotlin and Rust cores' router tests.

import SceForgeRuntime
import SceMesh
import SceRuntime
import Testing

/// What a router test did not expect, as a thrown failure so a helper can stand where a value is
/// needed.
struct Unexpected: Error, CustomStringConvertible {
    var description: String
}

func envelopeID(_ n: Int) -> [UInt8] {
    [UInt8](repeating: UInt8(truncatingIfNeeded: n), count: 16)
}

func decodeEnvelope(_ bytes: [UInt8]) throws -> Envelope {
    var cursor = SceCursor(bytes)
    do {
        return try Envelope.decode(&cursor)
    } catch {
        throw Unexpected(description: "not an envelope: \(error)")
    }
}

func encodeEnvelope(_ envelope: Envelope) throws -> [UInt8] {
    guard let bytes = encodeToBytes({ sink in try envelope.encode(into: &sink) }) else {
        throw Unexpected(description: "the envelope did not encode")
    }
    return bytes
}

func done(_ routed: Routed) throws -> [Effect] {
    switch routed {
    case let .done(effects): return effects
    case let .refused(error): throw Unexpected(description: "refused: \(error)")
    }
}

func sendRequest(_ target: String, _ event: String, _ data: String, sendID: String = "send.1") -> HostSendRequest {
    HostSendRequest(
        processorType: meshProcessorType, eventName: event, target: target, sendID: sendID, eventData: data)
}

@Suite struct RouterTests {
    /// A binding with ordering required on a transport that does not order.
    private let ordered = PeerConfig(
        transport: "wss",
        buffer: OutboundBuffer(maxPending: 4, maxAgeMs: 0),
        retry: nil,
        stampSequence: true,
        delivery: Delivery(dedup: true, ordered: true),
        // The peer these tests bind is `hmi`; `pair` gives each side its own.
        responders: ["hmi"],
        deadlineMs: nil,
        replyEvents: []
    )

    private var unordered: PeerConfig {
        var config = ordered
        config.stampSequence = false
        config.delivery = Delivery(dedup: true, ordered: false)
        return config
    }

    private func config(_ base: PeerConfig, responders: [String]) -> PeerConfig {
        var config = base
        config.responders = responders
        return config
    }

    /// Two routers bound to each other, the sender's transport ready.
    private func pair(_ base: PeerConfig) throws -> (ecu: Router, hmi: Router) {
        let ecu = Router(machine: "ecu", dedupWindow: 8, gapTimeoutMs: 50)
        let hmi = Router(machine: "hmi", dedupWindow: 8, gapTimeoutMs: 50)
        ecu.addPeer("hmi", config: config(base, responders: ["hmi"]))
        hmi.addPeer("ecu", config: config(base, responders: ["ecu"]))
        #expect(try done(ecu.peerReady("hmi", nowMs: 0)).isEmpty)
        return (ecu, hmi)
    }

    private func transmitted(_ routed: Routed) throws -> [UInt8] {
        let effects = try done(routed)
        guard effects.count == 1, case let .transmit(peer, bytes) = effects[0] else {
            throw Unexpected(description: "expected one transmission, got \(effects)")
        }
        #expect(peer == "hmi")
        return bytes
    }

    private func events(_ effects: [Effect]) throws -> [String] {
        try effects.map { effect in
            guard case let .deliver(event, _, _, _, _) = effect else {
                throw Unexpected(description: "expected only deliveries, got \(effects)")
            }
            return event
        }
    }

    @Test func aSendToAReadyPeerCrossesAsTheEnginesEventData() throws {
        let (ecu, hmi) = try pair(ordered)
        let bytes = try transmitted(
            ecu.send(sendRequest("#hmi", "speed.changed", #"{"kph":42}"#), id: envelopeID(1), nowMs: 0))
        #expect(
            try done(hmi.receive(peer: "ecu", bytes: bytes, nowMs: 0))
                == [.deliver(event: "speed.changed", data: #"{"kph":42}"#, source: "ecu", sendID: "send.1", invokeID: nil)]
        )
    }

    @Test func withoutABufferASendIsTransmittedBeforeThePeerIsReady() throws {
        let ecu = Router(machine: "ecu", dedupWindow: 8, gapTimeoutMs: 50)
        var unbuffered = unordered
        unbuffered.buffer = nil
        ecu.addPeer("hmi", config: unbuffered)
        _ = try transmitted(ecu.send(sendRequest("#hmi", "ping", ""), id: envelopeID(7), nowMs: 0))
    }

    @Test func aSendWithNoDataTravelsWithNoPayload() throws {
        let (ecu, hmi) = try pair(ordered)
        let bytes = try transmitted(ecu.send(sendRequest("#hmi", "ping", ""), id: envelopeID(2), nowMs: 0))
        #expect(try decodeEnvelope(bytes).datacontenttype == PayloadCodec.none)
        #expect(
            try done(hmi.receive(peer: "ecu", bytes: bytes, nowMs: 0))
                == [.deliver(event: "ping", data: "", source: "ecu", sendID: "send.1", invokeID: nil)])
    }

    @Test func aSendWithoutAnIdCarriesNoSubject() throws {
        let (ecu, hmi) = try pair(ordered)
        let bytes = try transmitted(
            ecu.send(sendRequest("#hmi", "ping", "", sendID: ""), id: envelopeID(3), nowMs: 0))
        #expect(try decodeEnvelope(bytes).subject == nil)
        #expect(
            try done(hmi.receive(peer: "ecu", bytes: bytes, nowMs: 0))
                == [.deliver(event: "ping", data: "", source: "ecu", sendID: nil, invokeID: nil)])
    }

    @Test func anOrderedBindingStampsFromOneSoTheReceiverRestoresSendOrder() throws {
        let (ecu, hmi) = try pair(ordered)
        var sent: [[UInt8]] = []
        for (index, event) in ["a", "b", "c"].enumerated() {
            sent.append(
                try transmitted(ecu.send(sendRequest("#hmi", event, ""), id: envelopeID(index + 1), nowMs: 0)))
        }
        #expect(try sent.map { try decodeEnvelope($0).sequence_no } == [1, 2, 3])
        #expect(try events(done(hmi.receive(peer: "ecu", bytes: sent[0], nowMs: 0))) == ["a"])
        #expect(try done(hmi.receive(peer: "ecu", bytes: sent[2], nowMs: 0)).isEmpty)
        #expect(try events(done(hmi.receive(peer: "ecu", bytes: sent[1], nowMs: 0))) == ["b", "c"])
    }

    @Test func anUnorderedBindingDoesNotStamp() throws {
        let (ecu, _) = try pair(unordered)
        let bytes = try transmitted(ecu.send(sendRequest("#hmi", "a", ""), id: envelopeID(1), nowMs: 0))
        #expect(try decodeEnvelope(bytes).sequence_no == nil)
    }

    @Test func anUnstampedEnvelopeOnAnOrderedBindingIsMissingItsSequence() throws {
        let (ecu, _) = try pair(unordered)
        let hmi = Router(machine: "hmi", dedupWindow: 8, gapTimeoutMs: 50)
        hmi.addPeer("ecu", config: ordered)
        let bytes = try transmitted(ecu.send(sendRequest("#hmi", "a", ""), id: envelopeID(1), nowMs: 0))
        #expect(
            try done(hmi.receive(peer: "ecu", bytes: bytes, nowMs: 0))
                == [.raise(peer: "ecu", signal: .missingSequence(source: "ecu"))])
    }

    @Test func aResentEnvelopeIsDeliveredOnce() throws {
        let (ecu, hmi) = try pair(unordered)
        let bytes = try transmitted(ecu.send(sendRequest("#hmi", "a", ""), id: envelopeID(7), nowMs: 0))
        #expect(try events(done(hmi.receive(peer: "ecu", bytes: bytes, nowMs: 0))) == ["a"])
        #expect(try done(hmi.receive(peer: "ecu", bytes: bytes, nowMs: 1)).isEmpty)
    }

    @Test func aPeerNotYetReadyHoldsTheSendUntilItIs() throws {
        let ecu = Router(machine: "ecu", dedupWindow: 8, gapTimeoutMs: 50)
        ecu.addPeer("hmi", config: ordered)
        #expect(try done(ecu.send(sendRequest("#hmi", "a", ""), id: envelopeID(1), nowMs: 0)).isEmpty)
        _ = try transmitted(ecu.peerReady("hmi", nowMs: 10))
    }

    @Test func aMeshTargetWithNoBindingIsUnreachableNotUnserved() throws {
        let ecu = Router(machine: "ecu", dedupWindow: 8, gapTimeoutMs: 50)
        #expect(
            try done(ecu.send(sendRequest("#nowhere", "a", ""), id: envelopeID(1), nowMs: 0))
                == [.raise(peer: "nowhere", signal: .transportUnavailable)])
    }

    @Test func aTargetThatIsNotAMeshPeerIsRefusedToTheHost() {
        let ecu = Router(machine: "ecu", dedupWindow: 8, gapTimeoutMs: 50)
        for target in ["hmi", "#", "#_internal", "#_parent", ""] {
            #expect(
                ecu.send(sendRequest(target, "a", ""), id: envelopeID(1), nowMs: 0)
                    == .refused(.notMeshTarget(target)))
        }
    }

    @Test func bytesThatAreNotAnEnvelopeAreReportedAsCorruptCbor() throws {
        let hmi = Router(machine: "hmi", dedupWindow: 8, gapTimeoutMs: 50)
        hmi.addPeer("ecu", config: ordered)
        #expect(
            try done(hmi.receive(peer: "ecu", bytes: [0xFF], nowMs: 0))
                == [.raise(peer: "ecu", signal: .envelopeCorrupt(source: nil, codec: "cbor"))])
    }

    @Test func aPayloadTheEngineCannotBeHandedIsCorruptInItsOwnCodec() throws {
        let hmi = Router(machine: "hmi", dedupWindow: 8, gapTimeoutMs: 50)
        hmi.addPeer("ecu", config: unordered)
        let cases: [(PayloadCodec, String)] = [(.cbor, "cbor"), (.typed, "typed"), (.raw, "raw")]
        for (index, (codec, name)) in cases.enumerated() {
            var envelope = Envelope()
            envelope.id = envelopeID(index + 1)
            envelope.source = "ecu"
            envelope.event_type = "a"
            envelope.pattern = .fireForget
            envelope.datacontenttype = codec
            envelope.data = [1]
            #expect(
                try done(hmi.receive(peer: "ecu", bytes: encodeEnvelope(envelope), nowMs: 0))
                    == [.raise(peer: "ecu", signal: .envelopeCorrupt(source: "ecu", codec: name))])
        }
    }

    @Test func aCausalRoutesControlMessageNeverReachesTheEngine() throws {
        // docs/adr/0085, section 3.2: this core carries no causal route, and a `causalVector` that
        // reaches it is dropped, not delivered as an event.
        let hmi = Router(machine: "hmi", dedupWindow: 8, gapTimeoutMs: 50)
        hmi.addPeer("ecu", config: unordered)
        var envelope = Envelope()
        envelope.id = envelopeID(1)
        envelope.source = "ecu"
        envelope.pattern = .causalVector
        envelope.datacontenttype = PayloadCodec.none
        #expect(try done(hmi.receive(peer: "ecu", bytes: encodeEnvelope(envelope), nowMs: 0)).isEmpty)
    }

    @Test func aRaisedRowCarriesTheBindingItWasObservedOn() throws {
        let ecu = Router(machine: "ecu", dedupWindow: 8, gapTimeoutMs: 50)
        ecu.addPeer("hmi", config: ordered)
        let unreachable = try done(ecu.send(sendRequest("#nowhere", "a", ""), id: envelopeID(1), nowMs: 0))
        guard unreachable.count == 1, case let .raise(peer, signal) = unreachable[0] else {
            throw Unexpected(description: "expected one raised row, got \(unreachable)")
        }
        #expect(
            signal.eventData(binding: ecu.binding(peer))
                == #"{"errorName":"communication","reason":"TRANSPORT_UNAVAILABLE","target":"nowhere"}"#)
        _ = try done(ecu.peerReady("hmi", nowMs: 0))
        let lost = try done(ecu.peerNotReady("hmi"))
        guard lost.count == 1, case let .raise(lostPeer, lostSignal) = lost[0] else {
            throw Unexpected(description: "expected one raised row, got \(lost)")
        }
        #expect(
            lostSignal.eventData(binding: ecu.binding(lostPeer))
                == #"{"errorName":"communication","reason":"TRANSPORT_UNAVAILABLE","target":"hmi","transport":"wss"}"#
        )
    }

    @Test func aPeerTheHostNeverBoundIsTheHostsError() {
        let hmi = Router(machine: "hmi", dedupWindow: 8, gapTimeoutMs: 50)
        #expect(hmi.receive(peer: "ecu", bytes: [0xFF], nowMs: 0) == .refused(.unknownPeer("ecu")))
        #expect(hmi.peerReady("ecu", nowMs: 0) == .refused(.unknownPeer("ecu")))
    }

    // §mesh-9.5: `<invoke type="sce:mesh-rpc">`, the requester's half

    private let wire = [UInt8](repeating: 9, count: 16)
    private let wireText = "09090909-0909-0909-0909-090909090909"

    /// `ecu` bound to `hmi` with a binding-level deadline, and to `mallory`, which is not in `hmi`'s
    /// responder set.
    private func requester(deadlineMs: Int64?) -> Router {
        let ecu = Router(machine: "ecu", dedupWindow: 8, gapTimeoutMs: 50)
        var base = unordered
        base.buffer = nil
        base.responders = ["hmi"]
        base.deadlineMs = deadlineMs
        ecu.addPeer("hmi", config: base)
        ecu.addPeer("mallory", config: config(base, responders: ["mallory"]))
        return ecu
    }

    private func ask(_ src: String, deadlineMs: String?) -> HostInvokeRequest {
        var params = [meshEventParam: ["service.request.force"]]
        if let deadlineMs {
            params[meshDeadlineParam] = [deadlineMs]
        }
        return HostInvokeRequest(
            processorType: "sce:mesh-rpc", invokeID: "ask", src: src, params: params, eventData: #"{"n":3}"#,
            token: 7)
    }

    /// Start `ask("#hmi")` at monotonic 10, wall 1000, returning the request envelope's bytes.
    private func started(_ ecu: Router, deadlineMs: String?) throws -> [UInt8] {
        let invoked = ecu.invoke(ask("#hmi", deadlineMs: deadlineMs), wireID: wire, id: envelopeID(1), nowMs: 10, nowUnixMs: 1000)
        guard case let .started(effects) = invoked else {
            throw Unexpected(description: "expected the request to start, got \(invoked)")
        }
        return try transmitted(.done(effects))
    }

    private func reply(_ n: Int, _ status: RpcStatus?, _ message: String?, _ data: String) throws -> [UInt8] {
        var envelope = Envelope()
        envelope.id = envelopeID(n)
        envelope.source = "hmi"
        envelope.event_type = "service.response.force"
        envelope.pattern = .rpcReply
        envelope.datacontenttype = data.isEmpty ? PayloadCodec.none : .json
        envelope.data = Array(data.utf8)
        envelope.invoke_id = wire
        envelope.rpc_status = status
        envelope.rpc_error_message = message
        return try encodeEnvelope(envelope)
    }

    @Test func aRequestCarriesItsEventPayloadAndWireIDs() throws {
        let envelope = try decodeEnvelope(started(requester(deadlineMs: 500), deadlineMs: nil))
        #expect(envelope.pattern == .rpcRequest)
        #expect(envelope.event_type == "service.request.force")
        #expect(String(decoding: envelope.data, as: UTF8.self) == #"{"n":3}"#)
        #expect(envelope.datacontenttype == .json)
        #expect(envelope.invoke_id == wire)
        #expect(envelope.id == envelopeID(1))
        // §mesh-9.5 precedence: the binding's deadline when the invoke gives none.
        #expect(envelope.deadline_unix_ms == 1500)
    }

    @Test func theInvokesOwnDeadlineWinsOverTheBindings() throws {
        #expect(try decodeEnvelope(started(requester(deadlineMs: 500), deadlineMs: "200")).deadline_unix_ms == 1200)
    }

    @Test func aRequestWithNoDeadlineCarriesNone() throws {
        let ecu = requester(deadlineMs: nil)
        #expect(try decodeEnvelope(started(ecu, deadlineMs: nil)).deadline_unix_ms == nil)
        #expect(try done(ecu.tick(nowMs: Int64.max)).isEmpty, "no deadline, no expiry")
    }

    @Test func aDeadlineBeyondTheClocksRangeIsHeldAtItsMaximum() throws {
        let ecu = requester(deadlineMs: nil)
        let envelope = try decodeEnvelope(started(ecu, deadlineMs: "18446744073709551615"))
        #expect(envelope.deadline_unix_ms == UInt64(Int64.max))
    }

    @Test func anOkReplyCompletesTheInvocationOnce() throws {
        let ecu = requester(deadlineMs: nil)
        _ = try started(ecu, deadlineMs: nil)
        #expect(
            try done(ecu.receive(peer: "hmi", bytes: reply(2, .ok, nil, #"{"force":12}"#), nowMs: 20))
                == [.complete(invokeID: "ask", token: 7, data: #"{"force":12}"#, source: "hmi")])
        // A second answer finds nothing waiting.
        #expect(try done(ecu.receive(peer: "hmi", bytes: reply(3, .ok, nil, "1"), nowMs: 21)).isEmpty)
    }

    /// The key is optional on the wire; the requester reads its absence as `ok`, as the C++ core does.
    @Test func aReplyWithNoStatusIsOk() throws {
        let ecu = requester(deadlineMs: nil)
        _ = try started(ecu, deadlineMs: nil)
        #expect(
            try done(ecu.receive(peer: "hmi", bytes: reply(2, nil, nil, ""), nowMs: 20))
                == [.complete(invokeID: "ask", token: 7, data: "", source: "hmi")])
    }

    @Test func aFailedReplyFailsTheInvocationWithTheStatusByName() throws {
        let ecu = requester(deadlineMs: nil)
        _ = try started(ecu, deadlineMs: nil)
        #expect(
            try done(ecu.receive(peer: "hmi", bytes: reply(2, .unavailable, "busy", ""), nowMs: 20))
                == [
                    .fail(
                        invokeID: "ask", token: 7,
                        data:
                            #"{"errorName":"invoke","reason":"unavailable","detail":"busy","source":"hmi","invoke_id":"\#(wireText)"}"#,
                        source: "hmi")
                ])
    }

    /// §mesh-14.6: a reply from outside the responder set is row 14 and leaves the request answerable.
    @Test func aReplyFromAnUndeclaredPeerIsRefusedAndTheRequestStays() throws {
        let ecu = requester(deadlineMs: nil)
        _ = try started(ecu, deadlineMs: nil)
        #expect(
            // The envelope's word, reported as written; what refused it was the binding it came in on.
            try done(ecu.receive(peer: "mallory", bytes: reply(2, .ok, nil, "1"), nowMs: 20))
                == [.raise(peer: "mallory", signal: .rpcReplyFromUndeclaredPeer(source: "hmi", invokeID: wireText))])
        #expect(
            try done(ecu.receive(peer: "hmi", bytes: reply(3, .ok, nil, "2"), nowMs: 21))
                == [.complete(invokeID: "ask", token: 7, data: "2", source: "hmi")])
    }

    /// §mesh-9.5 `<cancel>`: nothing on the wire, and a later answer is dropped.
    @Test func aCancelledRequestDropsItsAnswer() throws {
        let ecu = requester(deadlineMs: 100)
        _ = try started(ecu, deadlineMs: nil)
        ecu.cancelInvoke(invokeID: "ask", token: 7)
        #expect(try done(ecu.receive(peer: "hmi", bytes: reply(2, .ok, nil, "1"), nowMs: 20)).isEmpty)
        #expect(try done(ecu.tick(nowMs: 1000)).isEmpty, "a cancelled request has no deadline")
    }

    @Test func aDeadlineFailsTheInvocationAsDeadlineExceededWithNoSource() throws {
        let ecu = requester(deadlineMs: 100)
        _ = try started(ecu, deadlineMs: nil)
        // Kept against the monotonic clock the request started at (10).
        #expect(try done(ecu.tick(nowMs: 109)).isEmpty)
        #expect(
            try done(ecu.tick(nowMs: 110))
                == [
                    .fail(
                        invokeID: "ask", token: 7,
                        data: #"{"errorName":"invoke","reason":"deadlineExceeded","invoke_id":"\#(wireText)"}"#,
                        source: nil)
                ])
        #expect(try done(ecu.receive(peer: "hmi", bytes: reply(2, .ok, nil, "1"), nowMs: 120)).isEmpty)
    }

    /// An `ok` the engine cannot be handed is row 4 and ends nothing: the request still waits for an
    /// answer it can use.
    @Test func anUnreadableOkReplyLeavesTheRequestWaiting() throws {
        let ecu = requester(deadlineMs: nil)
        _ = try started(ecu, deadlineMs: nil)
        var raw = Envelope()
        raw.id = envelopeID(2)
        raw.source = "hmi"
        raw.event_type = "service.response.force"
        raw.pattern = .rpcReply
        raw.datacontenttype = .raw
        raw.data = [0xFF]
        raw.invoke_id = wire
        #expect(
            try done(ecu.receive(peer: "hmi", bytes: encodeEnvelope(raw), nowMs: 20))
                == [.raise(peer: "hmi", signal: .envelopeCorrupt(source: "hmi", codec: "raw"))])
        #expect(
            try done(ecu.receive(peer: "hmi", bytes: reply(3, .ok, nil, "1"), nowMs: 21))
                == [.complete(invokeID: "ask", token: 7, data: "1", source: "hmi")])
    }

    /// §mesh-9.5's pre-envelope tier: a target that cannot reach the wire refuses the invocation.
    @Test func aRequestThatCannotReachTheWireIsRefused() {
        let ecu = requester(deadlineMs: nil)
        #expect(
            ecu.invoke(ask("#nobody", deadlineMs: nil), wireID: wire, id: envelopeID(1), nowMs: 0, nowUnixMs: 0)
                == .refused(
                    #"{"errorName":"execution","reason":"INVOKE_SRC_NOT_FOUND","detail":"no binding for '#nobody'"}"#))
        let refused = ecu.invoke(ask("hmi", deadlineMs: nil), wireID: wire, id: envelopeID(1), nowMs: 0, nowUnixMs: 0)
        guard case let .refused(data) = refused else {
            Issue.record("expected a refusal, got \(refused)")
            return
        }
        #expect(data.contains("names no Mesh peer"), "\(data)")
    }

    // §mesh-10.7: the responder's half, the request in and its reply out

    /// `hmi`, answering `ecu`: `service.response.force` is a reply to it.
    private func responder() -> Router {
        let hmi = Router(machine: "hmi", dedupWindow: 8, gapTimeoutMs: 50)
        var base = unordered
        base.buffer = nil
        base.responders = ["ecu"]
        base.replyEvents = ["service.response.force"]
        hmi.addPeer("ecu", config: base)
        return hmi
    }

    private func answer(_ event: String, invokeID: String) -> HostSendRequest {
        var request = sendRequest("#ecu", event, #"{"force":12}"#)
        request.invokeID = invokeID
        return request
    }

    private func sentToEcu(_ routed: Routed) throws -> [UInt8] {
        let effects = try done(routed)
        guard effects.count == 1, case let .transmit(peer, bytes) = effects[0] else {
            throw Unexpected(description: "expected one transmission, got \(routed)")
        }
        #expect(peer == "ecu")
        return bytes
    }

    /// The request arrives as the event it names, with its wire invokeid as `_event.invokeid`.
    @Test func aRequestIsDeliveredWithItsWireInvokeid() throws {
        let bytes = try started(requester(deadlineMs: nil), deadlineMs: nil)
        #expect(
            try done(responder().receive(peer: "ecu", bytes: bytes, nowMs: 0))
                == [
                    .deliver(
                        event: "service.request.force", data: #"{"n":3}"#, source: "ecu", sendID: nil,
                        invokeID: wireText)
                ])
    }

    /// The whole round trip: the reply sent while handling the request goes out as `rpcReply` and
    /// completes it.
    @Test func aReplySentWhileHandlingTheRequestAnswersIt() throws {
        let ecu = requester(deadlineMs: nil)
        let hmi = responder()
        _ = try done(hmi.receive(peer: "ecu", bytes: started(ecu, deadlineMs: nil), nowMs: 0))

        let bytes = try sentToEcu(
            hmi.send(answer("service.response.force", invokeID: wireText), id: envelopeID(5), nowMs: 0))
        let reply = try decodeEnvelope(bytes)
        #expect(reply.pattern == .rpcReply)
        #expect(reply.invoke_id == wire)
        #expect(reply.rpc_status == .ok)

        #expect(
            try done(ecu.receive(peer: "hmi", bytes: bytes, nowMs: 20))
                == [.complete(invokeID: "ask", token: 7, data: #"{"force":12}"#, source: "hmi")])
    }

    /// Only the events the deployment names as replies are stamped; a notification cannot retire the
    /// invocation.
    @Test func aSendThatIsNotAReplyIsNotStampedAsOne() throws {
        let envelope = try decodeEnvelope(
            sentToEcu(responder().send(answer("status.changed", invokeID: wireText), id: envelopeID(5), nowMs: 0)))
        #expect(envelope.pattern == .fireForget)
        #expect(envelope.invoke_id == nil)
        #expect(envelope.rpc_status == nil)
    }

    /// A reply sent with no request current has no invokeid to carry.
    @Test func aReplyWithNoRequestCurrentCarriesNoInvokeid() throws {
        let envelope = try decodeEnvelope(
            sentToEcu(responder().send(answer("service.response.force", invokeID: ""), id: envelopeID(5), nowMs: 0)))
        #expect(envelope.pattern == .rpcReply)
        #expect(envelope.invoke_id == nil)
    }

    /// §mesh-16.7 row 5: the link a request waits on is lost; a reply or a deadline afterwards ends
    /// nothing.
    @Test func aLostLinkIsRow5ForEachRequestWaitingOnIt() throws {
        let ecu = requester(deadlineMs: 100)
        _ = try started(ecu, deadlineMs: nil)
        #expect(
            try done(ecu.peerNotReady("hmi"))
                == [.raise(peer: "hmi", signal: .invokeChildLost(invokeID: wireText, target: "hmi"))])
        #expect(try done(ecu.receive(peer: "hmi", bytes: reply(2, .ok, nil, "1"), nowMs: 20)).isEmpty)
        #expect(try done(ecu.tick(nowMs: 1000)).isEmpty)
    }

    @Test func aRequestWithoutItsEventParamIsTheHostsError() {
        var request = ask("#hmi", deadlineMs: nil)
        request.params.removeValue(forKey: meshEventParam)
        #expect(
            requester(deadlineMs: nil).invoke(request, wireID: wire, id: envelopeID(1), nowMs: 0, nowUnixMs: 0)
                == .failed(.missingParam(meshEventParam)))
    }
}
