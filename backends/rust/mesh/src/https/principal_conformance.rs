// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What every adapter of the port of proofs ([`IdentityProof`]) is held to (docs/adr/0116, D8): a
//! good proof names its person, and a proof made for another challenge, signed by a key the port
//! does not trust, made for another audience, by another issuer, or judged out of its time, is
//! refused, each by its name; bytes that are no proof are refused too.
//!
//! An adapter runs these functions from its own `#[test]`s, with a harness that makes proofs as the
//! identity provider it trusts would, and as one wrong in one part would ([`ProofHarness`]). The
//! clock is the one the functions give; nothing here waits on time. A failed expectation panics,
//! as a test's does.
//!
//! [`TestProof`] is a port for tests, run through the kit here: whoever holds its key makes any
//! proof, so it proves nothing a deployment may rely on, and no deployment names it.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use sce_rust_crypto::equal::same_secret;
use sce_rust_crypto::mac::{hmac_sha256, TAG_BYTES};
use sce_rust_session_store::Person;

use super::principal::{Challenge, IdentityProof, Offering, ProofRefused};

/// How a harness makes a proof: as the identity provider the port trusts would, or wrong in one
/// part.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Made {
    /// As the trusted provider makes it, for this server.
    Good,
    /// Signed by a key the port does not trust.
    SignedByAnother,
    /// Made for another audience than this server.
    ForAnotherAudience,
    /// Naming an issuer the port does not trust.
    ByAnotherIssuer,
}

/// What gives the functions of this module their port and their proofs.
pub trait ProofHarness {
    type Port: IdentityProof;

    /// The port under test.
    fn port(&self) -> Self::Port;

    /// The issuer a good proof names.
    fn issuer(&self) -> String;

    /// How long after it was made the port takes a proof: one judged that long after is refused.
    fn valid_ms(&self) -> u64;

    /// A proof of `subject` for `challenge`, made at `made_ms`, as `made` says. `made_ms` is a whole
    /// second, as an OpenID Connect `auth_time` is.
    fn proof(&self, subject: &str, challenge: &Challenge, made_ms: u64, made: Made) -> Vec<u8>;
}

/// When the functions' proofs are made: a whole second.
const MADE_MS: u64 = 1_800_000_000_000;

/// The challenge of an offer, and of the offer of the next incarnation.
fn challenges() -> (Challenge, Challenge) {
    let offering = Offering {
        server: "server",
        machine: "phone",
        instance: "a1",
        key: "key-a",
        incarnation: 7,
    };
    (
        Challenge::of(&offering),
        Challenge::of(&Offering {
            incarnation: 8,
            ..offering
        }),
    )
}

/// A good proof, made for the challenge it is judged against and judged a second after it was
/// made, names its person: the issuer the port trusts, the subject it was made for, and the
/// moment it was made.
pub fn a_good_proof_names_its_person(harness: &impl ProofHarness) {
    let (challenge, _) = challenges();
    let proof = harness.proof("alice", &challenge, MADE_MS, Made::Good);
    let person = harness
        .port()
        .verify(&proof, &challenge, MADE_MS + 1_000)
        .unwrap_or_else(|refused| panic!("a good proof was refused: {refused}"));
    assert_eq!(person.issuer, harness.issuer());
    assert_eq!(person.subject, "alice");
    assert_eq!(person.proved_ms, MADE_MS, "when the person proved it");
}

/// A proof made for another challenge (another offer, instance, key or incarnation) is refused.
/// Red without it: a person told who never proved it there.
pub fn a_proof_of_another_challenge_is_refused(harness: &impl ProofHarness) {
    let (challenge, next) = challenges();
    let proof = harness.proof("alice", &next, MADE_MS, Made::Good);
    assert_eq!(
        harness.port().verify(&proof, &challenge, MADE_MS + 1_000),
        Err(ProofRefused::Challenge)
    );
}

/// A proof signed by a key the port does not trust is refused.
pub fn a_proof_signed_by_another_key_is_refused(harness: &impl ProofHarness) {
    let (challenge, _) = challenges();
    let proof = harness.proof("alice", &challenge, MADE_MS, Made::SignedByAnother);
    assert_eq!(
        harness.port().verify(&proof, &challenge, MADE_MS + 1_000),
        Err(ProofRefused::Signature)
    );
}

/// A proof made for another audience than this server is refused.
pub fn a_proof_for_another_audience_is_refused(harness: &impl ProofHarness) {
    let (challenge, _) = challenges();
    let proof = harness.proof("alice", &challenge, MADE_MS, Made::ForAnotherAudience);
    assert_eq!(
        harness.port().verify(&proof, &challenge, MADE_MS + 1_000),
        Err(ProofRefused::Audience)
    );
}

/// A proof naming an issuer the port does not trust is refused.
pub fn a_proof_of_another_issuer_is_refused(harness: &impl ProofHarness) {
    let (challenge, _) = challenges();
    let proof = harness.proof("alice", &challenge, MADE_MS, Made::ByAnotherIssuer);
    assert_eq!(
        harness.port().verify(&proof, &challenge, MADE_MS + 1_000),
        Err(ProofRefused::Issuer)
    );
}

/// A proof judged as long after it was made as the port takes one, or judged that long before, is
/// refused.
pub fn a_proof_out_of_its_time_is_refused(harness: &impl ProofHarness) {
    let (challenge, _) = challenges();
    let proof = harness.proof("alice", &challenge, MADE_MS, Made::Good);
    let port = harness.port();
    assert_eq!(
        port.verify(&proof, &challenge, MADE_MS + harness.valid_ms()),
        Err(ProofRefused::Time),
        "no longer good"
    );
    assert_eq!(
        port.verify(&proof, &challenge, MADE_MS - harness.valid_ms()),
        Err(ProofRefused::Time),
        "made later than it is judged"
    );
}

/// Bytes that are no proof are refused.
pub fn bytes_that_are_no_proof_are_refused(harness: &impl ProofHarness) {
    let (challenge, _) = challenges();
    assert_eq!(
        harness
            .port()
            .verify(b"no proof", &challenge, MADE_MS + 1_000),
        Err(ProofRefused::Malformed)
    );
}

/// A port of proofs for tests (docs/adr/0116, D8). A proof is its issuer, its audience, its
/// subject, the text of the challenge it was made for, each after its length in four bytes, most
/// significant first, and the moment it was made in eight, followed by the HMAC-SHA-256 of all of
/// it under a key: whoever holds the key makes any proof, so it proves nothing a deployment may
/// rely on, and no deployment names it. It takes a proof from the moment it was made for
/// `valid_ms`, and names the method `test`.
pub struct TestProof {
    key: [u8; 32],
    issuer: String,
    audience: String,
    valid_ms: u64,
}

impl TestProof {
    /// The port that trusts the proofs `key` signs, naming `issuer`, for `audience`, for `valid_ms`.
    pub fn new(key: [u8; 32], issuer: &str, audience: &str, valid_ms: u64) -> Self {
        Self {
            key,
            issuer: issuer.to_string(),
            audience: audience.to_string(),
            valid_ms,
        }
    }

    /// A proof of `subject`, naming `issuer`, for `audience` and `challenge`, made at `made_ms`,
    /// signed under `key`.
    pub fn make(
        key: &[u8; 32],
        issuer: &str,
        audience: &str,
        subject: &str,
        challenge: &Challenge,
        made_ms: u64,
    ) -> Vec<u8> {
        let mut proof = Vec::new();
        for field in [issuer, audience, subject, challenge.text().as_str()] {
            let length = u32::try_from(field.len()).unwrap_or(u32::MAX);
            proof.extend_from_slice(&length.to_be_bytes());
            proof.extend_from_slice(field.as_bytes());
        }
        proof.extend_from_slice(&made_ms.to_be_bytes());
        let tag = hmac_sha256(key, &proof);
        proof.extend_from_slice(&tag);
        proof
    }
}

/// The fields a proof of [`TestProof`] holds before its tag.
struct Fields {
    issuer: String,
    audience: String,
    subject: String,
    challenge: String,
    made_ms: u64,
}

/// The fields of `signed`, which must be read to its end.
fn fields(signed: &[u8]) -> Option<Fields> {
    let mut rest = signed;
    let mut texts: Vec<String> = Vec::new();
    for _ in 0..4 {
        let (length, after) = rest.split_first_chunk::<4>()?;
        let length = usize::try_from(u32::from_be_bytes(*length)).ok()?;
        if after.len() < length {
            return None;
        }
        let (text, after) = after.split_at(length);
        texts.push(String::from_utf8(text.to_vec()).ok()?);
        rest = after;
    }
    let made: [u8; 8] = rest.try_into().ok()?;
    let mut texts = texts.into_iter();
    Some(Fields {
        issuer: texts.next()?,
        audience: texts.next()?,
        subject: texts.next()?,
        challenge: texts.next()?,
        made_ms: u64::from_be_bytes(made),
    })
}

impl IdentityProof for TestProof {
    fn verify(
        &self,
        proof: &[u8],
        challenge: &Challenge,
        now_ms: u64,
    ) -> Result<Person, ProofRefused> {
        let signed_length = proof
            .len()
            .checked_sub(TAG_BYTES)
            .ok_or(ProofRefused::Malformed)?;
        let (signed, tag) = proof.split_at(signed_length);
        let fields = fields(signed).ok_or(ProofRefused::Malformed)?;
        if !same_secret(&hmac_sha256(&self.key, signed), tag) {
            return Err(ProofRefused::Signature);
        }
        if fields.issuer != self.issuer {
            return Err(ProofRefused::Issuer);
        }
        if fields.audience != self.audience {
            return Err(ProofRefused::Audience);
        }
        if fields.challenge != challenge.text() {
            return Err(ProofRefused::Challenge);
        }
        if now_ms < fields.made_ms || now_ms >= fields.made_ms.saturating_add(self.valid_ms) {
            return Err(ProofRefused::Time);
        }
        Ok(Person {
            issuer: fields.issuer,
            subject: fields.subject,
            proved_ms: fields.made_ms,
            method: "test".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: [u8; 32] = [7; 32];
    const ANOTHER_KEY: [u8; 32] = [8; 32];
    const ISSUER: &str = "https://id.example";
    const AUDIENCE: &str = "server";

    /// The kit's harness over [`TestProof`].
    struct OverTestProof;

    impl ProofHarness for OverTestProof {
        type Port = TestProof;

        fn port(&self) -> TestProof {
            TestProof::new(KEY, ISSUER, AUDIENCE, 60_000)
        }

        fn issuer(&self) -> String {
            ISSUER.to_string()
        }

        fn valid_ms(&self) -> u64 {
            60_000
        }

        fn proof(&self, subject: &str, challenge: &Challenge, made_ms: u64, made: Made) -> Vec<u8> {
            match made {
                Made::Good => TestProof::make(&KEY, ISSUER, AUDIENCE, subject, challenge, made_ms),
                Made::SignedByAnother => {
                    TestProof::make(&ANOTHER_KEY, ISSUER, AUDIENCE, subject, challenge, made_ms)
                }
                Made::ForAnotherAudience => {
                    TestProof::make(&KEY, ISSUER, "elsewhere", subject, challenge, made_ms)
                }
                Made::ByAnotherIssuer => TestProof::make(
                    &KEY,
                    "https://another.example",
                    AUDIENCE,
                    subject,
                    challenge,
                    made_ms,
                ),
            }
        }
    }

    #[test]
    fn a_good_proof_names_its_person() {
        super::a_good_proof_names_its_person(&OverTestProof);
    }

    #[test]
    fn a_proof_of_another_challenge_is_refused() {
        super::a_proof_of_another_challenge_is_refused(&OverTestProof);
    }

    #[test]
    fn a_proof_signed_by_another_key_is_refused() {
        super::a_proof_signed_by_another_key_is_refused(&OverTestProof);
    }

    #[test]
    fn a_proof_for_another_audience_is_refused() {
        super::a_proof_for_another_audience_is_refused(&OverTestProof);
    }

    #[test]
    fn a_proof_of_another_issuer_is_refused() {
        super::a_proof_of_another_issuer_is_refused(&OverTestProof);
    }

    #[test]
    fn a_proof_out_of_its_time_is_refused() {
        super::a_proof_out_of_its_time_is_refused(&OverTestProof);
    }

    #[test]
    fn bytes_that_are_no_proof_are_refused() {
        super::bytes_that_are_no_proof_are_refused(&OverTestProof);
    }
}
