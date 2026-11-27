// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The members of open machines (ADR 0113, D4) and the person each incarnation acts for (ADR
//! 0116, D1), sealed (ADR 0109, D8; ADR 0116, D7).
//!
//! - A member is kept under its name, the name its binding gives the pair (machine, instance), as
//!   a write's incarnations name it (`Write::under`); its incarnation, its mark, its lease and its
//!   delegation's `until` are the wrapped store's, which compares them.
//! - The person an incarnation acts for is the record of the payload `[issuer, subject,
//!   proved_ms, method, sealed]`, sealed for [`STORE_MEMBERS`] and the entry of the member's name,
//!   a colon and the incarnation in decimal, so a person moved to another incarnation, or to
//!   another member, does not open. In the wrapped store the person's subject is the entry of the
//!   person's key, by which a revocation or a renewal finds them, its issuer and its method are
//!   empty, and its moment is 0.

use sce_forge_runtime::cbor::{
    read_bytes, read_text, read_uint, write_array_head, write_bytes, write_text, write_uint,
};
use sce_rust_session_store::{
    moment, Admission, Delegation, Ended, Failure, Joining, Member, Membership, MembershipStore,
    Offered, Person, PersonKey, Renewal, Revocation, Terms,
};
use zeroize::Zeroizing;

use super::{decoded, encoded, expect_items, Reseal, Resealed, SealError, Sealed};
use crate::cose::put;

/// The name the persons of a `MembershipStore` are sealed for.
pub const STORE_MEMBERS: &str = "member_delegations";

/// How many members one page of a re-seal pass lists.
const PAGE: usize = 256;

/// The payload of `delegation`'s person and of the bytes kept with it, its moment as a store
/// keeps it.
fn payload(delegation: &Delegation) -> Zeroizing<Vec<u8>> {
    let person = &delegation.person;
    let lengths =
        person.issuer.len() + person.subject.len() + person.method.len() + delegation.sealed.len();
    encoded(lengths + 45, |w| {
        put(write_array_head(w, 5));
        put(write_text(w, &person.issuer));
        put(write_text(w, &person.subject));
        put(write_uint(w, moment(person.proved_ms)));
        put(write_text(w, &person.method));
        put(write_bytes(w, &delegation.sealed));
    })
}

/// The person and the bytes kept with it that a delegation's payload holds.
fn person_of(plaintext: &[u8]) -> Result<(Person, Vec<u8>), SealError> {
    decoded(plaintext, |c| {
        expect_items(c, 5)?;
        let issuer = read_text(c, None)?.to_string();
        let subject = read_text(c, None)?.to_string();
        let proved_ms = read_uint(c)?;
        let method = read_text(c, None)?.to_string();
        let sealed = read_bytes(c, None)?.to_vec();
        Ok((
            Person {
                issuer,
                subject,
                proved_ms,
                method,
            },
            sealed,
        ))
    })
}

/// The entry the delegation of `member` under `incarnation` is sealed for.
fn delegation_entry(member: &str, incarnation: u64) -> String {
    format!("{member}:{incarnation}")
}

impl<S> Sealed<S> {
    /// The entry the person of `key` is found by: the entry of the issuer's length in bytes in
    /// decimal, a colon, the issuer and the subject, which the length keeps apart.
    fn person_entry(&self, key: PersonKey<'_>) -> String {
        self.digest_of(&format!(
            "{}:{}{}",
            key.issuer.len(),
            key.issuer,
            key.subject
        ))
    }

    /// `delegation` of `member` under `incarnation`, as the wrapped store keeps it.
    fn seal_delegation(
        &self,
        member: &str,
        incarnation: u64,
        delegation: &Delegation,
    ) -> Result<Delegation, Failure> {
        let record = self.seal(
            STORE_MEMBERS,
            delegation_entry(member, incarnation).as_bytes(),
            &payload(delegation),
        )?;
        Ok(Delegation {
            person: Person {
                issuer: String::new(),
                subject: self.person_entry(delegation.person.key()),
                proved_ms: 0,
                method: String::new(),
            },
            until_ms: delegation.until_ms,
            sealed: record,
        })
    }

    /// The delegation the wrapped store keeps as `held` for `member` under `incarnation`, opened,
    /// and refused when its person is not the one the wrapped store finds it by.
    fn open_delegation(
        &self,
        member: &str,
        incarnation: u64,
        held: &Delegation,
    ) -> Result<Delegation, Failure> {
        let plaintext = self.open(
            STORE_MEMBERS,
            delegation_entry(member, incarnation).as_bytes(),
            &held.sealed,
        )?;
        let (person, sealed) = person_of(&plaintext)?;
        if self.person_entry(person.key()) != held.person.subject {
            return Err(SealError::Refused.into());
        }
        Ok(Delegation {
            person,
            until_ms: held.until_ms,
            sealed,
        })
    }

    /// `member`, as the wrapped store keeps it under `name`, its delegation opened.
    fn opened_member(&self, name: &str, member: Member) -> Result<Member, Failure> {
        let delegation = member
            .delegation
            .as_ref()
            .map(|held| self.open_delegation(name, member.incarnation, held))
            .transpose()?;
        Ok(Member {
            delegation,
            ..member
        })
    }
}

impl<S: MembershipStore> Sealed<S> {
    /// Write every person sealed under a key other than the active one again under the active
    /// one, each by the wrapped store's replacement from the bytes the pass read: one whose member
    /// changed or ended in between is left, and needs no later pass if it ended. One that does
    /// not open is counted and left.
    pub fn reseal_members(&self) -> Result<Resealed, Failure> {
        let mut resealed = Resealed::default();
        let mut after: Option<String> = None;
        loop {
            let page = self.inner.delegated_after(after.as_deref(), PAGE)?;
            let Some(last) = page.last().cloned() else {
                break;
            };
            for name in &page {
                let Some(member) = self.inner.membership(name)?.member else {
                    continue;
                };
                let Some(held) = member.delegation else {
                    continue;
                };
                let entry = delegation_entry(name, member.incarnation);
                match self.reseal_record(STORE_MEMBERS, entry.as_bytes(), &held.sealed)? {
                    Reseal::Current => {}
                    Reseal::Refused => resealed.refused += 1,
                    Reseal::Record(record) => resealed.count(self.inner.reseal_delegation(
                        name,
                        member.incarnation,
                        &held.sealed,
                        &record,
                    )?),
                }
            }
            after = Some(last);
        }
        Ok(resealed)
    }
}

impl<S: MembershipStore> MembershipStore for Sealed<S> {
    fn offer_as(
        &self,
        member: &str,
        nonce: &str,
        joining: Joining,
        ttl_ms: u64,
        now_ms: u64,
    ) -> Result<Offered, Failure> {
        self.inner.offer_as(member, nonce, joining, ttl_ms, now_ms)
    }

    /// The person sealed for the incarnation it would bind, so that it opens for no other; the
    /// wrapped store decides whether it binds it.
    fn admit_as(
        &self,
        member: &str,
        incarnation: u64,
        mark: u64,
        delegation: Option<&Delegation>,
        terms: Terms,
        now_ms: u64,
    ) -> Result<Admission, Failure> {
        let sealed = delegation
            .map(|delegation| self.seal_delegation(member, incarnation, delegation))
            .transpose()?;
        self.inner
            .admit_as(member, incarnation, mark, sealed.as_ref(), terms, now_ms)
    }

    fn membership(&self, member: &str) -> Result<Membership, Failure> {
        let held = self.inner.membership(member)?;
        Ok(Membership {
            member: held
                .member
                .map(|kept| self.opened_member(member, kept))
                .transpose()?,
            offer: held.offer,
        })
    }

    fn end_expired(&self, before_ms: u64, limit: usize) -> Result<Vec<Ended>, Failure> {
        self.inner.end_expired(before_ms, limit)
    }

    /// A person's revocation by the entry the wrapped store finds them by.
    fn end_revoked(&self, revocation: Revocation<'_>) -> Result<Vec<Ended>, Failure> {
        match revocation {
            Revocation::Person(person) => {
                let entry = self.person_entry(person);
                self.inner.end_revoked(Revocation::Person(PersonKey {
                    issuer: "",
                    subject: &entry,
                }))
            }
            named => self.inner.end_revoked(named),
        }
    }

    fn renew_delegation(
        &self,
        member: &str,
        incarnation: u64,
        person: PersonKey<'_>,
        until_ms: u64,
        now_ms: u64,
    ) -> Result<Renewal, Failure> {
        let entry = self.person_entry(person);
        self.inner.renew_delegation(
            member,
            incarnation,
            PersonKey {
                issuer: "",
                subject: &entry,
            },
            until_ms,
            now_ms,
        )
    }

    fn delegated_after(&self, after: Option<&str>, limit: usize) -> Result<Vec<String>, Failure> {
        self.inner.delegated_after(after, limit)
    }

    /// The person of `member` under `incarnation`, opened and held to `held`, sealed again with
    /// `sealed`.
    fn reseal_delegation(
        &self,
        member: &str,
        incarnation: u64,
        held: &[u8],
        sealed: &[u8],
    ) -> Result<bool, Failure> {
        let Some(kept) = self
            .inner
            .membership(member)?
            .member
            .filter(|kept| kept.incarnation == incarnation)
        else {
            return Ok(false);
        };
        let Some(stored) = kept.delegation else {
            return Ok(false);
        };
        let opened = self.open_delegation(member, incarnation, &stored)?;
        if opened.sealed != held {
            return Ok(false);
        }
        let replacement = self.seal(
            STORE_MEMBERS,
            delegation_entry(member, incarnation).as_bytes(),
            &payload(&Delegation {
                sealed: sealed.to_vec(),
                ..opened
            }),
        )?;
        self.inner
            .reseal_delegation(member, incarnation, &stored.sealed, &replacement)
    }
}
