// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The rule a host puts in front of its machine and behind it.
//!
//! Who may write the list is what the administrators' certificates say, and nothing a peer says
//! about itself: a write that arrives is admitted when its author, the node it names as the one
//! that stamped it, is a member whose role lets them write and the signature it carries is true
//! under the key their certificate gave. A write that leaves carries its author's signature:
//! made here when the author is this host's own person, and otherwise the one it arrived with,
//! which is how a hub hands on what a member wrote without being able to make it.
//!
//! One [`Trust`] is both halves of one host: it is cloned, and a clone is put in front of the
//! machine as its [`Admission`] and another behind it as its [`Sealer`].

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard};

use sce_rust_mesh::endpoint::{Admission, Sealer};
use serde_json::{json, Value};

use crate::certificate::Certificate;
use crate::encoding;
use crate::identity::{Identity, PublicKey};
use crate::message::{self, SIGNATURE_FIELD};

/// The role from which a member may write.
const WRITER: u8 = 2;

/// Why a certificate was not taken in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrustError {
    /// It is for another list.
    WrongCalendar,
    /// Its issuer is not an administrator this host was told to trust.
    UnknownIssuer,
    /// Its issuer did not sign it, or it was edited after.
    BadSignature,
    /// The host holds a certificate for the node that is as new, or newer.
    Stale { held: u64 },
}

struct Member {
    key: PublicKey,
    role: u8,
    serial: u64,
}

struct State {
    calendar: String,
    administrators: Vec<PublicKey>,
    members: BTreeMap<u64, Member>,
    own: Option<(u64, Identity)>,
    /// What each write that was admitted was signed with, by the bytes it is signed as: what a
    /// host that relays a write sends it on with.
    signatures: BTreeMap<Vec<u8>, [u8; 64]>,
    /// The events that arrive with no signature and make a copy forget, which this host does
    /// not take ([`UNSIGNED_REFUSED`], and [`Trust::allow_unsigned`]).
    unsigned_refused: Vec<String>,
}

/// The events a host that trusts nobody refuses unless it was told to take them: `todo.floor`
/// is the word that an instant is settled, on which a copy forgets the tombstones up to it,
/// and a copy that forgot a tombstone takes a write it named in again. It carries no
/// signature, so a host that relays it could settle an instant its copies have not, and bring
/// back with the old signed writes what was deleted. A host that trusts the one that settles
/// (its own hub) allows it.
pub const UNSIGNED_REFUSED: [&str; 1] = ["todo.floor"];

/// The trust of one host in one list. Clones share it.
#[derive(Clone)]
pub struct Trust {
    state: Arc<Mutex<State>>,
}

impl Trust {
    /// A host of the list `calendar` that trusts the certificates `administrators` sign, and
    /// knows no member yet.
    pub fn new(calendar: &str, administrators: &[PublicKey]) -> Self {
        Self {
            state: Arc::new(Mutex::new(State {
                calendar: calendar.to_string(),
                administrators: administrators.to_vec(),
                members: BTreeMap::new(),
                own: None,
                signatures: BTreeMap::new(),
                unsigned_refused: UNSIGNED_REFUSED.iter().map(|e| e.to_string()).collect(),
            })),
        }
    }

    /// Take `events` from a peer although they carry no signature, because this host trusts
    /// the peer that settles ([`UNSIGNED_REFUSED`]).
    pub fn allow_unsigned(&self, events: &[&str]) {
        self.state()
            .unsigned_refused
            .retain(|refused| !events.contains(&refused.as_str()));
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// This host's person: the node their writes are stamped with, and the key that signs them.
    /// A host that has none (a hub) signs nothing of its own.
    pub fn set_identity(&self, node: u64, identity: Identity) {
        self.state().own = Some((node, identity));
    }

    /// Take a certificate in, when its issuer is trusted, it is signed, it is for this list,
    /// and it is newer than what is held for the node.
    pub fn accept(&self, certificate: &Certificate) -> Result<(), TrustError> {
        let mut state = self.state();
        if certificate.calendar != state.calendar {
            return Err(TrustError::WrongCalendar);
        }
        if !state.administrators.contains(&certificate.issuer) {
            return Err(TrustError::UnknownIssuer);
        }
        if !certificate.is_signed_by_its_issuer() {
            return Err(TrustError::BadSignature);
        }
        if let Some(held) = state.members.get(&certificate.node) {
            if held.serial >= certificate.serial {
                return Err(TrustError::Stale { held: held.serial });
            }
        }
        state.members.insert(
            certificate.node,
            Member {
                key: certificate.key,
                role: certificate.role,
                serial: certificate.serial,
            },
        );
        Ok(())
    }

    /// The role of `node`, 0 when it is not a member.
    pub fn role(&self, node: u64) -> u8 {
        self.state()
            .members
            .get(&node)
            .map_or(0, |member| member.role)
    }

    /// What tells a machine the roster the certificates make, as `(event, data)` for the host
    /// to raise: a `todo.role` for each node the host holds a certificate for, a member that
    /// was taken off included, so that a role the machine held is overwritten.
    pub fn role_events(&self) -> Vec<(String, String)> {
        self.state()
            .members
            .iter()
            .map(|(node, member)| {
                (
                    "todo.role".to_string(),
                    json!({"node": node.to_string(), "role": member.role}).to_string(),
                )
            })
            .collect()
    }

    /// The signatures this host holds for the writes it relays, to keep across a restart.
    pub fn signatures(&self) -> Vec<(Vec<u8>, [u8; 64])> {
        self.state()
            .signatures
            .iter()
            .map(|(message, signature)| (message.clone(), *signature))
            .collect()
    }

    /// Take back signatures [`Self::signatures`] gave: each one that a member's key of today
    /// verifies is kept, and the rest are dropped, since what a host reads back from its own
    /// storage is not taken for true. Returns how many were kept.
    pub fn restore_signatures(&self, entries: &[(Vec<u8>, [u8; 64])]) -> usize {
        let mut state = self.state();
        let mut kept = 0;
        for (message, signature) in entries {
            let true_of_a_member = state
                .members
                .values()
                .any(|member| member.key.verifies(message, signature));
            if true_of_a_member {
                state.signatures.insert(message.clone(), *signature);
                kept += 1;
            }
        }
        kept
    }
}

impl Admission for Trust {
    fn admit(&mut self, _peer: &str, event: &str, data: &str) -> Result<Option<String>, String> {
        let mut state = self.state();
        if state
            .unsigned_refused
            .iter()
            .any(|refused| refused == event)
        {
            return Err(format!(
                "{event}: it carries no signature and this host does not take it on a peer's word"
            ));
        }
        if !message::SIGNED_EVENTS.contains(&event) {
            return Ok(None);
        }
        let mut value: Value =
            serde_json::from_str(data).map_err(|_| format!("{event}: the data is not JSON"))?;
        let signature = value
            .as_object_mut()
            .and_then(|object| object.remove(SIGNATURE_FIELD))
            .ok_or_else(|| format!("{event}: it carries no signature"))?;
        let signature: [u8; 64] = signature
            .as_str()
            .and_then(encoding::decode)
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or_else(|| format!("{event}: its signature is not a signature"))?;
        let signing = match message::signing(&state.calendar, event, &value) {
            Some(Ok(signing)) => signing,
            Some(Err(error)) => return Err(format!("{event}: {error:?}")),
            None => return Ok(None),
        };
        let member = state
            .members
            .get(&signing.author)
            .ok_or_else(|| format!("{event}: node {} is not a member", signing.author))?;
        if member.role < WRITER {
            return Err(format!(
                "{event}: node {} may not write (role {})",
                signing.author, member.role
            ));
        }
        if !member.key.verifies(&signing.message, &signature) {
            return Err(format!(
                "{event}: the signature is not node {}'s",
                signing.author
            ));
        }
        state.signatures.insert(signing.message, signature);
        Ok(Some(value.to_string()))
    }
}

impl Sealer for Trust {
    fn seal(&mut self, _target: &str, event: &str, data: &str) -> Option<String> {
        if !message::SIGNED_EVENTS.contains(&event) {
            return None;
        }
        let state = self.state();
        let mut value: Value = serde_json::from_str(data).ok()?;
        let signing = message::signing(&state.calendar, event, &value)?.ok()?;
        let signature = match &state.own {
            Some((node, identity)) if *node == signing.author => identity.sign(&signing.message),
            _ => *state.signatures.get(&signing.message)?,
        };
        value.as_object_mut()?.insert(
            SIGNATURE_FIELD.to_string(),
            Value::String(encoding::encode(&signature)),
        );
        Some(value.to_string())
    }
}
