// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The reference relay of a shared list (ADR 0034).
//!
//! A relay sits between the devices of a list whose members do not trust the host between them. It
//! stores what members sign and gives it back; it holds no key and reads nothing it holds; and it
//! decides nothing a member's host would then believe, because a host checks what a relay gives it
//! exactly as it checks what anyone gives it (`sce-rust-trust`). What it is for is that devices need
//! not be online together, and that spam and the removed are refused before they take room.
//!
//! * [`relay`] - the relay: verify, keep, give back, by session and allowance.
//! * [`protocol`] - the requests and responses, as JSON texts.
//! * [`token`] - the login-bound token that names a device of one list, and nothing about a person.
//! * [`limits`] - the allowance of each device and the bounds on a request and a reply.
//! * [`store`] - the append-only record a relay starts from, checked again as it is read.
//! * [`client`] - a host's courier to a relay, the round it makes as a state machine that does no
//!   I/O itself, the link it speaks over, and the sealer for a host that has no hub.
//! * [`driver`] - the one loop that carries an exchange (a round, or the peer core's) over a link,
//!   with a deadline and a cancellation.
//! * `wss` (feature) - the WebSocket binding's client: the reference [`Transport`] a device reaches
//!   a relay through, over `ws://` or `wss://`.
//! * `tls` (with `wss`) - the TLS the binding runs on, shared with the Mesh's: `ring`'s
//!   cryptography, and the authorities a client believes.
//! * `serve` (feature) - the WebSocket binding's server, which carries the request and response
//!   texts to a relay, and terminates TLS when it is given a certificate.
//!
//! What carries the texts is a [`Transport`], whose contract is the crate `sce-rust-relay-transport`
//! (re-exported here): a binding written elsewhere, the deployment transport's adapter among them,
//! implements it there and depends on nothing of this crate.
//!
//! A relay is generic over where it keeps its records and over how it is reached, and is specific
//! to the events whose rules `sce-rust-trust` knows: the writes and notes of the standard shared
//! record list (`sce:std/shared/record_list.scxml`), which is the one document it carries.

#![forbid(unsafe_code)]

pub mod client;
pub mod driver;
pub mod hub;
pub mod limits;
pub mod protocol;
pub mod relay;
pub mod store;
pub mod token;

#[cfg(feature = "wss")]
pub mod wss;

// The TLS the WebSocket binding runs on, the Mesh's binding's too: one provider (`ring`), and the
// authorities a client believes.
#[cfg(feature = "wss")]
pub use sce_rust_mesh::tls;

#[cfg(feature = "serve")]
pub mod serve;

pub use client::{
    Courier, CourierError, CourierState, DirectLink, Keep, Kept, Link, LinkError, Publication,
    Round, RoundKind, RoundMachine, Step,
};
pub use driver::{drive, Driven, Exchange};
pub use hub::{slot_of, Hub, Shards};
pub use limits::Limits;
pub use protocol::{Code, Record, Request, Response};
pub use relay::Relay;
pub use sce_rust_relay_transport::{
    Notice, Notices, Signal, Transport, TransportError, Watch, NOTICES_KEPT,
};
pub use store::{FileStore, MemoryStore, Store};
pub use token::{Authority, KeyTextError, Session, TokenError};
