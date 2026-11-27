// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What every implementation of [`MembershipStore`] is held to (ADR 0113, D4 to D6; module
//! [`membership`](crate::membership)).
//!
//! The contract is the documentation of [`MembershipStore`] and of [`Write::under`]; these
//! functions are that documentation as histories with the answers a store must give, and the
//! parts of docs/formal/OpenInstances.tla a store keeps, each named by the configuration of the
//! model that takes it out. The two stores of this crate run them
//! (`tests/every_membership_store_keeps_one_contract.rs`), and a store written elsewhere runs the
//! same functions from its own `#[test]`s, as with [`crate::conformance`].
//!
//! * [`a_join_offers_and_the_exchange_that_names_it_confirms`]: a join makes no member; the
//!   exchange that names its offer does, and takes the offer; the next exchange renews it.
//! * [`a_repeated_join_is_offered_the_same_incarnation`] (`without_same_offer`): a join repeated
//!   while its offer stands is offered it again, and a join of another nonce replaces it.
//! * [`a_late_join_puts_no_live_member_out`] (`without_handshake`, `Undisturbed`): a join that
//!   arrives while a member stands changes nothing of it until an exchange names its offer.
//! * [`what_the_store_does_not_hold_is_fenced`] (`without_fence`): an incarnation neither the
//!   member's nor a standing offer's, or one that ran out, takes nothing in.
//! * [`a_mark_only_rises_and_every_handle_reads_it`] (`without_durable_mark`): the low-water mark
//!   is the store's, the larger of the ones given, and outlives the handle that wrote it.
//! * [`a_write_keeps_a_run_only_under_an_incarnation_the_store_holds`] (`without_recheck`, the
//!   condition of the write): a write made under an incarnation that ended, or ran out, keeps
//!   nothing; one that is kept leases its member for `lease_ms`.
//! * [`no_member_is_taken_past_the_bound`] (`without_limit`, `Bounded`, `with_eviction`): a
//!   new member is not taken while the bound stands, none that stands is put out, and one whose
//!   lease ran out does not count.
//! * [`what_ran_out_is_swept_and_told`]: members and offers that ran out are removed, those that
//!   ran out first first, and the members are told.
//! * [`a_batch_judges_each_write_by_its_own_incarnations`]: a batch keeps each write whose
//!   incarnations stand and refuses each other alone.
//!
//! Each function needs a harness whose store holds nothing when it starts. The clock is the one
//! the functions give; nothing here waits on time. A failed expectation panics, as a test's does.
//!
//! [`Write::under`]: crate::Write::under

use crate::batch::{Batched, Decided};
use crate::membership::{
    Admission, Ended, Member, Membership, MembershipStore, Offer, Offered, Terms, Under,
};
use crate::outbox::{Append, Write, Written};
use crate::{AcquireError, Conflict, Lease, SessionStore, WriteError};

/// What gives the functions of this module their store.
pub trait MembershipHarness {
    type Store: MembershipStore;

    /// Another handle on the one store this harness holds: it sees everything every other handle
    /// wrote. For a store in a database, a connection of its own, as a second process would have.
    fn handle(&self) -> Self::Store;
}

/// The terms the functions hold members to: a member is provisional for a second, and leased for
/// ten once a session kept it; two stand at most.
const TERMS: Terms = Terms {
    lease_ms: 10_000,
    provisional_ms: 1_000,
    max_members: 2,
};

/// The names of three instances of one open machine.
const PHONE: &str = "phone/a1";
const OTHER: &str = "phone/b2";
const THIRD: &str = "phone/c3";

/// The incarnation `member` is a member under once a join with `nonce` at `now_ms` and the
/// exchange that names its offer at the same moment confirmed it.
fn joined(store: &impl MembershipStore, member: &str, nonce: &str, now_ms: u64) -> u64 {
    let offered = offer(store, member, nonce, now_ms);
    match store
        .admit(member, offered.incarnation, 1, TERMS, now_ms)
        .expect("confirming an offer")
    {
        Admission::Confirmed { .. } => offered.incarnation,
        other => panic!("the offer of {member} at {now_ms} was not confirmed: {other:?}"),
    }
}

/// What a join of `member` with `nonce` at `now_ms` was offered, standing for `provisional_ms`.
fn offer(store: &impl MembershipStore, member: &str, nonce: &str, now_ms: u64) -> Offered {
    store
        .offer(member, nonce, TERMS.provisional_ms, now_ms)
        .expect("a join is answered")
}

fn admit(
    store: &impl MembershipStore,
    member: &str,
    incarnation: u64,
    mark: u64,
    now_ms: u64,
) -> Admission {
    store
        .admit(member, incarnation, mark, TERMS, now_ms)
        .expect("an exchange is judged")
}

fn held(store: &impl MembershipStore, member: &str) -> Membership {
    store.membership(member).expect("reading a membership")
}

fn granted(acquired: Result<Lease, AcquireError>) -> Lease {
    acquired.unwrap_or_else(|error| panic!("a lease on a free key: {error}"))
}

fn conflict<T: core::fmt::Debug>(written: Result<T, WriteError>) -> Conflict {
    match written {
        Err(WriteError::Conflict(conflict)) => conflict,
        other => panic!("expected a conflict, got {other:?}"),
    }
}

/// A join makes no member; the exchange that names its offer does, in one write that takes the
/// offer; the next exchange renews the member's lease from its own moment.
pub fn a_join_offers_and_the_exchange_that_names_it_confirms(harness: &impl MembershipHarness) {
    let store = harness.handle();
    let offered = offer(&store, PHONE, "n1", 100);
    assert_ne!(
        offered.incarnation, 0,
        "an incarnation is a number of the counter"
    );
    assert_eq!(offered.expires_ms, 1_100);
    assert_eq!(
        held(&store, PHONE),
        Membership {
            member: None,
            offer: Some(Offer {
                incarnation: offered.incarnation,
                nonce: "n1".to_string(),
                expires_ms: 1_100,
            }),
        },
        "a join offers and makes no member"
    );
    assert_eq!(
        admit(&store, PHONE, offered.incarnation, 1, 200),
        Admission::Confirmed {
            mark: 1,
            ended: None
        }
    );
    assert_eq!(
        held(&store, PHONE),
        Membership {
            member: Some(Member {
                incarnation: offered.incarnation,
                mark: 1,
                expires_ms: 1_200,
                kept: false,
                lease_ms: TERMS.lease_ms,
            }),
            offer: None,
        },
        "a confirmation takes the offer, and leases the member provisionally"
    );
    assert_eq!(
        admit(&store, PHONE, offered.incarnation, 1, 300),
        Admission::Current { mark: 1 }
    );
    assert_eq!(
        held(&store, PHONE).member.map(|member| member.expires_ms),
        Some(1_300),
        "an exchange renews the lease from its moment"
    );
}

/// A join repeated while its offer stands is offered the same incarnation, standing as long as it
/// stood; a join of another nonce is offered a new one in its place, and the one it replaced is
/// held no more. Red without the same offer: a member put out before its first request.
pub fn a_repeated_join_is_offered_the_same_incarnation(harness: &impl MembershipHarness) {
    let store = harness.handle();
    let first = offer(&store, PHONE, "n1", 100);
    assert_eq!(
        offer(&store, PHONE, "n1", 500),
        first,
        "an answer lost and asked for again changes nothing"
    );
    let other = offer(&store, PHONE, "n2", 600);
    assert!(
        other.incarnation > first.incarnation,
        "another join draws a new incarnation: {} after {}",
        other.incarnation,
        first.incarnation
    );
    assert_eq!(
        admit(&store, PHONE, first.incarnation, 1, 700),
        Admission::Fenced { current: None },
        "an offer replaced is held no more"
    );
    let late = offer(&store, PHONE, "n2", 1_600);
    assert_ne!(
        late.incarnation, other.incarnation,
        "an offer that ran out is not offered again"
    );
}

/// A join that arrives while a member stands, a late copy of an earlier one or a new one, changes
/// nothing of the member: it goes on under its incarnation until an exchange names the new offer,
/// which ends it. Red without the handshake: a repeated join puts a live member out.
pub fn a_late_join_puts_no_live_member_out(harness: &impl MembershipHarness) {
    let store = harness.handle();
    let member = joined(&store, PHONE, "n1", 100);
    let late = offer(&store, PHONE, "n0", 300);
    assert_eq!(
        held(&store, PHONE).member.map(|held| held.incarnation),
        Some(member),
        "a join puts no member out"
    );
    assert_eq!(
        admit(&store, PHONE, member, 2, 400),
        Admission::Current { mark: 2 },
        "the member is not disturbed"
    );
    assert_eq!(
        admit(&store, PHONE, late.incarnation, 1, 500),
        Admission::Confirmed {
            mark: 1,
            ended: Some(member)
        },
        "the exchange that names the new offer ends the member before it"
    );
    assert_eq!(
        admit(&store, PHONE, member, 2, 600),
        Admission::Fenced {
            current: Some(late.incarnation)
        }
    );
}

/// An incarnation neither the member's nor a standing offer's takes nothing in, nor does a member
/// or an offer that ran out, at the moment it runs out. Red without the fence: a request that
/// takes effect twice.
pub fn what_the_store_does_not_hold_is_fenced(harness: &impl MembershipHarness) {
    let store = harness.handle();
    assert_eq!(
        admit(&store, PHONE, 999_999, 1, 100),
        Admission::Fenced { current: None }
    );
    assert_eq!(
        held(&store, PHONE),
        Membership::default(),
        "a fenced exchange takes nothing in"
    );
    let member = joined(&store, PHONE, "n1", 100);
    assert_eq!(
        admit(&store, PHONE, member, 5, 1_100),
        Admission::Fenced { current: None },
        "a lease runs out at its moment"
    );
    assert_eq!(
        held(&store, PHONE).member.map(|held| held.mark),
        Some(1),
        "a fenced exchange takes no mark"
    );
    let offered = offer(&store, OTHER, "n1", 100);
    assert_eq!(
        admit(&store, OTHER, offered.incarnation, 1, 1_100),
        Admission::Fenced { current: None },
        "an offer runs out at its moment"
    );
    assert_eq!(held(&store, OTHER).member, None);
}

/// The low-water mark is the store's: a confirmation takes the one it carries, an exchange raises
/// it and never lowers it, and a handle opened after reads it. Records are dropped below this
/// mark, which a crash does not take back. Red without the durable mark: a request that takes
/// effect twice.
pub fn a_mark_only_rises_and_every_handle_reads_it(harness: &impl MembershipHarness) {
    let store = harness.handle();
    let offered = offer(&store, PHONE, "n1", 100);
    assert_eq!(
        admit(&store, PHONE, offered.incarnation, 3, 100),
        Admission::Confirmed {
            mark: 3,
            ended: None
        }
    );
    assert_eq!(
        admit(&store, PHONE, offered.incarnation, 7, 200),
        Admission::Current { mark: 7 }
    );
    assert_eq!(
        admit(&store, PHONE, offered.incarnation, 4, 300),
        Admission::Current { mark: 7 },
        "a mark carried late does not lower the one held"
    );
    let other = harness.handle();
    assert_eq!(
        held(&other, PHONE).member.map(|member| member.mark),
        Some(7),
        "another handle reads the mark the store holds"
    );
}

/// A write made under an incarnation keeps its run only while the store holds it at the write's
/// moment: once the member joined again and confirmed, or once its lease ran out, the write is
/// refused whole, its state and its appends with it. A write that is kept makes its member kept
/// and leases it for `lease_ms` from the write. Red without the condition of the write: a run
/// kept after its incarnation ended.
pub fn a_write_keeps_a_run_only_under_an_incarnation_the_store_holds(
    harness: &impl MembershipHarness,
) {
    let store = harness.handle();
    let lease = granted(store.acquire("room", "a", 60_000, 0));
    let first = joined(&store, PHONE, "n1", 100);
    let under = [Under {
        member: PHONE,
        incarnation: first,
    }];
    let written = store
        .write(
            "room",
            &Write {
                now_ms: 200,
                under: &under,
                ..Write::put(b"one", None, 0, lease.fence)
            },
        )
        .expect("a write under the incarnation its member stands under");
    let member = held(&store, PHONE).member.expect("a member");
    assert!(member.kept, "a write kept a run of it");
    assert_eq!(
        member.expires_ms, 10_200,
        "a member a session kept is leased for lease_ms from the write"
    );

    let next = offer(&store, PHONE, "n2", 300);
    assert_eq!(
        admit(&store, PHONE, next.incarnation, 1, 300),
        Admission::Confirmed {
            mark: 1,
            ended: Some(first)
        }
    );
    let appends = [Append {
        client: PHONE,
        incarnation: "i1",
        sender: Some("room"),
        envelope: b"answer",
    }];
    assert_eq!(
        conflict(store.write(
            "room",
            &Write {
                now_ms: 400,
                under: &under,
                outbox: &appends,
                ..Write::put(b"two", None, written.version, lease.fence)
            },
        )),
        Conflict::Incarnation {
            presented: first,
            current: Some(next.incarnation)
        }
    );
    assert_eq!(
        store
            .get("room")
            .expect("reading")
            .map(|stored| stored.state),
        Some(b"one".to_vec()),
        "a refused write keeps no state"
    );
    assert_eq!(
        store.outbox(PHONE).expect("reading an outbox"),
        None,
        "a refused write appends nothing"
    );

    let late = [Under {
        member: PHONE,
        incarnation: next.incarnation,
    }];
    assert_eq!(
        conflict(store.write(
            "room",
            &Write {
                now_ms: 1_300,
                under: &late,
                ..Write::put(b"three", None, written.version, lease.fence)
            },
        )),
        Conflict::Incarnation {
            presented: next.incarnation,
            current: None
        },
        "a member no session kept runs out after provisional_ms"
    );
    let unknown = [Under {
        member: OTHER,
        incarnation: next.incarnation,
    }];
    assert_eq!(
        conflict(store.write(
            "room",
            &Write {
                now_ms: 500,
                under: &unknown,
                ..Write::put(b"four", None, written.version, lease.fence)
            },
        )),
        Conflict::Incarnation {
            presented: next.incarnation,
            current: None
        },
        "an incarnation is held for its own member alone"
    );
    let kept: Result<Written, WriteError> = store.write(
        "room",
        &Write {
            now_ms: 600,
            under: &late,
            ..Write::put(b"five", None, written.version, lease.fence)
        },
    );
    assert!(kept.is_ok(), "the standing incarnation keeps: {kept:?}");
}

/// No member is taken while as many stand as the bound allows, and none that stands is put out
/// for a new one: the offer waits, and is confirmed once a member's lease ran out, which no longer
/// counts. Red without the limit: more members than the bound; red with eviction: a request that
/// takes effect twice.
pub fn no_member_is_taken_past_the_bound(harness: &impl MembershipHarness) {
    let store = harness.handle();
    let one = Terms {
        max_members: 1,
        ..TERMS
    };
    let phone = offer(&store, PHONE, "n1", 0);
    assert!(matches!(
        store.admit(PHONE, phone.incarnation, 1, one, 0),
        Ok(Admission::Confirmed { .. })
    ));
    let other = store
        .offer(OTHER, "n1", 5_000, 100)
        .expect("a join is answered");
    assert_eq!(
        store
            .admit(OTHER, other.incarnation, 1, one, 200)
            .expect("judged"),
        Admission::Full
    );
    assert_eq!(held(&store, OTHER).member, None, "nothing was taken");
    assert!(
        held(&store, OTHER).offer.is_some(),
        "the offer still stands"
    );
    assert_eq!(
        store
            .admit(PHONE, phone.incarnation, 1, one, 300)
            .expect("judged"),
        Admission::Current { mark: 1 },
        "no member that stands is put out"
    );
    assert_eq!(
        store
            .admit(OTHER, other.incarnation, 1, one, 1_300)
            .expect("judged"),
        Admission::Confirmed {
            mark: 1,
            ended: None
        },
        "a member whose lease ran out does not count"
    );
}

/// Members and offers that ran out by a moment are removed, those that ran out first first, at
/// most a limit of each, and the members are answered; what still stands is kept.
pub fn what_ran_out_is_swept_and_told(harness: &impl MembershipHarness) {
    let store = harness.handle();
    let phone = joined(&store, PHONE, "n1", 0);
    let other = joined(&store, OTHER, "n1", 500);
    store
        .offer(THIRD, "n1", 800, 0)
        .expect("a join is answered");
    assert_eq!(
        store.end_expired(1_000, 10).expect("sweeping"),
        vec![Ended {
            member: PHONE.to_string(),
            incarnation: phone
        }]
    );
    assert_eq!(held(&store, PHONE), Membership::default());
    assert_eq!(
        held(&store, THIRD).offer,
        None,
        "an offer that ran out is removed"
    );
    assert!(held(&store, OTHER).member.is_some(), "what stands is kept");
    assert_eq!(
        store.end_expired(2_000, 0).expect("sweeping"),
        Vec::new(),
        "a limit of none removes none"
    );
    assert_eq!(
        store.end_expired(2_000, 1).expect("sweeping"),
        vec![Ended {
            member: OTHER.to_string(),
            incarnation: other
        }]
    );
}

/// A batch keeps each write whose incarnations its members stand under, and refuses each other
/// alone, as the writes made one by one would be.
pub fn a_batch_judges_each_write_by_its_own_incarnations(harness: &impl MembershipHarness) {
    let store = harness.handle();
    let a = granted(store.acquire("a", "p", 60_000, 0));
    let b = granted(store.acquire("b", "p", 60_000, 0));
    let phone = joined(&store, PHONE, "n1", 100);
    let ended = joined(&store, OTHER, "n1", 100);
    joined(&store, OTHER, "n2", 200);
    let standing = [Under {
        member: PHONE,
        incarnation: phone,
    }];
    let gone = [Under {
        member: OTHER,
        incarnation: ended,
    }];
    let decided: Vec<Decided> = store
        .write_many(&[
            Batched::Write {
                key: "a",
                write: Write {
                    now_ms: 300,
                    under: &standing,
                    ..Write::put(b"kept", None, 0, a.fence)
                },
            },
            Batched::Write {
                key: "b",
                write: Write {
                    now_ms: 300,
                    under: &gone,
                    ..Write::put(b"refused", None, 0, b.fence)
                },
            },
        ])
        .expect("a batch is made");
    assert!(decided[0].is_ok(), "{:?}", decided[0]);
    assert!(
        matches!(
            decided[1],
            Err(WriteError::Conflict(Conflict::Incarnation {
                presented,
                ..
            })) if presented == ended
        ),
        "{:?}",
        decided[1]
    );
    assert!(store.get("a").expect("reading").is_some());
    assert_eq!(store.get("b").expect("reading"), None);
}
