// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What an administrator signs to make a person a member of a list: the person's node, the
//! key their writes will be signed by, the role they hold, and the time from which it holds.
//!
//! A host that relays a certificate cannot make one, since it does not hold the
//! administrator's key; and a copy that was told which administrator keys to trust when it was
//! set up needs nothing else from the host to know who may write. Certificates of one node
//! make its history: a write or a note made at a time is judged by the certificate with the
//! highest `serial` among those whose `from` is at or before that time, so that a member taken
//! off (role 0, from a time) is still the writer of what they wrote before it, a new key (a
//! lost device replaced) judges only what is made from its time, and the same set of
//! certificates gives the same answer whatever order a host received them in.

use serde_json::{json, Value};

use crate::encoding;
use crate::identity::{Identity, PublicKey};

const DOMAIN: &str = "sce-trust/certificate/2";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Certificate {
    /// The list it is for: a certificate of one list is nothing in another.
    pub calendar: String,
    /// The member's node, the `nodeId` their writes are stamped with.
    pub node: u64,
    /// 0 takes the member off, 1 may read, 2 may also write, 3 may also do what an owner does.
    pub role: u8,
    /// The key the member's writes are signed by.
    pub key: PublicKey,
    /// Which of the certificates for this node wins among those that apply at a time.
    pub serial: u64,
    /// The time, in the clock the machines stamp with, from which this certificate says what
    /// the member is. A member taken off is taken off from a time.
    pub from: i64,
    /// The administrator key that signed it.
    pub issuer: PublicKey,
    pub signature: [u8; 64],
}

impl Certificate {
    /// A certificate signed by `administrator`.
    pub fn issue(
        administrator: &Identity,
        calendar: &str,
        node: u64,
        role: u8,
        key: PublicKey,
        serial: u64,
        from: i64,
    ) -> Self {
        let mut certificate = Self {
            calendar: calendar.to_string(),
            node,
            role,
            key,
            serial,
            from,
            issuer: administrator.public(),
            signature: [0; 64],
        };
        certificate.signature = administrator.sign(&certificate.message());
        certificate
    }

    /// The bytes that are signed: every field but the signature, each with its length.
    fn message(&self) -> Vec<u8> {
        let mut message = Vec::new();
        for part in [
            DOMAIN,
            &self.calendar,
            &self.node.to_string(),
            &self.role.to_string(),
            &self.key.to_text(),
            &self.serial.to_string(),
            &self.from.to_string(),
            &self.issuer.to_text(),
        ] {
            message.extend_from_slice(&(part.len() as u64).to_be_bytes());
            message.extend_from_slice(part.as_bytes());
        }
        message
    }

    /// Whether the issuer signed this certificate. Whether the issuer is one the copy trusts
    /// is [`crate::Trust::accept`]'s to ask.
    pub fn is_signed_by_its_issuer(&self) -> bool {
        self.issuer.verifies(&self.message(), &self.signature)
    }

    /// The certificate as the JSON a host sends to another.
    pub fn to_json(&self) -> String {
        json!({
            "calendar": self.calendar,
            "node": self.node.to_string(),
            "role": self.role,
            "key": self.key.to_text(),
            "serial": self.serial.to_string(),
            "from": self.from.to_string(),
            "issuer": self.issuer.to_text(),
            "sig": encoding::encode(&self.signature),
        })
        .to_string()
    }

    pub fn from_json(text: &str) -> Option<Self> {
        let value: Value = serde_json::from_str(text).ok()?;
        let text_of = |name: &str| value.get(name)?.as_str();
        Some(Self {
            calendar: text_of("calendar")?.to_string(),
            node: text_of("node")?.parse().ok()?,
            role: u8::try_from(value.get("role")?.as_u64()?).ok()?,
            key: PublicKey::from_text(text_of("key")?)?,
            serial: text_of("serial")?.parse().ok()?,
            from: text_of("from")?.parse().ok()?,
            issuer: PublicKey::from_text(text_of("issuer")?)?,
            signature: encoding::decode(text_of("sig")?)?.try_into().ok()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn admin() -> Identity {
        Identity::from_seed([9; 32])
    }

    #[test]
    fn a_certificate_is_signed_by_its_issuer_and_by_no_edit_of_it() {
        let member = Identity::from_seed([1; 32]);
        let certificate = Certificate::issue(&admin(), "home", 1, 2, member.public(), 1, 0);
        assert!(certificate.is_signed_by_its_issuer());
        for edited in [
            Certificate {
                role: 3,
                ..certificate.clone()
            },
            Certificate {
                node: 2,
                ..certificate.clone()
            },
            Certificate {
                serial: 2,
                ..certificate.clone()
            },
            Certificate {
                from: 1,
                ..certificate.clone()
            },
            Certificate {
                calendar: "work".into(),
                ..certificate.clone()
            },
            Certificate {
                key: Identity::from_seed([2; 32]).public(),
                ..certificate.clone()
            },
        ] {
            assert!(!edited.is_signed_by_its_issuer(), "{edited:?}");
        }
    }

    #[test]
    fn a_certificate_reads_back_from_the_json_it_is_sent_as() {
        let member = Identity::from_seed([1; 32]);
        let certificate =
            Certificate::issue(&admin(), "home", u64::MAX, 2, member.public(), 7, -5000);
        let json = certificate.to_json();
        assert_eq!(Certificate::from_json(&json), Some(certificate));
        assert_eq!(Certificate::from_json("{}"), None);
        assert_eq!(Certificate::from_json("not json"), None);
    }
}
