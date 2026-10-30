// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The reference host of the standard host features (docs/SCE_HOST_FEATURES.md), as the
// operations a generated machine's host-run invokes ask for.
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

/// The standard host features' semantics, over a clock.
public final class ReferenceHost {
    /// Every operation type this host serves, in the order the specification lists them.
    public static let types = [
        "sce-host-clock-now",
        "sce-host-clock-zone",
    ]

    private let clock: any ClockSource

    /// Failures to inject: the operation type to the code it fails with.
    private var faults: [String: String] = [:]

    /// The invocations the host was told to stop, in the order told.
    public private(set) var cancelled: [String] = []

    /// The highest monotonic reading answered, which no answer goes below.
    private var monotonicFloor: Int64?

    private var starts: [HostStart] = []

    /// A host reading `clock`.
    public init(clock: any ClockSource) {
        self.clock = clock
    }

    /// Injects failures: each operation type named fails with the code it maps to, once
    /// the request has been accepted. An empty map ends them.
    public func setFaults(_ faults: [String: String]) {
        self.faults = faults
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
        if let code = faults[operation] {
            return failure(operation, code: code)
        }
        switch operation {
        case "sce-host-clock-now":
            return clockNow()
        default:
            return clockZone()
        }
    }

    /// Tells the host to stop invocation `id`, because its state exited or its deadline
    /// passed. What it held for it is dropped.
    public func cancel(_ id: String) {
        cancelled.append(id)
    }

    // MARK: - Records

    /// The failure of `operation`: its result record with the fields at their zero and
    /// `error` the code, so a machine reads the failure as a typed field.
    private func failure(_ operation: String, code: String) -> HostOutcome {
        switch operation {
        case "sce-host-clock-now":
            return .done(
                payloadJSON(.signed64("utc_ms", 0), .signed64("monotonic_ms", 0), .text("error", code))
            )
        default:
            return .done(
                payloadJSON(.text("zone_id", ""), .integer("utc_offset_s", Int32(0)), .text("error", code))
            )
        }
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
        return .done(
            payloadJSON(
                .signed64("utc_ms", reading.utcMs),
                .signed64("monotonic_ms", monotonicFloor ?? reading.monotonicMs),
                .text("error", "")
            )
        )
    }

    private func clockZone() -> HostOutcome {
        guard let zone = clock.zone() else {
            return failure("sce-host-clock-zone", code: "unavailable")
        }
        return .done(
            payloadJSON(.text("zone_id", zone.id), .integer("utc_offset_s", zone.utcOffsetS), .text("error", ""))
        )
    }
}
