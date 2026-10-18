// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What a socket needs of a relay, and a relay that is many.
//!
//! A [`Hub`] knows whether a token is its own and answers a request text made with one. A relay
//! behind one lock is a hub (`Mutex<Relay>`): every request of every list waits for the one before
//! it, so a machine of any number of cores does the work of one. [`Shards`] is a hub of relays that
//! each hold a share of the lists, so that the requests of different lists run at once and the
//! requests of one list still run one after another, which is the order a list's records need.
//!
//! A list belongs to a slot by its name ([`slot_of`]), with a hash that does not change from one
//! run, one machine or one version to the next: a slot's records are in its own store, and a list
//! that moved to another slot would be a list that lost its records. The number of slots is
//! therefore fixed when a deployment begins. Which machine holds which slot is a separate table,
//! and moving a slot is moving its store.

use std::io;
use std::sync::Mutex;

use crate::limits::Limits;
use crate::protocol::{Code, Request, Response};
use crate::relay::{token_reason, Relay};
use crate::store::Store;
use crate::token::{Authority, Session, TokenError};

/// What a transport asks of a relay: to know a token, and to answer a request made with one.
pub trait Hub: Send + Sync + 'static {
    /// Whether `token` is one this hub issued and has not expired at `now_ms`, and for whom.
    fn authenticate(&self, token: &str, now_ms: u64) -> Result<Session, TokenError>;

    /// The response text to the request text `request`, made with `token` at `now_ms`.
    fn handle(&self, token: &str, request: &str, now_ms: u64) -> String;
}

impl<S: Store + Send + 'static> Hub for Mutex<Relay<S>> {
    fn authenticate(&self, token: &str, now_ms: u64) -> Result<Session, TokenError> {
        self.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .authenticate(token, now_ms)
    }

    fn handle(&self, token: &str, request: &str, now_ms: u64) -> String {
        self.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .handle(token, request, now_ms)
    }
}

/// The slot of the list `list` among `slots`. FNV-1a over the name's bytes: the same answer on
/// every run, which `std`'s hasher, seeded afresh each run, is not.
pub fn slot_of(list: &str, slots: usize) -> usize {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in list.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    (hash % slots.max(1) as u64) as usize
}

/// A relay of `count` slots, each a relay of its own with its own lock and its own store.
pub struct Shards<S: Store> {
    authority: Authority,
    max_request_bytes: usize,
    slots: Vec<Mutex<Relay<S>>>,
}

impl<S: Store + Send + 'static> Shards<S> {
    /// Open `count` slots. `store_of` gives the store of the slot it is asked for: the same store
    /// for the same slot on every run, or the slot opens without what it held.
    pub fn open(
        count: usize,
        mut store_of: impl FnMut(usize) -> io::Result<S>,
        authority: Authority,
        limits: Limits,
    ) -> io::Result<Self> {
        if count == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "a relay of no slots holds no list",
            ));
        }
        let mut slots = Vec::with_capacity(count);
        for slot in 0..count {
            slots.push(Mutex::new(Relay::open(
                store_of(slot)?,
                authority.clone(),
                limits,
            )?));
        }
        Ok(Self {
            max_request_bytes: limits.max_request_bytes,
            authority,
            slots,
        })
    }

    /// How many slots the relay has.
    pub fn count(&self) -> usize {
        self.slots.len()
    }
}

impl<S: Store + Send + 'static> Hub for Shards<S> {
    fn authenticate(&self, token: &str, now_ms: u64) -> Result<Session, TokenError> {
        self.authority.verify(token, now_ms)
    }

    /// The token, the size and the shape of the request are judged before any lock is taken, so
    /// that a request that is refused, or one that waits for its list, holds up no other list.
    fn handle(&self, token: &str, request: &str, now_ms: u64) -> String {
        let session = match self.authority.verify(token, now_ms) {
            Ok(session) => session,
            Err(error) => {
                return Response::refused(Code::Unauthorized, token_reason(error)).to_json()
            }
        };
        if request.len() > self.max_request_bytes {
            return Response::refused(
                Code::TooLarge,
                format!("a request is at most {} bytes", self.max_request_bytes),
            )
            .to_json();
        }
        let request = match Request::from_json(request) {
            Ok(request) => request,
            Err(reason) => return Response::refused(Code::Invalid, reason).to_json(),
        };
        let slot = &self.slots[slot_of(&session.list, self.slots.len())];
        slot.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .call(&session, request, now_ms)
            .to_json()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::MemoryStore;

    #[test]
    fn a_list_has_one_slot_and_it_is_the_same_from_one_run_to_the_next() {
        // Pinned values: a change to the hash moves every list of every deployment.
        assert_eq!(slot_of("home", 64), 46);
        assert_eq!(slot_of("work", 64), 0);
        assert_eq!(slot_of("", 64), 37);
        for list in ["a", "b", "load-0", "load-1"] {
            assert_eq!(slot_of(list, 16), slot_of(list, 16));
            assert!(slot_of(list, 16) < 16);
        }
        assert_eq!(slot_of("anything", 1), 0);
    }

    #[test]
    fn lists_spread_over_the_slots() {
        let mut held = [0usize; 8];
        for n in 0..800 {
            held[slot_of(&format!("list-{n}"), 8)] += 1;
        }
        assert!(
            held.iter().all(|count| (60..=140).contains(count)),
            "{held:?}"
        );
    }

    #[test]
    fn a_relay_of_no_slots_is_refused() {
        let opened = Shards::open(
            0,
            |_| Ok(MemoryStore::new()),
            Authority::new([1; 32], "x"),
            Limits::default(),
        );
        assert!(opened.is_err());
    }

    #[test]
    fn a_bad_token_and_a_bad_request_are_answered_without_a_slot() {
        let authority = Authority::new([2; 32], "x");
        let shards = Shards::open(
            4,
            |_| Ok(MemoryStore::new()),
            authority.clone(),
            Limits::default(),
        )
        .unwrap();
        let code_of = |answer: String| match Response::from_json(&answer) {
            Ok(Response::Refused { code, .. }) => code,
            other => panic!("expected a refusal, got {other:?}"),
        };
        let answer = shards.handle("nonsense", &Request::Heads.to_json(), 0);
        assert_eq!(code_of(answer), Code::Unauthorized);
        let token = authority.issue("home", 1, u64::MAX);
        let answer = shards.handle(&token, "not json", 0);
        assert_eq!(code_of(answer), Code::Invalid);
        let long = "x".repeat(Limits::default().max_request_bytes + 1);
        let answer = shards.handle(&token, &long, 0);
        assert_eq!(code_of(answer), Code::TooLarge);
    }

    #[test]
    fn a_list_asked_of_one_slot_is_not_found_in_another() {
        // Two lists in two slots: each is held by its own relay, and a request is answered by the
        // slot its token's list names, so the one list's log is never read from the other's.
        let authority = Authority::new([3; 32], "x");
        let shards = Shards::open(
            8,
            |_| Ok(MemoryStore::new()),
            authority.clone(),
            Limits::default(),
        )
        .unwrap();
        let (a, b) = ("home", "work");
        assert_ne!(slot_of(a, 8), slot_of(b, 8));
        for list in [a, b] {
            let token = authority.issue(list, 1, u64::MAX);
            let answer = shards.handle(&token, &Request::Entries { from: 0 }.to_json(), 0);
            // The relay holds no log of either yet: an empty one, not a refusal and not the other's.
            assert!(
                matches!(
                    Response::from_json(&answer),
                    Ok(Response::Entries { len: 0, .. })
                ),
                "{list}: {answer}"
            );
        }
    }
}
