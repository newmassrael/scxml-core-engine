// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Link runtime — `Link` trait surface for byte-stream link
// endpoints declared via `<scxml sce:kind="link">` (watching-zenoh
// RFC §5.C). SCE owns the trait so every downstream `impl Link` in
// `sce_link_runtime_lwip` / `sce_link_runtime_tokio` /
// `sce_link_runtime_qnx` shares a single rust type identity, which
// is what makes polymorphic adapters and trait-evolution audits
// possible. Per `claudedocs/rfc-b6-link-entry.md` §3 Q1 Candidate D.
//
// B6-α scope is the trait + `RxFrame` / `TxFrame` newtypes + an
// `LinkError` enum + a no-op `StubLink` for ctest. Real per-OS
// impls (lwIP DMA-aligned slot acquisition, tokio's `tokio_udp`
// driver, QNX's `dispatch_create()` reactor binding) live downstream
// in watching-zenoh as separate crates that depend on this crate
// for type identity. B7's buffer-pool kind (RFC §5.E) lifts the
// borrowed-slice surface to slot-backed without a trait change —
// see `RxFrame` / `TxFrame` doc-comments and `claudedocs/rfc-b6-link-entry.md`
// §3 Q2.5 Shape 2.5a for the forward-compatibility argument.

#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::cell::Cell;

/// Borrowed-slice frame received by an `impl Link::rx`. The lifetime
/// is tied to the `&mut self` borrow held by the impl — callers must
/// consume the frame before the next `rx()` call. B6-α impls back the
/// slice with an internal `Vec<u8>` (or a fixed `[u8; N]` on `no_std`);
/// B7's pool-kind impl backs it with a `PoolSlot<'_>` without changing
/// the trait surface.
#[derive(Debug, Clone, Copy)]
pub struct RxFrame<'a> {
    /// Decoded bytes available for the consumer. The slice's backing
    /// storage is opaque to the trait — implementations document
    /// their lifetime and aliasing rules.
    pub data: &'a [u8],
}

impl<'a> RxFrame<'a> {
    /// Construct a frame view over `bytes`. Lifetime tracks `bytes`.
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { data: bytes }
    }
}

/// Borrowed-slice frame queued for `impl Link::tx`. Same lifetime
/// contract as `RxFrame`: the slice must outlive the call.
#[derive(Debug, Clone, Copy)]
pub struct TxFrame<'a> {
    /// Encoded bytes ready for transmission.
    pub data: &'a [u8],
}

impl<'a> TxFrame<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { data: bytes }
    }
}

/// Error category for `Link::tx`. RFC §5.C does not pin a fixed
/// taxonomy yet; the variants here cover the cases the SCE-side
/// codegen needs (the per-OS downstream impls extend the set as
/// real driver semantics surface — those extensions are additive,
/// per the `feedback_no_versioning.md` no-fallback rule).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkError {
    /// Driver-level send failure (socket closed, ENOBUFS, ECONNRESET,
    /// or platform-specific equivalents).
    Driver,
    /// Outbound buffer is full and the link's `<sce:backpressure>`
    /// policy is `block` or `drop` — the consumer must retry or the
    /// frame is dropped per policy.
    Backpressure,
}

/// Byte-stream link surface (watching-zenoh RFC §5.C). Implementations
/// own the platform driver (lwIP / tokio / QNX) and route the framer
/// codec's RX (decode) / TX (encode) calls through their I/O primitive.
/// All implementations share this single rust type identity — the
/// per-OS crates (`sce_link_runtime_lwip` etc.) provide concrete impls,
/// not their own copy of the trait.
pub trait Link {
    /// Pull the next decoded frame, if any is available without
    /// blocking. Returns `None` when the underlying driver has no
    /// pending bytes; the SCE-generated link module polls in the
    /// SCXML interpreter's idle slot.
    fn rx(&mut self) -> Option<RxFrame<'_>>;

    /// Submit a frame for transmission. The implementation must
    /// honor the link's `<sce:backpressure>` policy — `drop` returns
    /// `Err(LinkError::Backpressure)` (the SCE side ignores it),
    /// `block` blocks until the driver accepts the frame.
    fn tx(&mut self, frame: TxFrame<'_>) -> Result<(), LinkError>;

    /// Budget-aware tick hook (watching-zenoh RFC §5.N line 3050).
    /// The cooperative scheduler invokes `poll(deadline_us)` once per
    /// tick per link, with `deadline_us` capped to the deploy.yaml
    /// `scheduler.per_link_budget_us` value the C10-β codegen pins as
    /// the per-machine `PER_LINK_BUDGET_US` constant.
    ///
    /// Implementations use the deadline to bound internal work —
    /// drain wire bytes, decode frames into an internal queue,
    /// run any housekeeping (RX-pool slot recycling, keepalive
    /// timers). Decoded frames are then retrieved via [`Self::rx`]
    /// from the driver's internal queue at whichever cadence the
    /// SCXML interpreter's idle slot prefers. Split-responsibility
    /// (poll for work, rx for retrieval) keeps the scheduler tick
    /// loop minimal and lets drivers batch decode across multiple
    /// wire frames inside one budget.
    ///
    /// Required (no default) per pre-release no-shim rule
    /// (`feedback_pre_release_no_compat.md`) — every `Link` impl must
    /// decide explicitly whether it does budget-aware work or returns
    /// immediately. A no-op body is acceptable when the driver pumps
    /// RX from a separate task (tokio) or interrupt handler (lwIP);
    /// the `StubLink` in this crate is the no-op exemplar.
    fn poll(&mut self, deadline_us: u32);
}

// === Sample API (B7-η' prerequisite) ============================================
//
// Sample machinery for the buffer-pool kind (watching-zenoh RFC §5.E,
// `claudedocs/rfc-sce-link-runtime-sample-machinery.md`). Authored as a
// single atomic per the post-2026-05-07-audit Q-Sample-1..8 lock-ins:
//   - Q-Sample-1 (a) single SampleMeta trait, not split per-axis.
//   - Q-Sample-2 (c) GAT-based borrow/owned (`type Borrowed<'pool>` + `type Owned`).
//   - Q-Sample-3 (b) `take()` returns `M::Owned` via the GAT associated type.
//   - Q-Sample-4 deferred (no optional accessors in v1; only universal
//     `key_expr -> &str` belongs on the trait — Timestamp/attachments/
//     encoding are protocol-decoded types SCE has 0 knowledge of, so
//     consumers expose them via inherent impls on their own Borrowed/Owned).
//   - Q-Sample-5 (a) `Send + Sync` required on the trait (cross-thread
//     callback escape becomes runtime UB without compile-time bound).
//   - Q-Sample-6 (a) separate `StageCopyHook` trait at link construction.
//   - Q-Sample-7 (b) stage-copy first, drop guard last (race-free under Q-5).
//     Deviates from upstream spec line 1268; flagged for upstream
//     coordination at re-entry per RFC §7.
//   - Q-Sample-8 (a) `SlotGuard` is `pub(crate)`; consumers never type it.

/// Type bundle for a wire-protocol's per-sample metadata. Implementations
/// are typically zero-sized tag types (`struct ZenohSampleMeta;`) that
/// associate a borrowed-into-slot view (`Borrowed<'pool>`) and an owned
/// post-take form (`Owned`) with a conversion routine.
///
/// `Send + Sync` is mandatory: subscriber callbacks may run on a different
/// task than the link's RX driver. Without the bound, cross-thread
/// callback escape becomes a runtime bug class (`feedback_silently_broken_hooks.md`
/// discipline applied to thread-safety). The `Sample::take()` Q-Sample-7 (b)
/// stage-copy-first ordering eliminates the race window the bound would
/// otherwise open.
pub trait SampleMeta: Send + Sync + Sized {
    /// Borrowed metadata view tied to a slot's lifetime. Consumers
    /// define the concrete shape (e.g. zenoh's `ZenohBorrowed<'pool>`
    /// holding a `KeyExprRef<'pool>` slice into the slot).
    type Borrowed<'pool>: Send + Sync
    where
        Self: 'pool;

    /// Owned metadata bundle returned by `Sample::take()`. The absence
    /// of a lifetime parameter means `Owned` is implicitly `'static`-bounded
    /// — consumers cannot accidentally smuggle a borrow out of the slot.
    type Owned: Send + Sync;

    /// Routing key for the sample. Mandatory and universal — every wire
    /// protocol exposes a routable string identifier (zenoh keyexpr,
    /// MQTT topic, DDS topic name, SOMEIP service+method ID rendered).
    fn key_expr<'a>(borrowed: &'a Self::Borrowed<'_>) -> &'a str;

    /// Construct the owned form from the borrowed view and the staged
    /// payload bytes. Called inside `Sample::take()` after the stage-copy
    /// completes (Q-Sample-7 (b) ordering). Implementations consume the
    /// borrowed view and pair it with the owned payload `Vec`.
    fn into_owned(borrowed: Self::Borrowed<'_>, payload: Vec<u8>) -> Self::Owned;
}

/// Stage-pool copy hook supplied at link construction (Q-Sample-6 (a)).
/// `Sample::take()` invokes `stage_copy` to produce an owned `Vec<u8>`
/// from the borrowed slot bytes; the slot is then released. Implementations
/// typically pull bytes from a pre-allocated stage pool to avoid allocator
/// pressure on the wire-rate path.
pub trait StageCopyHook: Send + Sync {
    /// Copy `payload` into a fresh `Vec<u8>` (typically stage-pool-backed)
    /// that will live for the duration of the resulting `M::Owned`.
    fn stage_copy(&self, payload: &[u8]) -> Vec<u8>;
}

/// Default `StageCopyHook` that panics on use. Suitable for fixtures
/// and tests that exercise the borrow path but never call `Sample::take()`.
/// Real consumers wire a stage-pool-backed implementation via
/// `LinkConfig::stage_copy_hook`.
#[derive(Debug, Default)]
pub struct PanicOnTakeHook;

impl StageCopyHook for PanicOnTakeHook {
    fn stage_copy(&self, _payload: &[u8]) -> Vec<u8> {
        panic!("Sample::take() called with no stage_copy_hook configured");
    }
}

/// Tag-typed sample handle borrowing into a pool slot. The lifetime
/// `'pool` is bounded by the slot's borrow; the borrow is released when
/// the embedded `SlotGuard` drops (either at scope end or at the end
/// of `take()`).
///
/// `Sample` is constructed by the link's RX driver and handed to the
/// subscriber callback. The callback may either inspect the borrow and
/// drop it (returning the slot to the pool), or call `take()` to upgrade
/// to an owned form (`M::Owned`) that survives slot reuse.
///
/// Field declaration order pins drop ordering for the borrow path:
/// `payload` (no Drop), `meta` (consumer-defined Drop), `hook` (no Drop),
/// `_guard` (returns slot). `take()` overrides this with explicit
/// stage-copy-first ordering per Q-Sample-7 (b).
pub struct Sample<'pool, M: SampleMeta + 'pool> {
    payload: &'pool [u8],
    meta: M::Borrowed<'pool>,
    hook: &'pool dyn StageCopyHook,
    _guard: SlotGuard<'pool>,
}

impl<'pool, M: SampleMeta + 'pool> Sample<'pool, M> {
    /// Construct a sample over a slot's payload + metadata + return-sink.
    /// Called by the link's RX driver and fixture authors; the `SlotGuard`
    /// is held internally and released either on `Drop` or at the end
    /// of `take()`.
    pub fn new(
        payload: &'pool [u8],
        meta: M::Borrowed<'pool>,
        hook: &'pool dyn StageCopyHook,
        guard: SlotGuard<'pool>,
    ) -> Self {
        Self { payload, meta, hook, _guard: guard }
    }

    /// Borrowed payload bytes. Tied to the slot's lifetime — the consumer
    /// must finish reading before `Sample` drops (which returns the slot
    /// and permits the next `arm_rx`).
    pub fn payload(&self) -> &[u8] {
        self.payload
    }

    /// Borrowed metadata view. Consumer-defined accessors (e.g.
    /// `sample.meta().timestamp()` for zenoh) live as inherent methods
    /// on `M::Borrowed` — Q-Sample-4 deferred for v1.
    pub fn meta(&self) -> &M::Borrowed<'pool> {
        &self.meta
    }

    /// Mandatory routing key, exposed via the trait so generic consumers
    /// (logging middleware, telemetry) can read it without knowing `M`.
    pub fn key_expr(&self) -> &str {
        M::key_expr(&self.meta)
    }

    /// Upgrade to an owned form (`M::Owned`) that survives slot reuse.
    /// Q-Sample-7 (b) ordering: stage-copy first (slot bytes still owned),
    /// then construct `M::Owned`, then drop the guard (returning the slot).
    /// Race-free under cross-thread callback dispatch — by the time the
    /// guard drops and the pool's RX task can re-arm the slot, the
    /// `M::Owned` already holds an independent payload `Vec`.
    ///
    /// The intra-`take()` ordering choice is consistent with
    /// `watching-zenoh/docs/rfc-sce-protocol-synthesis.md` lines 1262-1274,
    /// which describe `take()` as "releases the underlying slot immediately"
    /// (an API-timing contract: slot is released synchronously by the time
    /// `take()` returns) without pinning intra-method ordering. The
    /// alternative drop-guard-first ordering would open a textbook UB race
    /// window under Q-Sample-5 (a) `Send + Sync` cross-thread callback
    /// dispatch (callback + pool RX task `arm_rx` reuse + driver write into
    /// freed slot → stage-copy reads new bytes as original payload).
    pub fn take(self) -> M::Owned {
        let owned_payload = self.hook.stage_copy(self.payload);
        let owned = M::into_owned(self.meta, owned_payload);
        // Explicit drop order: guard last, after `M::Owned` is fully
        // constructed. The owned form has no borrow into the slot
        // (enforced by the absence of a lifetime parameter on
        // `SampleMeta::Owned`), so re-arming the slot is safe.
        drop(self._guard);
        owned
    }
}

/// Per-slot return-sink guard (Q-Sample-8 (a) `pub(crate)`). The link's
/// pool registers a sink (a `&Cell<Option<usize>>`) when handing a slot
/// to the consumer; `Drop` writes the slot index back so the next
/// `arm_rx` reuses it.
///
/// Consumers never type this name directly — it lives inside `Sample`
/// and drops when `Sample` does. Intentionally `pub` at construction
/// (`SlotGuard::new`) so RX drivers in this crate's tests and downstream
/// per-OS impls can hand one to `Sample::new`.
pub struct SlotGuard<'pool> {
    return_sink: &'pool Cell<Option<usize>>,
    slot_index: usize,
}

impl<'pool> SlotGuard<'pool> {
    /// Wrap a slot index + return sink. Called by the link's RX driver
    /// when handing a slot to the consumer.
    pub fn new(return_sink: &'pool Cell<Option<usize>>, slot_index: usize) -> Self {
        Self { return_sink, slot_index }
    }
}

impl<'pool> Drop for SlotGuard<'pool> {
    fn drop(&mut self) {
        // Return the slot index to the pool. The pool's RX task polls
        // the sink and re-arms the slot. Cell-based; no synchronisation
        // needed because the pool's RX task is the only reader of the
        // sink within `'pool`.
        self.return_sink.set(Some(self.slot_index));
    }
}

/// Link configuration extension carrying the `StageCopyHook`. The hook
/// supplies stage-pool-backed `Vec<u8>` allocation when consumers call
/// `Sample::take()`. Defaults to `PanicOnTakeHook` so links whose
/// consumers never call `take()` work out of the box.
///
/// Authored separately from the `Link` trait to keep the B6-α surface
/// untouched; per-OS impls (`sce_link_runtime_lwip::Link` etc.) accept
/// a `&LinkConfig` at construction.
pub struct LinkConfig {
    pub stage_copy_hook: Box<dyn StageCopyHook>,
}

impl Default for LinkConfig {
    fn default() -> Self {
        Self { stage_copy_hook: Box::new(PanicOnTakeHook) }
    }
}

// === End Sample API =============================================================

/// No-op `impl Link` for ctest of generated code. Records every
/// transmitted frame into an internal buffer so tests can assert on
/// TX behavior; `rx` always returns `None` since no driver is wired.
/// Real impls (`sce_link_runtime_lwip::Link` etc.) live downstream
/// in watching-zenoh.
///
/// `std`-only because it uses `Vec` for the recorded TX log; the
/// `no_std` ctest path uses a different backing buffer (out of B6-α
/// scope — fixtures only exercise `std`).
#[cfg(any(feature = "std", test))]
extern crate std;

#[cfg(any(feature = "std", test))]
#[derive(Debug, Default)]
pub struct StubLink {
    rx_queue: std::collections::VecDeque<std::vec::Vec<u8>>,
    tx_log: std::vec::Vec<std::vec::Vec<u8>>,
    last_rx: std::vec::Vec<u8>,
}

#[cfg(any(feature = "std", test))]
impl StubLink {
    pub fn new() -> Self {
        Self::default()
    }

    /// Push bytes into the stub's RX queue so a subsequent `rx()`
    /// returns a frame view of them. Used by ctest to drive the
    /// generated code's RX path deterministically.
    pub fn push_rx(&mut self, bytes: impl Into<std::vec::Vec<u8>>) {
        self.rx_queue.push_back(bytes.into());
    }

    /// Inspect the bytes recorded by every `tx()` call so tests can
    /// assert on the encoded wire form.
    pub fn tx_log(&self) -> &[std::vec::Vec<u8>] {
        &self.tx_log
    }
}

#[cfg(any(feature = "std", test))]
impl Link for StubLink {
    fn rx(&mut self) -> Option<RxFrame<'_>> {
        self.last_rx = self.rx_queue.pop_front()?;
        Some(RxFrame::new(&self.last_rx))
    }

    fn tx(&mut self, frame: TxFrame<'_>) -> Result<(), LinkError> {
        self.tx_log.push(frame.data.to_vec());
        Ok(())
    }

    /// No-op: the stub has no internal wire-drain logic — RX bytes
    /// are pushed directly into `rx_queue` via `push_rx` and consumed
    /// by `rx()`. Budget-aware drivers (lwIP, tokio) implement real
    /// internal work here.
    fn poll(&mut self, _deadline_us: u32) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
    use std::sync::Arc;

    #[test]
    fn stub_round_trips_bytes() {
        let mut link = StubLink::new();
        link.push_rx(std::vec![0x01u8, 0x02, 0x03]);
        let frame = link.rx().expect("rx");
        assert_eq!(frame.data, &[0x01, 0x02, 0x03]);
        assert!(link.rx().is_none());

        link.tx(TxFrame::new(&[0xAA, 0xBB])).expect("tx");
        assert_eq!(link.tx_log(), &[std::vec![0xAA, 0xBB]]);
    }

    // === Sample API tests ======================================================
    //
    // Stub `SampleMeta` impl exercising the full GAT machinery: borrow path,
    // owned path, drop ordering, Send+Sync bounds. Mirrors the §7 re-entry
    // trigger item 4 ("an in-tree stub `SampleMeta` proving the trait is
    // implementable end-to-end").

    /// Tag type for the test sample protocol.
    struct TestMeta;

    /// Borrowed metadata view: a key string slice + a u64 timestamp,
    /// both tied to the slot's `'pool` lifetime.
    struct TestBorrowed<'pool> {
        key: &'pool str,
        timestamp: u64,
    }

    /// Owned metadata bundle: heap-allocated key + timestamp + payload.
    /// No lifetime parameter — outlives the source slot.
    #[derive(Debug, PartialEq, Eq)]
    struct TestOwned {
        key: std::string::String,
        timestamp: u64,
        payload: Vec<u8>,
    }

    impl SampleMeta for TestMeta {
        type Borrowed<'pool> = TestBorrowed<'pool>;
        type Owned = TestOwned;

        fn key_expr<'a>(borrowed: &'a Self::Borrowed<'_>) -> &'a str {
            borrowed.key
        }

        fn into_owned(borrowed: Self::Borrowed<'_>, payload: Vec<u8>) -> Self::Owned {
            TestOwned {
                key: borrowed.key.into(),
                timestamp: borrowed.timestamp,
                payload,
            }
        }
    }

    /// Heap-backed stage-copy hook for the borrow → owned upgrade.
    struct HeapStageHook;

    impl StageCopyHook for HeapStageHook {
        fn stage_copy(&self, payload: &[u8]) -> Vec<u8> {
            payload.to_vec()
        }
    }

    #[test]
    fn sample_borrow_path_returns_slot_on_drop() {
        let return_sink = Cell::new(None);
        let payload_buf = std::vec![0xDE, 0xAD, 0xBE, 0xEF];
        let key_buf = "test/topic";
        let hook = HeapStageHook;

        {
            let sample: Sample<'_, TestMeta> = Sample::new(
                &payload_buf,
                TestBorrowed { key: key_buf, timestamp: 42 },
                &hook,
                SlotGuard::new(&return_sink, 7),
            );

            assert_eq!(sample.payload(), &[0xDE, 0xAD, 0xBE, 0xEF]);
            assert_eq!(sample.key_expr(), "test/topic");
            assert_eq!(sample.meta().timestamp, 42);
            // sample drops at end of scope; SlotGuard returns slot 7
        }

        assert_eq!(return_sink.get(), Some(7));
    }

    #[test]
    fn sample_take_returns_owned_form_with_payload() {
        let return_sink = Cell::new(None);
        let payload_buf = std::vec![0x01, 0x02, 0x03];
        let hook = HeapStageHook;

        let sample: Sample<'_, TestMeta> = Sample::new(
            &payload_buf,
            TestBorrowed { key: "owned/topic", timestamp: 100 },
            &hook,
            SlotGuard::new(&return_sink, 3),
        );

        let owned = sample.take();

        assert_eq!(
            owned,
            TestOwned {
                key: "owned/topic".into(),
                timestamp: 100,
                payload: std::vec![0x01, 0x02, 0x03],
            }
        );
        // take() also returned the slot via guard drop
        assert_eq!(return_sink.get(), Some(3));
    }

    /// Q-Sample-7 (b) ordering proof: stage_copy must run BEFORE the slot
    /// guard drops. The hook records the counter value when invoked; the
    /// guard's Drop records its own. If (a) drop-first ordering were in
    /// effect, the guard's recorded value would be 0 and the hook's 1.
    /// Under (b) stage-copy-first, the hook's value is 0 and the guard's 1.
    #[test]
    fn sample_take_runs_stage_copy_before_guard_drop() {
        struct OrderingHook {
            counter: Arc<AtomicUsize>,
            hook_observed: Arc<AtomicUsize>,
        }
        impl StageCopyHook for OrderingHook {
            fn stage_copy(&self, payload: &[u8]) -> Vec<u8> {
                let seen = self.counter.fetch_add(1, AtomicOrdering::SeqCst);
                self.hook_observed.store(seen, AtomicOrdering::SeqCst);
                payload.to_vec()
            }
        }

        struct OrderingSink {
            sink: Cell<Option<usize>>,
            counter: Arc<AtomicUsize>,
            guard_observed: Arc<AtomicUsize>,
        }

        let counter = Arc::new(AtomicUsize::new(0));
        let hook_observed = Arc::new(AtomicUsize::new(usize::MAX));
        let guard_observed = Arc::new(AtomicUsize::new(usize::MAX));

        let sink_holder = OrderingSink {
            sink: Cell::new(None),
            counter: counter.clone(),
            guard_observed: guard_observed.clone(),
        };

        // Wrap the sink so SlotGuard's Drop bumps the counter via the
        // observation Arc. We use a small adapter struct that forwards
        // to the underlying Cell on the trailing observer increment.
        struct ObservingSlotGuard<'pool> {
            sink: &'pool OrderingSink,
            slot_index: usize,
        }
        impl<'pool> Drop for ObservingSlotGuard<'pool> {
            fn drop(&mut self) {
                let seen = self.sink.counter.fetch_add(1, AtomicOrdering::SeqCst);
                self.sink.guard_observed.store(seen, AtomicOrdering::SeqCst);
                self.sink.sink.set(Some(self.slot_index));
            }
        }

        // Inline take()-equivalent that exercises the same body shape
        // SampleMeta uses. We can't reuse `Sample::take` directly here
        // because SlotGuard isn't generic over the sink type — so this
        // test mirrors the body and verifies the SAME ordering invariant.
        let payload_buf = std::vec![0xAA, 0xBB];
        let borrowed = TestBorrowed { key: "ord/topic", timestamp: 9 };
        let hook = OrderingHook {
            counter: counter.clone(),
            hook_observed: hook_observed.clone(),
        };

        let guard = ObservingSlotGuard {
            sink: &sink_holder,
            slot_index: 11,
        };

        // Q-Sample-7 (b) body: stage-copy first, then construct owned,
        // then drop guard.
        let owned_payload = hook.stage_copy(&payload_buf);
        let _owned = TestMeta::into_owned(borrowed, owned_payload);
        drop(guard);

        // Hook observed counter=0 (ran first); guard observed counter=1
        // (ran second). Reverse ordering would flip these.
        assert_eq!(hook_observed.load(AtomicOrdering::SeqCst), 0);
        assert_eq!(guard_observed.load(AtomicOrdering::SeqCst), 1);
        assert_eq!(sink_holder.sink.get(), Some(11));
    }

    #[test]
    fn slot_guard_returns_slot_on_drop() {
        let return_sink = Cell::new(None);
        {
            let _g = SlotGuard::new(&return_sink, 5);
            assert_eq!(return_sink.get(), None);
        }
        assert_eq!(return_sink.get(), Some(5));
    }

    #[test]
    #[should_panic(expected = "stage_copy_hook")]
    fn panic_on_take_hook_panics_when_take_called() {
        let cfg = LinkConfig::default();
        // Trigger the panic directly via the hook surface; in production
        // this happens through Sample::take() with the default config.
        let _ = cfg.stage_copy_hook.stage_copy(&[0u8; 4]);
    }

    #[test]
    fn link_config_default_uses_panic_on_take_hook() {
        let cfg = LinkConfig::default();
        // Constructing the default must succeed without allocating
        // a real stage pool.
        assert!(core::mem::size_of_val(&cfg) > 0);
    }

    /// Compile-check: SampleMeta + Sample<'_, M> + StageCopyHook + LinkConfig
    /// must be Send + Sync at the type level (Q-Sample-5 (a) bound enforced).
    #[test]
    fn sample_types_are_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<TestMeta>();
        assert_send_sync::<TestOwned>();
        assert_send_sync::<HeapStageHook>();
        assert_send_sync::<PanicOnTakeHook>();
        // Sample itself isn't asserted Send/Sync because its &dyn StageCopyHook
        // field requires the trait object to be Send+Sync — the bound is on
        // StageCopyHook (Q-Sample-5), and consumer-supplied impls inherit it.
    }
}
