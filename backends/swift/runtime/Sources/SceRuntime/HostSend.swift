// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The Event I/O Processors a host serves (§scxml-6.2.5).
//
// The clause makes the processor identifier extensible, so the set of types is open by design: a
// host declares the types it serves to the build, so that the generated machine dispatches a send
// and does not refuse it, and registers a handler for each at run time. The Swift port of Kotlin's
// `StateMachineEngine.registerEventProcessor` and Rust's `HostSendRequest`, field for field.

/// The prefix SCE keeps for the processors it defines itself (`sce:mesh`, `sce:mesh-rpc`).
///
/// A host that registered a handler under it would replace SCE's processor with nothing on the
/// wire saying so, so a general registration refuses it and a processor of SCE's own is installed
/// only by a call that names itself one (`registerMeshRouter`). The rule is the leading prefix,
/// spelled exactly: a name that merely contains it, or spells it in another case, is a host's to
/// use. Every copy of the rule reads `sce-build/tests/fixtures/host_processor/reserved_type_cases.json`.
public let reservedProcessorTypePrefix = "sce:"

/// The type of the Mesh router's `<send>` (SCE_MESH.md §19): the build lowers a send to a peer to a
/// send of this type, and the host serves it with the router it registered.
public let meshProcessorType = "sce:mesh"

/// Whether `processorType` is one SCE keeps for itself.
public func isReservedProcessorType(_ processorType: String) -> Bool {
    processorType.utf8.starts(with: reservedProcessorTypePrefix.utf8)
}

/// The text of the `error.execution` a `<send>` to `processorType` raises when the host declared the
/// type to the build and registered no handler for it: the sentence the Rust and Kotlin engines
/// raise for the same fact.
public func unregisteredHostProcessorMessage(_ processorType: String) -> String {
    "<send type='\(processorType)'> names a processor the host declared but never registered"
}

/// Why a registration was refused.
public enum HostRegistrationError: Error, Equatable, Sendable {
    /// The type is under the prefix SCE keeps (`reservedProcessorTypePrefix`); `call` is the
    /// registration that refused it.
    case reservedType(call: String, type: String)
}

/// A message a send carries as the value the generated code holds, and never as an encoding: what
/// the test profile's test-port processor is handed by a send of a specific template, a record of
/// the type the event schema's unit declares (docs/SCE_TEST_PROFILE.md §12.3; docs/adr/0091, C2).
///
/// The value is any value Swift compares and shares across threads, which every type a template's
/// value is held in is: a payload struct, a number, a `Bool`, a `String`, a `[UInt8]`, an array or an
/// optional of those, and a closed enum's unit. It is held by an existential, so the request that
/// carries it stays `Equatable` and `Sendable`, and two of them are equal where they hold values of one
/// type that are equal. A reader takes it back by its type (`value as? T`).
public struct HostSendValue: Sendable, Equatable {
    /// The value.
    public let value: any Equatable & Sendable

    public init(_ value: some Equatable & Sendable) {
        self.value = value
    }

    public static func == (lhs: HostSendValue, rhs: HostSendValue) -> Bool {
        holdsEqual(lhs.value, rhs.value)
    }
}

/// Whether `other` is a value of `held`'s type equal to it: a value of another type is never equal.
private func holdsEqual<T: Equatable>(_ held: T, _ other: any Equatable) -> Bool {
    guard let other = other as? T else { return false }
    return held == other
}

/// What a `<send>` addressed to a host-served processor said.
///
/// Every field is what the document wrote, not an interpretation of it. `processorType` is
/// present although the handler was looked up by it, because one handler may serve several types.
/// `target` is passed through uninterpreted: the specification leaves a target's meaning to the
/// processor that serves it. `params` is a multi-map because `<param>` may repeat and every value
/// must be delivered.
public struct HostSendRequest: Sendable, Equatable {
    public var processorType: String
    public var eventName: String
    public var target: String
    public var content: String
    public var params: [String: [String]]
    public var sendID: String

    /// The event's data exactly as a local delivery of this send would carry it in `_event.data`
    /// (§scxml-5.10): the namelist and `<param>` pairs as JSON, or the `<content>`, or empty.
    /// Computed once by the generated send site, so a host that forwards the event (the Mesh router
    /// puts it in an envelope) forwards the engine's serialisation and not a second one made from
    /// `params` that could differ.
    public var eventData: String

    /// `_event.invokeid` of the event being processed when the `<send>` executed, empty when it had
    /// none. A send made while handling an event carries its invokeid back out, as a W3C child's
    /// send to its parent does (§scxml-6.4.1); a Mesh router matches the reply a document sends to
    /// the request it received by nothing else (SCE_MESH.md §10.7). Taken when the send executes, so
    /// a delayed one carries the invokeid current then, not when it fires.
    public var invokeID: String

    /// The message as the value the generated code holds, for a processor whose messages are values
    /// and never encodings: the test profile's test-port processor, whose send carries its specific
    /// template's value (docs/SCE_TEST_PROFILE.md §12.3; docs/adr/0091, C2). Nil for every other
    /// send, whose data is `eventData`, so a send site that names no value is unchanged.
    ///
    /// A saved state does not carry it: a test-port send has no delay (the test profile's P6), so none
    /// is pending when a machine is saved, and a save that finds one waiting refuses by name rather
    /// than restore a send without the value it was made with.
    public var value: HostSendValue?

    public init(
        processorType: String,
        eventName: String = "",
        target: String = "",
        content: String = "",
        params: [String: [String]] = [:],
        sendID: String = "",
        eventData: String = "",
        invokeID: String = "",
        value: HostSendValue? = nil
    ) {
        self.processorType = processorType
        self.eventName = eventName
        self.target = target
        self.content = content
        self.params = params
        self.sendID = sendID
        self.eventData = eventData
        self.invokeID = invokeID
        self.value = value
    }
}

/// One event a host-served act produced.
///
/// The engine raises each on the external queue, which is where a reply from outside the machine
/// belongs (§scxml-C-1). A name the machine does not declare is dropped, as any such event is.
public struct HostSendResponse: Sendable, Equatable {
    public var eventName: String
    public var eventData: String

    /// `_event.origin` and `_event.origintype` of the reply, in the host's own terms
    /// (docs/adr/0090, section 2): the host processor's type and the address a `<send>` of
    /// that type reaches the party at. Nil when the reply names no party, and then both fields
    /// read empty; the engine never fills in the SCXML processor's type for a host.
    public var origin: EventOrigin?

    public init(eventName: String, eventData: String = "", origin: EventOrigin? = nil) {
        self.eventName = eventName
        self.eventData = eventData
        self.origin = origin
    }
}

/// What a host registers for one declared processor type.
///
/// Answers with the events the act produced, IN ORDER. A list and not a single reply, because an
/// act can produce two observations the document must see in a particular order, and every other
/// way of saying so costs portability or hides state. An empty list is "performed, nothing to
/// report", the common case of a fire-and-forget act and of real work that will answer later through
/// the host's own loop; it is not an error.
public typealias HostSendHandler = (HostSendRequest) -> [HostSendResponse]

/// What an engine holds of the processors a host registered. Internal, held by `Engine.hostSends`.
struct HostSendBook {
    var handlers: [String: HostSendHandler] = [:]
}

extension Engine {
    /// Registers what performs every `<send type="<processorType>">` this machine executes
    /// (§scxml-6.2.5).
    ///
    /// A type declared to the build and never registered raises `error.execution` at the send,
    /// because from the document's side an act nobody performed is the same either way.
    /// Registering twice for one type replaces the handler: two handlers for one type would make
    /// dispatch depend on registration order, and a host that registers again during a run means to
    /// change what serves the act.
    ///
    /// - Throws: `HostRegistrationError.reservedType` when `processorType` is under
    ///   `reservedProcessorTypePrefix`; the Mesh router has its own door, `registerMeshRouter`.
    public func registerEventProcessor(
        _ processorType: String, handler: @escaping HostSendHandler
    ) throws(HostRegistrationError) {
        if isReservedProcessorType(processorType) {
            throw .reservedType(call: "registerEventProcessor", type: processorType)
        }
        hostSends.handlers[processorType] = handler
    }

    /// Registers `router` as the Event I/O Processor Mesh sends reach (SCE_MESH.md §19): the build
    /// lowers a `<send target="#peer">` to a `<send type="sce:mesh">`.
    ///
    /// The one way to serve `meshProcessorType`: `registerEventProcessor` refuses the reserved
    /// prefix, so a router is only ever installed by a call that names itself one. Unregistered, a
    /// Mesh send raises `error.execution` as any host processor without a handler does; a
    /// registered router that cannot reach the peer raises `error.communication` itself.
    public func registerMeshRouter(_ router: @escaping HostSendHandler) {
        hostSends.handlers[meshProcessorType] = router
    }

    /// Whether a handler is registered for `processorType`.
    public func hasEventProcessor(_ processorType: String) -> Bool {
        hostSends.handlers[processorType] != nil
    }

    /// Performs a host-served `<send>` now (§scxml-6.2.5): the registered handler is given the
    /// request and each event it answers is raised on the external queue, in the order it answered.
    ///
    /// A type nobody registered raises `error.execution` and nothing else: an act nobody performed.
    /// The error names the send that caused it by the request's `sendID`, the id the document wrote
    /// or the one the machine generated for it (§scxml-6.2.4 + §scxml-5.10, test 332), whether the
    /// send was made now or its delay is over: a delayed send reaches here from the scheduler with
    /// the request it made. The handler is the only dispatch there is; the engine has no knowledge
    /// of what a processor does.
    public func performHostSend(_ request: HostSendRequest) {
        guard let handler = hostSends.handlers[request.processorType] else {
            if let errorEvent = policy.getEventFromName("error.execution") {
                // The sentence every backend's engine raises for this fact, so a consumer matching on
                // it does not have to know which engine ran the send or whether it carried a delay.
                var unserved = EventWithMetadata.platformError(
                    errorEvent,
                    message: unregisteredHostProcessorMessage(request.processorType)
                )
                unserved.metadata.sendID = request.sendID
                raise(unserved)
            }
            return
        }
        // The external queue is where a reply from outside the machine belongs (§scxml-C-1); a name
        // the machine does not declare is dropped there. Each reply carries the origin the host
        // named for it, and none when it named no party (docs/adr/0090, section 2).
        for response in handler(request) {
            raiseExternalByName(response.eventName, eventData: response.eventData, origin: response.origin)
        }
    }

    /// Schedules a host-served `<send>` to be performed when `delay` is over (§scxml-6.2.4).
    /// Returns the send id, which `<cancel sendid>` reaches while the act waits (§scxml-6.3).
    ///
    /// A delay postpones a send; it does not change what it says, so the request is the one of the
    /// moment the send was made, its `invokeID` included. A wait of zero is no deferral: the act is
    /// performed now, before this returns, and nothing is queued.
    @discardableResult
    public func scheduleHostSend(delay: Duration, request: HostSendRequest, sender: String) -> String {
        if delay <= .zero {
            performHostSend(request)
            return request.sendID
        }
        return scheduleRoutedEvent(
            nil,
            delay: delay,
            sendID: request.sendID,
            eventData: request.eventData,
            sender: sender,
            route: ScheduledRoute(kind: .host, eventName: request.eventName, host: request)
        )
    }
}
