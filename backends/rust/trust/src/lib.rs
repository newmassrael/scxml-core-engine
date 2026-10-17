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
//! * [`identity`] - a person's key: it signs, and what it signed is checked by the public key.
//! * [`membership`] - the list's log of who its administrators and members are, what each may do
//!   from when, which epoch its content is sealed under, and whether its hub is trusted: an
//!   append-only chain of entries signed by administrators, so that a host believes nothing a
//!   hub says about who the members are, and two histories of one position are a fork it
//!   reports.
//! * [`backup`] - a device's secrets (its key, its MLS state, the keys of the epochs it holds) under a
//!   passphrase: Argon2id and an authenticated cipher, a cost a copy cannot make a host pay without
//!   limit, and no key of an epoch the list had destroyed.
//! * [`audit`] - the record of a list that someone who is not in it checks with the hash of the
//!   genesis and no key: the log, the operations as they travelled, the checkpoints, and a report of
//!   what holds and what does not.
//! * [`checkpoint`] - what a member's device says it holds (an instant, the digest of its copy up
//!   to it, the head of every chain it has in full), signed; and the settlement the checkpoints
//!   of every writer make together, which a host checks alone, with no administrator, and which
//!   shows a relay that withholds an operation.
//! * [`message`] - the bytes a write is signed as, written from the event's own fields and the
//!   list's name, so a signature neither moves to another write nor to another list.
//! * [`secrecy`] - keeping the content of a write from the hub: a key for each epoch of the
//!   list, held by its members, seals the number and the text of a write under its stamp, and is
//!   given to a member sealed under what their key and the giver's agree on. What the hub holds
//!   is the stamp and a text it cannot read.
//! * [`trust`] - the rule a host puts in front of its machine and behind it: a write leaves
//!   with its author's signature, and one that arrives is admitted when its author is a member
//!   who may write and the signature is true. The signature is written into an event as it
//!   leaves and taken out as it arrives, so no machine holds one; a host that relays a write
//!   it did not make keeps the signature it arrived with, to send it on.
//!
//! The roles a grant carries are the ones the machine knows (1 may read, 2 may also write, 3 may
//! also do what an owner does), and [`trust::Trust::role_events`] is how the host tells its
//! machine the roster the log makes, so that the machine's own refusal of an edit that its member
//! may not make follows from the same source.

#![forbid(unsafe_code)]

pub mod audit;
pub mod backup;
pub mod checkpoint;
pub mod encoding;
pub mod identity;
pub mod membership;
pub mod message;
pub mod secrecy;
pub mod trust;

pub use checkpoint::{Checkpoint, Head, Settlement};
pub use identity::{Identity, PublicKey};
pub use membership::{Body, Entry, Log, LogError, Profile};
pub use trust::{
    CheckpointError, Divergence, GrantError, RotateError, Rotation, TakeError, Taken, Trust,
};
