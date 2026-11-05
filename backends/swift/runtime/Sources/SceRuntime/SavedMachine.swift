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
// A delayed `<send>` still waiting is part of the state, and is saved as the wall-clock moment it comes due, not
// as a wait: the host that saves and the one that restores read the same wall clock, and a timer that ran out
// while the process was dead is due as soon as the machine is back. The wall clock is a parameter of `save` and
// of `restore`, so a test or a replay owns it as it owns the engine's own clock.
//
// What this runtime does not save yet is refused by name, on the machine that holds it and not only when a
// state that names it is read: a machine whose document has an `<invoke>`, or one of a kind of variable it does
// not hold yet, is generated without the save API.
//
// Cross-language siblings: `com.sce.runtime.StateMachineEngine` (Kotlin) and `sce_rust_runtime::saved_state`
// (Rust).

import Foundation

/// The wall clock's reading, in milliseconds since the Unix epoch: what `Engine.save` and `Engine.restore` read
/// the moment a delayed `<send>` comes due against, unless the host gives them its own reading.
public func unixWallClockMs() -> Int64 {
    Int64((Date().timeIntervalSince1970 * 1000).rounded(.down))
}

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

    /// Each record schema a variable is of, by the event it declares, ordered by it: the revision the machine
    /// holds it at, what each revision declared, and the variables that hold a record of it. Empty for a document
    /// that holds none.
    public let records: [SavedRecordLineage]

    /// Each enum a value of the state is of, by its key, ordered by it. Empty for a document that holds none.
    public let enums: [SavedEnumLineage]

    /// The configuration of a product line the machine was generated for (docs/adr/0057, section 7): the
    /// features it turns on, sorted, each once; nil for a machine of no product line.
    public let features: [String]?

    public init(
        first: String, revised: String, format2First: String, format2Revised: String,
        records: [SavedRecordLineage] = [], enums: [SavedEnumLineage] = [], features: [String]? = nil
    ) {
        self.first = first
        self.revised = revised
        self.format2First = format2First
        self.format2Revised = format2Revised
        self.records = records
        self.enums = enums
        self.features = features
    }
}

/// A `<history>` of a document, as a saved state names it: by the id the document gives it, which the state
/// keys what the history recorded by.
public struct SavedHistory: Sendable, Equatable {
    /// The id the document gives the `<history>`.
    public let id: String

    /// The number the policy issued for it.
    public let history: HistoryID

    /// Whether it is a deep history, which records the atomic descendants and not the children.
    public let deep: Bool

    public init(id: String, history: HistoryID, deep: Bool) {
        self.id = id
        self.history = history
        self.deep = deep
    }
}

/// A policy whose machine can be saved and restored. The generator writes the conformance for a machine whose
/// state the runtime holds whole.
public protocol SavedPolicy: InvocablePolicy {
    /// What a saved state of this machine is bound to.
    static var savedShape: SavedShape { get }

    /// The document's `<history>`s, ordered by id: the order a saved state keys them in. None for a document
    /// that declares none.
    static var savedHistories: [SavedHistory] { get }

    /// Gives a `<history>` the states a saved run recorded for it, which the engine has judged could have been
    /// recorded. Called only for a history `savedHistories` names.
    func restoreHistory(_ history: HistoryID, to states: [State])

    /// Every variable, by document id, in declaration order, each as a saved state holds it.
    func savedVariables() -> [SavedEntry<JSONValue>]

    /// Reads and judges every variable `saved` holds, and changes none: the type each is held to, the bound of a
    /// text or a list. What it returns writes the values it read into the machine and cannot fail, so the
    /// restore applies it only once every other judgment has passed.
    func readSavedVariables(from saved: SavedState) throws -> () -> Void

    /// The id the document gives each of its `<invoke type="scxml">`s, in document order: what a saved state names
    /// a running child, a count of starts and a send waiting for a child by. None for a document that invokes none.
    var savedInvokeIDs: [String] { get }

    /// Every `<invoke>` whose child session is running, in document order, each with the child's whole state
    /// (docs/adr/0045). A child that has ended is not running, though its entry may stay until its state exits.
    func savedChildren(wallNowMs: Int64) throws -> [SavedChild]

    /// How many times each `<invoke>` has started, which numbers the next start's id: by the id the document gives
    /// the `<invoke>`, in document order, those that have started at least once.
    func savedInvokeStarts() -> [SavedEntry<Int64>]

    /// Judges the running children `saved` lists and the counts of starts it holds, and changes none: each child is
    /// one the document starts and its session is of the shape of the document it runs, down to the last
    /// grandchild. What it returns puts the children back, once the machine itself stands where it was saved: each
    /// resumed where it stood in a format-2 state, each started again from its beginning in a format-1 one.
    func readSavedChildren(from saved: SavedState, engine: Engine<Self>, wallNowMs: Int64) throws -> () -> Void
}

extension SavedPolicy {
    /// A document that declares no `<history>` has none to name.
    public static var savedHistories: [SavedHistory] { [] }

    public func restoreHistory(_ history: HistoryID, to states: [State]) {
        preconditionFailure("this document declares no <history>, so a saved state has none to restore")
    }

    public var savedInvokeIDs: [String] { [] }

    public func savedChildren(wallNowMs: Int64) throws -> [SavedChild] { [] }

    public func savedInvokeStarts() -> [SavedEntry<Int64>] { [] }

    /// A document that invokes nothing holds no child, and a state that names one is not of this machine.
    public func readSavedChildren(from saved: SavedState, engine: Engine<Self>, wallNowMs: Int64) throws -> () -> Void {
        if let child = saved.invokes.ids.first {
            throw StateRefusal(
                "the saved state holds a running child session '\(child)', which this machine does not start")
        }
        if let counted = saved.invokeStarts.first {
            throw StateRefusal("invokestarts counts '\(counted.key)', which the document does not invoke")
        }
        return {}
    }
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
    /// waiting that the format cannot carry. `wallNowMs` is what time it is on the wall clock, which each
    /// waiting send is written against.
    public func save(wallNowMs: Int64 = unixWallClockMs()) throws -> SavedState {
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
        let pending = try savedPendingSends(wallNowMs: wallNowMs)
        let children = try policy.savedChildren(wallNowMs: wallNowMs)
        // Document order, which is also entry order: the same text from every run that stands in the same place.
        let active = getActiveStates().sorted { policy.getDocumentOrder($0) < policy.getDocumentOrder($1) }
        // Saved under the name the event arrived under, so a restore gives the machine the same `_event.name`:
        // the document's own event for it, or the one it extends.
        let external = savedExternalEvents().map { queued in
            let arrived = queued.metadata.name
            return savedEvent(queued, name: arrived.isEmpty ? policy.getEventName(queued.event) : arrived)
        }
        // State ids in document order, keyed by the history's id and ordered by it, so the text one machine
        // writes is every backend's. A history that has recorded nothing is absent: a resumed machine takes its
        // default transition, as the saved one would have.
        let history = Policy.savedHistories.compactMap { declared -> SavedEntry<[String]>? in
            guard let recorded = policy.historyValue(declared.history) else { return nil }
            let ordered = recorded.sorted { policy.getDocumentOrder($0) < policy.getDocumentOrder($1) }
            return SavedEntry(declared.id, ordered.map { policy.getStateName($0) })
        }
        // A schema past its first revision is named with the digest of what that revision declares, and the state
        // is of the revised shape; one whose schemas are all in their first is the text it was before schemas had
        // revisions.
        let shape = Policy.savedShape
        let revisions = shape.currentRevisions()
        return SavedState(
            shape: revisions.isEmpty ? shape.format2First : shape.format2Revised,
            configuration: active.map { policy.getStateName($0) },
            current: policy.getStateName(getCurrentState()), variables: policy.savedVariables(), history: history,
            pending: pending, invokes: .resumed(children), sessionID: policy.sessionID,
            invokeStarts: policy.savedInvokeStarts(), autoSendSeq: Int64(generatedSendIDCount),
            external: external, revisions: revisions, features: shape.features)
    }

    /// Stands this machine where `saved` left one, in place of `initialize`: no `<onentry>` runs and no `<data>`
    /// is evaluated, since the saved run did both. A refused restore leaves the machine as it was.
    ///
    /// Refused, by name, for a machine that has started, a state of another document, of another product-line
    /// configuration, or one that names a state, an event or a variable this machine does not have; for a
    /// value its variable's type or bound cannot hold; and for what this runtime does not restore yet: a
    /// running child session, a host-run invocation, a send to another session of the tree.
    ///
    /// The delayed sends `saved` holds are armed against this machine's clock, which is installed before the
    /// restore as it is before `initialize`: each comes due `due - wallNowMs` after now, `wallNowMs` being what
    /// time it is on the wall clock the saved `due`s were written against, and one already due comes due now,
    /// behind the ones due before it.
    public func restore(_ saved: SavedState, wallNowMs: Int64 = unixWallClockMs()) throws {
        try prepareRestore(saved, wallNowMs: wallNowMs)()
    }

    /// Judges `saved` against this machine, every judgment a restore makes, and changes nothing; what it returns
    /// stands the machine where `saved` left one and cannot fail. `restore` is the two in a row. A parent judges
    /// the sessions it resumes through this before it changes anything, so one child's refusal leaves the whole
    /// tree as it was (docs/adr/0045, section 3).
    public func prepareRestore(_ saved: SavedState, wallNowMs: Int64) throws -> () -> Void {
        guard !isRunning else {
            throw StateRefusal("the machine has started: a saved state is restored into one that has not")
        }
        // The state as one of this machine: each record saved at an earlier revision of its schema holds the
        // defaults of the fields added since.
        let saved = try SavedState.upgrade(saved, against: Policy.savedShape)
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
        // What each `<history>` recorded, read as states of this document: a history the document does not
        // declare, and a value it could not have recorded (a state outside its parent, a compound state with two
        // active children) are refused, since a resumed machine entered through a history must land in a
        // configuration its document could have been in.
        var histories: [(history: HistoryID, states: [Policy.State])] = []
        for entry in saved.history {
            guard let declared = Policy.savedHistories.first(where: { $0.id == entry.key }) else {
                throw StateRefusal(
                    "the saved state records the history '\(entry.key)', which the document does not declare")
            }
            let states = try entry.value.map { name -> Policy.State in
                guard let found = policy.getStateFromName(name) else {
                    throw StateRefusal("the history '\(entry.key)' records '\(name)', which the document does not name")
                }
                return found
            }
            let verdict = validateHistory(
                policy: policy, parent: policy.getHistoryParent(declared.history), deep: declared.deep,
                recorded: states)
            guard verdict.kind == .accepted else {
                throw StateRefusal(
                    "the history '\(entry.key)' records states it could not have: \(historyReason(verdict, policy))")
            }
            histories.append((history: declared.history, states: states))
        }
        let external = try saved.external.map { event -> EventWithMetadata<Policy.Event> in
            guard let resolved = resolveEventByName(event.name) else {
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
        let waiting = try judgeSavedSends(saved.pending, invokeIDs: policy.savedInvokeIDs)
        let apply = try policy.readSavedVariables(from: saved)
        guard saved.autoSendSeq >= 0 else {
            throw StateRefusal("the saved count of generated send ids is negative")
        }
        let verdict = validateConfiguration(policy: policy, configuration: configuration, current: current)
        guard verdict == .accepted else {
            throw StateRefusal("the saved configuration is not one of this document: \(verdict)")
        }
        // The children last: each is judged down to its last grandchild, and none is built into the tree before
        // every judgment of this machine has passed.
        let resumeChildren = try policy.readSavedChildren(from: saved, engine: self, wallNowMs: wallNowMs)
        // Past every judgment: what follows cannot fail.
        return { [self] in
            let entered = enterAt(configuration: configuration, current: current)
            precondition(entered == .accepted, "the restore judged this configuration and enterAt refused it")
            if let bound = saved.boundSessionID {
                policy.sessionID = bound
            }
            apply()
            for recorded in histories {
                policy.restoreHistory(recorded.history, to: recorded.states)
            }
            armSavedSends(waiting, wallNowMs: wallNowMs)
            restoreSavedEngineParts(external: external, generatedSendIDCount: UInt64(saved.autoSendSeq))
            resumeChildren()
        }
    }
}

/// Why a history could not have recorded the states a saved state gives it, in words a refusal can carry.
private func historyReason<Policy: StatePolicy>(_ verdict: HistoryRejection<Policy.State>, _ policy: Policy) -> String {
    func name(_ state: Policy.State?) -> String { state.map { policy.getStateName($0) } ?? "a state" }
    switch verdict.kind {
    case .accepted: return "none"
    case .empty: return "it records no state"
    case .duplicate: return "it records \(name(verdict.state)) twice"
    case .notBelow: return "\(name(verdict.state)) is not below the history's parent \(name(verdict.parent))"
    case .notAChild:
        return "\(name(verdict.state)) is not a child of \(name(verdict.parent)), which a shallow history records"
    case .notAtomic: return "\(name(verdict.state)) is not atomic, which a deep history records"
    case .arity: return "they are not a configuration of the parent's subtree (\(verdict.arity))"
    }
}

/// A child session held by what a parent drives it through (`ChildEngine`) that can be saved: the slot of a
/// hybrid `<invoke>` holds whichever candidate its `srcexpr` named, so the policy type the session runs is not
/// known where the parent saves it.
public protocol SavedChildEngine: ChildEngine {
    /// The session's whole state, as `Engine.save` writes it.
    func saveSession(wallNowMs: Int64) throws -> SavedState
}

extension InvokedSession: SavedChildEngine where Policy: SavedPolicy {
    public func saveSession(wallNowMs: Int64) throws -> SavedState {
        try engine.save(wallNowMs: wallNowMs)
    }
}

extension InvokedSession where Policy: SavedPolicy {
    /// Judges `saved`, the state of this session as its parent's saved state holds it as the child at `index`,
    /// down to its last grandchild, and changes nothing; what it returns resumes the session. A refusal names the
    /// child by its path from the parent (`invokes[0].session`).
    public func prepareSavedChild(_ saved: SavedState, index: Int, wallNowMs: Int64) throws -> () -> Void {
        do {
            return try engine.prepareRestore(saved, wallNowMs: wallNowMs)
        } catch let refusal as StateRefusal {
            throw refusal.within("invokes[\(index)].session")
        }
    }
}
