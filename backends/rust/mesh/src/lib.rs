// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! SCE Mesh for Rust hosts.
//!
//! Two layers, and only the second is written by hand:
//!
//! * [`generated`] — the standard Mesh documents (`sce:std/mesh`): the
//!   envelope codec every backend generates from one document, and the pure
//!   delivery rules (duplicate suppression, receiver ordering, outbound
//!   buffering, retry). They are the same rules on every backend because
//!   they are the same documents.
//! * [`inbound`] — the router core's receive half: the state those rules are
//!   applied to, one sender at a time. It decides nothing itself; every
//!   decision is a call into [`generated`].
//!
//! Transport-free by design (SCE_MESH.md §mesh-6.4): bytes come in from
//! whichever transport the host wires, and the clock is an argument, so the
//! core runs the same under a test as under a socket.

#![forbid(unsafe_code)]

#[cfg(not(feature = "alloc"))]
compile_error!(
    "sce-rust-mesh needs the `alloc` feature: the router core holds envelopes on the heap, \
     and the generated envelope codec's growable facade is gated on it"
);

extern crate alloc;

pub mod generated;
pub mod inbound;
