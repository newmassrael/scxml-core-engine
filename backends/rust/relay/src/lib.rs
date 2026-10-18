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
//! * [`client`] - a host's courier to a relay, the link it speaks over, and the sealer for a host
//!   that has no hub.
//! * `serve` (feature) - the WebSocket binding that carries the request and response texts.
//!
//! A relay is generic over where it keeps its records and over how it is reached, and is specific
//! to the events whose rules `sce-rust-trust` knows (the writes and notes of the to-do list).

#![forbid(unsafe_code)]

pub mod client;
pub mod hub;
pub mod limits;
pub mod protocol;
pub mod relay;
pub mod store;
pub mod token;

#[cfg(feature = "serve")]
pub mod serve;

pub use client::{Courier, CourierError, DirectLink, Keep, Link, LinkError, Round};
pub use hub::{slot_of, Hub, Shards};
pub use limits::Limits;
pub use protocol::{Code, Record, Request, Response};
pub use relay::Relay;
pub use store::{FileStore, MemoryStore, Store};
pub use token::{Authority, Session, TokenError};
