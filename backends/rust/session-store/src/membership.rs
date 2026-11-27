// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The members of an open machine, as a server holds them (ADR 0113, D4 and D5;
//! docs/formal/OpenInstances.tla).
//!
//! A machine of a deployment may run as an open set of instances, each a sender of its own that
//! no deployment lists. A server binds each as it arrives, and holds what it binds by these rules:
//!
//! * **A join offers.** An exchange that names no incarnation is answered with an incarnation the
//!   store draws from its one counter, so none is given twice, across a restart either
//!   ([`MembershipStore::offer`]). The offer is kept for the join's nonce: a join repeated while
//!   its offer stands is offered the same incarnation again, so an answer lost and asked for again
//!   changes nothing. An offer replaces the one before it, and puts no member out.
//! * **An exchange confirms.** The first exchange that names the offer makes the instance a member
//!   under it, in one write, and ends the incarnation it had before ([`MembershipStore::admit`]).
//!   A join that arrives late only offers, so it puts no live member out: the three-way handshake
//!   of RFC 9293 section 3.5.
//! * **A member is leased.** Each exchange renews its lease. A member whose messages no session
//!   kept lives `provisional_ms` past its last exchange; once a write keeps a run made under it
//!   (`Write::under`) it lives `lease_ms` ([`Terms`]). What has run out is a member no longer,
//!   whether or not [`MembershipStore::end_expired`] has removed it yet.
//! * **An incarnation the store does not hold is fenced** ([`Admission::Fenced`]): the exchange
//!   takes nothing in, and its instance learns that what it waited on has an unknown outcome.
//! * **No member is taken past the bound** ([`Admission::Full`]): a member whose lease ran out
//!   does not count, and none that stands is put out for a new one.
//! * **The low-water mark** of a member is the lowest number of its incarnation whose answer it
//!   has not: it only rises, and a session drops the completion records below the one the store
//!   holds, never below one a process saw and a crash may take back.
//!
//! A member is named by text the store does not read: the name its binding gives the pair
//! (machine, instance), as an outbox's client is named. An incarnation is a number of the store's
//! counter, so it names one member alone.
//!
//! [`Write::under`]: crate::Write::under

use std::error::Error;
use std::fmt;

use crate::{Failure, SessionStore};

/// What a deployment holds the members of an open machine to (ADR 0113, D4 and D6). No number of
/// it has a default: each is the deployment's, as a lease's length is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Terms {
    /// `lease_ms`: how long a member a session kept a message of stays one past its last exchange.
    pub lease_ms: u64,
    /// `provisional_ms`: how long a member no session kept a message of stays one past its last
    /// exchange, and how long an offer stands, unless the offer waits for a proof that takes
    /// longer (ADR 0116, D2).
    pub provisional_ms: u64,
    /// `max_instances`: the most members that stand at once.
    pub max_members: u64,
}

/// The incarnation a join was offered, and when the offer stops standing unless an exchange
/// confirms it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Offered {
    pub incarnation: u64,
    pub expires_ms: u64,
}

/// How the store judged an exchange that names an incarnation ([`MembershipStore::admit`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admission {
    /// The member's own incarnation: its lease was renewed, and `mark` is its low-water mark as
    /// the store now holds it, the larger of the one held and the one the exchange carried.
    Current { mark: u64 },
    /// The offer it names was confirmed by this write: the instance is a member under it, `mark`
    /// is its low-water mark, and `ended` the incarnation it held before, which the same write
    /// ended.
    Confirmed { mark: u64, ended: Option<u64> },
    /// The store holds no such incarnation for the member: the exchange took nothing in.
    /// `current` is the member's incarnation, `None` when no member stands.
    Fenced { current: Option<u64> },
    /// The offer stands, and as many members stand as the bound allows: nothing was taken, and
    /// the exchange may be made again.
    Full,
}

/// A member as the store holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Member {
    /// The incarnation it is a member under.
    pub incarnation: u64,
    /// Its low-water mark: every number of its incarnation below this has its answer.
    pub mark: u64,
    /// When it stops being a member unless an exchange renews it, in milliseconds since the Unix
    /// epoch.
    pub expires_ms: u64,
    /// Whether a write kept a run made under it (`Write::under`).
    ///
    /// [`Write::under`]: crate::Write::under
    pub kept: bool,
    /// The lease it is renewed to once a write kept a run of it: the `lease_ms` of the terms it
    /// was last admitted under.
    pub lease_ms: u64,
}

/// An offer that waits for the exchange that confirms it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer {
    pub incarnation: u64,
    /// The nonce of the join it answers.
    pub nonce: String,
    pub expires_ms: u64,
}

/// What the store holds of one member's name: the member, and an offer, each when it holds one.
/// Either may have run out and not yet been removed ([`MembershipStore::end_expired`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Membership {
    pub member: Option<Member>,
    pub offer: Option<Offer>,
}

/// An incarnation a write was made under ([`Write::under`]): the write keeps nothing unless the
/// store holds it.
///
/// [`Write::under`]: crate::Write::under
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Under<'a> {
    /// The member's name.
    pub member: &'a str,
    pub incarnation: u64,
}

/// A member [`MembershipStore::end_expired`] removed: what a host held for it ends with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ended {
    pub member: String,
    pub incarnation: u64,
}

/// The members of open machines, kept beside the sessions whose writes are made under them (ADR
/// 0113, D4 and D5). Every method is one transaction.
pub trait MembershipStore: SessionStore {
    /// Answer a join of `member` that carries `nonce`, at `now_ms`: the offer that stands for the
    /// same nonce, as it stands; or else an incarnation drawn from the store's counter, offered
    /// until `now_ms + ttl_ms` in place of any offer before it. The member, if one stands, is not
    /// touched.
    fn offer(
        &self,
        member: &str,
        nonce: &str,
        ttl_ms: u64,
        now_ms: u64,
    ) -> Result<Offered, Failure>;

    /// Judge an exchange of `member` that names `incarnation` and carries the low-water mark
    /// `mark`, at `now_ms`, under `terms`, and write what it takes in:
    ///
    /// * the member's own incarnation, standing: its lease renewed from `now_ms` (`lease_ms`
    ///   once a write kept a run of it, `provisional_ms` before), its mark raised to `mark`
    ///   when that is above it ([`Admission::Current`]);
    /// * the offer that stands for it: unless as many other members stand as
    ///   [`Terms::max_members`] ([`Admission::Full`], nothing written), the instance is a member
    ///   under it with the mark `mark` (at least 1) and a provisional lease, the offer is taken,
    ///   and the member it had before is ended ([`Admission::Confirmed`]);
    /// * anything else: [`Admission::Fenced`], nothing written.
    fn admit(
        &self,
        member: &str,
        incarnation: u64,
        mark: u64,
        terms: Terms,
        now_ms: u64,
    ) -> Result<Admission, Failure>;

    /// What the store holds of `member`.
    fn membership(&self, member: &str) -> Result<Membership, Failure>;

    /// Remove at most `limit` members and at most `limit` offers that ran out at or before
    /// `before_ms`, those that ran out first first, and answer the members removed, in that
    /// order: what a host held for each (its window, its outbox) ends with it.
    fn end_expired(&self, before_ms: u64, limit: usize) -> Result<Vec<Ended>, Failure>;
}

/// The refusal of a store that keeps no members, given a write made under an incarnation: it
/// serves no open machine. Every store of this crate keeps them ([`MembershipStore`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoMembership;

impl fmt::Display for NoMembership {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(
            "the store keeps no members, so it serves no open machine and keeps no write made \
             under an incarnation (docs/adr/0113, D5)",
        )
    }
}

impl Error for NoMembership {}

/// The decisions of membership, written once for every store, as the crate's rules of the leases
/// are.
pub(crate) mod judge {
    use super::{Admission, Member, Offer, Terms};
    use crate::{rules, Conflict};

    /// Whether what runs out at `expires_ms` still stands at `now_ms`: as a lease, it runs out
    /// at that moment.
    pub(crate) fn stands(expires_ms: u64, now_ms: u64) -> bool {
        now_ms < expires_ms
    }

    /// Whether a join with `nonce` is offered `offered` again at `now_ms`: it stands, and it
    /// answers a join of the same nonce.
    pub(crate) fn offers_again(offered: Option<&Offer>, nonce: &str, now_ms: u64) -> bool {
        offered.is_some_and(|offer| offer.nonce == nonce && stands(offer.expires_ms, now_ms))
    }

    /// What an exchange is to write, decided before anything is written.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(crate) enum Judged {
        /// The member stands under the incarnation: write it as `member`.
        Renew { member: Member },
        /// The offer is confirmed: `member` takes the member's place and the offer is taken.
        Confirm { member: Member, ended: Option<u64> },
        /// Nothing is written.
        Refuse(Admission),
    }

    impl Judged {
        /// What the exchange is answered.
        pub(crate) fn admission(self) -> Admission {
            match self {
                Judged::Renew { member } => Admission::Current { mark: member.mark },
                Judged::Confirm { member, ended } => Admission::Confirmed {
                    mark: member.mark,
                    ended,
                },
                Judged::Refuse(admission) => admission,
            }
        }
    }

    /// Judge an exchange that names `incarnation` with the mark `mark` at `now_ms`, over the
    /// member `held` and the offer `offered` of its name (standing or not), with `others` the
    /// members of other names that stand at `now_ms`.
    pub(crate) fn admit(
        held: Option<&Member>,
        offered: Option<&Offer>,
        others: u64,
        incarnation: u64,
        mark: u64,
        terms: Terms,
        now_ms: u64,
    ) -> Judged {
        // A number a store keeps is at most the largest signed 64-bit one, as a moment is: no
        // instance numbers that many requests in one incarnation, and no lease is that long.
        let mark = mark.min(crate::LATEST_MOMENT);
        let lease_ms = terms.lease_ms.min(crate::LATEST_MOMENT);
        let standing = held.filter(|member| stands(member.expires_ms, now_ms));
        if let Some(member) = standing.filter(|member| member.incarnation == incarnation) {
            let lease = if member.kept {
                terms.lease_ms
            } else {
                terms.provisional_ms
            };
            return Judged::Renew {
                member: Member {
                    mark: member.mark.max(mark),
                    expires_ms: rules::expiry(now_ms, lease),
                    lease_ms,
                    ..*member
                },
            };
        }
        let confirms = offered.is_some_and(|offer| {
            offer.incarnation == incarnation && stands(offer.expires_ms, now_ms)
        });
        if !confirms {
            return Judged::Refuse(Admission::Fenced {
                current: standing.map(|member| member.incarnation),
            });
        }
        if others >= terms.max_members {
            return Judged::Refuse(Admission::Full);
        }
        Judged::Confirm {
            member: Member {
                incarnation,
                mark: mark.max(1),
                expires_ms: rules::expiry(now_ms, terms.provisional_ms),
                kept: false,
                lease_ms,
            },
            ended: held.map(|member| member.incarnation),
        }
    }

    /// Whether a write made under `incarnation` may keep anything, over the member `held` of the
    /// name it gives, at the write's moment `now_ms`: only while the member stands under it.
    pub(crate) fn admit_under(
        held: Option<&Member>,
        incarnation: u64,
        now_ms: u64,
    ) -> Result<(), Conflict> {
        let current = held
            .filter(|member| stands(member.expires_ms, now_ms))
            .map(|member| member.incarnation);
        if current == Some(incarnation) {
            Ok(())
        } else {
            Err(Conflict::Incarnation {
                presented: incarnation,
                current,
            })
        }
    }

    /// `member` once a write at `now_ms` kept a run made under it: kept, and leased for its
    /// `lease_ms` from the write, unless it already stands longer.
    pub(crate) fn kept(member: &Member, now_ms: u64) -> Member {
        Member {
            kept: true,
            expires_ms: member
                .expires_ms
                .max(rules::expiry(now_ms, member.lease_ms)),
            ..*member
        }
    }
}

#[cfg(test)]
mod tests {
    use super::judge::{admit, admit_under, kept, offers_again, Judged};
    use super::*;
    use crate::Conflict;

    const TERMS: Terms = Terms {
        lease_ms: 1_000,
        provisional_ms: 100,
        max_members: 2,
    };

    fn member(incarnation: u64, expires_ms: u64) -> Member {
        Member {
            incarnation,
            mark: 3,
            expires_ms,
            kept: false,
            lease_ms: TERMS.lease_ms,
        }
    }

    fn offer(incarnation: u64, expires_ms: u64) -> Offer {
        Offer {
            incarnation,
            nonce: "n1".to_string(),
            expires_ms,
        }
    }

    /// docs/formal/OpenInstances.tla, `SEdge`: the member's own incarnation renews it and raises
    /// its mark, and never lowers it. Red if the mark is taken as given, or the lease is not
    /// renewed from the exchange.
    #[test]
    fn the_members_own_incarnation_renews_it_and_raises_its_mark() {
        let held = member(7, 500);
        let judged = admit(Some(&held), None, 0, 7, 9, TERMS, 400);
        assert_eq!(judged.admission(), Admission::Current { mark: 9 });
        assert!(matches!(judged, Judged::Renew { member } if member.expires_ms == 500));
        let lower = admit(Some(&held), None, 0, 7, 1, TERMS, 400);
        assert_eq!(lower.admission(), Admission::Current { mark: 3 });
        let kept_one = Member { kept: true, ..held };
        assert!(matches!(
            admit(Some(&kept_one), None, 0, 7, 1, TERMS, 400),
            Judged::Renew { member } if member.expires_ms == 1_400
        ));
    }

    /// The handshake: an offer is confirmed by the exchange that names it, which ends the
    /// incarnation before it; a join alone changes no member. Red if a confirmation keeps the
    /// earlier member, or takes a mark below 1.
    #[test]
    fn an_exchange_that_names_the_offer_confirms_it_and_ends_the_member_before() {
        let held = member(7, 500);
        let offered = offer(9, 450);
        let judged = admit(Some(&held), Some(&offered), 0, 9, 0, TERMS, 400);
        assert_eq!(
            judged.admission(),
            Admission::Confirmed {
                mark: 1,
                ended: Some(7)
            }
        );
        let Judged::Confirm { member, .. } = judged else {
            panic!("a confirmation");
        };
        assert_eq!(member.incarnation, 9);
        assert_eq!(
            member.expires_ms, 500,
            "provisional until a session keeps it"
        );
        assert!(!member.kept);
    }

    /// The fence: an incarnation neither the member's nor a standing offer's takes nothing in,
    /// and one that ran out is held no more. Red if a run-out member or offer is taken.
    #[test]
    fn what_the_store_does_not_hold_is_fenced() {
        let held = member(7, 500);
        assert_eq!(
            admit(Some(&held), None, 0, 6, 1, TERMS, 400).admission(),
            Admission::Fenced { current: Some(7) }
        );
        assert_eq!(
            admit(Some(&held), None, 0, 7, 1, TERMS, 500).admission(),
            Admission::Fenced { current: None },
            "a lease runs out at its moment"
        );
        assert_eq!(
            admit(None, Some(&offer(9, 450)), 0, 9, 1, TERMS, 450).admission(),
            Admission::Fenced { current: None },
            "an offer runs out at its moment"
        );
    }

    /// The bound: a new member is not taken while as many others stand; one whose own name
    /// stands is not counted against itself. Red if the bound is not read.
    #[test]
    fn no_member_is_taken_past_the_bound() {
        assert_eq!(
            admit(None, Some(&offer(9, 450)), 2, 9, 1, TERMS, 400).admission(),
            Admission::Full
        );
        assert!(matches!(
            admit(None, Some(&offer(9, 450)), 1, 9, 1, TERMS, 400),
            Judged::Confirm { .. }
        ));
    }

    /// A join repeated while its offer stands is offered it again, and no other join is.
    #[test]
    fn a_join_is_offered_again_only_for_its_nonce_while_the_offer_stands() {
        let offered = offer(9, 450);
        assert!(offers_again(Some(&offered), "n1", 449));
        assert!(!offers_again(Some(&offered), "n2", 449));
        assert!(!offers_again(Some(&offered), "n1", 450));
        assert!(!offers_again(None, "n1", 0));
    }

    /// The condition of the write: kept only under the incarnation the member stands under at
    /// the write's moment, and a kept member is leased from it. Red if a member that ran out, or
    /// one of another incarnation, admits the write.
    #[test]
    fn a_write_is_kept_only_under_a_standing_incarnation() {
        let held = member(7, 500);
        assert_eq!(admit_under(Some(&held), 7, 499), Ok(()));
        assert_eq!(
            admit_under(Some(&held), 7, 500),
            Err(Conflict::Incarnation {
                presented: 7,
                current: None
            })
        );
        assert_eq!(
            admit_under(Some(&held), 6, 10),
            Err(Conflict::Incarnation {
                presented: 6,
                current: Some(7)
            })
        );
        assert_eq!(
            admit_under(None, 6, 10),
            Err(Conflict::Incarnation {
                presented: 6,
                current: None
            })
        );
        let renewed = kept(&held, 450);
        assert!(renewed.kept);
        assert_eq!(renewed.expires_ms, 1_450);
    }
}
