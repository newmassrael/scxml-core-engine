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
//! * **A member may act for a person** (ADR 0116, D1 and D6; docs/formal/Principals.tla): the
//!   person a proof bound for the offered incarnation, kept with the member the exchange that
//!   carried it confirmed ([`MembershipStore::admit_as`]), until a moment no exchange moves
//!   ([`Delegation`]). An incarnation acts for at most one person, from its confirmation to its
//!   end, and for nobody when none was bound; no later exchange under it changes either. Past its
//!   `until` the member stands no more, as past its lease, so no write made under it keeps
//!   anything. A renewal moves `until` alone, and only for the same person
//!   ([`MembershipStore::renew_delegation`]); a revocation ends, in one write, every incarnation it
//!   names ([`MembershipStore::end_revoked`]).
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

/// How the store judged an exchange that names an incarnation ([`MembershipStore::admit_as`]).
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

/// A person as an identity provider proved them (ADR 0116, D2 and D3): what the host's port of
/// proofs answers for a proof it judged good, and what a document reads of the person an instance
/// acts for. No display name, address or group is among its fields: those are the application's
/// data, and facts of a stream (D5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Person {
    /// Who proved them: the identity provider's issuer (OpenID Connect's `iss`).
    pub issuer: String,
    /// Who they are to their issuer (`sub`): unique within the issuer alone, so the two are one
    /// key ([`PersonKey`]).
    pub subject: String,
    /// When they proved it (`auth_time`), in milliseconds since the Unix epoch.
    pub proved_ms: u64,
    /// How they proved it, as the port that judged the proof names it.
    pub method: String,
}

impl Person {
    /// The key the person is found by: their issuer and their subject.
    pub fn key(&self) -> PersonKey<'_> {
        PersonKey {
            issuer: &self.issuer,
            subject: &self.subject,
        }
    }
}

/// Who a person is, as a store finds them: a subject is unique only within its issuer, so the two
/// are one key (OpenID Connect Core 1.0, section 2, `sub`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PersonKey<'a> {
    pub issuer: &'a str,
    pub subject: &'a str,
}

/// The person an incarnation acts for, and until when (ADR 0116, D1 and D6): a part of the
/// membership, written by the confirmation that bound it and ended with the incarnation, never a
/// record with a life of its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delegation {
    pub person: Person,
    /// `until`: the moment the incarnation stops acting for the person, and so ends, in
    /// milliseconds since the Unix epoch: `person_ttl_ms` after the proof or the last renewal. No
    /// exchange moves it, and a renewal moves it alone ([`MembershipStore::renew_delegation`]).
    pub until_ms: u64,
    /// Bytes kept with the delegation that the store never reads: empty in one the edge writes. A
    /// store that wraps another and seals what it keeps (ADR 0109, D8) keeps the person's sealed
    /// record here in the one it wraps, and of the person in that one's other fields only the
    /// entry it finds them by.
    pub sealed: Vec<u8>,
}

/// A member as the store holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    /// The incarnation it is a member under.
    pub incarnation: u64,
    /// Its low-water mark: every number of its incarnation below this has its answer.
    pub mark: u64,
    /// When its lease runs out unless an exchange renews it, in milliseconds since the Unix epoch.
    /// It stops being a member then, or at its delegation's `until` when that comes first
    /// ([`Member::ends_ms`]).
    pub expires_ms: u64,
    /// Whether a write kept a run made under it (`Write::under`).
    ///
    /// [`Write::under`]: crate::Write::under
    pub kept: bool,
    /// The lease it is renewed to once a write kept a run of it: the `lease_ms` of the terms it
    /// was last admitted under.
    pub lease_ms: u64,
    /// The person its incarnation acts for, or `None` when it acts for nobody (ADR 0116, D1):
    /// bound by the exchange that confirmed it, and never by a later one.
    pub delegation: Option<Delegation>,
}

impl Member {
    /// When it stops being a member unless an exchange renews it: the end of its lease, or its
    /// delegation's `until` when that comes first, which no exchange moves.
    pub fn ends_ms(&self) -> u64 {
        self.delegation
            .as_ref()
            .map_or(self.expires_ms, |delegation| {
                self.expires_ms.min(delegation.until_ms)
            })
    }
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

/// A member [`MembershipStore::end_expired`] or [`MembershipStore::end_revoked`] removed: what a host
/// held for it ends with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ended {
    pub member: String,
    pub incarnation: u64,
}

/// What a revocation names (ADR 0116, D6). Every incarnation it names ends, in one write, by the
/// one way an incarnation ends: the store holds it no more, so an exchange under it is fenced and
/// a write made under it keeps nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revocation<'a> {
    /// Every incarnation that acts for the person: what an operator, or the adapter of an identity
    /// provider that tells (back-channel logout), ends.
    Person(PersonKey<'a>),
    /// The incarnation the member of this name is a member under, whoever it acts for.
    Instance(&'a str),
    /// The incarnation `incarnation` of the member `member`, when it is the one it is a member
    /// under: what an instance that gives its incarnation up ends.
    Incarnation { member: &'a str, incarnation: u64 },
}

/// How the store judged a renewal of a delegation ([`MembershipStore::renew_delegation`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Renewal {
    /// The delegation stands until `until_ms`, the later of the one held and the one given.
    Renewed { until_ms: u64 },
    /// The store holds no such incarnation standing for the member: nothing was written.
    /// `current` is the member's incarnation, `None` when no member stands.
    Fenced { current: Option<u64> },
    /// The incarnation acts for nobody, or for another person: nothing was written. A proof of
    /// another person renews nothing; it is a join (ADR 0116, D6).
    Refused,
}

/// The members of open machines, kept beside the sessions whose writes are made under them (ADR
/// 0113, D4 and D5), with the person each incarnation acts for (ADR 0116, D1). Every method is
/// one transaction.
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

    /// [`MembershipStore::admit_as`] for an exchange that carries no proof of a person: an offer it
    /// confirms makes a member that acts for nobody.
    fn admit(
        &self,
        member: &str,
        incarnation: u64,
        mark: u64,
        terms: Terms,
        now_ms: u64,
    ) -> Result<Admission, Failure> {
        self.admit_as(member, incarnation, mark, None, terms, now_ms)
    }

    /// Judge an exchange of `member` that names `incarnation` and carries the low-water mark
    /// `mark`, and `delegation` when it carries a proof of a person the host judged good for the
    /// incarnation it names, at `now_ms`, under `terms`, and write what it takes in:
    ///
    /// * the member's own incarnation, standing (its lease and its delegation's `until` both after
    ///   `now_ms`): its lease renewed from `now_ms` (`lease_ms` once a write kept a run of it,
    ///   `provisional_ms` before), its mark raised to `mark` when that is above it, and the person
    ///   it acts for, or its acting for nobody, as it was: `delegation` is not read
    ///   ([`Admission::Current`]);
    /// * the offer that stands for it: unless as many other members stand as
    ///   [`Terms::max_members`] ([`Admission::Full`], nothing written), the instance is a member
    ///   under it with the mark `mark` (at least 1) and a provisional lease, acting for
    ///   `delegation`'s person until its `until`, or for nobody without one; the offer is taken,
    ///   and the member it had before is ended ([`Admission::Confirmed`]);
    /// * anything else: [`Admission::Fenced`], nothing written.
    fn admit_as(
        &self,
        member: &str,
        incarnation: u64,
        mark: u64,
        delegation: Option<&Delegation>,
        terms: Terms,
        now_ms: u64,
    ) -> Result<Admission, Failure>;

    /// What the store holds of `member`.
    fn membership(&self, member: &str) -> Result<Membership, Failure>;

    /// Remove at most `limit` members and at most `limit` offers that ran out at or before
    /// `before_ms` (a member at the end of its lease or of its delegation, [`Member::ends_ms`]),
    /// those that ran out first first, and answer the members removed, in that order: what a host
    /// held for each (its window, its outbox) ends with it.
    fn end_expired(&self, before_ms: u64, limit: usize) -> Result<Vec<Ended>, Failure>;

    /// End every incarnation `revocation` names, standing or run out and not yet removed, in one
    /// write, and answer the members removed, in the order of their names' UTF-8 bytes: what a
    /// host held for each ends with it. An offer is no incarnation yet, and stays.
    fn end_revoked(&self, revocation: Revocation<'_>) -> Result<Vec<Ended>, Failure>;

    /// Renew the delegation of `member` under `incarnation` for `person` until `until_ms`, at
    /// `now_ms` (ADR 0116, D6): a proof of the same person, or the host's answer that the identity
    /// provider's session stands. When the member stands under the incarnation and acts for that
    /// person, its `until` is moved to `until_ms` unless it is later already, and nothing else is
    /// moved: the person's `proved_ms` and `method` stay those of the proof that bound them, and
    /// the lease is the exchanges' ([`Renewal::Renewed`]). Otherwise nothing is written
    /// ([`Renewal::Fenced`], [`Renewal::Refused`]).
    fn renew_delegation(
        &self,
        member: &str,
        incarnation: u64,
        person: PersonKey<'_>,
        until_ms: u64,
        now_ms: u64,
    ) -> Result<Renewal, Failure>;

    /// At most `limit` names of the members that act for a person, standing or run out and not
    /// yet removed, in the order of their UTF-8 bytes, each after `after` when it is given. A pass
    /// over every delegation (the re-seal of ADR 0109, D8) reads them a page at a time, each page
    /// after the last name of the one before.
    fn delegated_after(&self, after: Option<&str>, limit: usize) -> Result<Vec<String>, Failure>;

    /// Replace the [`Delegation::sealed`] bytes of the member `member` under `incarnation` with
    /// `sealed` when they are `held`, every other field kept, and answer whether it did: `false`,
    /// writing nothing, when the store holds no such member, it acts for nobody, or its bytes are
    /// others. This is how a store that seals what it keeps seals a person again under its active
    /// key (ADR 0109, D8).
    fn reseal_delegation(
        &self,
        member: &str,
        incarnation: u64,
        held: &[u8],
        sealed: &[u8],
    ) -> Result<bool, Failure>;
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
    use super::{
        Admission, Delegation, Member, Offer, Person, PersonKey, Renewal, Revocation, Terms,
    };
    use crate::{moment, rules, Conflict};

    /// Whether what runs out at `expires_ms` still stands at `now_ms`: as a lease, it runs out
    /// at that moment.
    pub(crate) fn stands(expires_ms: u64, now_ms: u64) -> bool {
        now_ms < expires_ms
    }

    /// Whether `member` still stands at `now_ms`: neither its lease nor its delegation ran out.
    pub(crate) fn member_stands(member: &Member, now_ms: u64) -> bool {
        stands(member.ends_ms(), now_ms)
    }

    /// Whether a join with `nonce` is offered `offered` again at `now_ms`: it stands, and it
    /// answers a join of the same nonce.
    pub(crate) fn offers_again(offered: Option<&Offer>, nonce: &str, now_ms: u64) -> bool {
        offered.is_some_and(|offer| offer.nonce == nonce && stands(offer.expires_ms, now_ms))
    }

    /// What an exchange carries that the store judges: the incarnation it names, its low-water
    /// mark, and the delegation a proof it carries binds.
    #[derive(Debug, Clone, Copy)]
    pub(crate) struct Exchange<'a> {
        pub(crate) incarnation: u64,
        pub(crate) mark: u64,
        pub(crate) delegation: Option<&'a Delegation>,
    }

    /// What an exchange is to write, decided before anything is written.
    #[derive(Debug, Clone, PartialEq, Eq)]
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
        pub(crate) fn admission(&self) -> Admission {
            match self {
                Judged::Renew { member } => Admission::Current { mark: member.mark },
                Judged::Confirm { member, ended } => Admission::Confirmed {
                    mark: member.mark,
                    ended: *ended,
                },
                Judged::Refuse(admission) => *admission,
            }
        }
    }

    /// Judge `exchange` at `now_ms`, over the member `held` and the offer `offered` of its name
    /// (standing or not), with `others` the members of other names that stand at `now_ms`.
    pub(crate) fn admit(
        held: Option<&Member>,
        offered: Option<&Offer>,
        others: u64,
        exchange: Exchange<'_>,
        terms: Terms,
        now_ms: u64,
    ) -> Judged {
        // A number a store keeps is at most the largest signed 64-bit one, as a moment is: no
        // instance numbers that many requests in one incarnation, and no lease is that long.
        let mark = exchange.mark.min(crate::LATEST_MOMENT);
        let lease_ms = terms.lease_ms.min(crate::LATEST_MOMENT);
        let standing = held.filter(|member| member_stands(member, now_ms));
        if let Some(member) = standing.filter(|member| member.incarnation == exchange.incarnation) {
            let lease = if member.kept {
                terms.lease_ms
            } else {
                terms.provisional_ms
            };
            // ADR 0116, D1: an exchange under an incarnation changes neither whom it acts for nor
            // until when, whatever proof it carries.
            return Judged::Renew {
                member: Member {
                    mark: member.mark.max(mark),
                    expires_ms: rules::expiry(now_ms, lease),
                    lease_ms,
                    ..member.clone()
                },
            };
        }
        let confirms = offered.is_some_and(|offer| {
            offer.incarnation == exchange.incarnation && stands(offer.expires_ms, now_ms)
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
                incarnation: exchange.incarnation,
                mark: mark.max(1),
                expires_ms: rules::expiry(now_ms, terms.provisional_ms),
                kept: false,
                lease_ms,
                delegation: exchange.delegation.map(kept_delegation),
            },
            ended: held.map(|member| member.incarnation),
        }
    }

    /// `delegation` as a store keeps it: no moment of it later than the latest a store holds.
    fn kept_delegation(delegation: &Delegation) -> Delegation {
        Delegation {
            person: Person {
                proved_ms: moment(delegation.person.proved_ms),
                ..delegation.person.clone()
            },
            until_ms: moment(delegation.until_ms),
            sealed: delegation.sealed.clone(),
        }
    }

    /// Whether a write made under `incarnation` may keep anything, over the member `held` of the
    /// name it gives, at the write's moment `now_ms`: only while the member stands under it, its
    /// lease and its delegation's `until` both read against that moment.
    pub(crate) fn admit_under(
        held: Option<&Member>,
        incarnation: u64,
        now_ms: u64,
    ) -> Result<(), Conflict> {
        let current = held
            .filter(|member| member_stands(member, now_ms))
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
    /// `lease_ms` from the write, unless it already stands longer. Its delegation is not moved.
    pub(crate) fn kept(member: &Member, now_ms: u64) -> Member {
        Member {
            kept: true,
            expires_ms: member
                .expires_ms
                .max(rules::expiry(now_ms, member.lease_ms)),
            ..member.clone()
        }
    }

    /// What a renewal of the delegation of `incarnation` for `person` until `until_ms`, at
    /// `now_ms`, makes of the member `held`: how it is answered, and the member to write when it
    /// renews.
    pub(crate) fn renew(
        held: Option<&Member>,
        incarnation: u64,
        person: PersonKey<'_>,
        until_ms: u64,
        now_ms: u64,
    ) -> (Renewal, Option<Member>) {
        let standing = held.filter(|member| member_stands(member, now_ms));
        let Some(member) = standing.filter(|member| member.incarnation == incarnation) else {
            return (
                Renewal::Fenced {
                    current: standing.map(|member| member.incarnation),
                },
                None,
            );
        };
        match &member.delegation {
            Some(delegation) if delegation.person.key() == person => {
                let until_ms = delegation.until_ms.max(moment(until_ms));
                let renewed = Member {
                    delegation: Some(Delegation {
                        until_ms,
                        ..delegation.clone()
                    }),
                    ..member.clone()
                };
                (Renewal::Renewed { until_ms }, Some(renewed))
            }
            _ => (Renewal::Refused, None),
        }
    }

    /// Whether `revocation` names the member `held` of the name `name`.
    pub(crate) fn revokes(revocation: &Revocation<'_>, name: &str, held: &Member) -> bool {
        match *revocation {
            Revocation::Person(person) => held
                .delegation
                .as_ref()
                .is_some_and(|delegation| delegation.person.key() == person),
            Revocation::Instance(member) => member == name,
            Revocation::Incarnation {
                member,
                incarnation,
            } => member == name && held.incarnation == incarnation,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::judge::{admit, admit_under, kept, offers_again, renew, revokes, Exchange, Judged};
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
            delegation: None,
        }
    }

    fn offer(incarnation: u64, expires_ms: u64) -> Offer {
        Offer {
            incarnation,
            nonce: "n1".to_string(),
            expires_ms,
        }
    }

    /// An exchange that names `incarnation` with the mark `mark` and carries no proof.
    fn exchange(incarnation: u64, mark: u64) -> Exchange<'static> {
        Exchange {
            incarnation,
            mark,
            delegation: None,
        }
    }

    /// The delegation to the person `subject` of one issuer, proved at 10, until `until_ms`.
    fn delegation(subject: &str, until_ms: u64) -> Delegation {
        Delegation {
            person: Person {
                issuer: "https://id.example".to_string(),
                subject: subject.to_string(),
                proved_ms: 10,
                method: "pwd".to_string(),
            },
            until_ms,
            sealed: Vec::new(),
        }
    }

    /// docs/formal/OpenInstances.tla, `SEdge`: the member's own incarnation renews it and raises
    /// its mark, and never lowers it. Red if the mark is taken as given, or the lease is not
    /// renewed from the exchange.
    #[test]
    fn the_members_own_incarnation_renews_it_and_raises_its_mark() {
        let held = member(7, 500);
        let judged = admit(Some(&held), None, 0, exchange(7, 9), TERMS, 400);
        assert_eq!(judged.admission(), Admission::Current { mark: 9 });
        assert!(matches!(judged, Judged::Renew { member } if member.expires_ms == 500));
        let lower = admit(Some(&held), None, 0, exchange(7, 1), TERMS, 400);
        assert_eq!(lower.admission(), Admission::Current { mark: 3 });
        let kept_one = Member { kept: true, ..held };
        assert!(matches!(
            admit(Some(&kept_one), None, 0, exchange(7, 1), TERMS, 400),
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
        let judged = admit(Some(&held), Some(&offered), 0, exchange(9, 0), TERMS, 400);
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
        assert_eq!(member.delegation, None, "no proof binds nobody's person");
    }

    /// The fence: an incarnation neither the member's nor a standing offer's takes nothing in,
    /// and one that ran out is held no more. Red if a run-out member or offer is taken.
    #[test]
    fn what_the_store_does_not_hold_is_fenced() {
        let held = member(7, 500);
        assert_eq!(
            admit(Some(&held), None, 0, exchange(6, 1), TERMS, 400).admission(),
            Admission::Fenced { current: Some(7) }
        );
        assert_eq!(
            admit(Some(&held), None, 0, exchange(7, 1), TERMS, 500).admission(),
            Admission::Fenced { current: None },
            "a lease runs out at its moment"
        );
        assert_eq!(
            admit(None, Some(&offer(9, 450)), 0, exchange(9, 1), TERMS, 450).admission(),
            Admission::Fenced { current: None },
            "an offer runs out at its moment"
        );
    }

    /// The bound: a new member is not taken while as many others stand; one whose own name
    /// stands is not counted against itself. Red if the bound is not read.
    #[test]
    fn no_member_is_taken_past_the_bound() {
        assert_eq!(
            admit(None, Some(&offer(9, 450)), 2, exchange(9, 1), TERMS, 400).admission(),
            Admission::Full
        );
        assert!(matches!(
            admit(None, Some(&offer(9, 450)), 1, exchange(9, 1), TERMS, 400),
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

    /// ADR 0116, D2: the exchange that confirms an offer and carries a proof binds its person,
    /// with its `until`, to the incarnation it confirms. Red if the delegation is not kept.
    #[test]
    fn a_confirmation_binds_the_person_its_proof_names() {
        let bound = delegation("alice", 5_000);
        let judged = admit(
            None,
            Some(&offer(9, 450)),
            0,
            Exchange {
                incarnation: 9,
                mark: 1,
                delegation: Some(&bound),
            },
            TERMS,
            400,
        );
        let Judged::Confirm { member, .. } = judged else {
            panic!("a confirmation");
        };
        assert_eq!(member.delegation, Some(bound));
    }

    /// ADR 0116, D1 (docs/formal/Principals.tla, `OnePerson`): an exchange under an incarnation
    /// changes neither the person it acts for nor its `until`, nor binds one to an incarnation that
    /// acts for nobody. Red if a later exchange's delegation is written.
    #[test]
    fn an_exchange_under_an_incarnation_changes_whom_it_acts_for_in_nothing() {
        let alice = Member {
            delegation: Some(delegation("alice", 5_000)),
            ..member(7, 500)
        };
        let bob = delegation("bob", 9_000);
        let carrying = Exchange {
            incarnation: 7,
            mark: 4,
            delegation: Some(&bob),
        };
        let Judged::Renew { member: renewed } = admit(Some(&alice), None, 0, carrying, TERMS, 400)
        else {
            panic!("a renewal of the lease");
        };
        assert_eq!(renewed.delegation, alice.delegation);
        let Judged::Renew { member: nobody } =
            admit(Some(&member(7, 500)), None, 0, carrying, TERMS, 400)
        else {
            panic!("a renewal of the lease");
        };
        assert_eq!(
            nobody.delegation, None,
            "nobody's incarnation stays nobody's"
        );
    }

    /// ADR 0116, D6 (docs/formal/Principals.tla, `NothingAfterEnd`): a member stands no longer
    /// than its delegation's `until`, however long its lease: no exchange is taken in and no write
    /// made under it is kept from that moment. Red if `until` is not read against the write's own
    /// moment.
    #[test]
    fn a_member_ends_at_its_delegations_until() {
        let held = Member {
            delegation: Some(delegation("alice", 450)),
            ..member(7, 500)
        };
        assert_eq!(held.ends_ms(), 450);
        assert_eq!(admit_under(Some(&held), 7, 449), Ok(()));
        assert_eq!(
            admit_under(Some(&held), 7, 450),
            Err(Conflict::Incarnation {
                presented: 7,
                current: None
            })
        );
        assert_eq!(
            admit(Some(&held), None, 0, exchange(7, 1), TERMS, 450).admission(),
            Admission::Fenced { current: None }
        );
        assert_eq!(
            kept(&held, 449).ends_ms(),
            450,
            "a kept run extends the lease and not the delegation"
        );
    }

    /// ADR 0116, D6: a renewal moves `until` alone, never back, and only for the person the
    /// incarnation acts for; one of another person, of nobody's incarnation or of an incarnation
    /// that ended renews nothing. Red if a renewal moves the proof's moment, or is taken for
    /// another person.
    #[test]
    fn a_renewal_moves_until_alone_for_the_same_person() {
        let held = Member {
            delegation: Some(delegation("alice", 450)),
            ..member(7, 500)
        };
        let alice = PersonKey {
            issuer: "https://id.example",
            subject: "alice",
        };
        let (renewal, renewed) = renew(Some(&held), 7, alice, 900, 400);
        assert_eq!(renewal, Renewal::Renewed { until_ms: 900 });
        let renewed = renewed.expect("the member to write");
        let kept_delegation = renewed.delegation.expect("still alice's");
        assert_eq!(kept_delegation.until_ms, 900);
        assert_eq!(
            kept_delegation.person.proved_ms, 10,
            "a renewal is not a proof"
        );
        assert_eq!(
            renew(Some(&held), 7, alice, 300, 400).0,
            Renewal::Renewed { until_ms: 450 },
            "a renewal never brings until back"
        );
        let bob = PersonKey {
            issuer: "https://id.example",
            subject: "bob",
        };
        assert_eq!(
            renew(Some(&held), 7, bob, 900, 400),
            (Renewal::Refused, None)
        );
        let other_issuer = PersonKey {
            issuer: "https://other.example",
            subject: "alice",
        };
        assert_eq!(
            renew(Some(&held), 7, other_issuer, 900, 400).0,
            Renewal::Refused,
            "a subject is a person only within its issuer"
        );
        assert_eq!(
            renew(Some(&member(7, 500)), 7, alice, 900, 400).0,
            Renewal::Refused
        );
        assert_eq!(
            renew(Some(&held), 6, alice, 900, 400).0,
            Renewal::Fenced { current: Some(7) }
        );
        assert_eq!(
            renew(Some(&held), 7, alice, 900, 450).0,
            Renewal::Fenced { current: None },
            "an ended delegation is not renewed"
        );
    }

    /// ADR 0116, D6: a revocation names a person, an instance or one incarnation of it. Red if a
    /// person's revocation ends a member that acts for nobody or for another.
    #[test]
    fn a_revocation_names_its_person_its_instance_or_its_incarnation() {
        let alice = PersonKey {
            issuer: "https://id.example",
            subject: "alice",
        };
        let hers = Member {
            delegation: Some(delegation("alice", 900)),
            ..member(7, 500)
        };
        let his = Member {
            delegation: Some(delegation("bob", 900)),
            ..member(8, 500)
        };
        assert!(revokes(&Revocation::Person(alice), "a", &hers));
        assert!(!revokes(&Revocation::Person(alice), "b", &his));
        assert!(!revokes(&Revocation::Person(alice), "c", &member(9, 500)));
        assert!(revokes(&Revocation::Instance("a"), "a", &hers));
        assert!(!revokes(&Revocation::Instance("a"), "b", &his));
        assert!(revokes(
            &Revocation::Incarnation {
                member: "a",
                incarnation: 7
            },
            "a",
            &hers
        ));
        assert!(!revokes(
            &Revocation::Incarnation {
                member: "a",
                incarnation: 6
            },
            "a",
            &hers
        ));
    }
}
