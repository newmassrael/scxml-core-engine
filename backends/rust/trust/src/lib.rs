// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What the host of a shared list keeps that its machines cannot.
//!
//! A copy of a shared list may be run by someone other than the people who use it. What stops
//! that host from putting a write in a member's name, or from letting a reader write, cannot
//! be in the machine: a machine of the statically typed data model reads the payload of an
//! event and no other member of `_event`, so it can tell neither who sent an event nor
//! whether a signature it carries is true. It is kept here, in the host of every copy, which is
//! the one party each copy's user can trust: their own device.
//!
//! * [`identity`] — a person's key: it signs, and what it signed is checked by the public key.
//! * [`certificate`] — what an administrator signs to make a person a member with a role. The
//!   administrator's key is what a copy is told to trust when it is set up, and everything
//!   else (who is a member, what each may do) follows from certificates that key signed, so
//!   a host that relays them cannot make one.
//! * [`message`] — the bytes a write is signed as, written from the event's own fields and the
//!   list's name, so a signature neither moves to another write nor to another list.
//! * [`trust`] — the rule a host puts in front of its machine and behind it: a write leaves
//!   with its author's signature, and one that arrives is admitted when its author is a member
//!   who may write and the signature is true. The signature is written into an event as it
//!   leaves and taken out as it arrives, so no machine holds one; a host that relays a write
//!   it did not make keeps the signature it arrived with, to send it on.
//!
//! The roles a certificate carries are the ones the machine knows (1 may read, 2 may also
//! write, 3 may also do what an owner does), and [`trust::Trust::role_events`] is how the host
//! tells its machine the roster the certificates make, so that the machine's own refusal of an
//! edit that its member may not make follows from the same source.

#![forbid(unsafe_code)]

pub mod certificate;
pub mod encoding;
pub mod identity;
pub mod message;
pub mod trust;

pub use certificate::Certificate;
pub use identity::{Identity, PublicKey};
pub use trust::{Trust, TrustError};
