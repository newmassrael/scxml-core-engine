// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Foundation
import Synchronization

// Centralized unique id generation.
//
// Ports Rust `unique_id_generator` from
// `backends/rust/runtime/src/helpers/unique_id_generator.rs`. Thread-safe id
// generation for sessions, sends, invokes, events and other identifiers.

/// Ensures uniqueness within the same millisecond.
private let globalCounter = Atomic<UInt64>(0)

/// Provides unique platform ids for invoke identifiers.
private let invokeCounter = Atomic<UInt64>(0)

/// The current time in milliseconds since the Unix epoch.
private func currentTimestampMillis() -> UInt64 {
    UInt64(Date().timeIntervalSince1970 * 1000)
}

/// The next value of the shared counter. The counter wraps and does not trap.
private func nextGlobalCounter() -> UInt64 {
    globalCounter.wrappingAdd(1, ordering: .relaxed).oldValue
}

/// A unique id with the given prefix.
///
/// The format is `{prefix}_{timestamp}{counter}`, the two numbers written one
/// after the other with nothing between them.
public func generateUniqueID(_ prefix: String) -> String {
    let timestamp = currentTimestampMillis()
    let counter = nextGlobalCounter()
    return "\(prefix)_\(timestamp)\(counter)"
}

/// A unique session id; `prefix` is `session` where none is given.
public func generateSessionID(prefix: String = "") -> String {
    generateUniqueID(prefix.isEmpty ? "session" : prefix)
}

/// A unique send id for event scheduling.
public func generateSendID() -> String {
    generateUniqueID("send")
}

/// A unique invoke id for SCXML invoke operations.
///
/// §scxml-6.4 (test 224): when `stateID` is given, the format is
/// `{stateID}.platform_{counter}` for compliance.
public func generateInvokeID(stateID: String = "") -> String {
    if stateID.isEmpty {
        return generateUniqueID("invoke")
    }
    return "\(stateID).platform_\(nextGlobalCounter())"
}

/// A unique event id for HTTP event processing.
public func generateEventID() -> String {
    generateUniqueID("event")
}

/// A unique correlation id for concurrent operations.
public func generateCorrelationID() -> String {
    generateUniqueID("corr")
}

/// A unique action id for SCXML actions; `prefix` is `action` where none is given.
public func generateActionID(prefix: String = "") -> String {
    generateUniqueID(prefix.isEmpty ? "action" : prefix)
}

/// A numeric session id, for legacy compatibility.
public func generateNumericSessionID() -> UInt64 {
    let timestamp = currentTimestampMillis()
    let counter = nextGlobalCounter()
    return timestamp &* 1000 &+ counter
}

/// A monotonically increasing counter for invoke id generation. It stands in for
/// Rust's `self as *const _ as usize`.
public func nextInvokeCounter() -> UInt64 {
    invokeCounter.wrappingAdd(1, ordering: .relaxed).newValue
}

/// Resets the shared counter, for tests only.
public func resetForTesting() {
    globalCounter.store(0, ordering: .relaxed)
}
