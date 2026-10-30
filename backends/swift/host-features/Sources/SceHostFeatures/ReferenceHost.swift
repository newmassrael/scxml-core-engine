// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The reference host of the standard host features (docs/SCE_HOST_FEATURES.md): clock,
// key-value store and notifications, as the operations a generated machine's host-run
// invokes ask for.
//
// A `ReferenceHost` is the whole of a feature's semantics as one object. It is told the
// start of an operation (its type and the request's text parameters) and answers with an
// outcome: a completion, a refusal, or nothing yet. It knows no engine, so the same object
// serves a generated machine through `serve`, a test, or another embedding. Time comes
// from a `ClockSource`, the embedding's to supply.

import SceRuntime

/// The request of an operation as the engine hands it to a host: each `<param>` by name,
/// as text. A 64-bit integer is its decimal digits.
public typealias HostParams = [String: String]

/// What a host answers a start with.
public enum HostOutcome: Equatable, Sendable {
    /// A completion: the result record, as the JSON a completion carries. A failure is
    /// such a record whose `error` is a code of the operation's closed set.
    case done(String)

    /// A host that would not start the operation: the reason, as the JSON
    /// `error.execution` carries.
    case refuse(String)

    /// A host that has started the operation and answers later, or never.
    case hold
}

/// A start the host was given: the operation, the invocation's id and the request's
/// parameters.
public struct HostStart: Equatable, Sendable {
    public var operation: String
    public var id: String
    public var params: HostParams

    public init(operation: String, id: String, params: HostParams) {
        self.operation = operation
        self.id = id
        self.params = params
    }
}

/// The refusal the specification gives an argument the operation does not accept.
private let invalidArgument = #"{"reason":"invalid_argument"}"#

/// A key of the store: compared and ordered by its UTF-8 bytes, which is the order the
/// specification gives. A `String` compares by canonical equivalence, which would make two
/// spellings of one character one key.
private struct StoreKey: Hashable, Comparable {
    let text: String

    static func == (lhs: StoreKey, rhs: StoreKey) -> Bool {
        lhs.text.utf8.elementsEqual(rhs.text.utf8)
    }

    func hash(into hasher: inout Hasher) {
        for byte in text.utf8 {
            hasher.combine(byte)
        }
    }

    static func < (lhs: StoreKey, rhs: StoreKey) -> Bool {
        lhs.text.utf8.lexicographicallyPrecedes(rhs.text.utf8)
    }
}

/// One notification the host holds. What was posted is held as it was given: the host
/// shows text as written.
private struct PostedNotification {
    let id: String
    let title: String
    let body: String
}

/// The standard host features' semantics, over a clock.
public final class ReferenceHost {
    /// Every operation type this host serves, in the order the specification lists them.
    public static let types = [
        "sce-host-clock-now",
        "sce-host-clock-zone",
        "sce-host-kv-get",
        "sce-host-kv-put",
        "sce-host-kv-delete",
        "sce-host-kv-next-key",
        "sce-host-notify-permission",
        "sce-host-notify-request-permission",
        "sce-host-notify-post",
        "sce-host-notify-cancel",
    ]

    private let clock: any ClockSource

    /// Failures to inject: the operation type to the code it fails with.
    private var faults: [String: String] = [:]

    /// The invocations the host was told to stop, in the order told.
    public private(set) var cancelled: [String] = []

    /// The highest monotonic reading answered, which no answer goes below.
    private var monotonicFloor: Int64?

    private var starts: [HostStart] = []

    // The key-value store, in memory, with the quota the embedding set.
    private var store: [StoreKey: String] = [:]
    private var quota: Int?

    // The platform's tray, as two lists, and what it says about permission.
    private var permission = "undetermined"
    private var userDecision = "none"
    /// Kept to be shown at their instant, in the order posted.
    private var waitingList: [PostedNotification] = []
    /// Handed to the platform, in the order posted.
    private var shownList: [PostedNotification] = []

    /// A host reading `clock`, with an empty store, nothing posted, and the notification
    /// permission undetermined.
    public init(clock: any ClockSource) {
        self.clock = clock
    }

    /// Injects failures: each operation type named fails with the code it maps to, once
    /// the request has been accepted. An empty map ends them.
    public func setFaults(_ faults: [String: String]) {
        self.faults = faults
    }

    /// Bounds the store to `keys` keys; a write of a new key past it fails
    /// `quota_exceeded`.
    public func setQuota(_ keys: Int) {
        quota = keys
    }

    /// The notification permission as the platform says it: `granted`, `denied` or
    /// `undetermined`.
    public func setPermission(_ state: String) {
        permission = state
    }

    /// What the user chooses when asked: `granted`, `denied`, or `none` for a host with no
    /// user to ask.
    public func setUserDecision(_ decision: String) {
        userDecision = decision
    }

    /// The store's keys and values, in key order.
    public func storeEntries() -> [(key: String, value: String)] {
        store.keys.sorted().map { (key: $0.text, value: store[$0] ?? "") }
    }

    /// The ids of the waiting notifications, in the order posted.
    public func waiting() -> [String] {
        waitingList.map(\.id)
    }

    /// The ids of the shown notifications, in the order posted.
    public func shown() -> [String] {
        shownList.map(\.id)
    }

    /// The starts the host was given since this was last called, oldest first.
    public func takeStarts() -> [HostStart] {
        let taken = starts
        starts = []
        return taken
    }

    /// Starts the operation of invocation `id` with `params`.
    ///
    /// Nil for a type that is not one of the standard host features: the host serves
    /// none of it. Otherwise the arguments are judged first (a refusal starts nothing),
    /// then a failure the embedding injected, then the operation itself.
    public func start(operation: String, id: String, params: HostParams) -> HostOutcome? {
        guard Self.types.contains(operation) else {
            return nil
        }
        starts.append(HostStart(operation: operation, id: id, params: params))
        let refused: Bool
        switch operation {
        case "sce-host-kv-get", "sce-host-kv-delete", "sce-host-kv-put":
            refused = !validKey(params["key"] ?? "")
        case "sce-host-notify-post", "sce-host-notify-cancel":
            refused = (params["id"] ?? "").isEmpty
        default:
            refused = false
        }
        if refused {
            return .refuse(invalidArgument)
        }
        if let code = faults[operation] {
            return failure(operation, code: code)
        }
        switch operation {
        case "sce-host-clock-now":
            return clockNow()
        case "sce-host-clock-zone":
            return clockZone()
        case "sce-host-kv-get":
            return kvGet(params["key"] ?? "")
        case "sce-host-kv-put":
            return kvPut(params["key"] ?? "", params["value"] ?? "")
        case "sce-host-kv-delete":
            return kvDelete(params["key"] ?? "")
        case "sce-host-kv-next-key":
            return kvNextKey(after: params["after"] ?? "", prefix: params["prefix"] ?? "")
        case "sce-host-notify-permission":
            return notifyPermission()
        case "sce-host-notify-request-permission":
            return notifyRequestPermission()
        case "sce-host-notify-post":
            return notifyPost(
                id: params["id"] ?? "",
                title: params["title"] ?? "",
                body: params["body"] ?? "",
                atMs: params["at_ms"] ?? ""
            )
        default:
            return notifyCancel(params["id"] ?? "")
        }
    }

    /// Tells the host to stop invocation `id`, because its state exited or its deadline
    /// passed. What it held for it is dropped.
    public func cancel(_ id: String) {
        cancelled.append(id)
    }

    // MARK: - Records

    /// A result record as the JSON a completion carries, its `error` empty.
    private func record(_ fields: PayloadPair...) -> HostOutcome {
        .done(payloadJSON(fields + [.text("error", "")]))
    }

    /// The failure of `operation`: its result record with the fields at their zero and
    /// `error` the code, so a machine reads the failure as a typed field.
    private func failure(_ operation: String, code: String) -> HostOutcome {
        let zero: [PayloadPair]
        switch operation {
        case "sce-host-clock-now":
            zero = [.signed64("utc_ms", 0), .signed64("monotonic_ms", 0)]
        case "sce-host-clock-zone":
            zero = [.text("zone_id", ""), .integer("utc_offset_s", Int32(0))]
        case "sce-host-kv-get":
            zero = [.truth("found", false), .text("value", "")]
        case "sce-host-kv-put":
            zero = [.truth("replaced", false)]
        case "sce-host-kv-delete", "sce-host-notify-cancel":
            zero = [.truth("existed", false)]
        case "sce-host-kv-next-key":
            zero = [.truth("found", false), .text("key", "")]
        case "sce-host-notify-permission", "sce-host-notify-request-permission":
            zero = [.text("state", "")]
        default:
            zero = [.text("status", "")]
        }
        return .done(payloadJSON(zero + [.text("error", code)]))
    }

    // MARK: - Clock

    private func clockNow() -> HostOutcome {
        guard let reading = clock.read() else {
            return failure("sce-host-clock-now", code: "unavailable")
        }
        // No answer goes below one already given.
        if let floor = monotonicFloor {
            monotonicFloor = max(floor, reading.monotonicMs)
        } else {
            monotonicFloor = reading.monotonicMs
        }
        return record(
            .signed64("utc_ms", reading.utcMs),
            .signed64("monotonic_ms", monotonicFloor ?? reading.monotonicMs)
        )
    }

    private func clockZone() -> HostOutcome {
        guard let zone = clock.zone() else {
            return failure("sce-host-clock-zone", code: "unavailable")
        }
        return record(.text("zone_id", zone.id), .integer("utc_offset_s", zone.utcOffsetS))
    }

    // MARK: - Key-value store

    /// A key holds no U+0000 and is not empty (its 256-byte bound is the record's own).
    private func validKey(_ key: String) -> Bool {
        !key.isEmpty && !key.utf8.contains(0)
    }

    private func kvGet(_ key: String) -> HostOutcome {
        if let value = store[StoreKey(text: key)] {
            return record(.truth("found", true), .text("value", value))
        }
        return record(.truth("found", false), .text("value", ""))
    }

    private func kvPut(_ key: String, _ value: String) -> HostOutcome {
        let slot = StoreKey(text: key)
        let replaced = store[slot] != nil
        if !replaced, let quota, store.count >= quota {
            return failure("sce-host-kv-put", code: "quota_exceeded")
        }
        store[slot] = value
        return record(.truth("replaced", replaced))
    }

    private func kvDelete(_ key: String) -> HostOutcome {
        let existed = store.removeValue(forKey: StoreKey(text: key)) != nil
        return record(.truth("existed", existed))
    }

    /// The first key, in ascending order of UTF-8 bytes, that is greater than `after` and
    /// begins with `prefix`; `after` is excluded, and an empty `after` starts at the least
    /// key.
    private func kvNextKey(after: String, prefix: String) -> HostOutcome {
        let floor = StoreKey(text: after)
        for key in store.keys.sorted() {
            guard after.isEmpty || floor < key else {
                continue
            }
            if key.text.utf8.starts(with: prefix.utf8) {
                return record(.truth("found", true), .text("key", key.text))
            }
        }
        return record(.truth("found", false), .text("key", ""))
    }

    // MARK: - Notifications

    /// Removes what `id` names from both lists; whether it named anything.
    private func withdraw(_ id: String) -> Bool {
        let named = { (item: PostedNotification) in item.id.utf8.elementsEqual(id.utf8) }
        let removed = waitingList.contains(where: named) || shownList.contains(where: named)
        waitingList.removeAll(where: named)
        shownList.removeAll(where: named)
        return removed
    }

    private func notifyPermission() -> HostOutcome {
        record(.text("state", permission))
    }

    private func notifyRequestPermission() -> HostOutcome {
        if permission == "undetermined" {
            // A host with no user to ask answers denied.
            permission = userDecision == "granted" ? "granted" : "denied"
        }
        return notifyPermission()
    }

    private func notifyPost(id: String, title: String, body: String, atMs: String) -> HostOutcome {
        guard permission == "granted" else {
            return record(.text("status", "denied"))
        }
        let at = Int64(atMs) ?? 0
        guard let reading = clock.read() else {
            return failure("sce-host-notify-post", code: "unavailable")
        }
        _ = withdraw(id)
        let item = PostedNotification(id: id, title: title, body: body)
        if at > reading.utcMs {
            waitingList.append(item)
            return record(.text("status", "scheduled"))
        }
        shownList.append(item)
        return record(.text("status", "posted"))
    }

    private func notifyCancel(_ id: String) -> HostOutcome {
        record(.truth("existed", withdraw(id)))
    }
}
