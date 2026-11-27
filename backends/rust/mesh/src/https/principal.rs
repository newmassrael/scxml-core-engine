// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The person behind an instance (docs/adr/0116, D2; docs/formal/Principals.tla): the challenge a
//! proof of a person is made for, the port that judges a proof, and the edge's judgement of a join
//! that says a proof follows and of the exchange that confirms it with one.
//!
//! * **A join says that a proof will follow** ([`Edge::join`]). The server offers an incarnation
//!   as for any join (docs/adr/0113, D4), and the offer waits `login_ms` for its proof, no
//!   default; only an exchange that carries a person confirms it. The incarnation the instance
//!   has meanwhile goes on.
//! * **The challenge is not drawn; it is computed** ([`Challenge::of`]): a hash, under a name of
//!   its own, of the server machine, the instance, the thumbprint of the key its exchanges are
//!   signed by and the offered incarnation. The server keeps nothing for it, and what a challenge
//!   must be (chosen by the server, for one instance and one key, of use once) is what an offered
//!   incarnation already is: an offer is confirmed once. OpenID Connect Core 1.0, section
//!   15.5.2, names the form, a `nonce` that is a hash of a value of the session.
//! * **The host's port judges the proof** ([`IdentityProof`]) against the challenge of the offer the
//!   exchange names, made with the key that signed the exchange, and the edge binds the person it
//!   answers to that incarnation in the one write of the confirmation, until `person_ttl_ms` from
//!   that moment ([`Edge::confirm`]). So a proof stolen without the key, replayed, or made for
//!   another instance or another incarnation is refused, and the offer stands until `login_ms` for
//!   a good one. No later exchange carries a proof or a token: the key's signature is enough.
//!
//! The proof travels in the body of the exchange that confirms the offer, which the instance's key
//! signs with `if-match` (SCE_MESH.md section 22.9); the dialing side computes the same challenge
//! from the offer it was given ([`Challenge::of`]) and gives it to its identity provider.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use sha2::{Digest, Sha256};

use sce_rust_session_store::{
    Admission, Delegation, Failure, Joining, MembershipStore, Offered, Person, Terms,
};

use crate::peers::instance_peer;

/// The name the challenge is a hash under, so that no hash of the same values made for another
/// purpose is one.
pub const CHALLENGE_NAME: &str = "sce-mesh-principal-challenge-1";

/// The bytes of a challenge: SHA-256's.
pub const CHALLENGE_BYTES: usize = 32;

/// The offer a proof of a person is made for: the server machine it was offered by, the instance
/// it was offered to, the key that instance's exchanges are signed by, and the incarnation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Offering<'a> {
    /// The server machine the instance joins.
    pub server: &'a str,
    /// The open machine the instance is an instance of.
    pub machine: &'a str,
    /// The instance id (docs/adr/0113, D1).
    pub instance: &'a str,
    /// The thumbprint of the key the instance's exchanges are signed by: the `keyid` of its
    /// request signatures (SCE_MESH.md section 22.9; RFC 9679).
    pub key: &'a str,
    /// The incarnation it was offered.
    pub incarnation: u64,
}

/// What a proof of a person carries for one offer ([`Challenge::of`]): an OpenID Connect ID Token
/// as its `nonce`, a WebAuthn assertion as its `challenge`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Challenge([u8; CHALLENGE_BYTES]);

impl Challenge {
    /// The challenge of `offering`: the SHA-256 of [`CHALLENGE_NAME`], the server machine, the
    /// machine, the instance and the key's thumbprint, each after its length in eight bytes, most
    /// significant first, and the incarnation in eight bytes, most significant first. A change of
    /// any part is another challenge.
    pub fn of(offering: &Offering<'_>) -> Self {
        let mut hash = Sha256::new();
        for part in [
            CHALLENGE_NAME,
            offering.server,
            offering.machine,
            offering.instance,
            offering.key,
        ] {
            hash.update((part.len() as u64).to_be_bytes());
            hash.update(part.as_bytes());
        }
        hash.update(offering.incarnation.to_be_bytes());
        Self(hash.finalize().into())
    }

    /// Its bytes.
    pub fn bytes(&self) -> &[u8; CHALLENGE_BYTES] {
        &self.0
    }

    /// Its text, as a proof carries it: base64url without padding.
    pub fn text(&self) -> String {
        URL_SAFE_NO_PAD.encode(self.0)
    }
}

impl fmt::Debug for Challenge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Challenge({})", self.text())
    }
}

/// Why a port refused a proof. Nothing was written: the offer stands, until it runs out, for a
/// good one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProofRefused {
    /// The bytes are no proof the port reads.
    Malformed,
    /// It was made for another challenge: another offer, server, instance, key or incarnation.
    Challenge,
    /// It is not signed by a key the port trusts for its issuer.
    Signature,
    /// It was made for another audience than this server.
    Audience,
    /// Its issuer is not one the port trusts.
    Issuer,
    /// It is not good at the moment it was judged: made later, or no longer good.
    Time,
}

impl fmt::Display for ProofRefused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Malformed => "the bytes are no proof of a person",
            Self::Challenge => "the proof was made for another challenge",
            Self::Signature => "the proof is not signed by a key trusted for its issuer",
            Self::Audience => "the proof was made for another audience",
            Self::Issuer => "the proof's issuer is not trusted",
            Self::Time => "the proof is not good at this moment",
        })
    }
}

impl std::error::Error for ProofRefused {}

/// The host's port of proofs of a person (docs/adr/0116, D2 and D8): the identity providers and
/// their adapters are the application's, and each adapter is held to the kit of module
/// `principal_conformance`. With OpenID Connect the server's program is the confidential client
/// and exchanges the code, so a browser holds no token of the identity provider.
pub trait IdentityProof: Send + Sync {
    /// The person `proof` proves, when it is a proof the port trusts, made for `challenge` and
    /// for this server's audience, and good at `now_ms`: their issuer and subject, when and how they
    /// proved it.
    fn verify(
        &self,
        proof: &[u8],
        challenge: &Challenge,
        now_ms: u64,
    ) -> Result<Person, ProofRefused>;
}

/// What a deployment holds the people behind its instances to (docs/adr/0116, D2 and D6). Neither
/// number has a default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Login {
    /// `login_ms`: how long an offer waits for its proof.
    pub login_ms: u64,
    /// `person_ttl_ms`: how long an incarnation acts for its person after the proof, or after the
    /// last renewal (`MembershipStore::renew_delegation`).
    pub person_ttl_ms: u64,
}

/// The exchange that confirms an offer with a proof of a person: who sent it, the key that signed
/// it, the incarnation it names in `if-match`, its low-water mark, and the proof its body carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Confirming<'a> {
    pub machine: &'a str,
    pub instance: &'a str,
    /// The thumbprint of the key whose signature over the exchange the server verified.
    pub key: &'a str,
    pub incarnation: u64,
    pub mark: u64,
    pub proof: &'a [u8],
}

/// How the edge judged an exchange that confirms an offer with a proof ([`Edge::confirm`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Proved {
    /// The port judged the proof good for the offer the exchange names, and the store judged the
    /// exchange: [`Admission::Confirmed`] when it bound the person to the incarnation.
    Judged(Admission),
    /// The port refused the proof: nothing was written, and the offer stands, until it runs out,
    /// for a good one.
    Refused(ProofRefused),
}

/// The edge of a server machine whose open machine's instances act for people: the store its
/// members are kept in, the port its proofs are judged by, and the terms of both.
pub struct Edge<'a, S: ?Sized, P: ?Sized> {
    pub store: &'a S,
    pub port: &'a P,
    /// The server machine the instances join: the first part of every challenge.
    pub server: &'a str,
    pub terms: Terms,
    pub login: Login,
}

impl<S: MembershipStore + ?Sized, P: IdentityProof + ?Sized> Edge<'_, S, P> {
    /// Answer a join of `instance` of `machine` that carries `nonce` and says a proof follows, at
    /// `now_ms`: an offer that waits `login_ms` for the exchange that carries one, offered again to a
    /// repeat of the join while it stands.
    pub fn join(
        &self,
        machine: &str,
        instance: &str,
        nonce: &str,
        now_ms: u64,
    ) -> Result<Offered, Failure> {
        self.store.offer_as(
            &instance_peer(machine, instance),
            nonce,
            Joining::Proof,
            self.login.login_ms,
            now_ms,
        )
    }

    /// Judge `exchange` at `now_ms`: the port judges its proof against the challenge of the offer it
    /// names, made with the key that signed it, and refuses it or answers the person; the store
    /// then judges the exchange, binding that person to the incarnation it confirms until
    /// `person_ttl_ms` from `now_ms` (docs/adr/0116, D2 and D6). An exchange that names the member's own
    /// incarnation, a confirmation repeated, is an exchange under it, which changes whom it acts
    /// for in nothing.
    pub fn confirm(&self, exchange: &Confirming<'_>, now_ms: u64) -> Result<Proved, Failure> {
        let challenge = Challenge::of(&Offering {
            server: self.server,
            machine: exchange.machine,
            instance: exchange.instance,
            key: exchange.key,
            incarnation: exchange.incarnation,
        });
        let person = match self.port.verify(exchange.proof, &challenge, now_ms) {
            Ok(person) => person,
            Err(refused) => return Ok(Proved::Refused(refused)),
        };
        let delegation = Delegation {
            person,
            until_ms: now_ms.saturating_add(self.login.person_ttl_ms),
            sealed: Vec::new(),
        };
        self.store
            .admit_as(
                &instance_peer(exchange.machine, exchange.instance),
                exchange.incarnation,
                exchange.mark,
                Some(&delegation),
                self.terms,
                now_ms,
            )
            .map(Proved::Judged)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::https::principal_conformance::TestProof;
    use sce_rust_session_store::{Conflict, MemoryStore, SessionStore, Under, Write, WriteError};

    const SERVER: &str = "server";
    const ISSUER: &str = "https://id.example";
    const KEY: [u8; 32] = [7; 32];
    const TERMS: Terms = Terms {
        lease_ms: 10_000,
        provisional_ms: 1_000,
        max_members: 4,
    };
    const LOGIN: Login = Login {
        login_ms: 30_000,
        person_ttl_ms: 5_000,
    };

    fn port() -> TestProof {
        TestProof::new(KEY, ISSUER, SERVER, 60_000)
    }

    fn offering(key: &'static str, incarnation: u64) -> Offering<'static> {
        Offering {
            server: SERVER,
            machine: "phone",
            instance: "a1",
            key,
            incarnation,
        }
    }

    /// The exchange of `phone/a1`, signed by `key-a`, that names `incarnation` and carries `proof`.
    fn confirming(incarnation: u64, proof: &[u8]) -> Confirming<'_> {
        Confirming {
            machine: "phone",
            instance: "a1",
            key: "key-a",
            incarnation,
            mark: 1,
            proof,
        }
    }

    /// A proof of `subject` the provider the port trusts made at `made_ms` for `offering`.
    fn proof(subject: &str, offering: &Offering<'_>, made_ms: u64) -> Vec<u8> {
        TestProof::make(
            &KEY,
            ISSUER,
            SERVER,
            subject,
            &Challenge::of(offering),
            made_ms,
        )
    }

    /// docs/formal/Principals.tla, `without_bound_instance` and `without_bound_incarnation`: the
    /// challenge names the server, the instance, its key and the incarnation, so a proof made for
    /// one of them is no proof for another. Red if any part is left out of the hash.
    #[test]
    fn a_challenge_is_bound_to_its_server_instance_key_and_incarnation() {
        let one = offering("key-a", 7);
        assert_eq!(Challenge::of(&one), Challenge::of(&one), "it is computed");
        for other in [
            Offering {
                server: "elsewhere",
                ..one
            },
            Offering {
                machine: "tablet",
                ..one
            },
            Offering {
                instance: "b2",
                ..one
            },
            Offering {
                key: "key-b",
                ..one
            },
            Offering {
                incarnation: 8,
                ..one
            },
        ] {
            assert_ne!(Challenge::of(&one), Challenge::of(&other), "{other:?}");
        }
        assert_ne!(
            Challenge::of(&Offering {
                machine: "ph",
                instance: "onea1",
                ..one
            }),
            Challenge::of(&one),
            "each part is kept apart from the next by its length"
        );
        assert_eq!(
            Challenge::of(&one).text().len(),
            43,
            "base64url of 32 bytes"
        );
    }

    /// docs/adr/0116, D2: a join that says a proof follows waits `login_ms`; an exchange without a
    /// proof does not confirm it, nor one whose proof was made for another incarnation or another
    /// key (a proof given by another device), and the offer stands; the exchange whose proof was
    /// made for it binds the person until `person_ttl_ms` from that moment. Red if the edge judges a
    /// proof against any challenge but the offer's own.
    #[test]
    fn only_a_proof_made_for_the_offer_confirms_it() {
        let store = MemoryStore::new();
        let port = port();
        let edge = Edge {
            store: &store,
            port: &port,
            server: SERVER,
            terms: TERMS,
            login: LOGIN,
        };
        let offered = edge.join("phone", "a1", "n1", 100).expect("a join");
        assert_eq!(offered.expires_ms, 30_100, "the offer waits login_ms");
        assert_eq!(
            store
                .admit("phone/a1", offered.incarnation, 1, TERMS, 200)
                .expect("an exchange"),
            Admission::Unproved
        );
        let earlier = proof("alice", &offering("key-a", offered.incarnation - 1), 300);
        assert_eq!(
            edge.confirm(&confirming(offered.incarnation, &earlier), 400)
                .expect("judged"),
            Proved::Refused(ProofRefused::Challenge),
            "a proof of who was there before"
        );
        let thiefs = proof("mallory", &offering("key-t", offered.incarnation), 300);
        assert_eq!(
            edge.confirm(&confirming(offered.incarnation, &thiefs), 400)
                .expect("judged"),
            Proved::Refused(ProofRefused::Challenge),
            "a proof made for another device's key"
        );
        assert!(
            store
                .membership("phone/a1")
                .expect("reading")
                .offer
                .is_some(),
            "a refused proof leaves the offer standing"
        );
        let good = proof("alice", &offering("key-a", offered.incarnation), 20_000);
        assert_eq!(
            edge.confirm(&confirming(offered.incarnation, &good), 20_000)
                .expect("judged"),
            Proved::Judged(Admission::Confirmed {
                mark: 1,
                ended: None
            })
        );
        let delegation = store
            .membership("phone/a1")
            .expect("reading")
            .member
            .and_then(|member| member.delegation)
            .expect("the person the proof names");
        assert_eq!(
            (
                delegation.person.issuer.as_str(),
                delegation.person.subject.as_str()
            ),
            (ISSUER, "alice")
        );
        assert_eq!(delegation.until_ms, 25_000, "person_ttl_ms from the proof");
    }

    /// docs/adr/0116, D6 (docs/formal/Principals.tla, `without_condition`): a run whose write comes
    /// at or after the incarnation's `until` keeps nothing, however long the member's lease. Red
    /// if the edge binds a person with no `until` the write reads.
    #[test]
    fn a_run_written_after_the_persons_until_keeps_nothing() {
        let store = MemoryStore::new();
        let port = port();
        let edge = Edge {
            store: &store,
            port: &port,
            server: SERVER,
            terms: TERMS,
            login: LOGIN,
        };
        let offered = edge.join("phone", "a1", "n1", 0).expect("a join");
        let made = proof("alice", &offering("key-a", offered.incarnation), 100);
        assert!(matches!(
            edge.confirm(&confirming(offered.incarnation, &made), 100),
            Ok(Proved::Judged(Admission::Confirmed { .. }))
        ));
        let lease = store.acquire("room", "a", 60_000, 0).expect("a lease");
        let under = [Under {
            member: "phone/a1",
            incarnation: offered.incarnation,
        }];
        let written = store
            .write(
                "room",
                &Write {
                    now_ms: 600,
                    under: &under,
                    ..Write::put(b"kept", None, 0, lease.fence)
                },
            )
            .expect("a run kept while the person stands");
        assert!(matches!(
            store.write(
                "room",
                &Write {
                    now_ms: 5_100,
                    under: &under,
                    ..Write::put(b"late", None, written.version, lease.fence)
                },
            ),
            Err(WriteError::Conflict(Conflict::Incarnation {
                current: None,
                ..
            }))
        ));
    }
}
