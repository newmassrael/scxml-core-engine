// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// A `datamodel="sce-static"` machine's whole state, saved at a macrostep boundary and restored into a new
// process in place of `initialize` (SCE Accepted Subset, "Saving and restoring"; docs/adr/0045).
//
// A host whose process can be killed saves the machine and gives it back later. What it saves is everything a
// macrostep boundary holds that the document cannot recompute: where the machine is (its configuration and
// current leaf), every variable, what each `<history>` recorded, the delayed `<send>`s still waiting, the
// running child sessions, the host-run invocations, the counters that keep a restored machine from reusing an
// id, and the external queue in order.
//
// The format is one JSON document (`sce-saved-state`), the same on every backend, so what one backend saved
// another can read. Format 1 names a running child by its id, which a restore starts again from its
// beginning. Format 2 saves a tree of sessions: each running child is saved inside its parent with its own
// state, which a restore resumes where it stood. Format 3 is format 2 with an event's origin held as the one
// pair the processor that delivered it wrote, and the sending session's id beside it (docs/adr/0090, section
// 7). All three are read. A tree is written as format 3, whichever format it was read from, and a state that
// names its running children by id alone is written as format 1.
//
// This file is the document: its model, the reading of its text and the writing of it. Whether a state is
// one a machine may be restored from (its shape, its features, its revisions) is the machine's to judge, and
// how a machine saves and restores is the generated code's and the engine's.
//
// Cross-language siblings: `com.sce.runtime.SavedState` (Kotlin) and `sce_rust_runtime::saved_state` (Rust).

/// Why a saved state cannot be restored, or a machine cannot be saved.
public struct StateRefusal: Error, Equatable, Sendable, CustomStringConvertible {
    public let message: String

    public init(_ message: String) {
        self.message = message
    }

    public var description: String { message }

    /// What a refusal inside a child session of a tree begins with, before the session's path.
    private static let sessionAt = "the session at "

    /// This refusal, of the child session at `path` of the session that reads it (`invokes[0].session`): a
    /// tree is refused as a whole, and the reason names the session it is about by its path from the root
    /// (docs/adr/0045, section 2), `invokes[0].session.invokes[1].session` for a grandchild.
    public func within(_ path: String) -> StateRefusal {
        if message.hasPrefix(Self.sessionAt) {
            return StateRefusal("\(Self.sessionAt)\(path).\(message.dropFirst(Self.sessionAt.count))")
        }
        return StateRefusal("\(Self.sessionAt)\(path): \(message)")
    }
}

/// One member of an object a saved state writes in an order of its own: the variables in declaration order, a
/// history by its id, a count of starts by the invoke's id. An array and not a dictionary, since the text of a
/// state is the same on every backend only if its members are written in the same order.
public struct SavedEntry<Value: Equatable & Sendable>: Equatable, Sendable {
    public let key: String
    public let value: Value

    public init(_ key: String, _ value: Value) {
        self.key = key
        self.value = value
    }
}

/// The revision of one record schema (or enum) a state was saved at.
public struct SavedRevision: Equatable, Sendable {
    /// The schema's `sce:revision` where the state was saved, 2 or later.
    public let revision: Int

    /// The SHA-256, as lowercase hex, of what that revision declares: each field it has, with its type and its
    /// enum's variant names. A restoring machine holds the same digest of each of its own schema's revisions,
    /// so two builds that disagree about what one revision declares are told apart.
    public let fields: String

    public init(revision: Int, fields: String) {
        self.revision = revision
        self.fields = fields
    }
}

/// An event of a saved external queue.
public struct SavedEvent: Equatable, Sendable {
    public var name: String
    public var data: String
    public var type: String
    public var sendID: String

    /// The pair the processor that delivered the event wrote (docs/adr/0090, section 1), nil for an event no
    /// processor stamped.
    public var origin: EventOrigin?

    /// The id of the session that sent the event, written by the processor that delivered it from that session
    /// (the SCXML Event I/O Processor for a local session, the Mesh transport for a remote child or parent); nil
    /// for every other sender. Independent of `origin`'s address, which is the reply address that processor
    /// wrote, but never beside a nil `origin`: a session that sent an event has an address a reply reaches.
    public var sender: String?

    public var invokeID: String

    /// The token a host-run invocation's completion or failure was stamped with when the engine accepted it,
    /// nil for any other event. A completion that was accepted and is still queued has to keep the stamp across
    /// a save, or the restored machine would refuse the one answer it was waiting for. Written as it was, not
    /// guessed from the event's name.
    public var hostInvokeToken: Int64?

    public init(
        name: String, data: String = "", type: String = "external", sendID: String = "", origin: EventOrigin? = nil,
        sender: String? = nil, invokeID: String = "", hostInvokeToken: Int64? = nil
    ) {
        self.name = name
        self.data = data
        self.type = type
        self.sendID = sendID
        self.origin = origin
        self.sender = sender
        self.invokeID = invokeID
        self.hostInvokeToken = hostInvokeToken
    }
}

/// An `<invoke>` a declared host invoker is running, as it is started again: every field is what the request
/// the host was handed carried, so the restarted invocation reads as the one the document began.
public struct SavedHostInvoke: Equatable, Sendable {
    /// The `type` the `<invoke>` named.
    public var processorType: String

    /// The invoke's id, which `done.invoke.<id>` names.
    public var invokeID: String

    /// `<invoke src>`, empty when the document named none.
    public var src: String

    /// `<param>` values by name; a repeated name keeps every value in document order.
    public var params: [String: [String]]

    /// The namelist and `<param>` pairs as JSON, as the request carried them.
    public var data: String

    /// Inline `<content>`, empty when the document carried none.
    public var content: String

    /// When its deadline comes due, in milliseconds since the Unix epoch on the wall clock of the host that
    /// saved it; nil for an invocation that has none.
    public var due: Int64?

    public init(
        processorType: String, invokeID: String, src: String = "", params: [String: [String]] = [:],
        data: String = "", content: String = "", due: Int64? = nil
    ) {
        self.processorType = processorType
        self.invokeID = invokeID
        self.src = src
        self.params = params
        self.data = data
        self.content = content
        self.due = due
    }
}

/// What a waiting `<send>` does when it comes due: its own event on this session's external queue or on its
/// internal queue, an act a host-served processor performs, or an event for the session that started this one
/// or for a running child of it, which the tree holds (docs/adr/0045). A delayed send to any other session is
/// none of them, and a document that makes one is generated without the save API.
///
/// A `raise`, `parent` or `child` act is this session's own send, which the SCXML Event I/O Processor delivers:
/// it holds the id of the session that sent it, and the event it delivers carries that session's pair
/// (`scxmlOrigin(sender:)`), which is what a text of format 3 writes beside the id (docs/adr/0090, section 7). The
/// sender is nil only for a send read from a format-1 text that stored none, which names no session; a format-2
/// send that stored none is read as sent by the session that holds it. An `internal` act
/// delivers an internal event, which has no origin (§scxml-5.10.1), so it holds no sender.
public enum SavedAct: Equatable, Sendable {
    /// An event for the session that started this one (`#_parent`). A tree's only (formats 2 and 3).
    case parent(event: String, data: String, sendID: String, sender: String?)

    /// An event for a running child of this session (`#_<invokeid>`). A tree's only (formats 2 and 3).
    case child(invokeID: String, event: String, data: String, sendID: String, sender: String?)

    /// An event for this session's external queue.
    case raise(event: String, data: String, sendID: String, sender: String?)

    /// An event for this session's internal queue.
    case `internal`(event: String, data: String, sendID: String)

    /// A `<send>` a host-served processor performs, as it is performed when it comes due: every field is what
    /// the document wrote, so a handler sees the request it would have seen had there been no delay.
    case host(
        processorType: String, event: String, target: String, content: String, params: [String: [String]],
        sendID: String, data: String, invokeID: String
    )
}

/// One delayed `<send>` that has not been delivered yet.
public struct SavedSend: Equatable, Sendable {
    /// When it comes due, in milliseconds since the Unix epoch on the wall clock of the host that saved it. A
    /// delayed send is saved as the moment it comes due and not as a wait: a wait would start again when the
    /// process came back, and a timer that ran out while it was dead would be late by as long as it was dead.
    public var due: Int64

    /// What it does when it comes due.
    public var act: SavedAct

    public init(due: Int64, act: SavedAct) {
        self.due = due
        self.act = act
    }
}

/// The running child sessions of a saved session, as the format it was written in holds them.
public enum SavedInvokes: Equatable, Sendable {
    /// Format 1: the id of each `<invoke>` whose child is running. The text holds nothing of a child, so a
    /// restore starts each again from its beginning, under the same id.
    case restarted([String])

    /// Formats 2 and 3: each running child with its whole state, resumed by a restore where it stood.
    case resumed([SavedChild])

    /// The id the document gives each running `<invoke>`, in order.
    public var ids: [String] {
        switch self {
        case .restarted(let ids): return ids
        case .resumed(let children): return children.map(\.id)
        }
    }
}

/// A running child session, saved inside the session that started it (docs/adr/0045, section 1).
public struct SavedChild: Equatable, Sendable {
    /// The id the document gives the `<invoke>` that started it.
    public var id: String

    /// Which start of that `<invoke>` this is in its parent, counted from 1.
    public var start: Int64

    /// The id the child was started under, as `_event.invokeid` and an `idlocation` variable hold it: carried,
    /// not computed again.
    public var invokeID: String

    /// The document the child runs: the child's name for a static `<invoke>`, the candidate's for a hybrid one.
    public var document: String

    /// The child's own state, its running children included.
    public var session: SavedState

    public init(id: String, start: Int64, invokeID: String, document: String, session: SavedState) {
        self.id = id
        self.start = start
        self.invokeID = invokeID
        self.document = document
        self.session = session
    }
}

/// A saved session: one machine's state, or, in formats 2 and 3, a tree of them.
public struct SavedState: Equatable, Sendable {
    /// Format 1: a state that names its running children by id alone.
    public static let format1 = 1

    /// Format 2: a tree of sessions, each running child saved inside its parent (docs/adr/0045).
    public static let format2 = 2

    /// Format 3: format 2 with an event's origin held as one pair and its sender beside it (docs/adr/0090,
    /// section 7). The shape of a session is format 2's: the pair changes no document.
    public static let format3 = 3

    /// The newest format version this runtime writes, which it writes for every tree; it reads this one and
    /// formats 1 and 2.
    public static let format = format3

    /// The stability status of the saved-state wire surface, held in lockstep with `x-sce-schema-status` in
    /// `schemas/sce-saved-state.v1.schema.json`, `schemas/sce-saved-state.v2.schema.json` and
    /// `schemas/sce-saved-state.v3.schema.json`, the surface's three formats (`SCE_WIRE_CONTRACTS.md`). A flip to
    /// `"stable"` changes all four in one commit.
    public static let schemaStatus = "pre-release"

    /// The shape of the document the machine was generated from: a digest the generator computes of every
    /// state with its kind and parent and every variable with its type, not the document's text, so a later
    /// build whose guards, actions or comments changed restores it and one that renamed, re-typed or moved
    /// something a saved state names refuses it.
    public var shape: String

    /// The active configuration, as state ids.
    public var configuration: [String]

    /// The current leaf, as a state id: which of a `<parallel>`'s regions the machine last stood in, which the
    /// configuration alone cannot say.
    public var current: String

    /// Every variable, by document id, in declaration order, each value as JSON holds it: a number, except a
    /// 64-bit integer, written as a text so no reader that holds numbers as doubles loses its low bits; a byte
    /// string as a text of its bytes; a record as an object of its fields; a list as an array.
    public var variables: [SavedEntry<JSONValue>]

    /// What each `<history>` recorded when its parent was last exited, as state ids in document order, keyed
    /// by the history's id and ordered by it. A history that has recorded nothing is absent: a resumed machine
    /// takes its default transition, as the saved one would have.
    public var history: [SavedEntry<[String]>]

    /// The delayed `<send>`s still waiting, in the order they would be delivered: earliest first, entries due
    /// at the same moment in the order they were sent.
    public var pending: [SavedSend]

    /// The `<invoke>`s whose child session is running, in document order. One whose child has ended is absent:
    /// its `done.invoke` is in the external queue, or already taken, so it is not started twice.
    public var invokes: SavedInvokes

    /// The session's own id (`_sessionid`), which events held in the tree name sessions by and a restore binds
    /// again (docs/adr/0045). An XML Nmtoken in a tree; empty in a format-1 state, which holds none.
    public var sessionID: String

    /// How many times each `<invoke>` of the document has started in this session, by its id, for each
    /// started at least once, in document order: a start after a restore is numbered on from it. Empty in a
    /// format-1 state, which holds none.
    public var invokeStarts: [SavedEntry<Int64>]

    /// The `<invoke>`s a declared host invoker is running, by type and id. A restored machine starts each again
    /// from the request it was started with, with the deadline it had left.
    public var hostInvokes: [SavedHostInvoke]

    /// The token the next host-run start receives. Carried on, so that a start a restored machine makes is
    /// never given a token an earlier run already handed to a host that may still answer with it.
    public var hostInvokeToken: Int64

    /// How many ids the machine has generated for a `<send idlocation>`: the number the last one carried.
    /// Carried on, so that an id a restored machine generates is never one an earlier run already handed to
    /// the document, which may still hold it in a variable.
    public var autoSendSeq: Int64

    /// The external queue, front first: events raised to the machine that it has not yet been driven through.
    /// Only the internal queue is empty at a macrostep boundary, so a state that left these out would lose them.
    public var external: [SavedEvent]

    /// The revision each record schema and enum past its first was saved at, ordered by key. Empty when every
    /// schema the machine holds is in its first revision, which is every state saved before schemas had
    /// revisions.
    public var revisions: [SavedEntry<SavedRevision>]

    /// The configuration of a product line the machine was built for (docs/adr/0057, section 7): the features
    /// it turns on, sorted, each once; empty for the configuration that turns none on. Nil for a machine of no
    /// product line, whose text carries no `features`.
    public var features: [String]?

    public init(
        shape: String, configuration: [String], current: String, variables: [SavedEntry<JSONValue>],
        history: [SavedEntry<[String]>] = [], pending: [SavedSend] = [],
        invokes: SavedInvokes = .restarted([]), sessionID: String = "", invokeStarts: [SavedEntry<Int64>] = [],
        hostInvokes: [SavedHostInvoke] = [], hostInvokeToken: Int64 = 0, autoSendSeq: Int64 = 0,
        external: [SavedEvent] = [], revisions: [SavedEntry<SavedRevision>] = [], features: [String]? = nil
    ) {
        self.shape = shape
        self.configuration = configuration
        self.current = current
        self.variables = variables
        self.history = history
        self.pending = pending
        self.invokes = invokes
        self.sessionID = sessionID
        self.invokeStarts = invokeStarts
        self.hostInvokes = hostInvokes
        self.hostInvokeToken = hostInvokeToken
        self.autoSendSeq = autoSendSeq
        self.external = external
        self.revisions = revisions
        self.features = features
    }

    /// The format this state is written in: format 3 for a tree of sessions, whichever format it was read from
    /// (docs/adr/0090, section 7), and format 1 for a state that names its running children by id alone.
    public var formatVersion: Int {
        switch invokes {
        case .restarted: return Self.format1
        case .resumed: return Self.format
        }
    }

    /// The id the document gives each running `<invoke>`, in order.
    public var invokeIDs: [String] { invokes.ids }

    /// The id a restore binds the session to: the saved one, for a tree, since the session carries on and every
    /// child's id and a peer's address name it; nil for a format-1 state, which holds none, so the restored
    /// machine keeps the id it was created with.
    public var boundSessionID: String? {
        formatVersion == Self.format1 ? nil : sessionID
    }

    /// How many times the `<invoke>` `id` had started in this session when it was saved: 0 for one that had
    /// not, and for every one of a format-1 state, which holds no count.
    public func invokeStart(_ id: String) -> Int64 {
        invokeStarts.first { $0.key == id }?.value ?? 0
    }

    /// The variable `id`, or a refusal naming it.
    public func variable(_ id: String) throws -> JSONValue {
        guard let found = variables.first(where: { $0.key == id }) else {
            throw StateRefusal("the saved state has no variable '\(id)'")
        }
        return found.value
    }
}

// MARK: - Writing

private typealias Members = [(key: String, value: JSONValue)]

private func number(_ value: Int64) -> JSONValue { .number(String(value)) }

/// A count, a moment or a token as a saved state writes it: the text of its digits, as every 64-bit integer is.
private func counted(_ value: Int64) -> JSONValue { .text(String(value)) }

private func texts(_ values: [String]) -> JSONValue { .array(values.map { .text($0) }) }

/// `params` as a saved state writes them: by name in the order of the names' bytes, so the text one machine
/// writes is every backend's.
private func paramsJSON(_ params: [String: [String]]) -> JSONValue {
    .object(
        params.sorted { $0.key.utf8.lexicographicallyPrecedes($1.key.utf8) }
            .map { (key: $0.key, value: texts($0.value)) })
}

/// The members that hold an event's origin, in the text of `format`, in the place each record gives them
/// (docs/adr/0090, section 7, "Text form"). Format 3 writes the pair, `processor` then `address`, or `null`, and
/// then the sender's id or `null`: both always, so no writer decides when to leave one out. Format 1 writes the
/// one `origin` text by the rule format 2 used (`storedOriginText`), and an event of the queue (`typed`) the
/// pair's processor beside it as `origintype`, both empty for none.
private func originMembers(_ origin: EventOrigin?, sender: String?, format: Int, typed: Bool) -> Members {
    if format == SavedState.format1 {
        var members: Members = [(key: "origin", value: .text(storedOriginText(origin, sender: sender)))]
        if typed {
            members.append((key: "origintype", value: .text(origin?.processor ?? "")))
        }
        return members
    }
    var pair = JSONValue.null
    if let origin {
        pair = .object([
            (key: "processor", value: .text(origin.processor)),
            (key: "address", value: .text(origin.address)),
        ])
    }
    return [(key: "origin", value: pair), (key: "sender", value: sender.map(JSONValue.text) ?? .null)]
}

extension SavedEvent {
    fileprivate func json(format: Int) -> JSONValue {
        var members: Members = [
            (key: "name", value: .text(name)),
            (key: "data", value: .text(data)),
            (key: "type", value: .text(type)),
            (key: "sendid", value: .text(sendID)),
        ]
        members.append(contentsOf: originMembers(origin, sender: sender, format: format, typed: true))
        members.append((key: "invokeid", value: .text(invokeID)))
        members.append((key: "hostinvoketoken", value: hostInvokeToken.map(counted) ?? .null))
        return .object(members)
    }
}

extension SavedHostInvoke {
    fileprivate var json: JSONValue {
        .object([
            (key: "type", value: .text(processorType)),
            (key: "id", value: .text(invokeID)),
            (key: "src", value: .text(src)),
            (key: "params", value: paramsJSON(params)),
            (key: "data", value: .text(data)),
            (key: "content", value: .text(content)),
            (key: "due", value: due.map(counted) ?? .null),
        ])
    }
}

extension SavedSend {
    fileprivate func json(format: Int) -> JSONValue {
        var members: Members = [(key: "due", value: counted(due))]
        func append(_ pairs: [(String, JSONValue)]) {
            members.append(contentsOf: pairs.map { (key: $0.0, value: $0.1) })
        }
        // After `sendid`: the pair and the id of the session that sent it, which the event carries when it is
        // delivered; an internal act's event carries neither (docs/adr/0090, section 7).
        func sent(by sender: String?) {
            members.append(
                contentsOf: originMembers(sender.map(scxmlOrigin(sender:)), sender: sender, format: format, typed: false))
        }
        switch act {
        case .parent(let event, let data, let sendID, let sender):
            append([
                ("act", .text("parent")), ("event", .text(event)), ("data", .text(data)), ("sendid", .text(sendID)),
            ])
            sent(by: sender)
        case .child(let invokeID, let event, let data, let sendID, let sender):
            append([
                ("act", .text("child")), ("invokeid", .text(invokeID)), ("event", .text(event)),
                ("data", .text(data)), ("sendid", .text(sendID)),
            ])
            sent(by: sender)
        case .raise(let event, let data, let sendID, let sender):
            append([
                ("act", .text("raise")), ("event", .text(event)), ("data", .text(data)), ("sendid", .text(sendID)),
            ])
            sent(by: sender)
        case .internal(let event, let data, let sendID):
            append([
                ("act", .text("internal")), ("event", .text(event)), ("data", .text(data)),
                ("sendid", .text(sendID)),
            ])
            sent(by: nil)
        case .host(let type, let event, let target, let content, let params, let sendID, let data, let invokeID):
            append([
                ("act", .text("host")), ("type", .text(type)), ("event", .text(event)),
                ("target", .text(target)), ("content", .text(content)), ("params", paramsJSON(params)),
                ("sendid", .text(sendID)), ("data", .text(data)), ("invokeid", .text(invokeID)),
            ])
        }
        return .object(members)
    }
}

extension SavedChild {
    fileprivate func json(format: Int) -> JSONValue {
        .object([
            (key: "id", value: .text(id)),
            (key: "start", value: counted(start)),
            (key: "invokeid", value: .text(invokeID)),
            (key: "document", value: .text(document)),
            (key: "session", value: .object(session.sessionMembers(format: format))),
        ])
    }
}

extension SavedState {
    /// The state as `sce-saved-state` JSON, in the format it is written in (`formatVersion`): the root of the
    /// text, with its `format`, which applies to every session of the text.
    public func toJSON() -> String {
        let format = formatVersion
        var members: Members = [(key: "format", value: number(Int64(format)))]
        members.append(contentsOf: sessionMembers(format: format))
        return writeJSON(.object(members))
    }

    /// The members of this session, in a text of `format`, in the order every backend writes them: format 1's,
    /// and in a tree `sessionid` after `shape` and `invokestarts` after `invokes`, each running child's session
    /// written by the same rule.
    fileprivate func sessionMembers(format: Int) -> Members {
        var members: Members = [(key: "shape", value: .text(shape))]
        if case .resumed = invokes {
            members.append((key: "sessionid", value: .text(sessionID)))
        }
        members.append((key: "configuration", value: texts(configuration)))
        members.append((key: "current", value: .text(current)))
        members.append((key: "variables", value: .object(variables.map { (key: $0.key, value: $0.value) })))
        members.append(
            (key: "history", value: .object(history.map { (key: $0.key, value: texts($0.value)) })))
        members.append((key: "pending", value: .array(pending.map { $0.json(format: format) })))
        switch invokes {
        case .resumed(let children):
            members.append((key: "invokes", value: .array(children.map { $0.json(format: format) })))
            members.append(
                (key: "invokestarts", value: .object(invokeStarts.map { (key: $0.key, value: counted($0.value)) })))
        case .restarted(let ids):
            members.append((key: "invokes", value: texts(ids)))
        }
        members.append((key: "hostinvokes", value: .array(hostInvokes.map(\.json))))
        members.append((key: "hostinvoketoken", value: counted(hostInvokeToken)))
        members.append((key: "sendseq", value: counted(autoSendSeq)))
        members.append((key: "external", value: .array(external.map { $0.json(format: format) })))
        // Absent, not empty, when no schema is past its first revision: the text of such a state is the text
        // it was before revisions existed.
        if !revisions.isEmpty {
            members.append(
                (
                    key: "revisions",
                    value: .object(
                        revisions.map { entry in
                            (
                                key: entry.key,
                                value: .object([
                                    (key: "revision", value: number(Int64(entry.value.revision))),
                                    (key: "fields", value: .text(entry.value.fields)),
                                ])
                            )
                        })
                ))
        }
        // Absent for a machine of no product line, so the text of every other machine is the text it was
        // before configurations were recorded.
        if let features {
            members.append((key: "features", value: texts(features)))
        }
        return members
    }
}

// MARK: - Reading

private func isDigits(_ text: String) -> Bool {
    !text.isEmpty && text.utf8.allSatisfy { $0 >= 0x30 && $0 <= 0x39 }
}

/// A count of starts, as the text a saved state writes it as: a whole number from 1 with no leading zero,
/// within a signed 64-bit count, as every backend reads one.
private func readCount(_ written: String, _ what: String) throws -> Int64 {
    guard isDigits(written), !written.hasPrefix("0"), let value = Int64(written), value >= 1 else {
        throw StateRefusal("'\(what)' (\(written)) is not a count of starts, a whole number from 1")
    }
    return value
}

/// A host-run invocation's token, as the text a saved state writes it as: digits, within a signed 64-bit
/// count, as every backend reads one.
private func readToken(_ written: String, _ what: String) throws -> Int64 {
    guard isDigits(written), let value = Int64(written) else {
        throw StateRefusal("'\(what)' (\(written)) is not a whole number a token can be")
    }
    return value
}

/// A moment, as the text a saved state writes it as: milliseconds since the Unix epoch.
private func readMoment(_ written: String, _ what: String) throws -> Int64 {
    guard isDigits(written), let value = Int64(written) else {
        throw StateRefusal("'\(what)' (\(written)) is not a whole number of milliseconds")
    }
    return value
}

private func isLowerHexDigest(_ text: String) -> Bool {
    text.utf8.count == 64 && text.utf8.allSatisfy { ($0 >= 0x30 && $0 <= 0x39) || ($0 >= 0x61 && $0 <= 0x66) }
}

/// The members of an object, or a refusal naming `what`.
private func members(of value: JSONValue?, _ what: String) throws -> Members {
    guard case .object(let members)? = value else { throw StateRefusal("'\(what)' is not an object") }
    return members
}

private func array(of value: JSONValue?, _ what: String) throws -> [JSONValue] {
    guard case .array(let items)? = value else { throw StateRefusal("'\(what)' is not an array") }
    return items
}

private func text(_ value: JSONValue?, _ what: String) throws -> String {
    guard case .text(let text)? = value else { throw StateRefusal("'\(what)' is not a text") }
    return text
}

/// The member `key` of `object`, whose own name in a refusal is `what`.
private func need(_ object: JSONValue, _ key: String, _ what: String) throws -> JSONValue {
    guard let found = object.member(key) else {
        throw StateRefusal(what.isEmpty ? "the saved state has no '\(key)'" : "'\(what)' has no '\(key)'")
    }
    return found
}

private func requiredText(_ object: JSONValue, _ key: String, _ what: String) throws -> String {
    try text(need(object, key, what), what.isEmpty ? key : "\(what).\(key)")
}

/// The member `params` of `object`, read as an object of arrays of texts, by name.
private func readParams(_ object: JSONValue, _ what: String) throws -> [String: [String]] {
    let named = try members(of: need(object, "params", what), "\(what).params")
    var read: [String: [String]] = [:]
    for (name, value) in named {
        guard case .array(let items) = value else { throw StateRefusal("'\(what).params.\(name)' is not an array") }
        read[name] = try items.map { item in
            guard case .text(let text) = item else {
                throw StateRefusal("'\(what).params.\(name)' holds a value that is not a text")
            }
            return text
        }
    }
    return read
}

/// Whether `origin` is, byte for byte, the pair the SCXML Event I/O Processor writes for the session `sender`.
private func isSCXMLOrigin(_ origin: EventOrigin, of sender: String) -> Bool {
    let expected = scxmlOrigin(sender: sender)
    return origin.processor.utf8.elementsEqual(expected.processor.utf8)
        && origin.address.utf8.elementsEqual(expected.address.utf8)
}

/// The `origin` member of a format-3 record, as written: `null`, or the pair of a processor and an address, each
/// a text that is not empty, and nothing else (docs/adr/0090, section 7).
private func readOrigin(_ record: JSONValue, _ what: String) throws -> EventOrigin? {
    let named = "\(what).origin"
    let written = try need(record, "origin", what)
    guard case .object(let pair) = written else {
        if written == .null { return nil }
        throw StateRefusal("'\(named)' is neither null nor a pair of a processor and an address")
    }
    for (key, _) in pair where key != "processor" && key != "address" {
        throw StateRefusal("'\(named)' holds '\(key)', which a pair does not")
    }
    func member(_ key: String) throws -> String {
        let text = try requiredText(written, key, named)
        if text.isEmpty {
            throw StateRefusal("'\(named).\(key)' is empty, and a pair names both its members")
        }
        return text
    }
    return EventOrigin(processor: try member("processor"), address: try member("address"))
}

/// The `origin` and `sender` of a format-3 record, each as written and neither derived from the other
/// (docs/adr/0090, section 7). The origin is the reply address the delivering processor wrote, and the sender the
/// id of the session that sent the event, or `null` for a sender that is no session. No rule ties the address to
/// the sender: a remote child's event the Mesh transport delivers carries the child's id beside its endpoint. A
/// session that sent an event always has an address a reply reaches, so a sender beside a `null` origin is
/// refused.
private func readOriginAndSender(_ record: JSONValue, _ what: String) throws -> (origin: EventOrigin?, sender: String?) {
    let origin = try readOrigin(record, what)
    let sender: String?
    switch try need(record, "sender", what) {
    case .null:
        sender = nil
    case .text(let id):
        if !sessionIDRefusalOf(id).isEmpty {
            throw StateRefusal("'\(what).sender' (\(id)) is not a session id, an XML Nmtoken")
        }
        sender = id
    default:
        throw StateRefusal("'\(what).sender' is neither null nor a session id")
    }
    if let sender, origin == nil {
        throw StateRefusal(
            "'\(what).sender' is '\(sender)' beside a null 'origin', and the session that sent an event has an address a reply reaches"
        )
    }
    return (origin, sender)
}

extension SavedEvent {
    /// The event `value` of a queue, of a state of `format`.
    fileprivate static func read(_ value: JSONValue, _ what: String, format: Int) throws -> SavedEvent {
        _ = try members(of: value, what)
        let token = try need(value, "hostinvoketoken", what)
        let hostInvokeToken: Int64?
        switch token {
        case .null: hostInvokeToken = nil
        case .text(let written): hostInvokeToken = try readToken(written, "\(what).hostinvoketoken")
        default: throw StateRefusal("'\(what).hostinvoketoken' is neither a text nor null")
        }
        let carried: (origin: EventOrigin?, sender: String?)
        if format == SavedState.format3 {
            carried = try readOriginAndSender(value, what)
        } else {
            // docs/adr/0090, section 7: a format-1 or format-2 `origin` and `origintype` are read by the table of
            // that section, as the pair and the sender they stood for, whichever backend wrote them.
            carried = readStoredOrigin(
                try requiredText(value, "origin", what), originType: try requiredText(value, "origintype", what))
        }
        return SavedEvent(
            name: try requiredText(value, "name", what), data: try requiredText(value, "data", what),
            type: try requiredText(value, "type", what), sendID: try requiredText(value, "sendid", what),
            origin: carried.origin, sender: carried.sender,
            invokeID: try requiredText(value, "invokeid", what), hostInvokeToken: hostInvokeToken)
    }
}

extension SavedHostInvoke {
    fileprivate static func read(_ value: JSONValue, _ what: String) throws -> SavedHostInvoke {
        _ = try members(of: value, what)
        let written = try need(value, "due", what)
        let due: Int64?
        switch written {
        case .null: due = nil
        case .text(let moment): due = try readMoment(moment, "\(what).due")
        default: throw StateRefusal("'\(what).due' is neither a text nor null")
        }
        return SavedHostInvoke(
            processorType: try requiredText(value, "type", what), invokeID: try requiredText(value, "id", what),
            src: try requiredText(value, "src", what), params: try readParams(value, what),
            data: try requiredText(value, "data", what), content: try requiredText(value, "content", what),
            due: due)
    }
}

extension SavedSend {
    /// The waiting send `value`, of a state of `format` held by the session `sessionID` (empty in format 1, whose
    /// text names no session); a send to the parent or to a child is an act of a tree only.
    fileprivate static func read(_ value: JSONValue, _ what: String, format: Int, sessionID: String) throws
        -> SavedSend
    {
        _ = try members(of: value, what)
        let due = try readMoment(try requiredText(value, "due", what), "\(what).due")
        let name = try requiredText(value, "act", what)
        func field(_ key: String) throws -> String { try requiredText(value, key, what) }
        // The session that sent a `raise`, `parent` or `child` act, which is this session's own send
        // (docs/adr/0090, section 7). Format 3 holds the session's SCXML pair and its id, both judged and neither
        // derived from the other: an act without its sender, or whose pair is not that sender's, is refused.
        // Formats 1 and 2 hold one `origin` text, the sender's id or the location C11 stored, read by
        // `readStoredSender`. An empty one in format 2 is a send of the session that holds it, its `sessionid`;
        // in format 1 the text names no session, so it is nil, and the machine that restores it arms it as its own.
        func sentBy() throws -> String? {
            if format == SavedState.format3 {
                let carried = try readOriginAndSender(value, what)
                guard let id = carried.sender else {
                    throw StateRefusal("'\(what)' is a \(name) act with no sender, which a session's own send names")
                }
                guard let origin = carried.origin, isSCXMLOrigin(origin, of: id) else {
                    throw StateRefusal(
                        "'\(what).origin' is not the SCXML pair of its sender '\(id)', which a session's own send carries")
                }
                // Sent by the session the record sits in, root or child: a text that names another is not one
                // this session saved, and is refused rather than saved back under this session's id.
                guard id.utf8.elementsEqual(sessionID.utf8) else {
                    throw StateRefusal(
                        "'\(what).sender' is '\(id)', and this session is '\(sessionID)': a \(name) act is this session's own send"
                    )
                }
                return id
            }
            let written = try field("origin")
            let stored = readStoredSender(written)
            if stored.isEmpty { return sessionID.isEmpty ? nil : sessionID }
            // A stored origin that is not empty names the session that made the send, so what it reads as is a
            // session id, an XML Nmtoken, as a format-3 `sender` is. The prefix of an SCXML location alone is one
            // origin this refuses: it reads as itself, and '/' is no name character. A queued event's stored
            // origin is read by the section-7 table instead, and is not held to this.
            if !isNmtoken(stored) {
                throw StateRefusal(
                    "'\(what).origin' is '\(written)', which names no session: a \(name) act is a session's own send, and its stored origin is that session's id or its SCXML location"
                )
            }
            return stored
        }
        // An `internal` act delivers an internal event, which has no origin (§scxml-5.10.1): format 3 writes
        // both members `null` and refuses either one set; the `origin` text formats 1 and 2 stored beside it is
        // not read as the event's.
        func carriesNoOrigin() throws {
            guard format == SavedState.format3 else {
                _ = try field("origin")
                return
            }
            for key in ["origin", "sender"] {
                if try need(value, key, what) != .null {
                    throw StateRefusal(
                        "'\(what).\(key)' is not null, and an internal act's event has no origin and no sender")
                }
            }
        }
        let act: SavedAct
        switch name {
        case "parent" where format != SavedState.format1:
            act = .parent(
                event: try field("event"), data: try field("data"), sendID: try field("sendid"),
                sender: try sentBy())
        case "child" where format != SavedState.format1:
            act = .child(
                invokeID: try field("invokeid"), event: try field("event"), data: try field("data"),
                sendID: try field("sendid"), sender: try sentBy())
        case "raise":
            act = .raise(
                event: try field("event"), data: try field("data"), sendID: try field("sendid"),
                sender: try sentBy())
        case "internal":
            try carriesNoOrigin()
            act = .internal(event: try field("event"), data: try field("data"), sendID: try field("sendid"))
        case "host":
            act = .host(
                processorType: try field("type"), event: try field("event"), target: try field("target"),
                content: try field("content"), params: try readParams(value, what),
                sendID: try field("sendid"), data: try field("data"), invokeID: try field("invokeid"))
        default:
            if format == SavedState.format1 {
                throw StateRefusal("'\(what).act' is '\(name)', which is not raise, internal or host, the acts of format 1")
            }
            throw StateRefusal("'\(what).act' is '\(name)', which is not raise, internal, host, parent or child")
        }
        return SavedSend(due: due, act: act)
    }
}

extension SavedChild {
    /// The `index`th running child of a session of a tree of `format`, which the child's session is read in
    /// too. A refusal inside the child's own session names it by its path (`StateRefusal.within`).
    fileprivate static func read(_ value: JSONValue, index: Int, format: Int) throws -> SavedChild {
        let what = "invokes[\(index)]"
        _ = try members(of: value, what)
        func nonEmpty(_ key: String) throws -> String {
            let written = try requiredText(value, key, what)
            if written.isEmpty { throw StateRefusal("'\(what).\(key)' is empty") }
            return written
        }
        let start = try readCount(try nonEmpty("start"), "\(what).start")
        let session = try need(value, "session", what)
        guard case .object = session else { throw StateRefusal("'\(what).session' is not an object") }
        if session.member("format") != nil {
            throw StateRefusal("'\(what).session' states a format, which only the root of a tree does")
        }
        let read: SavedState
        do {
            read = try SavedState.readSession(session, format: format)
        } catch let refusal as StateRefusal {
            throw refusal.within("\(what).session")
        }
        return SavedChild(
            id: try nonEmpty("id"), start: start, invokeID: try nonEmpty("invokeid"),
            document: try nonEmpty("document"), session: read)
    }
}

extension SavedState {
    /// Reads `sce-saved-state` JSON, of format 1, 2 or 3, judged by its `format` before anything else. The shape
    /// is not judged here: the machine that restores it knows its own.
    public static func fromJSON(_ text: String) throws -> SavedState {
        let value: JSONValue
        do {
            value = try parseJSON(text)
        } catch {
            throw StateRefusal("the saved state is \(error)")
        }
        guard case .object = value else { throw StateRefusal("the saved state is not an object") }
        guard let written = value.member("format") else { throw StateRefusal("the saved state has no 'format'") }
        let format: Int
        switch written {
        case .number("1"): format = format1
        case .number("2"): format = format2
        case .number("3"): format = format3
        default:
            throw StateRefusal(
                "the saved state is format \(writeJSON(written)), and this runtime reads formats \(format1), \(format2) and \(format3)"
            )
        }
        return try readSession(value, format: format)
    }

    /// The session `object` holds, of a text of `format`: the root, or a child's `session`, which states no
    /// `format` of its own.
    fileprivate static func readSession(_ object: JSONValue, format: Int) throws -> SavedState {
        func field(_ name: String) throws -> JSONValue { try need(object, name, "") }
        let configuration = try array(of: field("configuration"), "configuration").map {
            try text($0, "configuration")
        }
        let variables = try members(of: field("variables"), "variables").map {
            SavedEntry($0.key, $0.value)
        }
        var history: [SavedEntry<[String]>] = []
        for (id, states) in try members(of: field("history"), "history") {
            history.append(
                SavedEntry(id, try array(of: states, "history.\(id)").map { try text($0, "history.\(id)") }))
        }
        // A tree's own members: the session's id, read before the waiting sends, which a format-2 text may hold
        // as sent by it without naming it (`SavedSend.read`), and its count of starts.
        var sessionID = ""
        if format != format1 {
            sessionID = try text(field("sessionid"), "sessionid")
            if !sessionIDRefusalOf(sessionID).isEmpty {
                throw StateRefusal("'sessionid' (\(sessionID)) is not an XML Nmtoken, which a session id is")
            }
        }
        let pending = try array(of: field("pending"), "pending").enumerated().map { index, item in
            try SavedSend.read(item, "pending[\(index)]", format: format, sessionID: sessionID)
        }
        let listed = try array(of: field("invokes"), "invokes")
        let invokes: SavedInvokes
        if format == format1 {
            invokes = .restarted(try listed.map { try text($0, "invokes") })
        } else {
            invokes = .resumed(
                try listed.enumerated().map { index, item in try SavedChild.read(item, index: index, format: format) })
        }
        var invokeStarts: [SavedEntry<Int64>] = []
        if format != format1 {
            for (id, count) in try members(of: field("invokestarts"), "invokestarts") {
                guard case .text(let written) = count else {
                    throw StateRefusal("'invokestarts.\(id)' is not a text")
                }
                invokeStarts.append(SavedEntry(id, try readCount(written, "invokestarts.\(id)")))
            }
        }
        let hostInvokes = try array(of: field("hostinvokes"), "hostinvokes").enumerated().map { index, item in
            try SavedHostInvoke.read(item, "hostinvokes[\(index)]")
        }
        let hostInvokeToken = try readToken(try text(field("hostinvoketoken"), "hostinvoketoken"), "hostinvoketoken")
        let autoSendSeq = try readToken(try text(field("sendseq"), "sendseq"), "sendseq")
        let external = try array(of: field("external"), "external").enumerated().map { index, item in
            try SavedEvent.read(item, "external[\(index)]", format: format)
        }
        // Optional, unlike every member above: a state whose schemas are all in their first revision leaves it
        // out, as every state saved before schemas had revisions does.
        var revisions: [SavedEntry<SavedRevision>] = []
        if let named = object.member("revisions") {
            let entries = try members(of: named, "revisions")
            if entries.isEmpty {
                throw StateRefusal(
                    "'revisions' names no schema; a state whose schemas are all in their first revision leaves it out"
                )
            }
            for (key, value) in entries {
                revisions.append(SavedEntry(key, try readRevision(value, "revisions.\(key)")))
            }
        }
        // Optional too: only a machine of a product line records the features it was built with, as a set, so
        // each is named once and in order.
        var features: [String]?
        if let named = object.member("features") {
            let names = try array(of: named, "features").map { try text($0, "features") }
            for (left, right) in zip(names, names.dropFirst()) where !left.utf8.lexicographicallyPrecedes(right.utf8) {
                throw StateRefusal("'features' is not in order or names a feature twice; it is a set, sorted")
            }
            features = names
        }
        return SavedState(
            shape: try text(field("shape"), "shape"), configuration: configuration,
            current: try text(field("current"), "current"), variables: variables, history: history,
            pending: pending, invokes: invokes, sessionID: sessionID, invokeStarts: invokeStarts,
            hostInvokes: hostInvokes, hostInvokeToken: hostInvokeToken, autoSendSeq: autoSendSeq,
            external: external, revisions: revisions, features: features)
    }

    private static func readRevision(_ value: JSONValue, _ what: String) throws -> SavedRevision {
        _ = try members(of: value, what)
        guard case .number(let written) = try need(value, "revision", what) else {
            throw StateRefusal("'\(what).revision' is not a number")
        }
        guard isDigits(written), let revision = Int(written), revision >= 2 else {
            throw StateRefusal(
                "'\(what).revision' (\(written)) is not a revision past the first; a schema in its first revision is left out"
            )
        }
        guard case .text(let digest) = try need(value, "fields", what), isLowerHexDigest(digest) else {
            throw StateRefusal("'\(what).fields' is not a SHA-256 written as lowercase hex")
        }
        return SavedRevision(revision: revision, fields: digest)
    }
}
