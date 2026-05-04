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
}

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
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
