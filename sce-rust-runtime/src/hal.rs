// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! Hardware Abstraction Layer trait for the runtime's std-touching surface.
//!
//! Per watching-zenoh RFC §5.J.2 line 1984:
//!
//! > The scheduler, event queue, and external-event hooks are the primary
//! > `std`-touching surfaces and must be re-expressed against `core::` + a
//! > small HAL trait (ticks, wake, irq-save).
//!
//! This module defines that 3-method trait and the two default impls
//! [`StdHal`] (host build, used by every existing consumer of this crate)
//! and [`NoOpHal`] (no_std fallback that panics on `wake` / `irq_save` until
//! a downstream MCU integration crate wires real platform code).
//!
//! ## Crate boundary (per RFC Q-RustNoStd-4 (d), 2026-05-12 refresh)
//!
//! Spec §5.J.2 originally named `sce_intrinsics_runtime` as the trait owner,
//! anticipating a separate runtime crate. C4 (`b0b2a059` / `3d4792dc` /
//! `8bc8a6c3`, 2026-05-10) closed the `<sce:extern>` intrinsic chain with
//! build-time registries only; no `sce_intrinsics_runtime` runtime crate was
//! created and none is planned. The HAL trait therefore lives in
//! `sce-rust-runtime` (this crate), which is the natural home: the runtime
//! crate owns the trait its scheduler parameterizes, mirroring the
//! `embedded-hal` / `defmt-rtt` Rust-ecosystem convention.
//!
//! ## Engine integration
//!
//! [`crate::Engine`] is parameterized as `Engine<P, H: Hal = StdHal>`. The
//! default type parameter keeps every existing call site
//! (`Engine::new(policy)`, generated `Engine<MyPolicy>` template emissions)
//! source-compatible — `Engine<MyPolicy>` resolves to
//! `Engine<MyPolicy, StdHal>`. A future no_std consumer can pick
//! `Engine<MyPolicy, MyMcuHal>` once their `Hal` impl exists.
//!
//! Atomic A (this file) authors the trait and its two default impls; it
//! wires the trait into `Engine` via two consumer methods
//! ([`crate::Engine::hal_now_ticks_ms`], [`crate::Engine::hal_wake`]) so the
//! HAL is reachable from the public Engine API today. Atomic B will adopt
//! `heapless::*` collections and re-express `PullScheduler` against the HAL
//! tick source.

use core::marker::PhantomData;

/// The 3-method HAL trait the runtime parameterizes its std-touching surface on.
///
/// Per watching-zenoh RFC §5.J.2 line 1984 the trait surface is **ticks**,
/// **wake**, and **irq-save**. Each method is a `fn` (associated, not
/// instance) so the trait can be used as a zero-sized type parameter:
/// generated code emits `Engine<MyPolicy, StdHal>` and the compiler
/// monomorphizes the HAL calls inline.
///
/// ## Object safety
///
/// The trait is **not** object-safe (no `dyn Hal`) because:
///
/// - Methods take no `&self` (so a vtable receiver is undefined).
/// - [`irq_save`](Hal::irq_save) is generic over `F`, `R`.
///
/// This is intentional — HAL selection is a compile-time decision (matches
/// `embedded-hal`-family conventions). Consumers that need runtime swap
/// should erase through a wrapper of their own.
///
/// ## Required impl semantics
///
/// Implementors MUST satisfy:
///
/// 1. [`now_ticks_ms`](Hal::now_ticks_ms) — monotonic, non-decreasing across
///    calls within a single process lifetime. The epoch is implementation-
///    defined; only differences between two calls carry meaning.
/// 2. [`wake`](Hal::wake) — wakes any task blocked waiting for the runtime's
///    next macrostep. On single-threaded hosts this is a no-op; on no_std
///    targets it typically signals an embassy executor or sets an RTOS event.
/// 3. [`irq_save`](Hal::irq_save) — runs `f` with interrupts disabled (or
///    the platform-equivalent critical section). On single-threaded hosts
///    this just calls `f()` directly; on no_std targets it pairs with the
///    `cortex-m` crate's `interrupt::free` or `critical-section`.
pub trait Hal {
    /// Monotonic millisecond tick count (W3C SCXML 6.2 delay scheduling clock).
    ///
    /// The epoch is implementation-defined; only differences between calls
    /// have meaning. Per the spec's "ticks" mandate the resolution is
    /// milliseconds, matching the W3C SCXML `<send delay>` granularity.
    fn now_ticks_ms() -> u64;

    /// Signal the runtime's external-event waiter that work is available.
    ///
    /// On single-threaded std builds this is a no-op (the macrostep loop
    /// is driven explicitly by [`crate::Engine::tick`] /
    /// [`crate::Engine::step`]). No_std consumers wire this to their
    /// executor's task waker (e.g. embassy's `signal::Signal::signal()`
    /// or an RTOS event-flag set).
    fn wake();

    /// Execute `f` with interrupts disabled (or platform-equivalent
    /// critical section).
    ///
    /// On single-threaded std builds this is `f()` direct-pass — there is
    /// no shared mutable state across threads to protect, and `Engine` is
    /// `!Sync` by design (see [`crate::Engine`] threading-model doc). On
    /// no_std targets, this pairs with `cortex-m::interrupt::free` or the
    /// `critical-section` crate's primary entry point.
    fn irq_save<F, R>(f: F) -> R
    where
        F: FnOnce() -> R;
}

/// Default HAL impl for std hosts.
///
/// Used by `Engine<P, H = StdHal>` (the default type parameter). All
/// existing consumers of this crate get [`StdHal`] transparently because
/// the type parameter defaults — no call-site change is required for
/// `Engine::new(policy)` / `Engine<MyPolicy>`.
///
/// ## Implementation notes
///
/// - `now_ticks_ms` uses [`std::time::Instant`]'s monotonic-since-process-
///   start clock, milliseconds. Process-lifetime monotonicity is guaranteed
///   by the platform clock.
/// - `wake` is a no-op (single-threaded by design — see [`crate::Engine`]
///   threading-model doc).
/// - `irq_save` direct-passes `f()` — no critical section is needed because
///   `Engine` is `!Sync`.
#[derive(Debug, Clone, Copy, Default)]
pub struct StdHal {
    _private: PhantomData<()>,
}

impl Hal for StdHal {
    fn now_ticks_ms() -> u64 {
        // SAFETY: the EPOCH OnceLock is process-monotonic; Instant is
        // guaranteed monotonic-non-decreasing on every supported platform.
        // The conversion `Duration::as_millis() -> u128` is wider than `u64`
        // by design; we narrow with `as u64` accepting the wrap at ~584
        // million years, which is well beyond any conceivable process
        // lifetime.
        use std::sync::OnceLock;
        use std::time::Instant;

        static EPOCH: OnceLock<Instant> = OnceLock::new();
        let epoch = *EPOCH.get_or_init(Instant::now);
        Instant::now().saturating_duration_since(epoch).as_millis() as u64
    }

    fn wake() {
        // Single-threaded std build: the macrostep loop is driven by
        // explicit Engine::tick() / Engine::step() calls. No waker to
        // notify.
    }

    fn irq_save<F, R>(f: F) -> R
    where
        F: FnOnce() -> R,
    {
        // Single-threaded std build: Engine is !Sync, so no critical
        // section is needed. Direct-pass.
        f()
    }
}

/// No-op HAL impl that panics on `wake` / `irq_save` and reports zero ticks.
///
/// Intended for no_std builds where the consumer has not yet wired a real
/// HAL impl. The fail-loud panic on `wake` / `irq_save` follows
/// `feedback_silently_broken_hooks.md`: a no_std consumer that forgets to
/// provide a real HAL hits a clear panic rather than silently breaking
/// (e.g., the runtime would deadlock on a `wake()` that never fires).
///
/// `now_ticks_ms` returns 0 (rather than panicking) because read-only tick
/// queries are safe to stub — anything depending on monotonicity will
/// notice immediately. This mirrors the `embedded-hal-mock` pattern.
///
/// ## Migration path
///
/// A future no_std consumer authoring an MCU integration crate (e.g.
/// `sce-rust-runtime-cortex-m`) should implement [`Hal`] over their
/// platform primitives and pick `Engine<MyPolicy, MyHal>` rather than
/// leaning on `NoOpHal`. The latter exists only to make the no_std
/// surface compile without third-party deps.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoOpHal {
    _private: PhantomData<()>,
}

impl Hal for NoOpHal {
    fn now_ticks_ms() -> u64 {
        // Stub returns 0 monotonic ticks. Consumers depending on real time
        // will notice the staleness immediately; this is intentional —
        // silent zero is preferable to a panic on a read-only query.
        0
    }

    fn wake() {
        // Fail-loud: the consumer must wire a real HAL before the runtime
        // can be driven externally on no_std.
        panic!(
            "NoOpHal::wake() invoked — no_std consumer must provide a real Hal \
             impl (see sce-rust-runtime hal module docs); NoOpHal exists only \
             to make the no_std surface compile, not to run statecharts."
        );
    }

    fn irq_save<F, R>(_f: F) -> R
    where
        F: FnOnce() -> R,
    {
        // Fail-loud: same rationale as `wake`. A consumer doing scheduled
        // event delivery from an interrupt context needs a real critical
        // section, not an unprotected pass-through.
        panic!(
            "NoOpHal::irq_save() invoked — no_std consumer must provide a real \
             Hal impl (see sce-rust-runtime hal module docs); NoOpHal exists \
             only to make the no_std surface compile, not to run statecharts."
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn std_hal_ticks_monotonic_non_decreasing() {
        let t0 = StdHal::now_ticks_ms();
        let t1 = StdHal::now_ticks_ms();
        let t2 = StdHal::now_ticks_ms();
        assert!(t0 <= t1, "Hal::now_ticks_ms must be non-decreasing: {t0} > {t1}");
        assert!(t1 <= t2, "Hal::now_ticks_ms must be non-decreasing: {t1} > {t2}");
    }

    #[test]
    fn std_hal_ticks_observe_sleep_progress() {
        let t0 = StdHal::now_ticks_ms();
        std::thread::sleep(std::time::Duration::from_millis(20));
        let t1 = StdHal::now_ticks_ms();
        let elapsed = t1 - t0;
        // Sleep is not exact; require ≥ 5 ms to allow for clock resolution
        // jitter on slower hosts (e.g. CI macOS runners).
        assert!(
            elapsed >= 5,
            "20 ms sleep should report ≥ 5 ticks via StdHal; got {elapsed}"
        );
    }

    #[test]
    fn std_hal_wake_is_noop() {
        // StdHal::wake() must not panic and must not change observable state.
        StdHal::wake();
        StdHal::wake();
        StdHal::wake();
    }

    #[test]
    fn std_hal_irq_save_runs_closure() {
        let mut counter = 0;
        StdHal::irq_save(|| {
            counter += 1;
        });
        StdHal::irq_save(|| {
            counter += 10;
        });
        assert_eq!(counter, 11);
    }

    #[test]
    fn std_hal_irq_save_returns_closure_value() {
        let v = StdHal::irq_save(|| 42_u32);
        assert_eq!(v, 42);
    }

    #[test]
    fn std_hal_irq_save_propagates_panic() {
        // The critical section MUST propagate panics from the inner
        // closure — silent suppression would hide bugs in production.
        let result = std::panic::catch_unwind(|| {
            StdHal::irq_save(|| {
                panic!("inner-closure-panic");
            });
        });
        let payload = result.expect_err("closure panic should propagate out of irq_save");
        let msg = payload
            .downcast_ref::<&'static str>()
            .copied()
            .unwrap_or("<not a &'static str>");
        assert_eq!(msg, "inner-closure-panic");
    }

    #[test]
    fn std_hal_is_zero_sized() {
        // Zero-sized type — the type parameter on Engine<P, H> must not
        // grow the engine struct.
        assert_eq!(core::mem::size_of::<StdHal>(), 0);
    }

    #[test]
    fn no_op_hal_ticks_returns_zero() {
        assert_eq!(NoOpHal::now_ticks_ms(), 0);
        assert_eq!(NoOpHal::now_ticks_ms(), 0);
    }

    #[test]
    #[should_panic(expected = "NoOpHal::wake() invoked")]
    fn no_op_hal_wake_panics_loud() {
        NoOpHal::wake();
    }

    #[test]
    #[should_panic(expected = "NoOpHal::irq_save() invoked")]
    fn no_op_hal_irq_save_panics_loud() {
        NoOpHal::irq_save(|| 0_u32);
    }

    #[test]
    fn no_op_hal_is_zero_sized() {
        assert_eq!(core::mem::size_of::<NoOpHal>(), 0);
    }

    #[test]
    fn std_hal_default_constructible() {
        // The struct must be `Default` so consumers can instantiate when
        // they need a typed value (most consumers won't — the type alone
        // suffices since methods are associated). Tests this compiles +
        // returns a unit-shaped value.
        let _h = StdHal::default();
        let _h = NoOpHal::default();
    }
}
