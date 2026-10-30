// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The invoker half of the host processor: what runs an `<invoke>` of a type no
// engine runs itself (§scxml-6.4.1). A port of the Go runtime's
// `backends/go/runtime/host_processor.go` (`PerformHostInvoke` and what follows
// it), field for field, so one host described once keeps one shape in every
// language.

/// An `<invoke>` the host runs, at the point its state was entered.
public struct HostInvokeRequest: Equatable {
    /// The `type` the `<invoke>` named.
    public var processorType: String

    /// The invoke's id (§scxml-6.4.1), auto-derived when the author declared none. The
    /// name the DOCUMENT waits on: a completion is `done.invoke.<invokeID>`, so a host
    /// that finishes later keeps it.
    public var invokeID: String

    /// `<invoke src="...">`, empty when the document named none. What a src means is
    /// the invoked processor's business.
    public var src: String

    /// The `<param>` values by name; a repeated name keeps every value in document
    /// order.
    public var params: [String: [String]]

    /// The namelist and `<param>` pairs as JSON, typed as the data model holds them:
    /// the text a local `<send>` with the same pairs would carry in `_event.data`.
    /// Empty when there are none. Computed once by the generated invoke site, so a
    /// host that forwards the request forwards the engine's serialisation and not a
    /// second one made from `params`, which are text and have lost their types.
    public var eventData: String

    /// Inline `<content>`, empty when the document carried none.
    public var content: String

    /// The session that started the invocation (`_sessionid`). The generated invoke
    /// site fills it; the engine holds no session of its own.
    public var sessionID: String

    /// Says this start is a restore starting the invocation again.
    public var restarted: Bool

    /// Which start of this invoke this is. The engine assigns it, and a host that
    /// finishes later hands it back to `completeHostInvoke`.
    ///
    /// The id alone cannot say it: a state that exits and is entered again starts the
    /// same `<invoke>` a second time under the same id, and a result the first run
    /// produces after it was cancelled would otherwise read as the second run's
    /// (§scxml-6.4). Distinct for every start of every invocation within one engine.
    public var token: UInt64

    public init(
        processorType: String,
        invokeID: String,
        src: String = "",
        params: [String: [String]] = [:],
        eventData: String = "",
        content: String = "",
        sessionID: String = "",
        restarted: Bool = false,
        token: UInt64 = 0
    ) {
        self.processorType = processorType
        self.invokeID = invokeID
        self.src = src
        self.params = params
        self.eventData = eventData
        self.content = content
        self.sessionID = sessionID
        self.restarted = restarted
        self.token = token
    }
}

/// An `<invoke>` the host was running, at the point its state exited.
public struct HostInvokeCancel: Equatable {
    /// The `type` the `<invoke>` named.
    public var processorType: String

    /// The invocation being cancelled: the id its start carried.
    public var invokeID: String

    /// The token its start carried, so a host running more than one start of the same
    /// id stops the right one.
    public var token: UInt64

    public init(processorType: String, invokeID: String, token: UInt64) {
        self.processorType = processorType
        self.invokeID = invokeID
        self.token = token
    }
}

/// One turn of a host-run invoke's lifecycle.
///
/// Both arms go to ONE registered handler and not to two callbacks, because a host
/// that can start an invocation and cannot stop it is not a working invoker, and two
/// registrations would make that state reachable.
public enum HostInvokeEvent: Equatable {
    /// The state was entered and the macrostep has settled: begin the invoked process.
    case start(HostInvokeRequest)

    /// The state exited: stop it. Delivered only for an invocation still running; one
    /// that never started has nothing to tear down, and one that already completed has
    /// nothing left to stop.
    case cancel(HostInvokeCancel)
}

/// A host invoker's answer to a start. Read only for a start; an answer to a cancel is
/// ignored, because there is nothing left for it to mean.
public struct HostInvokeResponse: Equatable {
    /// The payload of an immediate `done.invoke.<invokeID>`, for an invocation that
    /// completed before returning. Nil is the ordinary case: the work outlives the call
    /// and the host reports it through `completeHostInvoke` with the request's token.
    /// The engine never synthesises a completion the host did not report.
    public var doneData: String?

    /// Says the host could not start the invocation at all: `_event.data` of the
    /// `error.execution` the engine raises instead (§scxml-6.4.1: an invocation that
    /// cannot be started is an error of the element, not an `error.invoke` of a process
    /// that ran). A refused invocation never started: no cancel follows, and a
    /// completion reported for it later is refused. It overrides `doneData`, since
    /// nothing ran to complete.
    public var refusal: String?

    public init(doneData: String? = nil, refusal: String? = nil) {
        self.doneData = doneData
        self.refusal = refusal
    }

    /// The invocation completed before the start returned.
    public static func done(_ data: String) -> HostInvokeResponse {
        HostInvokeResponse(doneData: data)
    }

    /// The host could not start the invocation.
    public static func refused(_ reason: String) -> HostInvokeResponse {
        HostInvokeResponse(refusal: reason)
    }
}

/// A registered invoke-lifecycle handler. Nil is the answer of a start that is still
/// running.
public typealias HostInvokeHandler = (HostInvokeEvent) -> HostInvokeResponse?

/// Names a run: the `type` and the id of the `<invoke>`.
struct HostInvokeKey: Hashable {
    var processorType: String
    var invokeID: String
}

/// What an engine holds of its host-run invocations: the handlers, the runs still going
/// and the next token. Internal, held by `Engine.hostInvokes`.
struct HostInvokeBook {
    var handlers: [String: HostInvokeHandler] = [:]

    /// The token of the start still running under each key.
    var started: [HostInvokeKey: UInt64] = [:]

    var nextToken: UInt64 = 1
}

extension Engine {
    /// Registers what runs every `<invoke type="<processorType>">` this machine executes
    /// (§scxml-6.4.1). Registering twice for one type replaces the handler.
    public func registerInvoker(_ processorType: String, handler: @escaping HostInvokeHandler) {
        hostInvokes.handlers[processorType] = handler
    }

    /// Whether an invoker is registered for `processorType`.
    public func hasInvoker(_ processorType: String) -> Bool {
        hostInvokes.handlers[processorType] != nil
    }

    /// Starts a host-run invocation and reports whether anything ran (§scxml-6.4.1).
    ///
    /// False means no invoker was registered, which the generated site turns into
    /// `error.execution`: an invoke nobody ran is the same fact whether the type was
    /// undeclared or the handler was never wired up.
    ///
    /// The start is recorded here and not in the generated machine, because "is this one
    /// still running?" is the question both the cancel and a completion have to answer.
    /// The token is assigned and recorded BEFORE the handler runs, so a handler that
    /// completes at once completes an invocation the engine already knows is running.
    @discardableResult
    public func performHostInvoke(_ request: HostInvokeRequest) -> Bool {
        guard let handler = hostInvokes.handlers[request.processorType] else {
            return false
        }
        var request = request
        let token = hostInvokes.nextToken
        hostInvokes.nextToken += 1
        request.token = token
        let key = HostInvokeKey(processorType: request.processorType, invokeID: request.invokeID)
        hostInvokes.started[key] = token

        let response = handler(.start(request))
        if let refusal = response?.refusal {
            // §scxml-6.4.1: the host could not start it, so it never started: taken back
            // out of the running set, and the element's error raised with what the host
            // said.
            hostInvokes.started[key] = nil
            if let errorEvent = policy.getEventFromName("error.execution") {
                raise(.platformError(errorEvent, message: refusal))
            }
            return true
        }
        if let doneData = response?.doneData {
            // §scxml-6.4: a completion the host reported NOW takes the same door as one
            // it reports later, so the two cannot disagree about whether the invocation
            // is over.
            completeHostInvoke(
                processorType: request.processorType,
                invokeID: request.invokeID,
                token: token,
                doneData: doneData
            )
        }
        return true
    }

    /// Reports that a host-run invocation finished, raising its `done.invoke.<invokeID>`
    /// with `doneData` as `_event.data` (§scxml-6.4).
    ///
    /// `token` is the one the invocation's start carried. The completion is accepted
    /// only while that start is still running, and accepting it ends the invocation, so
    /// it is accepted at most once and the state's exit no longer cancels it. False,
    /// raising nothing, for an invocation that was cancelled, already ended, or
    /// restarted since: §scxml-6.4 has the processor ignore what a cancelled process
    /// sends, and a restarted `<invoke>` is a different process under the same id.
    @discardableResult
    public func completeHostInvoke(
        processorType: String,
        invokeID: String,
        token: UInt64,
        doneData: String,
        origin: String = "",
        originType: String = ""
    ) -> Bool {
        endHostInvoke(
            processorType: processorType,
            invokeID: invokeID,
            token: token,
            failed: false,
            data: doneData,
            origin: origin,
            originType: originType
        )
    }

    /// Reports that a host-run invocation failed, raising its `error.invoke.<invokeID>`
    /// with `errorData` as `_event.data` (§scxml-6.4). The same door and the same rules
    /// as `completeHostInvoke`, so a run ends once: by success, failure or deadline,
    /// whichever comes first.
    @discardableResult
    public func failHostInvoke(
        processorType: String,
        invokeID: String,
        token: UInt64,
        errorData: String,
        origin: String = "",
        originType: String = ""
    ) -> Bool {
        endHostInvoke(
            processorType: processorType,
            invokeID: invokeID,
            token: token,
            failed: true,
            data: errorData,
            origin: origin,
            originType: originType
        )
    }

    /// The one body behind a host-run invocation's completion and failure: the token
    /// check and the event.
    private func endHostInvoke(
        processorType: String,
        invokeID: String,
        token: UInt64,
        failed: Bool,
        data: String,
        origin: String,
        originType: String
    ) -> Bool {
        let key = HostInvokeKey(processorType: processorType, invokeID: invokeID)
        guard hostInvokes.started[key] == token else {
            return false
        }
        hostInvokes.started[key] = nil
        // The id is the DOCUMENT's, because `done.invoke.<id>` and `error.invoke.<id>`
        // are the names the author wrote a transition for.
        let name = failed ? errorInvokePrefix + invokeID : createDoneInvokeEventName(invokeID)
        var metadata = EventMetadata.external(sendID: "", origin: origin)
        if !originType.isEmpty {
            metadata.originType = originType
        }
        metadata.data = data
        // §scxml-5.10.1: the completion is an event of the invocation, so
        // `_event.invokeid` is the invocation's id.
        metadata.invokeID = invokeID
        metadata.hostInvokeToken = token
        if let arriving = arrivingEvent(name: name, metadata: metadata) {
            raiseExternalWithMeta(arriving)
        }
        return true
    }

    /// Stops a host-run invocation, if it is still running (§scxml-6.4).
    ///
    /// Unconditional at the call site: the engine knows whether this one is still
    /// running and stays silent when it never started or already ended, so the emitted
    /// exit chain needs no bookkeeping of its own. Returns whether the host was told.
    @discardableResult
    public func cancelHostInvoke(processorType: String, invokeID: String) -> Bool {
        let key = HostInvokeKey(processorType: processorType, invokeID: invokeID)
        guard let token = hostInvokes.started[key] else {
            return false
        }
        hostInvokes.started[key] = nil
        guard let handler = hostInvokes.handlers[processorType] else {
            return false
        }
        _ = handler(
            .cancel(HostInvokeCancel(processorType: processorType, invokeID: invokeID, token: token))
        )
        return true
    }
}
