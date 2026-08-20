// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The engine's source of "now" for `<send delay>`.

/// Where an [`Engine`](crate::Engine) reads the time it measures every
/// `<send delay>` from.
///
/// §scxml-6.2.2 says a delay "indicates how long the processor should wait
/// before dispatching the message", and says nothing about where the processor
/// reads the time from. Leaving that hardwired to the wall answers a question
/// the spec left to the host, and answers it the one way that cannot be
/// reproduced.
///
/// ## Why this is not [`Hal`](crate::Hal)
///
/// [`Hal::now_ticks_ms`](crate::Hal::now_ticks_ms) is an associated function
/// reached through `P::Hal`, so the clock a generated machine reads is fixed
/// when the machine is *compiled*. That is the right shape for the platform
/// primitives the HAL exists for — a firmware image has one tick source — but
/// it means one generated artifact cannot serve both a host on the wall clock
/// and a host that owns time, because they would need two policies. This enum
/// is an ordinary field on the engine instead: the same generated machine
/// takes either, chosen at run time, and [`SceClock::Hal`] is the default that
/// keeps existing call sites reading exactly what they read before.
///
/// It stays `Copy`, allocation-free and `dyn`-free so the no_std profile keeps
/// the surface the HAL was introduced for.
///
/// Deliberately not `PartialEq`: two [`SceClock::Source`]s are the same clock
/// when they read the same time source, and comparing the function pointers
/// answers a different question — Rust does not guarantee that two pointers to
/// one function are equal, nor that two pointers to different functions are
/// not. A host asking "which kind of clock is this" wants `matches!`.
///
/// ```
/// # use sce_rust_runtime::SceClock;
/// // The default: read the policy's HAL, which is the host's monotonic clock.
/// assert!(matches!(SceClock::default(), SceClock::Hal));
/// // Host-owned time — deterministic, and the only kind `advance_time_ms` moves.
/// assert!(matches!(SceClock::Manual(0), SceClock::Manual(0)));
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub enum SceClock {
    /// Read `<P::Hal as Hal>::now_ticks_ms()` — the host's monotonic wall
    /// clock under [`StdHal`](crate::StdHal), and whatever tick source an
    /// embedded consumer wired otherwise.
    ///
    /// The default, and what a production host wants.
    #[default]
    Hal,
    /// Host-owned time, in milliseconds since an origin of the host's
    /// choosing. The engine's "now" is exactly this value and moves only when
    /// [`Engine::advance_time_ms`](crate::Engine::advance_time_ms) moves it.
    ///
    /// A machine driven this way reaches the same configuration on every run
    /// regardless of the load on the machine it runs on, which is what a
    /// simulation, a replay, a discrete-event scheduler and a deterministic
    /// test all need.
    Manual(u64),
    /// A reading function the host supplies, returning milliseconds since an
    /// origin of its choosing.
    ///
    /// For a host whose time source is neither the policy's HAL nor its own
    /// bookkeeping — an RTOS tick counter reached through a C symbol, a media
    /// clock, a simulation running faster than real time. Must be
    /// non-decreasing, for the reason on [`Engine::now_ms`](crate::Engine::now_ms).
    ///
    /// A plain `fn` pointer rather than a closure so the variant stays `Copy`
    /// and allocation-free; a host needing captured state puts it behind the
    /// function itself.
    Source(fn() -> u64),
}
