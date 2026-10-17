// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The membership log of a list: who its administrators are, who its members are and what each
//! may do from when, which epoch its content is sealed under, and whether its hub is trusted.
//!
//! The log is an append-only chain of entries, each naming the hash of the one before it and
//! signed by administrators. A host holds the chain and judges every write, note and epoch by it,
//! so nothing a hub says about who the members are is believed on the hub's word. A person is
//! invited with the hash of the first entry (the genesis), which is all they have to be told in a
//! channel they trust: the rest is checked from there.
//!
//! What an entry needs to be admitted depends on what it does. Most entries (a member's role, a
//! new epoch) need one administrator's signature. A change to who the administrators are, or to
//! the hub's profile, needs the quorum the log set, so that one stolen administrator key cannot
//! make another administrator or take the list over.
//!
//! Two entries at one position are a **fork**: a host that is shown both has proof that an
//! administrator (or whoever holds a key) signed two histories, and reports it. Whoever it
//! concerns decides what to do; a host does not choose one.

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

use crate::encoding;
use crate::identity::{Identity, PublicKey};

/// The hash of an entry, and what the next one names.
pub type Hash = [u8; 32];

/// What comes before the first entry: no entry, so nothing to name.
pub const NO_ENTRY: Hash = [0; 32];

const DOMAIN: &str = "sce-trust/membership/1";

/// The role from which a member may write.
pub const WRITER: u8 = 2;

/// Whether the hub of the list is trusted. It is a fact of the log so that every host knows
/// which it is in, and nobody is surprised by it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    /// The hub is not trusted: hosts check every write and note, take no settled instant without
    /// an administrator's signature, and keep content from it.
    Untrusted,
    /// The hub is run by the list's own people: it may read, and its settled instants are taken.
    TrustedHub,
}

impl Profile {
    fn tag(self) -> &'static str {
        match self {
            Profile::Untrusted => "untrusted",
            Profile::TrustedHub => "trusted-hub",
        }
    }

    fn from_tag(tag: &str) -> Option<Self> {
        match tag {
            "untrusted" => Some(Profile::Untrusted),
            "trusted-hub" => Some(Profile::TrustedHub),
            _ => None,
        }
    }
}

/// What an entry does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Body {
    /// The first entry: who the administrators are and how many of them a change to that needs.
    Genesis {
        administrators: Vec<PublicKey>,
        quorum: u8,
        profile: Profile,
    },
    /// From the time `from` (in the clock the machines stamp with) the device of `node` is a
    /// member with `role` and signs with `key`; a role of 0 takes it off. Later entries for the
    /// node win among those whose `from` has been reached, and every one is kept, so what a
    /// device wrote before it was taken off is judged by the entry of that time.
    Grant {
        node: u64,
        role: u8,
        key: PublicKey,
        from: i64,
    },
    /// The next epoch of the list's content key begins. Hosts seal under it from here on and a
    /// host that has no key of it does not write until it is given one.
    Epoch { epoch: u32 },
    /// Administrators are added and removed, and the quorum becomes `quorum`.
    Administrators {
        add: Vec<PublicKey>,
        remove: Vec<PublicKey>,
        quorum: u8,
    },
    /// The hub's profile changes.
    Profile(Profile),
}

impl Body {
    /// How many administrators' signatures the entry needs, the log's quorum being `quorum`.
    fn required(&self, quorum: u8) -> usize {
        match self {
            Body::Genesis { .. } | Body::Grant { .. } | Body::Epoch { .. } => 1,
            Body::Administrators { .. } | Body::Profile(_) => usize::from(quorum),
        }
    }

    fn write(&self, out: &mut Vec<u8>) {
        match self {
            Body::Genesis {
                administrators,
                quorum,
                profile,
            } => {
                part(out, "genesis");
                part(out, &quorum.to_string());
                part(out, profile.tag());
                for key in administrators {
                    part(out, &key.to_text());
                }
            }
            Body::Grant {
                node,
                role,
                key,
                from,
            } => {
                part(out, "grant");
                part(out, &node.to_string());
                part(out, &role.to_string());
                part(out, &key.to_text());
                part(out, &from.to_string());
            }
            Body::Epoch { epoch } => {
                part(out, "epoch");
                part(out, &epoch.to_string());
            }
            Body::Administrators {
                add,
                remove,
                quorum,
            } => {
                part(out, "administrators");
                part(out, &quorum.to_string());
                part(out, &add.len().to_string());
                for key in add.iter().chain(remove) {
                    part(out, &key.to_text());
                }
            }
            Body::Profile(profile) => {
                part(out, "profile");
                part(out, profile.tag());
            }
        }
    }
}

fn part(out: &mut Vec<u8>, text: &str) {
    out.extend_from_slice(&(text.len() as u64).to_be_bytes());
    out.extend_from_slice(text.as_bytes());
}

/// One entry of the log, with the signatures that admit it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub list: String,
    pub index: u64,
    pub prev: Hash,
    pub body: Body,
    pub signatures: Vec<(PublicKey, [u8; 64])>,
}

impl Entry {
    /// The bytes every signer signs: the list, the position, the entry before and what this one
    /// does, and no signature.
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        part(&mut out, DOMAIN);
        part(&mut out, &self.list);
        part(&mut out, &self.index.to_string());
        part(&mut out, &encoding::encode(&self.prev));
        self.body.write(&mut out);
        out
    }

    /// The hash that names the entry: what it says and who signed it, signatures in the order of
    /// their keys, so that one entry has one name whoever collected the signatures.
    pub fn hash(&self) -> Hash {
        let mut signatures: Vec<_> = self.signatures.iter().collect();
        signatures.sort_by_key(|(key, _)| *key);
        let mut hash = Sha256::new();
        hash.update(self.signing_bytes());
        for (key, signature) in signatures {
            hash.update(key.to_bytes());
            hash.update(signature);
        }
        hash.finalize().into()
    }

    /// The entry as the JSON a host sends to another.
    pub fn to_json(&self) -> String {
        let body = match &self.body {
            Body::Genesis {
                administrators,
                quorum,
                profile,
            } => serde_json::json!({
                "kind": "genesis",
                "administrators": administrators.iter().map(|k| k.to_text()).collect::<Vec<_>>(),
                "quorum": quorum,
                "profile": profile.tag(),
            }),
            Body::Grant {
                node,
                role,
                key,
                from,
            } => serde_json::json!({
                "kind": "grant",
                "node": node.to_string(),
                "role": role,
                "key": key.to_text(),
                "from": from.to_string(),
            }),
            Body::Epoch { epoch } => serde_json::json!({"kind": "epoch", "epoch": epoch}),
            Body::Administrators {
                add,
                remove,
                quorum,
            } => serde_json::json!({
                "kind": "administrators",
                "add": add.iter().map(|k| k.to_text()).collect::<Vec<_>>(),
                "remove": remove.iter().map(|k| k.to_text()).collect::<Vec<_>>(),
                "quorum": quorum,
            }),
            Body::Profile(profile) => {
                serde_json::json!({"kind": "profile", "profile": profile.tag()})
            }
        };
        serde_json::json!({
            "list": self.list,
            "index": self.index.to_string(),
            "prev": encoding::encode(&self.prev),
            "body": body,
            "signatures": self.signatures.iter()
                .map(|(key, signature)| serde_json::json!([key.to_text(), encoding::encode(signature)]))
                .collect::<Vec<_>>(),
        })
        .to_string()
    }

    pub fn from_json(text: &str) -> Option<Self> {
        let value: serde_json::Value = serde_json::from_str(text).ok()?;
        let text_of = |value: &serde_json::Value, name: &str| -> Option<String> {
            Some(value.get(name)?.as_str()?.to_string())
        };
        let keys = |value: &serde_json::Value, name: &str| -> Option<Vec<PublicKey>> {
            value
                .get(name)?
                .as_array()?
                .iter()
                .map(|key| PublicKey::from_text(key.as_str()?))
                .collect()
        };
        let small = |value: &serde_json::Value, name: &str| -> Option<u8> {
            u8::try_from(value.get(name)?.as_u64()?).ok()
        };
        let body = value.get("body")?;
        let body = match body.get("kind")?.as_str()? {
            "genesis" => Body::Genesis {
                administrators: keys(body, "administrators")?,
                quorum: small(body, "quorum")?,
                profile: Profile::from_tag(&text_of(body, "profile")?)?,
            },
            "grant" => Body::Grant {
                node: text_of(body, "node")?.parse().ok()?,
                role: small(body, "role")?,
                key: PublicKey::from_text(&text_of(body, "key")?)?,
                from: text_of(body, "from")?.parse().ok()?,
            },
            "epoch" => Body::Epoch {
                epoch: u32::try_from(body.get("epoch")?.as_u64()?).ok()?,
            },
            "administrators" => Body::Administrators {
                add: keys(body, "add")?,
                remove: keys(body, "remove")?,
                quorum: small(body, "quorum")?,
            },
            "profile" => Body::Profile(Profile::from_tag(&text_of(body, "profile")?)?),
            _ => return None,
        };
        let signatures = value
            .get("signatures")?
            .as_array()?
            .iter()
            .map(|row| {
                Some((
                    PublicKey::from_text(row.get(0)?.as_str()?)?,
                    encoding::decode(row.get(1)?.as_str()?)?.try_into().ok()?,
                ))
            })
            .collect::<Option<Vec<_>>>()?;
        Some(Self {
            list: text_of(&value, "list")?,
            index: text_of(&value, "index")?.parse().ok()?,
            prev: encoding::decode(&text_of(&value, "prev")?)?
                .try_into()
                .ok()?,
            body,
            signatures,
        })
    }
}

/// An entry that is being written: its position and the entry before it are fixed, and
/// administrators add their signatures to it until it has enough.
#[derive(Debug, Clone)]
pub struct Draft {
    entry: Entry,
}

impl Draft {
    /// The entry signed by `administrator`, added to the signatures it has.
    pub fn sign(mut self, administrator: &Identity) -> Self {
        let key = administrator.public();
        if self.entry.signatures.iter().all(|(held, _)| *held != key) {
            let signature = administrator.sign(&self.entry.signing_bytes());
            self.entry.signatures.push((key, signature));
        }
        self
    }

    /// What every signer is asked to sign, for a quorum collected across hosts.
    pub fn signing_bytes(&self) -> Vec<u8> {
        self.entry.signing_bytes()
    }

    /// Add a signature another host made of [`Self::signing_bytes`].
    pub fn with_signature(mut self, key: PublicKey, signature: [u8; 64]) -> Self {
        if self.entry.signatures.iter().all(|(held, _)| *held != key) {
            self.entry.signatures.push((key, signature));
        }
        self
    }

    pub fn finish(self) -> Entry {
        self.entry
    }
}

/// Why an entry was not taken in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogError {
    /// It is for another list.
    WrongList,
    /// A position the log has not reached: the entries between are missing.
    Gap { expected: u64 },
    /// Two different entries at one position, or one that does not name the entry before it: proof
    /// that the chain forked. The index is where.
    Fork { index: u64 },
    /// The first entry is not the one the host was invited with.
    NotTheGenesis,
    /// A signature that does not verify.
    BadSignature,
    /// Fewer administrators signed than the entry needs.
    NotEnoughSignatures { needed: usize, had: usize },
    /// The entry does what the log does not allow (a role past 3, an epoch out of order, a log with
    /// no administrator left, a quorum that cannot be met).
    Invalid(&'static str),
}

/// What one grant says of a member from its time on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Era {
    pub from: i64,
    pub key: PublicKey,
    pub role: u8,
}

/// The state the entries of the log make, and the entries.
pub struct Log {
    list: String,
    /// The hash the host was invited with, when it was; the first entry must be it.
    expected_genesis: Option<Hash>,
    entries: Vec<Entry>,
    hashes: Vec<Hash>,
    administrators: Vec<PublicKey>,
    quorum: u8,
    profile: Profile,
    /// Each node's grants, by the position of the entry that made them.
    members: BTreeMap<u64, BTreeMap<u64, Era>>,
    epoch: u32,
}

impl Log {
    /// An empty log of `list`, that takes as its first entry the one named `genesis` and no
    /// other: a person invited into the list is told that hash, in a channel they trust.
    pub fn invited(list: &str, genesis: Hash) -> Self {
        Self::empty(list, Some(genesis))
    }

    /// An empty log of `list` that takes whatever genesis it is first given, for the host of
    /// whoever made the list.
    pub fn open(list: &str) -> Self {
        Self::empty(list, None)
    }

    fn empty(list: &str, expected_genesis: Option<Hash>) -> Self {
        Self {
            list: list.to_string(),
            expected_genesis,
            entries: Vec::new(),
            hashes: Vec::new(),
            administrators: Vec::new(),
            quorum: 1,
            profile: Profile::Untrusted,
            members: BTreeMap::new(),
            epoch: 0,
        }
    }

    pub fn list(&self) -> &str {
        &self.list
    }

    /// The entries held, in order.
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// The position the next entry takes, and the hash it names.
    pub fn head(&self) -> (u64, Hash) {
        (
            self.entries.len() as u64,
            self.hashes.last().copied().unwrap_or(NO_ENTRY),
        )
    }

    pub fn administrators(&self) -> &[PublicKey] {
        &self.administrators
    }

    pub fn quorum(&self) -> u8 {
        self.quorum
    }

    pub fn profile(&self) -> Profile {
        self.profile
    }

    /// The latest epoch the log begins, 0 when none.
    pub fn epoch(&self) -> u32 {
        self.epoch
    }

    /// Whether there is a first entry.
    pub fn is_started(&self) -> bool {
        !self.entries.is_empty()
    }

    /// The draft of the entry that follows the head, for administrators to sign.
    pub fn draft(&self, body: Body) -> Draft {
        let (index, prev) = self.head();
        Draft {
            entry: Entry {
                list: self.list.clone(),
                index,
                prev,
                body,
                signatures: Vec::new(),
            },
        }
    }

    /// The draft of the first entry of `list`.
    pub fn genesis_draft(list: &str, body: Body) -> Draft {
        Draft {
            entry: Entry {
                list: list.to_string(),
                index: 0,
                prev: NO_ENTRY,
                body,
                signatures: Vec::new(),
            },
        }
    }

    /// What the certificates of `node` say of it at the time `at`: the one with the latest entry
    /// among those from a time at or before it. `None` when none applies.
    pub fn era_at(&self, node: u64, at: i64) -> Option<&Era> {
        self.members
            .get(&node)?
            .values()
            .rev()
            .find(|era| era.from <= at)
    }

    /// The latest grant of each node.
    pub fn latest(&self) -> impl Iterator<Item = (u64, &Era)> {
        self.members
            .iter()
            .filter_map(|(node, eras)| Some((*node, eras.values().next_back()?)))
    }

    /// Every grant of each node, oldest first.
    pub fn eras(&self) -> impl Iterator<Item = (u64, Vec<&Era>)> {
        self.members
            .iter()
            .map(|(node, eras)| (*node, eras.values().collect()))
    }

    /// Whether `key` is one of the keys a member has had with a role that could write.
    pub fn was_a_writer(&self, key: &PublicKey) -> bool {
        self.members.values().any(|eras| {
            eras.values()
                .any(|era| era.key == *key && era.role >= WRITER)
        })
    }

    /// Take an entry in: known (it is the one already held at its position), or the next one and
    /// admitted by the administrators it needs. The log is changed only when it is.
    pub fn append(&mut self, entry: &Entry) -> Result<bool, LogError> {
        if entry.list != self.list {
            return Err(LogError::WrongList);
        }
        let held = self.entries.len() as u64;
        if entry.index < held {
            return if self.hashes[entry.index as usize] == entry.hash() {
                Ok(false)
            } else {
                Err(LogError::Fork { index: entry.index })
            };
        }
        if entry.index > held {
            return Err(LogError::Gap { expected: held });
        }
        let (_, prev) = self.head();
        if entry.prev != prev {
            return Err(LogError::Fork { index: entry.index });
        }
        if held == 0 {
            self.admit_genesis(entry)?;
        } else {
            self.admit(entry)?;
        }
        self.hashes.push(entry.hash());
        self.entries.push(entry.clone());
        Ok(true)
    }

    fn verified_signers(entry: &Entry, administrators: &[PublicKey]) -> Result<usize, LogError> {
        let bytes = entry.signing_bytes();
        let mut signers: Vec<PublicKey> = Vec::new();
        for (key, signature) in &entry.signatures {
            if !key.verifies(&bytes, signature) {
                return Err(LogError::BadSignature);
            }
            if administrators.contains(key) && !signers.contains(key) {
                signers.push(*key);
            }
        }
        Ok(signers.len())
    }

    fn admit_genesis(&mut self, entry: &Entry) -> Result<(), LogError> {
        let Body::Genesis {
            administrators,
            quorum,
            profile,
        } = &entry.body
        else {
            return Err(LogError::Invalid("the first entry is the genesis"));
        };
        if let Some(expected) = self.expected_genesis {
            if entry.hash() != expected {
                return Err(LogError::NotTheGenesis);
            }
        }
        if administrators.is_empty() {
            return Err(LogError::Invalid("a list has an administrator"));
        }
        if *quorum == 0 || usize::from(*quorum) > administrators.len() {
            return Err(LogError::Invalid("the quorum is met by the administrators"));
        }
        let had = Self::verified_signers(entry, administrators)?;
        if had < 1 {
            return Err(LogError::NotEnoughSignatures { needed: 1, had });
        }
        self.administrators = administrators.clone();
        self.quorum = *quorum;
        self.profile = *profile;
        Ok(())
    }

    fn admit(&mut self, entry: &Entry) -> Result<(), LogError> {
        let needed = entry.body.required(self.quorum);
        let had = Self::verified_signers(entry, &self.administrators)?;
        if had < needed {
            return Err(LogError::NotEnoughSignatures { needed, had });
        }
        let position = entry.index;
        match &entry.body {
            Body::Genesis { .. } => return Err(LogError::Invalid("there is one genesis")),
            Body::Grant {
                node,
                role,
                key,
                from,
            } => {
                if *role > 3 {
                    return Err(LogError::Invalid("a role is 0 to 3"));
                }
                self.members.entry(*node).or_default().insert(
                    position,
                    Era {
                        from: *from,
                        key: *key,
                        role: *role,
                    },
                );
            }
            Body::Epoch { epoch } => {
                if *epoch
                    != self
                        .epoch
                        .checked_add(1)
                        .ok_or(LogError::Invalid("epochs run out"))?
                {
                    return Err(LogError::Invalid("an epoch follows the one before it"));
                }
                self.epoch = *epoch;
            }
            Body::Administrators {
                add,
                remove,
                quorum,
            } => {
                let mut administrators = self.administrators.clone();
                for key in remove {
                    if !administrators.contains(key) {
                        return Err(LogError::Invalid("only an administrator is removed"));
                    }
                    administrators.retain(|held| held != key);
                }
                for key in add {
                    if !administrators.contains(key) {
                        administrators.push(*key);
                    }
                }
                if administrators.is_empty() {
                    return Err(LogError::Invalid("a list has an administrator"));
                }
                if *quorum == 0 || usize::from(*quorum) > administrators.len() {
                    return Err(LogError::Invalid("the quorum is met by the administrators"));
                }
                self.administrators = administrators;
                self.quorum = *quorum;
            }
            Body::Profile(profile) => self.profile = *profile,
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn admin(seed: u8) -> Identity {
        Identity::from_seed([seed; 32])
    }

    fn started(quorum: u8) -> Log {
        let mut log = Log::open("home");
        let genesis = Log::genesis_draft(
            "home",
            Body::Genesis {
                administrators: vec![admin(1).public(), admin(2).public(), admin(3).public()],
                quorum,
                profile: Profile::Untrusted,
            },
        )
        .sign(&admin(1))
        .finish();
        assert_eq!(log.append(&genesis), Ok(true));
        log
    }

    fn grant(log: &Log, node: u64, role: u8, from: i64, by: &Identity) -> Entry {
        log.draft(Body::Grant {
            node,
            role,
            key: admin(50 + node as u8).public(),
            from,
        })
        .sign(by)
        .finish()
    }

    #[test]
    fn an_entry_is_known_when_it_is_the_one_held_and_a_fork_when_it_is_not() {
        let mut log = started(2);
        let first = grant(&log, 1, 2, 0, &admin(1));
        assert_eq!(log.append(&first), Ok(true));
        assert_eq!(log.append(&first), Ok(false), "taken in once");

        let other = log
            .draft(Body::Grant {
                node: 1,
                role: 3,
                key: admin(51).public(),
                from: 0,
            })
            .sign(&admin(1))
            .finish();
        // `other` was drafted after `first`, so it is at index 2; make one at index 1 instead.
        let mut forked = other.clone();
        forked.index = 1;
        forked.prev = log.hashes[0];
        let forked = Draft { entry: forked }.sign(&admin(1)).finish();
        assert_eq!(log.append(&forked), Err(LogError::Fork { index: 1 }));
        assert_eq!(log.entries().len(), 2, "the log did not change");
    }

    #[test]
    fn an_entry_that_skips_one_or_does_not_name_the_one_before_is_refused() {
        let mut log = started(1);
        let mut skipping = grant(&log, 1, 2, 0, &admin(1));
        skipping.index = 5;
        assert_eq!(log.append(&skipping), Err(LogError::Gap { expected: 1 }));

        let mut wrong_prev = log.draft(Body::Epoch { epoch: 1 }).sign(&admin(1)).finish();
        wrong_prev.prev = [9; 32];
        assert_eq!(log.append(&wrong_prev), Err(LogError::Fork { index: 1 }));
    }

    #[test]
    fn an_entry_needs_the_signatures_its_kind_needs() {
        let mut log = started(2);
        // A member's role needs one administrator.
        let by_one = grant(&log, 1, 2, 0, &admin(2));
        assert_eq!(log.append(&by_one), Ok(true));

        // A change of administrators needs the quorum of two.
        let add = Body::Administrators {
            add: vec![admin(4).public()],
            remove: vec![],
            quorum: 2,
        };
        let one = log.draft(add.clone()).sign(&admin(1)).finish();
        assert_eq!(
            log.append(&one),
            Err(LogError::NotEnoughSignatures { needed: 2, had: 1 })
        );
        let twice = log
            .draft(add.clone())
            .sign(&admin(1))
            .sign(&admin(1))
            .finish();
        assert_eq!(
            log.append(&twice),
            Err(LogError::NotEnoughSignatures { needed: 2, had: 1 }),
            "one key twice is one signature"
        );
        let stranger = log
            .draft(add.clone())
            .sign(&admin(1))
            .sign(&admin(9))
            .finish();
        assert_eq!(
            log.append(&stranger),
            Err(LogError::NotEnoughSignatures { needed: 2, had: 1 }),
            "a key that is no administrator's is none"
        );
        let two = log.draft(add).sign(&admin(1)).sign(&admin(2)).finish();
        assert_eq!(log.append(&two), Ok(true));
        assert_eq!(log.administrators().len(), 4);
    }

    #[test]
    fn a_signature_that_does_not_verify_refuses_the_entry() {
        let mut log = started(1);
        let mut entry = grant(&log, 1, 2, 0, &admin(1));
        entry.signatures[0].1[0] ^= 1;
        assert_eq!(log.append(&entry), Err(LogError::BadSignature));
        let mut edited = grant(&log, 1, 2, 0, &admin(1));
        edited.body = Body::Grant {
            node: 1,
            role: 3,
            key: admin(51).public(),
            from: 0,
        };
        assert_eq!(log.append(&edited), Err(LogError::BadSignature));
    }

    #[test]
    fn a_person_invited_by_the_hash_takes_that_genesis_and_no_other() {
        let genesis = |seed: u8| {
            Log::genesis_draft(
                "home",
                Body::Genesis {
                    administrators: vec![admin(seed).public()],
                    quorum: 1,
                    profile: Profile::Untrusted,
                },
            )
            .sign(&admin(seed))
            .finish()
        };
        let real = genesis(1);
        let mut invited = Log::invited("home", real.hash());
        assert_eq!(invited.append(&genesis(2)), Err(LogError::NotTheGenesis));
        assert!(!invited.is_started());
        assert_eq!(invited.append(&real), Ok(true));
    }

    #[test]
    fn what_a_grant_says_holds_from_its_time_and_the_latest_entry_wins() {
        let mut log = started(1);
        let on = grant(&log, 1, 2, 0, &admin(1));
        log.append(&on).unwrap();
        let off = grant(&log, 1, 0, 1500, &admin(1));
        log.append(&off).unwrap();
        assert_eq!(log.era_at(1, 1000).map(|era| era.role), Some(2));
        assert_eq!(log.era_at(1, 1500).map(|era| era.role), Some(0));
        assert_eq!(
            log.era_at(1, -1),
            None,
            "before the first grant is no member"
        );
        assert_eq!(log.era_at(7, 0).map(|era| era.role), None);
        assert!(log.was_a_writer(&admin(51).public()));
    }

    #[test]
    fn an_epoch_follows_the_one_before_it_and_the_administrators_stay() {
        let mut log = started(1);
        let skipping = log.draft(Body::Epoch { epoch: 2 }).sign(&admin(1)).finish();
        assert_eq!(
            log.append(&skipping),
            Err(LogError::Invalid("an epoch follows the one before it"))
        );
        let next = log.draft(Body::Epoch { epoch: 1 }).sign(&admin(1)).finish();
        assert_eq!(log.append(&next), Ok(true));
        assert_eq!(log.epoch(), 1);

        let everyone = log
            .draft(Body::Administrators {
                add: vec![],
                remove: vec![admin(1).public(), admin(2).public(), admin(3).public()],
                quorum: 1,
            })
            .sign(&admin(1))
            .finish();
        assert_eq!(
            log.append(&everyone),
            Err(LogError::Invalid("a list has an administrator"))
        );
    }

    #[test]
    fn an_entry_reads_back_from_the_json_it_is_sent_as() {
        let log = started(2);
        for body in [
            Body::Grant {
                node: u64::MAX,
                role: 3,
                key: admin(51).public(),
                from: -5,
            },
            Body::Epoch { epoch: 7 },
            Body::Administrators {
                add: vec![admin(4).public()],
                remove: vec![admin(3).public()],
                quorum: 2,
            },
            Body::Profile(Profile::TrustedHub),
        ] {
            let entry = log.draft(body).sign(&admin(1)).sign(&admin(2)).finish();
            let again = Entry::from_json(&entry.to_json()).unwrap();
            assert_eq!(again, entry);
            assert_eq!(again.hash(), entry.hash());
        }
        assert_eq!(Entry::from_json("{}"), None);
    }
}
