// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
//!   decision is a call into [`generated`]. [`outbound`] is the send half,
//!   and [`router`] joins both to the engine's host-processor surface.
//!
//! Transport-free by design (SCE_MESH.md §mesh-6.4): bytes come in from
//! whichever transport the host wires, and the clock is an argument, so the
//! core runs the same under a test as under a socket. [`endpoint`] is where
//! a host supplies them — a transport, a clock, randomness — and where the
//! core meets an engine. [`peers`] is what the build generates from
//! deploy.yaml for one machine: its peers, each one's configuration and how
//! it is reached. [`checkpoint`] is what a machine whose deployment persists
//! it is kept as: its saved tree and its endpoints in one durable write, and
//! the session store a server keeps them in. [`pool`] serves a machine the
//! deployment declares a keyed pool as one session per key, each kept in a
//! session store as its checkpoint while it is out of memory.
//!
//! [`generated_rate`] is the standard rate limiter (`sce:std/rate`), generated
//! as [`generated`] is: the HTTPS server's edge judges every limit with it
//! (docs/adr/0109, D2), so a document's rate limit and the edge's are one
//! algorithm.

#![forbid(unsafe_code)]

#[cfg(not(feature = "alloc"))]
compile_error!(
    "sce-rust-mesh needs the `alloc` feature: the router core holds envelopes on the heap, \
     and the generated envelope codec's growable facade is gated on it"
);

extern crate alloc;

pub mod causal;
pub mod checkpoint;
#[cfg(feature = "conformance")]
pub mod conformance;
pub mod endpoint;
pub mod generated;
// Formatted by the script that writes it (generate.sh ends with rustfmt), so the
// workspace's format check does not read a tree a regeneration owns.
#[rustfmt::skip]
pub mod generated_rate;
#[cfg(feature = "https")]
pub mod https;
pub mod inbound;
pub mod inbox;
pub mod link;
pub mod outbound;
pub mod peers;
pub mod pool;
pub mod reconnect;
pub mod router;
pub mod rpc;
pub mod signal;
#[cfg(test)]
mod testing;
#[cfg(feature = "tls")]
pub mod tls;
#[cfg(feature = "wss")]
pub mod wss;
#[cfg(feature = "zenoh")]
pub mod zenoh;
