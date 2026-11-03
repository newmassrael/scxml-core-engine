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
use std::net::IpAddr;
use std::sync::{Mutex, MutexGuard};

use crate::limits::Limits;
use crate::login::{self, Asked, Doorway, Opened};
use crate::logs::refused;
use crate::protocol::{Code, Request};
use crate::relay::{token_reason, Lapsed, Relay};
use crate::store::Store;
use crate::token::{Authority, Session, TokenError};

/// What a transport asks of a relay: to know a token, to answer a request made with one, and, when
/// the relay offers login (ADR 0080), to answer a connection that presents none it takes.
pub trait Hub: Send + Sync + 'static {
    /// Whether `token` is one this hub issued and has not expired at `now_ms`, and for whom.
    fn authenticate(&self, token: &str, now_ms: u64) -> Result<Session, TokenError>;

    /// The response text to the request text `request`, made with `token` at `now_ms`.
    fn handle(&self, token: &str, request: &str, now_ms: u64) -> String;

    /// Whether a connection that presents no token this hub takes (none, or one it issued that
    /// expired) is taken from `source` at `now_ms`. A hub that offers no login takes none, which is
    /// what a hub that does not override this does.
    fn open_anonymous(&self, source: Option<IpAddr>, now_ms: u64) -> Opened {
        let _ = (source, now_ms);
        Opened::NotOffered
    }

    /// The response text to the request text `request`, made at `now_ms` on a connection from
    /// `source` that presents no token this hub takes: a challenge and a login when it offers
    /// login, and a refusal of everything else.
    fn handle_anonymous(&self, source: Option<IpAddr>, request: &str, now_ms: u64) -> String {
        let _ = (source, now_ms);
        login::not_offered(request).to_json()
    }

    /// The response text to the request text `request`, made at `now_ms` on a connection from
    /// `source` that presented `token`, or none: what a transport calls for each request. A request
    /// to log in (`login::is_anonymous`) is answered as on a connection with no token whatever it
    /// presented, so that a device whose token expired on a connection that stays open logs in on
    /// it; every other request is made with the token, and one with none is refused.
    fn answer(
        &self,
        token: Option<&str>,
        source: Option<IpAddr>,
        request: &str,
        now_ms: u64,
    ) -> String {
        match token {
            Some(token) if !login::is_anonymous(request) => self.handle(token, request, now_ms),
            _ => self.handle_anonymous(source, request, now_ms),
        }
    }
}

/// The value a lock holds, whether or not a thread panicked holding it: what a relay holds is
/// checked as it is taken, and is never half-written by one.
fn held<T>(lock: &Mutex<T>) -> MutexGuard<'_, T> {
    lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
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

    fn open_anonymous(&self, source: Option<IpAddr>, now_ms: u64) -> Opened {
        held(self).open_anonymous(source, now_ms)
    }

    fn handle_anonymous(&self, source: Option<IpAddr>, request: &str, now_ms: u64) -> String {
        held(self).anonymous(source, request, now_ms)
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
    /// What answers a connection with no token, one for every slot, when the relay offers login: a
    /// source's allowance and the nonces used are the relay's, not a slot's.
    doorway: Option<Mutex<Doorway>>,
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
            doorway: None,
        })
    }

    /// This relay offers login through `doorway` ([`Relay::with_login`]), one for all its slots.
    pub fn with_login(mut self, doorway: Doorway) -> Self {
        self.doorway = Some(Mutex::new(doorway));
        self
    }

    /// How many slots the relay has.
    pub fn count(&self) -> usize {
        self.slots.len()
    }

    /// Remove the lists created openly that no login has touched for `idle_ms` at `now_ms`, in
    /// every slot, one slot locked at a time ([`Relay::lapse`]): what a deployment that takes
    /// creations with no token runs at start and on a schedule.
    pub fn lapse(&self, idle_ms: u64, now_ms: u64) -> Lapsed {
        self.slots.iter().fold(Lapsed::default(), |lapsed, slot| {
            lapsed.and(held(slot).lapse(idle_ms, now_ms))
        })
    }

    /// The slot that holds `list`.
    fn slot(&self, list: &str) -> &Mutex<Relay<S>> {
        &self.slots[slot_of(list, self.slots.len())]
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
            Err(error) => return refused!(Code::Unauthorized, token_reason(error)).to_json(),
        };
        if request.len() > self.max_request_bytes {
            return refused!(
                Code::TooLarge,
                format!("a request is at most {} bytes", self.max_request_bytes),
            )
            .to_json();
        }
        let request = match Request::from_json(request) {
            Ok(request) => request,
            Err(reason) => return refused!(Code::Invalid, reason).to_json(),
        };
        let slot = &self.slots[slot_of(&session.list, self.slots.len())];
        slot.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .call(&session, request, now_ms)
            .to_json()
    }

    fn open_anonymous(&self, source: Option<IpAddr>, now_ms: u64) -> Opened {
        match &self.doorway {
            None => Opened::NotOffered,
            Some(doorway) => held(doorway).open(source, now_ms),
        }
    }

    /// The doorway and the list's slot are each locked for their part alone: the doorway to judge
    /// the request and to redeem a login, the slot to read its log, so that a login waits for no
    /// other list and no other login waits on a list.
    fn handle_anonymous(&self, source: Option<IpAddr>, request: &str, now_ms: u64) -> String {
        let Some(doorway) = &self.doorway else {
            return login::not_offered(request).to_json();
        };
        let asked = held(doorway).ask(source, request, self.max_request_bytes, now_ms);
        let response = match asked {
            Err(refused) => refused,
            Ok(Asked::Challenge { list, node }) => {
                let holds = held(self.slot(&list)).holds(&list);
                held(doorway).challenge(&list, node, holds, self.authority.audience(), now_ms)
            }
            // The doorway's allowances are asked before the slot takes the entries the login brought
            // (`Doorway::admissible`), and spent after the signature held.
            Ok(Asked::Login(checked)) => {
                let audience = self.authority.audience();
                let admissible = held(doorway).admissible(&checked, now_ms);
                let admitted = admissible.and_then(|()| {
                    held(self.slot(&checked.login().list)).admit_login(
                        checked.login(),
                        |key| checked.signed_by(key, audience),
                        now_ms,
                    )
                });
                match admitted {
                    Err(refused) => refused,
                    Ok(()) => held(doorway).redeem(&checked, &self.authority, now_ms),
                }
            }
            // The nonce and the relay's creations of the hour are spent under the doorway's lock,
            // and the list made under its slot's alone; the token is the doorway's again.
            Ok(Asked::Create(checked)) => {
                let spent = held(doorway).spend_create(&checked, now_ms);
                let created = spent.and_then(|()| {
                    held(self.slot(&checked.login().list)).take_create(
                        &checked,
                        self.authority.audience(),
                        now_ms,
                    )
                });
                match created {
                    Err(refused) => refused,
                    Ok(()) => held(doorway).token(checked.login(), &self.authority, now_ms),
                }
            }
            // The nonce is spent under the doorway's lock, and the invitation judged and the key
            // package kept under the list's alone, so that two joins of one node on two
            // connections are told apart by the record the first left.
            Ok(Asked::Join(checked)) => {
                let spent = {
                    let mut doorway = held(doorway);
                    doorway
                        .spend_join(&checked, now_ms)
                        .map(|()| doorway.policy().joins_outstanding_per_list)
                };
                match spent {
                    Err(refused) => refused,
                    Ok(outstanding) => {
                        let list = checked.join().invitation.list.clone();
                        held(self.slot(&list)).take_join(
                            checked.into_join(),
                            self.authority.audience(),
                            outstanding,
                            now_ms,
                        )
                    }
                }
            }
        };
        response.to_json()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Response;
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
