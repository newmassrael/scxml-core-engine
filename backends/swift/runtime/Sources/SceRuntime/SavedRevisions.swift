// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Reading a saved state taken at an earlier revision of a record schema or an enum (docs/adr/0044, docs/adr/0055).
//
// A record schema and an enum document each declare a revision. A state saved while every schema was in its first
// revision names none, and is of the first shape; one saved after a schema moved on names each such schema's
// revision and the digest of what it declared there, and is of the revised shape. A restore reads the state as
// one of this machine: each record saved at an earlier revision takes the default of every field a later revision
// added, and each judgment that depends on what the saved revision declared is made on the values as saved, before
// a default is written in.
//
// Cross-language siblings: `com.sce.runtime.SavedState.upgrade` (Kotlin) and
// `sce_rust_runtime::saved_state::upgrade` (Rust).

/// A field a revision past the first added to a record schema.
public struct SavedAddedField: Sendable, Equatable {
    /// The field's id, the member a saved record holds it under.
    public let id: String

    /// The revision that added it.
    public let since: Int

    /// Its default as saved-state JSON: the JSON a delivery carries for it, which is how a saved state writes a
    /// value of every type a field can have; `null` for an optional field that declares none.
    public let defaultJSON: String

    public init(id: String, since: Int, defaultJSON: String) {
        self.id = id
        self.since = since
        self.defaultJSON = defaultJSON
    }
}

/// A field a revision widened from `T` to `optional<T>`.
public struct SavedWidenedField: Sendable, Equatable {
    /// The field's id.
    public let id: String

    /// The revision that added the field: a record saved before it holds no member for the field.
    public let since: Int

    /// The revision that widened it: a record saved at an earlier revision, from `since` on, holds the field as a
    /// value of `T`, so a `null` there is refused.
    public let optionalSince: Int

    public init(id: String, since: Int, optionalSince: Int) {
        self.id = id
        self.since = since
        self.optionalSince = optionalSince
    }
}

/// A variable that holds a record of a schema, or a list of them.
public struct SavedRecordHolder: Sendable, Equatable {
    /// The variable's document id.
    public let variable: String

    /// Whether it holds a list of records rather than one.
    public let list: Bool

    public init(variable: String, list: Bool) {
        self.variable = variable
        self.list = list
    }
}

/// A record schema a machine's variables are of, as a saved state names it.
public struct SavedRecordLineage: Sendable, Equatable {
    /// The event the schema declares, which names it in `revisions`.
    public let event: String

    /// The schema's revision in this machine (1 when it declares none).
    public let revision: Int

    /// The digest of what each revision from the second to `revision` declares, in order; empty for a schema of
    /// one revision.
    public let revisions: [String]

    /// Each field a revision past the first added, in the schema's order.
    public let added: [SavedAddedField]

    /// Each field a revision widened to optional, in the schema's order.
    public let widened: [SavedWidenedField]

    /// The variables that hold a record of it, or a list of them, in declaration order.
    public let holders: [SavedRecordHolder]

    public init(
        event: String, revision: Int, revisions: [String], added: [SavedAddedField], widened: [SavedWidenedField],
        holders: [SavedRecordHolder]
    ) {
        self.event = event
        self.revision = revision
        self.revisions = revisions
        self.added = added
        self.widened = widened
        self.holders = holders
    }
}

/// A variant a revision past the first added to an enum.
public struct SavedAddedVariant: Sendable, Equatable {
    /// The variant's declared name, which a saved state holds it by.
    public let name: String

    /// The revision that added it.
    public let since: Int

    public init(name: String, since: Int) {
        self.name = name
        self.since = since
    }
}

/// Where a value of an enum is held.
public struct SavedEnumHolder: Sendable, Equatable {
    /// The variable's document id.
    public let variable: String

    /// The field of the record the variable holds, for a value held in a record; nil for a variable of the enum.
    public let field: String?

    /// Whether the variable holds a list of such records rather than one.
    public let list: Bool

    public init(variable: String, field: String?, list: Bool) {
        self.variable = variable
        self.field = field
        self.list = list
    }
}

/// An enum a value of a machine's state is of, as a saved state names it.
public struct SavedEnumLineage: Sendable, Equatable {
    /// `enum:` and the key of the enum's document, which names it in `revisions`.
    public let key: String

    /// The enum's revision in this machine (1 when it declares none).
    public let revision: Int

    /// The digest of what each revision from the second to `revision` declares, in order.
    public let revisions: [String]

    /// Each variant a revision past the first added, in the enum's order.
    public let added: [SavedAddedVariant]

    /// Where a value of it is held, in declaration order.
    public let holders: [SavedEnumHolder]

    public init(
        key: String, revision: Int, revisions: [String], added: [SavedAddedVariant], holders: [SavedEnumHolder]
    ) {
        self.key = key
        self.revision = revision
        self.revisions = revisions
        self.added = added
        self.holders = holders
    }
}

extension SavedShape {
    /// What a state of this machine names in `revisions` when it is saved: each record schema and each enum past
    /// its first revision, with the digest of what that revision declares. Empty when every schema is in its first
    /// revision, which is the state's text before schemas had revisions.
    func currentRevisions() -> [SavedEntry<SavedRevision>] {
        var named: [SavedEntry<SavedRevision>] = []
        for record in records {
            guard let fields = record.revisions.last else { continue }
            named.append(SavedEntry(record.event, SavedRevision(revision: record.revision, fields: fields)))
        }
        for lineage in enums {
            guard let fields = lineage.revisions.last else { continue }
            named.append(SavedEntry(lineage.key, SavedRevision(revision: lineage.revision, fields: fields)))
        }
        return named
    }
}

extension SavedState {
    /// `saved` as a state of the machine whose saved state is bound to `shape`, or the refusal that says why it is
    /// not one, judged before any variable is read as `checkShape` is.
    ///
    /// A state that names no revision is of the first shape; one that does is of the revised shape, and each
    /// revision it names is of a schema or an enum of this machine, no later than this machine's revision of it,
    /// and declares what this machine's revision of that number declares (the digests agree). Such a state is
    /// then read as one of this machine: each record of a schema saved at an earlier revision, a variable's or an
    /// element of a list of them, takes the default of every field a later revision added. A record that already
    /// holds such a field is refused, since the revision it was saved at does not declare it; so is a value that
    /// names a variant a later revision added, and one that holds `null` in a field a later revision widened to
    /// optional, since at the revision it was saved at the field was a value of its present type and no build
    /// wrote it absent (docs/adr/0055, section 6). What is not a record where a record is held is left for the
    /// variable's own read to refuse.
    ///
    /// The configuration `saved` was taken in is judged first, before the shape.
    public static func upgrade(_ saved: SavedState, against shape: SavedShape) throws -> SavedState {
        try checkFeatures(saved, shape.features)
        // A tree (formats 2 and 3) names an enum by its document and a format-1 state by the alias it is imported
        // under, so each is judged against its own shape (docs/adr/0045, section 2). Format 3 changes no shape.
        let first = saved.formatVersion == format1 ? shape.first : shape.format2First
        let revised = saved.formatVersion == format1 ? shape.revised : shape.format2Revised
        if saved.revisions.isEmpty {
            try checkShape(saved, first)
        } else if saved.shape == first {
            throw StateRefusal(
                "the saved state names revisions of its schemas and has the shape of a state that names none")
        } else {
            try checkShape(saved, revised)
        }
        for named in saved.revisions {
            let key = named.key
            // A record schema by the event it declares, an enum by its key.
            let known: (revision: Int, digests: [String])
            if let record = shape.records.first(where: { $0.event == key }) {
                known = (record.revision, record.revisions)
            } else if let lineage = shape.enums.first(where: { $0.key == key }) {
                known = (lineage.revision, lineage.revisions)
            } else {
                throw StateRefusal("'revisions' names '\(key)', which no record or enum of this machine is")
            }
            if named.value.revision > known.revision {
                throw StateRefusal(
                    "the saved state was saved at revision \(named.value.revision) of '\(key)', later than this "
                        + "machine's \(known.revision): a machine does not restore what it does not know")
            }
            // The digest of revision r is the (r - 2)th: the first has none.
            let index = named.value.revision - 2
            guard known.digests.indices.contains(index), known.digests[index] == named.value.fields else {
                throw StateRefusal(
                    "revision \(named.value.revision) of '\(key)' declared other members where the state was "
                        + "saved than it does in this machine")
            }
        }
        // An enum value saved at an earlier revision of its enum is one of that revision's variants. Judged on the
        // values as saved, before a record is grown by defaults, which are this machine's.
        for lineage in shape.enums {
            let at = saved.revisions.first(where: { $0.key == lineage.key })?.value.revision ?? 1
            let later = lineage.added.filter { $0.since > at }
            if later.isEmpty { continue }
            for holder in lineage.holders {
                try checkEnumHolder(saved, lineage, holder, at, later)
            }
        }
        // A record's field saved before the revision that widened it holds a value of `T`, so a `null` there is no
        // absence and the state is not one any build wrote. Judged on the values as saved, before a record is
        // grown by defaults, which are this machine's and may be `null` for a field added absent.
        for record in shape.records {
            let at = saved.revisions.first(where: { $0.key == record.event })?.value.revision ?? 1
            let held = record.widened.filter { $0.since <= at && at < $0.optionalSince }
            if held.isEmpty { continue }
            for holder in record.holders {
                try checkWidenedHolder(saved, record, holder, at, held)
            }
        }
        var grown = saved
        for record in shape.records {
            let at = saved.revisions.first(where: { $0.key == record.event })?.value.revision ?? 1
            let added = record.added.filter { $0.since > at }
            if added.isEmpty { continue }
            for holder in record.holders {
                guard let index = grown.variables.firstIndex(where: { $0.key == holder.variable }) else { continue }
                let value = grown.variables[index].value
                let upgraded: JSONValue
                if holder.list {
                    if case .array(let items) = value {
                        upgraded = .array(
                            try items.enumerated().map { position, item in
                                try grownRecord(item, "\(holder.variable)[\(position)]", record, at, added)
                            })
                    } else {
                        upgraded = value
                    }
                } else {
                    upgraded = try grownRecord(value, holder.variable, record, at, added)
                }
                grown.variables[index] = SavedEntry(holder.variable, upgraded)
            }
        }
        return grown
    }

    /// Refuses a value `holder` holds, in `saved`, that names one of `later`, the variants of `lineage` added
    /// after `at`, the revision it was saved at. What is not a text where one is held is left for the variable's
    /// own read to refuse.
    private static func checkEnumHolder(
        _ saved: SavedState, _ lineage: SavedEnumLineage, _ holder: SavedEnumHolder, _ at: Int,
        _ later: [SavedAddedVariant]
    ) throws {
        guard let value = saved.variables.first(where: { $0.key == holder.variable })?.value else { return }
        // Each value the holder holds, with what names it in a refusal.
        var held: [(what: String, value: JSONValue)] = []
        if let field = holder.field {
            if !holder.list {
                if let member = value.member(field) { held.append(("\(holder.variable).\(field)", member)) }
            } else if case .array(let items) = value {
                for (position, item) in items.enumerated() {
                    if let member = item.member(field) {
                        held.append(("\(holder.variable)[\(position)].\(field)", member))
                    }
                }
            }
        } else {
            held.append((holder.variable, value))
        }
        for (what, value) in held {
            guard case .text(let name) = value, let variant = later.first(where: { $0.name == name }) else {
                continue
            }
            throw StateRefusal(
                "'\(what)' holds '\(name)', which revision \(at) of '\(lineage.key)' does not declare: "
                    + "revision \(variant.since) added it")
        }
    }

    /// Refuses a `null` the record, or each record of the list, `holder` holds in `saved` has in a field of
    /// `widened`, the fields of `record` that a revision after `at`, the one it was saved at, widened to optional.
    private static func checkWidenedHolder(
        _ saved: SavedState, _ record: SavedRecordLineage, _ holder: SavedRecordHolder, _ at: Int,
        _ widened: [SavedWidenedField]
    ) throws {
        guard let value = saved.variables.first(where: { $0.key == holder.variable })?.value else { return }
        if holder.list {
            if case .array(let items) = value {
                for (position, item) in items.enumerated() {
                    try checkWidenedRecord(item, "\(holder.variable)[\(position)]", record, at, widened)
                }
            }
        } else {
            try checkWidenedRecord(value, holder.variable, record, at, widened)
        }
    }

    private static func checkWidenedRecord(
        _ value: JSONValue, _ what: String, _ record: SavedRecordLineage, _ at: Int, _ widened: [SavedWidenedField]
    ) throws {
        for field in widened {
            if case .null? = value.member(field.id) {
                throw StateRefusal(
                    "'\(what).\(field.id)' is null, which revision \(at) of '\(record.event)' holds as a value: "
                        + "revision \(field.optionalSince) widened it")
            }
        }
    }

    /// The saved record `value`, of `record` saved at revision `at`, with the default of each of `added`, the
    /// fields later revisions added; `what` names it in a refusal. A value that is not an object is returned as it
    /// is, for the record's own read to refuse.
    private static func grownRecord(
        _ value: JSONValue, _ what: String, _ record: SavedRecordLineage, _ at: Int, _ added: [SavedAddedField]
    ) throws -> JSONValue {
        guard case .object(var members) = value else { return value }
        for field in added {
            if members.contains(where: { $0.key.utf8.elementsEqual(field.id.utf8) }) {
                throw StateRefusal(
                    "'\(what)' holds '\(field.id)', which revision \(at) of '\(record.event)' does not declare: "
                        + "revision \(field.since) added it")
            }
            let parsed: JSONValue
            do {
                parsed = try parseJSON(field.defaultJSON)
            } catch {
                throw StateRefusal("the default of '\(field.id)' of '\(record.event)' is \(error)")
            }
            members.append((key: field.id, value: parsed))
        }
        return .object(members)
    }
}
