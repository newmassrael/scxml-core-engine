// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// §scxml-6.2: Send action helpers (target validation, routing classification,
// delay parsing).
//
// Ports Rust send helpers from `backends/rust/runtime/src/helpers/send.rs`. Every
// comparison here is by UTF-8 bytes, as on every other backend: a Swift `String`
// compares by canonical equivalence, and a target is an address, not prose.

/// Whether `target` is invalid (starts with `!`) (§scxml-6.2).
public func isInvalidTarget(_ target: String) -> Bool {
    target.utf8.starts(with: "!".utf8)
}

/// Whether `target` uses the internal event queue (§scxml-C-1). Target
/// `#_internal` routes events to the internal queue (high priority).
public func isInternalTarget(_ target: String) -> Bool {
    target.utf8.elementsEqual(internalTarget.utf8)
}

/// Whether `target` is a child invoke session (§scxml-6.4). Targets matching
/// `#_{invokeid}` (but not `#_parent`, `#_internal` or `#_scxml_*`) are child
/// invoke targets.
public func isChildInvokeTarget(_ target: String) -> Bool {
    if !target.utf8.starts(with: invokeTargetPrefix.utf8) {
        return false
    }
    // Exclude the special reserved targets.
    if target.utf8.elementsEqual(parentTarget.utf8) || target.utf8.elementsEqual(internalTarget.utf8) {
        return false
    }
    // Exclude SCXML session targets.
    if target.utf8.starts(with: scxmlSessionTargetPrefix.utf8) {
        return false
    }
    return true
}

/// The invoke id of a child target (§scxml-6.4). Given `#_{invokeid}`, returns
/// `{invokeid}`.
public func extractInvokeID(_ target: String) -> String {
    if target.utf8.starts(with: invokeTargetPrefix.utf8) {
        return String(decoding: Array(target.utf8).dropFirst(invokeTargetPrefix.utf8.count), as: UTF8.self)
    }
    return target
}

/// Whether `target` is an HTTP URL (§scxml-C-2).
public func isHTTPTarget(_ target: String) -> Bool {
    target.utf8.starts(with: "http://".utf8) || target.utf8.starts(with: "https://".utf8)
}

/// A refusal of a `<send>` argument, as a sentence for the `error.execution`'s
/// data.
public struct SendArgumentError: Error, Equatable, CustomStringConvertible {
    public var message: String

    public init(_ message: String) {
        self.message = message
    }

    public var description: String {
        message
    }
}

/// Validates a send target (§scxml-6.2). Throws when it is invalid, in which case
/// `error.execution` should be raised.
public func validateTarget(_ target: String) throws {
    if isInvalidTarget(target) {
        throw SendArgumentError("invalid target value: \(target)")
    }
}

/// Whether `target` is unreachable (§scxml-C-1). An empty or `undefined` target
/// indicates an unreachable session, requiring `error.communication`.
public func isUnreachableTarget(_ target: String) -> Bool {
    target.isEmpty || target.utf8.elementsEqual("undefined".utf8)
}

/// The peer a `<send target>` names, when it names one: `#` followed by at least
/// one character, where `#_` stays reserved for the targets §scxml-6.2.4 defines
/// (`#_internal`, `#_parent`, ...).
///
/// Every copy of the predicate reads `tests/mesh/mesh_target_cases.json`, so a
/// target one of them routes over Mesh is one they all do.
public func meshPeer(_ target: String) -> String? {
    var bytes = Array(target.utf8)
    guard bytes.first == UInt8(ascii: "#") else {
        return nil
    }
    bytes.removeFirst()
    if bytes.isEmpty || bytes.first == UInt8(ascii: "_") {
        return nil
    }
    return String(decoding: bytes, as: UTF8.self)
}

/// Whether a `<send target>` names a Mesh peer (see `meshPeer`).
public func isMeshTarget(_ target: String) -> Bool {
    meshPeer(target) != nil
}

/// Where a `<send>` to the SCXML Event I/O Processor goes (§scxml-6.2.4, §scxml-C-1).
public enum SendTargetKind: Sendable, Equatable {
    /// The sending session's own external queue.
    case selfExternal

    /// `#_internal`: the sending session's internal queue.
    case internalQueue

    /// `#_parent`: the session that invoked this one.
    case parent

    /// A session named by `#_scxml_<id>` or its published location.
    case session

    /// `#_<invokeid>`: an invocation of the sending session.
    case invocation

    /// `#<name>`: a machine an SCE Mesh deployment binds.
    case mesh

    /// A value naming no session: `error.communication` (§scxml-C-1).
    case unreachable

    /// A value this processor cannot address: `bogus`, `!invalid`, or a URI of any
    /// scheme but its own locations, an http(s) URL included, which only a
    /// BasicHTTP send reaches: `error.execution` (§scxml-6.2.4).
    case unsupported
}

/// A classified target value: its kind, and the session id (`.session`) or invoke
/// id (`.invocation`) it names.
public struct SendTarget: Sendable, Equatable {
    public var kind: SendTargetKind
    public var id: String

    public init(kind: SendTargetKind, id: String = "") {
        self.kind = kind
        self.id = id
    }
}

/// Reads a target value (§scxml-6.2.4, §scxml-C-1).
///
/// The table C++ `SendHelper::classifyTarget` holds, and every channel's: a
/// `target` written in the document and a `targetexpr` evaluated at run time are
/// the same value and go to the same place, sent at once or after a delay. A URI of
/// another scheme (C++'s `Uri` and `Http`, which its Interpreter's target factory
/// may serve) is `.unsupported` here, as it is for every generated machine.
public func classifyTarget(_ target: String, ownSessionID: String) -> SendTarget {
    if isUnreachableTarget(target) {
        return SendTarget(kind: .unreachable)
    }
    if isInvalidTarget(target) {
        return SendTarget(kind: .unsupported)
    }
    if isInternalTarget(target) {
        return SendTarget(kind: .internalQueue)
    }
    if target.utf8.elementsEqual(parentTarget.utf8) {
        return SendTarget(kind: .parent)
    }
    let bytes = Array(target.utf8)
    let sessionPrefix = Array(scxmlSessionTargetPrefix.utf8)
    if bytes.starts(with: sessionPrefix) {
        let sessionID = String(decoding: bytes.dropFirst(sessionPrefix.count), as: UTF8.self)
        if sessionID.isEmpty || sessionID.utf8.elementsEqual(ownSessionID.utf8) {
            return SendTarget(kind: .selfExternal)
        }
        return SendTarget(kind: .session, id: sessionID)
    }
    let located = sessionIDFromScxmlLocation(target)
    if !located.isEmpty {
        if located.utf8.elementsEqual(ownSessionID.utf8) {
            return SendTarget(kind: .selfExternal)
        }
        return SendTarget(kind: .session, id: located)
    }
    if isChildInvokeTarget(target) {
        return SendTarget(kind: .invocation, id: extractInvokeID(target))
    }
    if isMeshTarget(target) {
        return SendTarget(kind: .mesh)
    }
    return SendTarget(kind: .unsupported)
}

/// Whether a send type requires a target attribute (§scxml-C-2). The BasicHTTP
/// Event I/O Processor requires a target URL.
public func requiresTargetAttribute(sendType: String) -> Bool {
    sendType.utf8.elementsEqual(basicHTTPEventProcessorType.utf8)
}

/// Whether a send type is supported (§scxml-6.2). Supported types: the SCXML Event
/// Processor (default) and the BasicHTTP Event Processor.
public func isSupportedSendType(_ sendType: String) -> Bool {
    sendType.isEmpty
        || sendType.utf8.elementsEqual(scxmlEventProcessorType.utf8)
        || sendType.utf8.elementsEqual(basicHTTPEventProcessorType.utf8)
}

/// Validates BasicHTTP send parameters (§scxml-C-2). Throws when a target is
/// required but missing.
public func validateBasicHTTPSend(sendType: String, target: String, targetExpr: String) throws {
    if requiresTargetAttribute(sendType: sendType) && target.isEmpty && targetExpr.isEmpty {
        throw SendArgumentError("BasicHTTPEventProcessor requires target attribute")
    }
}

/// The largest delay any engine can hold, in milliseconds: the positive range of a
/// signed 64-bit count, because Kotlin's `Long` is signed and every engine answers
/// the same text the same way.
public let maxDelayMs = UInt64(Int64.max)

/// Reads a `<send>` delay as the CSS2 time §scxml-6.2 names.
///
/// The grammar is ARCHITECTURE.md's "Durations (Single Source of Truth)":
/// surrounding ASCII whitespace aside, a non-negative number (digits with an
/// optional fraction of at least one digit, or a leading "." and digits; no sign,
/// no exponent) followed directly by "ms" or "s", either case. The milliseconds are
/// computed in exact decimal and truncated, never through a float.
/// `tests/durations/css2_time.json` holds the cases every engine is measured
/// against.
///
/// Returns nil when the text is not a time (a bare number included), so the caller
/// raises the argument error and does not choose a wait.
public func parseDelayToMs(_ text: String) -> UInt64? {
    let isSpace = { (byte: UInt8) in byte == 0x20 || (byte >= 0x09 && byte <= 0x0D) }
    var all = ArraySlice(text.utf8)
    while let first = all.first, isSpace(first) {
        all = all.dropFirst()
    }
    while let last = all.last, isSpace(last) {
        all = all.dropLast()
    }
    let lower = { (byte: UInt8) in byte >= 0x41 && byte <= 0x5A ? byte + 0x20 : byte }

    let number: ArraySlice<UInt8>
    let scale: UInt64
    if all.count >= 2, lower(all[all.endIndex - 2]) == UInt8(ascii: "m"), lower(all[all.endIndex - 1]) == UInt8(ascii: "s") {
        number = all.dropLast(2)
        scale = 1
    } else if all.count >= 1, lower(all[all.endIndex - 1]) == UInt8(ascii: "s") {
        number = all.dropLast(1)
        scale = 1000
    } else {
        return nil
    }

    let point = number.firstIndex(of: UInt8(ascii: "."))
    let whole = point.map { number[number.startIndex..<$0] } ?? number
    let fraction = point.map { number[($0 + 1)...] } ?? ArraySlice<UInt8>()
    let allDigits = { (part: ArraySlice<UInt8>) in part.allSatisfy { $0 >= 0x30 && $0 <= 0x39 } }

    // A number is digits, digits "." digits, or "." digits: the fraction is never
    // empty, and there is at least one digit somewhere.
    if !allDigits(whole) || (point != nil && (fraction.isEmpty || !allDigits(fraction))) {
        return nil
    }
    if whole.isEmpty && point == nil {
        return nil
    }
    var ms: UInt64 = 0
    for byte in whole {
        let digit = UInt64(byte - 0x30)
        if ms > (maxDelayMs - digit) / 10 {
            return nil
        }
        ms = ms * 10 + digit
    }
    if ms > maxDelayMs / scale {
        return nil
    }
    ms *= scale
    // Only the fraction digits that name whole milliseconds count; the rest
    // truncate.
    var place = scale / 10
    for byte in fraction where place > 0 {
        let add = UInt64(byte - 0x30) * place
        if ms > maxDelayMs - add {
            return nil
        }
        ms += add
        place /= 10
    }
    return ms
}
