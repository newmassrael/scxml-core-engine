// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Foundation

/// Both readings of one instant.
public struct ClockReading: Equatable, Sendable {
    /// Milliseconds since 1970-01-01T00:00:00Z, POSIX time.
    public var utcMs: Int64

    /// Milliseconds since an origin fixed for the life of the process.
    public var monotonicMs: Int64

    public init(utcMs: Int64, monotonicMs: Int64) {
        self.utcMs = utcMs
        self.monotonicMs = monotonicMs
    }
}

/// The time zone a host reads local time in.
public struct ClockZone: Equatable, Sendable {
    /// An IANA identifier, or empty when only an offset is known.
    public var id: String

    /// Seconds east of UTC in force now.
    public var utcOffsetS: Int32

    public init(id: String, utcOffsetS: Int32) {
        self.id = id
        self.utcOffsetS = utcOffsetS
    }
}

/// Where a host's time comes from. A source that cannot be read answers nil, which the
/// host reports as `unavailable`.
public protocol ClockSource: AnyObject {
    func read() -> ClockReading?
    func zone() -> ClockZone?
}

/// The operating system's clock.
public final class SystemClockSource: ClockSource {
    private let origin = ContinuousClock.now

    public init() {}

    /// Both readings of the instant it is called at.
    public func read() -> ClockReading? {
        let elapsed = origin.duration(to: ContinuousClock.now)
        let (seconds, attoseconds) = elapsed.components
        let monotonicMs = seconds * 1000 + attoseconds / 1_000_000_000_000_000
        let utcMs = Int64((Date().timeIntervalSince1970 * 1000).rounded(.down))
        return ClockReading(utcMs: utcMs, monotonicMs: monotonicMs)
    }

    /// The offset in force now and the identifier of the zone the process runs in.
    public func zone() -> ClockZone? {
        let current = TimeZone.current
        return ClockZone(id: current.identifier, utcOffsetS: Int32(current.secondsFromGMT()))
    }
}

/// A clock a scenario sets: it reads what it was told. A nil member is a clock that
/// cannot be read.
public final class ScriptedClockSource: ClockSource {
    public var reading: ClockReading?
    public var zoneNow: ClockZone?

    /// A clock reading zero in no zone.
    public init() {
        reading = ClockReading(utcMs: 0, monotonicMs: 0)
        zoneNow = ClockZone(id: "", utcOffsetS: 0)
    }

    public func read() -> ClockReading? { reading }

    public func zone() -> ClockZone? { zoneNow }
}
