// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Saving a `datamodel="sce-static"` machine and restoring one (SCE Accepted Subset, "Saving and restoring";
// docs/adr/0045).
//
// What a machine is saved as is the document of `SavedState.swift`; this file is how a machine becomes one and
// is made again from one. The engine knows where a machine stands (its configuration, its current leaf, the
// events waiting on its external queue, the ids it has generated), and the machine's own policy, which the
// generator writes, knows its variables and the digest of its document's shape. A policy that can be saved
// conforms to `SavedPolicy`, and its engine then saves and restores it.
//
// A restore is all or nothing. Every judgment a restore makes (the shape, the configuration, each variable's
// type and bound, each event) comes before the first thing it changes, so a state that is refused leaves the
// machine as it was, which a host has no other way to tell.
//
// What this runtime does not save yet is refused by name, on the machine that holds it and not only when a
// state that names it is read: a machine whose document has a `<history>` or an `<invoke>`, or one of a kind of
// variable it does not hold yet, is generated without the save API, and a machine with a delayed `<send>`
// waiting is refused when it is saved.
//
// Cross-language siblings: `com.sce.runtime.StateMachineEngine` (Kotlin) and `sce_rust_runtime::saved_state`
// (Rust).

/// What a machine's saved state is bound to: the digests of the document's shape. The generator computes them,
/// and every saved state is written and judged against them.
public struct SavedShape: Sendable, Equatable {
    /// The shape a state names when every schema it holds is in its first revision: the states, histories,
    /// invocations and variables of the document, each with its kind and type.
    public let first: String

    /// The shape a state names when it names a later revision of any schema. A reader that predates revisions
    /// knows only the first shape, so it refuses such a state rather than read it without the fields it does not
    /// know. The same as `first` for a document that holds no schema.
    public let revised: String

    /// `first` as a format-2 state names it (docs/adr/0045, section 2): each enum-typed variable and field
    /// spelled by its enum's document rather than by the alias it is imported under. The same as `first` for a
    /// document that holds no enum.
    public let format2First: String

    /// `revised` as a format-2 state names it.
    public let format2Revised: String

    /// The configuration of a product line the machine was generated for (docs/adr/0057, section 7): the
    /// features it turns on, sorted, each once; nil for a machine of no product line.
    public let features: [String]?

    public init(
        first: String, revised: String, format2First: String, format2Revised: String, features: [String]? = nil
    ) {
        self.first = first
        self.revised = revised
        self.format2First = format2First
        self.format2Revised = format2Revised
        self.features = features
    }
}

/// A policy whose machine can be saved and restored. The generator writes the conformance for a machine whose
/// state the runtime holds whole.
public protocol SavedPolicy: InvocablePolicy {
    /// What a saved state of this machine is bound to.
    static var savedShape: SavedShape { get }

    /// Every variable, by document id, in declaration order, each as a saved state holds it.
    func savedVariables() -> [SavedEntry<JSONValue>]

    /// Reads and judges every variable `saved` holds, and changes none: the type each is held to, the bound of a
    /// text or a list. What it returns writes the values it read into the machine and cannot fail, so the
    /// restore applies it only once every other judgment has passed.
    func readSavedVariables(from saved: SavedState) throws -> () -> Void
}

extension SavedState {
    /// Refuses `saved` unless it was saved from a document of this `shape`: judged before any variable is read,
    /// so a state saved from another document is refused for that and not for whichever variable it happens to
    /// lack.
    public static func checkShape(_ saved: SavedState, _ shape: String) throws {
        if saved.shape != shape {
            throw StateRefusal(
                "the saved state is of a document of shape \(saved.shape), and this machine's is \(shape)")
        }
    }

    /// Refuses `saved` unless it was taken in the configuration `features` of a product line, or both are of
    /// none (docs/adr/0057, section 7): judged before the shape, which differs too between builds that differ in
    /// states or variables, so a state of another build is refused by the features that differ and not by a
    /// digest. A build with fewer features lacks states or data the state may hold, and one with more has data
    /// and invocations the saved run never initialised or started: either is refused. Carrying a state across
    /// configurations is a migration, which a restore does not do.
    public static func checkFeatures(_ saved: SavedState, _ features: [String]?) throws {
        func list(_ names: [String]) -> String { "[" + names.joined(separator: ", ") + "]" }
        guard let taken = saved.features else {
            if let features {
                throw StateRefusal(
                    "the saved state records no features, and this machine is built with the features \(list(features)) on"
                )
            }
            return
        }
        guard let features else {
            throw StateRefusal(
                "the saved state was taken with the features \(list(taken)) on, and this machine is built from a document of no feature model"
            )
        }
        if taken == features { return }
        let onlySaved = taken.filter { !features.contains($0) }
        let onlyBuilt = features.filter { !taken.contains($0) }
        var differ: [String] = []
        if !onlySaved.isEmpty { differ.append("\(list(onlySaved)) on only where it was saved") }
        if !onlyBuilt.isEmpty { differ.append("\(list(onlyBuilt)) on only in this build") }
        throw StateRefusal(
            "the saved state was taken with the features \(list(taken)) on, and this machine is built with \(list(features)): \(differ.joined(separator: ", "))"
        )
    }

    /// Refuses `saved` unless it is a state of the machine whose saved state is bound to `shape`, judged before
    /// any variable is read, as `checkShape` is: the configuration it was taken in first, then the shape, then
    /// the revisions it names. A machine of this runtime holds no schema that has a revision yet, so a state
    /// that names one is refused for naming it.
    public static func judge(_ saved: SavedState, against shape: SavedShape) throws {
        try checkFeatures(saved, shape.features)
        let first = saved.formatVersion == format2 ? shape.format2First : shape.first
        let revised = saved.formatVersion == format2 ? shape.format2Revised : shape.revised
        if saved.revisions.isEmpty {
            try checkShape(saved, first)
        } else if saved.shape == first {
            throw StateRefusal(
                "the saved state names revisions of its schemas and has the shape of a state that names none")
        } else {
            try checkShape(saved, revised)
        }
        if let named = saved.revisions.first {
            throw StateRefusal("'revisions' names '\(named.key)', which no record or enum of this machine is")
        }
    }
}

private func savedEvent<Event>(_ queued: EventWithMetadata<Event>, name: String) -> SavedEvent {
    let metadata = queued.metadata
    return SavedEvent(
        name: name, data: metadata.data, type: metadata.eventType.description, sendID: metadata.sendID,
        origin: metadata.origin, originType: metadata.originType, invokeID: metadata.invokeID,
        hostInvokeToken: metadata.hostInvokeToken.map { Int64(bitPattern: $0) })
}

extension Engine where Policy: SavedPolicy {
    /// The machine's whole state, at a macrostep boundary, as the document every backend reads
    /// (`SavedState.toJSON`). A host whose process can be killed saves the machine and gives it back later.
    ///
    /// Refused, by name, for a machine that is not running, one that has ended at a top-level `<final>`, one
    /// whose internal queue holds events (it is not at a macrostep boundary), and one with a delayed `<send>`
    /// waiting, which this runtime does not save yet.
    public func save() throws -> SavedState {
        // A machine that has ended is not running either, so the reason that says more is asked first.
        if terminalState != nil {
            throw StateRefusal("the machine has ended at a top-level <final>, so there is nothing to resume")
        }
        guard isRunning else {
            throw StateRefusal("the machine is not running, so there is nothing to save: start it first")
        }
        if internalQueueHoldsEvents {
            throw StateRefusal(
                "the machine is not at a macrostep boundary: its internal queue holds events a save would lose")
        }
        if holdsScheduledSends {
            throw StateRefusal("a delayed <send> is waiting, which this runtime does not save yet")
        }
        // Document order, which is also entry order: the same text from every run that stands in the same place.
        let active = getActiveStates().sorted { policy.getDocumentOrder($0) < policy.getDocumentOrder($1) }
        let external = savedExternalEvents().map { queued in
            savedEvent(queued, name: policy.getEventName(queued.event))
        }
        return SavedState(
            shape: Policy.savedShape.format2First, configuration: active.map { policy.getStateName($0) },
            current: policy.getStateName(getCurrentState()), variables: policy.savedVariables(),
            invokes: .resumed([]), sessionID: policy.sessionID, autoSendSeq: Int64(generatedSendIDCount),
            external: external, features: Policy.savedShape.features)
    }

    /// Stands this machine where `saved` left one, in place of `initialize`: no `<onentry>` runs and no `<data>`
    /// is evaluated, since the saved run did both. A refused restore leaves the machine as it was.
    ///
    /// Refused, by name, for a machine that has started, a state of another document, of another product-line
    /// configuration, or one that names a state, an event or a variable this machine does not have; for a
    /// value its variable's type or bound cannot hold; and for what this runtime does not restore yet: a
    /// running child session, a recorded history, a waiting delayed `<send>`, a host-run invocation.
    public func restore(_ saved: SavedState) throws {
        guard !isRunning else {
            throw StateRefusal("the machine has started: a saved state is restored into one that has not")
        }
        try SavedState.judge(saved, against: Policy.savedShape)
        if let child = saved.invokes.ids.first {
            throw StateRefusal(
                "the saved state holds a running child session '\(child)', which this machine does not start")
        }
        if let recorded = saved.history.first {
            throw StateRefusal(
                "the saved state records the history '\(recorded.key)', which this machine does not have")
        }
        if !saved.pending.isEmpty {
            throw StateRefusal("the saved state holds a waiting delayed <send>, which this runtime does not restore yet")
        }
        if let hosted = saved.hostInvokes.first {
            throw StateRefusal(
                "the saved state holds the host-run invocation '\(hosted.invokeID)', which this machine does not start")
        }
        func state(_ name: String, _ what: String) throws -> Policy.State {
            guard let found = policy.getStateFromName(name) else {
                throw StateRefusal("the saved \(what) names '\(name)', which is no state of this document")
            }
            return found
        }
        let configuration = try saved.configuration.map { try state($0, "configuration") }
        let current = try state(saved.current, "current state")
        let external = try saved.external.map { event -> EventWithMetadata<Policy.Event> in
            guard let resolved = policy.getEventFromName(event.name) else {
                throw StateRefusal("the saved external queue names '\(event.name)', which is no event of this document")
            }
            let kind: EventType
            switch event.type {
            case "external": kind = .externalEvent
            case "internal": kind = .internalEvent
            case "platform": kind = .platformEvent
            default:
                throw StateRefusal("the saved event '\(event.name)' is of the type '\(event.type)', which is no event type")
            }
            return EventWithMetadata(
                resolved,
                metadata: EventMetadata(
                    name: event.name, data: event.data, eventType: kind, sendID: event.sendID, origin: event.origin,
                    originType: event.originType, invokeID: event.invokeID,
                    hostInvokeToken: event.hostInvokeToken.map { UInt64(bitPattern: $0) }))
        }
        let apply = try policy.readSavedVariables(from: saved)
        guard saved.autoSendSeq >= 0 else {
            throw StateRefusal("the saved count of generated send ids is negative")
        }
        // The one judgment left is the engine's own: it validates the configuration before it changes anything.
        let verdict = enterAt(configuration: configuration, current: current)
        guard verdict == .accepted else {
            throw StateRefusal("the saved configuration is not one of this document: \(verdict)")
        }
        // Past every judgment: what follows cannot fail.
        if let bound = saved.boundSessionID {
            policy.sessionID = bound
        }
        apply()
        restoreSavedEngineParts(external: external, generatedSendIDCount: UInt64(saved.autoSendSeq))
    }
}
