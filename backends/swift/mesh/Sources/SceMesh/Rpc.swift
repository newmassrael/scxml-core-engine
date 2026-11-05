// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The requester's half of `<invoke type="sce:mesh-rpc">` (SCE_MESH.md §mesh-9.5): the correlation
// table a request is kept in until its reply, its deadline or its cancellation retires it, and the
// `_event.data` the engine is handed when it ends in an error. The Swift twin of backends/kotlin/mesh
// Rpc.kt and backends/rust/mesh/src/rpc.rs, byte for byte.
//
// The engine already carries the SCXML invocation: started once, cancelled when its state exits,
// ended by exactly one `done.invoke.<id>` or `error.invoke.<id>` (§mesh-19). What this adds is what
// only the router knows: which wire id answers which invocation, who may answer it (§mesh-14.6),
// and when it stops waiting.

import SceRuntime

/// The `<param>` a lowered Mesh request carries its event name in.
public let meshEventParam = "_mesh_event"

/// The `<param>` a lowered Mesh request carries its own deadline in, when the document gave one
/// (§mesh-9.5).
public let meshDeadlineParam = "_mesh_deadline_ms"

/// One request waiting for its answer.
struct Pending: Equatable {
    /// The SCXML invoke id the answer ends (`done.invoke.<id>`).
    var invokeID: String
    /// Which start of that invoke this is (the engine's token).
    var token: UInt64
    /// The peer it was sent to, whose link carries its reply.
    var target: String
    /// The machines whose reply may answer it (§mesh-14.6).
    var responders: [String]
    /// The monotonic time it stops waiting, if it has a deadline.
    var expiresMs: Int64?
}

/// How a reply that names a live request stands against it.
enum Answer: Equatable {
    /// No request is waiting on that id: answered, cancelled or expired already, so what arrives
    /// now is dropped.
    case unknown

    /// The request is waiting and the binding the reply arrived on may answer it.
    case admitted

    /// The request is waiting but that binding is outside its responder set (§mesh-16.7 row 14); it
    /// stays answerable.
    case undeclared
}

/// Every request this router has sent and not yet retired, keyed by the wire `invoke_id` the
/// requester minted for it, as RFC 4122 text (§mesh-9.5: not the SCXML invoke id, which never
/// crosses the wire).
final class Correlation {
    private var pending: [String: Pending] = [:]

    func register(wireID: [UInt8], _ request: Pending) {
        pending[uuidText(wireID)] = request
    }

    /// Whether a reply for `wireID` that arrived on `peer` may retire it. The binding a reply arrived
    /// on identifies its responder, never the envelope's `source`, which the sender writes
    /// (§mesh-9.5).
    func check(wireID: [UInt8], peer: String) -> Answer {
        guard let request = pending[uuidText(wireID)] else {
            return .unknown
        }
        return request.responders.contains(peer) ? .admitted : .undeclared
    }

    /// Whether a request is still waiting on `wireID`.
    func isWaiting(wireID: [UInt8]) -> Bool {
        pending[uuidText(wireID)] != nil
    }

    /// Retire the request `wireID`, returning it if it was still waiting.
    func retire(wireID: [UInt8]) -> Pending? {
        pending.removeValue(forKey: uuidText(wireID))
    }

    /// Forget the request for the start `token` of `invokeID`: its state exited, and nothing goes on
    /// the wire (§mesh-9.5).
    func cancel(invokeID: String, token: UInt64) {
        for (key, request) in pending where request.invokeID == invokeID && request.token == token {
            pending.removeValue(forKey: key)
        }
    }

    /// Forget every request sent to `peer`, whose link is gone, in wire-id order: no reply can come
    /// back on it (§mesh-16.7 row 5).
    func forgetSentTo(_ peer: String) -> [String] {
        let lost = pending.filter { $0.value.target == peer }.map(\.key).sorted()
        for key in lost {
            pending.removeValue(forKey: key)
        }
        return lost
    }

    /// Retire every request whose deadline is at or before `nowMs`, in wire-id order: the order the
    /// Rust core's map keeps, since lowercase hex sorts as the bytes do.
    func expire(nowMs: Int64) -> [(wireID: String, pending: Pending)] {
        let expired = pending.filter { entry in
            guard let expires = entry.value.expiresMs else { return false }
            return expires <= nowMs
        }.map(\.key).sorted()
        return expired.compactMap { key in
            pending.removeValue(forKey: key).map { (wireID: key, pending: $0) }
        }
    }
}

/// The `_event.data` of an `error.invoke.<id>` (§mesh-10.7.1, `errorName: "invoke"`): `status` by
/// the name `rpc_status.scxml` declares for it, the reply's message as `detail`, the machine that
/// answered as `source` (absent for a deadline the requester synthesised) and the wire `invoke_id`.
public func invokeErrorData(status: RpcStatus, detail: String?, source: String?, wireIDText: String) -> String {
    var json = ErrorData(errorName: "invoke", reason: status.declaredName)
    if let detail {
        json.text("detail", detail)
    }
    if let source {
        json.text("source", source)
    }
    json.text("invoke_id", wireIDText)
    return json.finish()
}

/// The `_event.data` of the `error.execution` a request that cannot reach the wire raises instead
/// of starting (§mesh-9.5's pre-envelope tier).
public func srcNotFoundData(_ detail: String) -> String {
    var json = ErrorData(errorName: "execution", reason: "INVOKE_SRC_NOT_FOUND")
    json.text("detail", detail)
    return json.finish()
}

private let hexDigits = Array("0123456789abcdef".utf8)

/// A 16-byte `id`, an envelope's or a request's wire `invoke_id`, as RFC 4122 section 3's canonical
/// text (`xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`, lowercase): the form §mesh-10.7 gives
/// `_event.invokeid` and §mesh-10.7.1 an `invoke_id` field, and the one the C++ core writes
/// (`SCE::uuid::to_string`), so a document cannot tell which core it runs beside.
public func uuidText(_ id: [UInt8]) -> String {
    var out: [UInt8] = []
    out.reserveCapacity(36)
    for (index, byte) in id.enumerated() {
        if index == 4 || index == 6 || index == 8 || index == 10 {
            out.append(UInt8(ascii: "-"))
        }
        out.append(hexDigits[Int(byte >> 4)])
        out.append(hexDigits[Int(byte & 0x0F)])
    }
    return String(decoding: out, as: UTF8.self)
}

/// The id a request's `_event.invokeid` names: `uuidText` read back, either case, as the C++ core's
/// `SCE::uuid::from_string` reads it. Nil for text that is not one: an event raised by something
/// other than a Mesh request.
public func parseUUIDText(_ text: String) -> [UInt8]? {
    let bytes = Array(text.utf8)
    guard bytes.count == 36 else { return nil }
    var digits: [UInt8] = []
    digits.reserveCapacity(32)
    for (index, byte) in bytes.enumerated() {
        if index == 8 || index == 13 || index == 18 || index == 23 {
            guard byte == UInt8(ascii: "-") else { return nil }
        } else {
            digits.append(byte)
        }
    }
    func value(_ digit: UInt8) -> UInt8? {
        switch digit {
        case UInt8(ascii: "0")...UInt8(ascii: "9"): digit - UInt8(ascii: "0")
        case UInt8(ascii: "a")...UInt8(ascii: "f"): digit - UInt8(ascii: "a") + 10
        case UInt8(ascii: "A")...UInt8(ascii: "F"): digit - UInt8(ascii: "A") + 10
        default: nil
        }
    }
    var out = [UInt8](repeating: 0, count: 16)
    for index in 0..<16 {
        guard let high = value(digits[2 * index]), let low = value(digits[2 * index + 1]) else {
            return nil
        }
        out[index] = (high << 4) | low
    }
    return out
}

/// A §mesh-10.7.1 `_event.data` object, written in the order its fields are added: `errorName` and
/// `reason` first, then what the caller adds.
struct ErrorData {
    private var fields: [String] = []

    init(errorName: String, reason: String) {
        text("errorName", errorName)
        text("reason", reason)
    }

    mutating func text(_ key: String, _ value: String) {
        fields.append("\"\(key)\":" + quoteJSON(value))
    }

    mutating func number(_ key: String, _ value: some BinaryInteger) {
        fields.append("\"\(key)\":\(value)")
    }

    func finish() -> String {
        "{" + fields.joined(separator: ",") + "}"
    }
}
