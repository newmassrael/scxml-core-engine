// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The timer HAL of the `timer` kind (SCE_FORGE.md Section 4.10): the interface a host implements
// once per platform, which schedules a callback against the platform's clock and event loop.
// Siblings: `com.sce.forge.runtime.Timer` (Kotlin) and the `Timer` trait (Rust).
//
// It is named `ForgeTimer` and not `Timer`: Foundation declares a `Timer` of its own, and a unit
// that imported both would have to say which one it means.

/// A clock a host provides: it calls `callback` after `delayMs`, or every `intervalMs`, until
/// it is cancelled.
public protocol ForgeTimer: AnyObject {
    /// Calls `callback` every `intervalMs` milliseconds until `cancel()`.
    func startPeriodic(intervalMs: Int64, callback: @escaping () -> Void)

    /// Calls `callback` once, after `delayMs` milliseconds, unless `cancel()` comes first.
    func startOneShot(delayMs: Int64, callback: @escaping () -> Void)

    /// Stops what was started. Idempotent: cancelling a timer that is not running does nothing.
    func cancel()
}
