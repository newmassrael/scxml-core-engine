// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
/// ## The default depends on the platform
///
/// An engine that is never given a clock reads [`SceClock::Hal`] where the
/// platform has an operating-system clock, and starts on
/// [`SceClock::Manual(0)`](SceClock::Manual) where it has none
/// (`wasm32-unknown-unknown`: a browser module is handed no clock by its
/// target, only by its host). A default that read a clock the platform does not
/// have would not be a default but a trap that fires at the first `initialize`,
/// so there time is the host's from the first call: it moves the engine with
/// [`Engine::advance_time_ms`](crate::Engine::advance_time_ms), or installs its
/// own reading with [`SceClock::Source`] through
/// [`Engine::set_clock`](crate::Engine::set_clock) before `initialize`.
///
/// Deliberately not `PartialEq`: two [`SceClock::Source`]s are the same clock
/// when they read the same time source, and comparing the function pointers
/// answers a different question — Rust does not guarantee that two pointers to
/// one function are equal, nor that two pointers to different functions are
/// not. A host asking "which kind of clock is this" wants `matches!`.
///
/// ```
/// # use sce_rust_runtime::SceClock;
/// // The default where the platform has a clock: read the policy's HAL, which
/// // is the host's monotonic clock.
/// # #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
/// assert!(matches!(SceClock::default(), SceClock::Hal));
/// // Host-owned time — deterministic, and the only kind `advance_time_ms` moves.
/// assert!(matches!(SceClock::Manual(0), SceClock::Manual(0)));
/// ```
#[derive(Debug, Clone, Copy)]
pub enum SceClock {
    /// Read `<P::Hal as Hal>::now_ticks_ms()` — the host's monotonic wall
    /// clock under [`StdHal`](crate::StdHal), and whatever tick source an
    /// embedded consumer wired otherwise.
    ///
    /// The default wherever the platform has a clock to read, and what a
    /// production host there wants. On a platform with none, [`StdHal`](crate::StdHal)
    /// cannot answer and asking for this clock stops the engine at the first
    /// reading with a message that names the alternatives.
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

impl SceClock {
    /// The clock an engine starts on, given whether its platform has an
    /// operating-system clock for [`StdHal`](crate::StdHal) to read.
    ///
    /// A function of its argument rather than of `cfg!` so that both answers
    /// can be tested on a host that has a clock; [`Default`] supplies the
    /// argument for the platform being compiled for.
    pub(crate) const fn start_clock(os_clock_present: bool) -> Self {
        if os_clock_present {
            SceClock::Hal
        } else {
            SceClock::Manual(0)
        }
    }
}

impl Default for SceClock {
    /// [`SceClock::Hal`] where the platform has a clock, [`SceClock::Manual`]
    /// at zero where it has none; see the type's documentation.
    fn default() -> Self {
        Self::start_clock(cfg!(not(all(
            target_arch = "wasm32",
            target_os = "unknown"
        ))))
    }
}

#[cfg(test)]
mod tests {
    use super::SceClock;

    #[test]
    fn a_platform_with_a_clock_starts_on_it() {
        assert!(matches!(SceClock::start_clock(true), SceClock::Hal));
    }

    #[test]
    fn a_platform_without_one_starts_on_host_owned_time_at_zero() {
        assert!(matches!(SceClock::start_clock(false), SceClock::Manual(0)));
    }

    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    #[test]
    fn the_default_on_a_host_with_a_clock_is_the_hal() {
        assert!(matches!(SceClock::default(), SceClock::Hal));
    }

    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    #[test]
    fn the_default_on_a_target_with_no_clock_is_host_owned_time() {
        assert!(matches!(SceClock::default(), SceClock::Manual(0)));
    }
}
