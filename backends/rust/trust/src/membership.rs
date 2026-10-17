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

use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest, Sha256};

use crate::checkpoint::Checkpoint;
use crate::encoding;
use crate::identity::{Identity, PublicKey};

/// The hash of an entry, and what the next one names.
pub type Hash = [u8; 32];

/// What comes before the first entry: no entry, so nothing to name.
pub const NO_ENTRY: Hash = [0; 32];

const DOMAIN: &str = "sce-trust/membership/1";

/// The role from which a member may write.
pub const WRITER: u8 = 2;

const LINK_DOMAIN: &str = "sce-trust/link/1";

/// What the device that links and the device that is linked both sign: the list, the node that
/// vouches, and the node and key it vouches for.
pub fn link_bytes(list: &str, parent: u64, node: u64, key: &PublicKey) -> Vec<u8> {
    let mut out = Vec::new();
    part(&mut out, LINK_DOMAIN);
    part(&mut out, list);
    part(&mut out, &parent.to_string());
    part(&mut out, &node.to_string());
    part(&mut out, &key.to_text());
    out
}

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
    /// From this position of the log the device of `node` is a member with `role` and signs with
    /// `key`; a role of 0 takes it off. An operation names the entry of the log it was made under,
    /// and is judged by the grant in force at that entry; and every grant is kept.
    ///
    /// A grant that takes writing away from a device that could write (a role below 2, or another
    /// key) carries `final_seq`: the number of the last operation of that device the
    /// administrators accept, which they read from what the hosts and the checkpoints have seen of
    /// the device's chain. An operation of the device beyond it is refused by every host whatever
    /// entry it names, so that a device cannot be removed by a time it alone can claim, and cannot
    /// go back to an entry before its removal to keep writing. Operations the device made and had
    /// not yet published are lost; that is the cost of a removal that nobody can talk round.
    Grant {
        node: u64,
        role: u8,
        key: PublicKey,
        final_seq: Option<u64>,
    },
    /// The next epoch of the list's content key begins. Hosts seal under it from here on and a
    /// host that has no key of it does not write until it is given one.
    Epoch { epoch: u32 },
    /// The next epoch begins, and its key is the MLS group's (RFC 9420): `commit` is the SHA-256
    /// of the commit message that moves the group to the epoch. The administrators sign that the
    /// commit is the one, so that the order of the group's epochs is the log's and a relay that
    /// reorders or makes up a commit is found out by the chain; a host applies no commit this log
    /// did not name. A list's epochs are all of this kind or all of the kind above, not both.
    MlsEpoch { epoch: u32, commit: Hash },
    /// From this position a new device of a person is a member: the device of `node`, with `role`
    /// and `key`, **linked by** the device of `parent`, which is a member and signed for it
    /// ([`link_bytes`], `parent_signature`) as the new key did too (`signature`, proof that whoever
    /// asks holds it). A person has several devices, each with its own key and its own node; this
    /// is the entry that records that one vouched for another, so that the log says which devices
    /// are one person's ([`Log::person_of`]) and an administrator who is shown a device by a relay
    /// holds proof from a member and not the relay's word. A linked device has no more than its
    /// parent's role, and is a node the log has not held before. Removing a person is one removal
    /// for each of their devices; the log does not do it by itself.
    LinkedGrant {
        node: u64,
        role: u8,
        key: PublicKey,
        parent: u64,
        parent_signature: [u8; 64],
        signature: [u8; 64],
    },
    /// The last operation accepted of a device that was taken off is raised to `final_seq`, because
    /// members' checkpoints, made before it was taken off, name operations of its chain up to there:
    /// what the device published to some and not yet to the administrator's host is not lost. The
    /// log checks each checkpoint (a writer other than the device made it, under an entry before the
    /// removal, and it names the device's chain to `final_seq`), and at least one is needed.
    Amend {
        node: u64,
        final_seq: u64,
        evidence: Vec<Checkpoint>,
    },
    /// The keys of epoch `epoch` are to be destroyed: every host that sees the entry destroys what it
    /// holds of them and takes no more, so that what was sealed under the epoch opens for no one that
    /// obeys it. It needs the quorum, since it cannot be undone, and it names an epoch before the one
    /// the list is in. A member that keeps a copy of a key keeps what it could read; this is what a
    /// list can ask of the hosts that mean to keep to it.
    Shred { epoch: u32 },
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
            Body::Genesis { .. }
            | Body::Grant { .. }
            | Body::LinkedGrant { .. }
            | Body::Amend { .. }
            | Body::Epoch { .. }
            | Body::MlsEpoch { .. } => 1,
            Body::Administrators { .. } | Body::Profile(_) | Body::Shred { .. } => {
                usize::from(quorum)
            }
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
                final_seq,
            } => {
                part(out, "grant");
                part(out, &node.to_string());
                part(out, &role.to_string());
                part(out, &key.to_text());
                part(
                    out,
                    &final_seq.map_or_else(|| "none".to_string(), |seq| seq.to_string()),
                );
            }
            Body::Epoch { epoch } => {
                part(out, "epoch");
                part(out, &epoch.to_string());
            }
            Body::MlsEpoch { epoch, commit } => {
                part(out, "mls-epoch");
                part(out, &epoch.to_string());
                part(out, &encoding::encode(commit));
            }
            Body::LinkedGrant {
                node,
                role,
                key,
                parent,
                parent_signature,
                signature,
            } => {
                part(out, "linked-grant");
                part(out, &node.to_string());
                part(out, &role.to_string());
                part(out, &key.to_text());
                part(out, &parent.to_string());
                part(out, &encoding::encode(parent_signature));
                part(out, &encoding::encode(signature));
            }
            Body::Amend {
                node,
                final_seq,
                evidence,
            } => {
                part(out, "amend");
                part(out, &node.to_string());
                part(out, &final_seq.to_string());
                part(out, &evidence.len().to_string());
                for checkpoint in evidence {
                    part(out, &checkpoint.to_json());
                }
            }
            Body::Shred { epoch } => {
                part(out, "shred");
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
                final_seq,
            } => serde_json::json!({
                "kind": "grant",
                "node": node.to_string(),
                "role": role,
                "key": key.to_text(),
                "final_seq": final_seq.map(|seq| seq.to_string()),
            }),
            Body::Epoch { epoch } => serde_json::json!({"kind": "epoch", "epoch": epoch}),
            Body::MlsEpoch { epoch, commit } => serde_json::json!({
                "kind": "mls_epoch",
                "epoch": epoch,
                "commit": encoding::encode(commit),
            }),
            Body::LinkedGrant {
                node,
                role,
                key,
                parent,
                parent_signature,
                signature,
            } => serde_json::json!({
                "kind": "linked_grant",
                "node": node.to_string(),
                "role": role,
                "key": key.to_text(),
                "parent": parent.to_string(),
                "parent_sig": encoding::encode(parent_signature),
                "sig": encoding::encode(signature),
            }),
            Body::Amend {
                node,
                final_seq,
                evidence,
            } => serde_json::json!({
                "kind": "amend",
                "node": node.to_string(),
                "final_seq": final_seq.to_string(),
                "evidence": evidence.iter().map(Checkpoint::to_value).collect::<Vec<_>>(),
            }),
            Body::Shred { epoch } => serde_json::json!({"kind": "shred", "epoch": epoch}),
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
                final_seq: match body.get("final_seq")? {
                    serde_json::Value::Null => None,
                    serde_json::Value::String(seq) => Some(seq.parse().ok()?),
                    _ => return None,
                },
            },
            "epoch" => Body::Epoch {
                epoch: u32::try_from(body.get("epoch")?.as_u64()?).ok()?,
            },
            "mls_epoch" => Body::MlsEpoch {
                epoch: u32::try_from(body.get("epoch")?.as_u64()?).ok()?,
                commit: encoding::decode(&text_of(body, "commit")?)?
                    .try_into()
                    .ok()?,
            },
            "linked_grant" => Body::LinkedGrant {
                node: text_of(body, "node")?.parse().ok()?,
                role: small(body, "role")?,
                key: PublicKey::from_text(&text_of(body, "key")?)?,
                parent: text_of(body, "parent")?.parse().ok()?,
                parent_signature: encoding::decode(&text_of(body, "parent_sig")?)?
                    .try_into()
                    .ok()?,
                signature: encoding::decode(&text_of(body, "sig")?)?.try_into().ok()?,
            },
            "amend" => Body::Amend {
                node: text_of(body, "node")?.parse().ok()?,
                final_seq: text_of(body, "final_seq")?.parse().ok()?,
                evidence: body
                    .get("evidence")?
                    .as_array()?
                    .iter()
                    .map(Checkpoint::from_value)
                    .collect::<Option<Vec<_>>>()?,
            },
            "shred" => Body::Shred {
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

/// What one grant says of a member from the entry that made it on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Era {
    /// The position of the entry of the log that made the grant.
    pub index: u64,
    pub key: PublicKey,
    pub role: u8,
    /// The last operation of the device before this grant that the administrators accept, when the
    /// grant takes writing away or changes the key ([`Body::Grant`]).
    pub cutoff: Option<u64>,
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
    /// The position of the entry that began each epoch, the first epoch first: what says which epoch
    /// the list was in at an entry, so that an operation is judged by the entry it names and not by
    /// how far this host's log has come since.
    epoch_began: Vec<u64>,
    /// The commit each epoch of an MLS list was begun by, and whether the list's epochs are MLS's
    /// (`None` until the first epoch).
    mls: Option<bool>,
    commits: BTreeMap<u32, Hash>,
    /// The device each linked device was linked by.
    parents: BTreeMap<u64, u64>,
    /// The epochs whose keys the list has had destroyed.
    shredded: BTreeSet<u32>,
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
            epoch_began: Vec::new(),
            mls: None,
            commits: BTreeMap::new(),
            parents: BTreeMap::new(),
            shredded: BTreeSet::new(),
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

    /// Whether the list's epochs are the MLS group's (and its key is the group's exporter), as
    /// against a key an administrator drew; `false` while the list has had no epoch.
    pub fn is_mls(&self) -> bool {
        self.mls == Some(true)
    }

    /// The node of the first device of the person whose device `node` is: itself when it was not
    /// linked by another.
    pub fn person_of(&self, node: u64) -> u64 {
        let mut root = node;
        // Each link is to a node that was a member before it, so the chain is finite.
        while let Some(parent) = self.parents.get(&root) {
            root = *parent;
        }
        root
    }

    /// Every device the log holds of the person whose device `node` is, the first and the ones linked
    /// from it, whether they are members now or were taken off.
    pub fn devices_of(&self, node: u64) -> Vec<u64> {
        let person = self.person_of(node);
        let mut devices: Vec<u64> = self
            .members
            .keys()
            .copied()
            .filter(|held| self.person_of(*held) == person)
            .collect();
        devices.sort_unstable();
        devices
    }

    /// Whether the keys of `epoch` have been destroyed by the list.
    pub fn is_shredded(&self, epoch: u32) -> bool {
        self.shredded.contains(&epoch)
    }

    /// The epochs whose keys the list has had destroyed.
    pub fn shredded(&self) -> impl Iterator<Item = u32> + '_ {
        self.shredded.iter().copied()
    }

    /// The hash of the commit message that began `epoch` of an MLS list, when the log names one.
    pub fn commit_of(&self, epoch: u32) -> Option<Hash> {
        self.commits.get(&epoch).copied()
    }

    /// The latest epoch the log begins, 0 when none.
    pub fn epoch(&self) -> u32 {
        self.epoch
    }

    /// The epoch the list was in at the entry in position `index`: the latest epoch an entry at or
    /// before it began, 0 when none had. An operation is judged by this and not by the log's latest
    /// epoch, because what a device wrote it wrote knowing the log up to the entry it names, and two
    /// hosts that were given the same entries in another order must judge it the same.
    pub fn epoch_at(&self, index: u64) -> u32 {
        self.epoch_began.partition_point(|began| *began <= index) as u32
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

    /// The latest entry held and its hash: what an operation names as the log it was made under.
    /// `None` when the log has no entry.
    pub fn tip(&self) -> Option<(u64, Hash)> {
        Some((
            self.entries.len().checked_sub(1)? as u64,
            *self.hashes.last()?,
        ))
    }

    /// The hash of the entry at `index`, when the log holds it.
    pub fn hash_at(&self, index: u64) -> Option<Hash> {
        self.hashes.get(usize::try_from(index).ok()?).copied()
    }

    /// What the log says of `node` as of the entry at `entry`: the latest grant at or before it.
    /// `None` when there is none, which is a device that was not yet a member.
    pub fn era_for(&self, node: u64, entry: u64) -> Option<&Era> {
        self.members
            .get(&node)?
            .values()
            .rev()
            .find(|era| era.index <= entry)
    }

    /// Every number at which a chain of `node`'s was ended by a grant, lowest first. A new chain of
    /// the node (another key, or the same device made a writer again) begins at the number after
    /// one of these, and names no operation before it.
    pub fn cutoffs(&self, node: u64) -> Vec<u64> {
        let mut cutoffs: Vec<u64> = self
            .members
            .get(&node)
            .map(|eras| eras.values().filter_map(|era| era.cutoff).collect())
            .unwrap_or_default();
        cutoffs.sort_unstable();
        cutoffs.dedup();
        cutoffs
    }

    /// The number the operations judged by `era` must be past: the last of the chains that the
    /// grants of `node` up to and including this one ended, 0 when none. An operation of the era
    /// numbered at or below it would be a number an earlier chain used.
    pub fn floor_before(&self, node: u64, era: &Era) -> u64 {
        self.members
            .get(&node)
            .map(|eras| {
                eras.values()
                    .filter(|earlier| earlier.index <= era.index)
                    .filter_map(|earlier| earlier.cutoff)
                    .max()
                    .unwrap_or(0)
            })
            .unwrap_or(0)
    }

    /// The last operation of `node`'s that is accepted when the operation is judged by `era`: the
    /// lowest cutoff among the later grants of the node, `None` when no later grant sets one. An
    /// operation with a number past it is refused whatever entry it names.
    pub fn cutoff_after(&self, node: u64, era: &Era) -> Option<u64> {
        self.members
            .get(&node)?
            .values()
            .filter(|later| later.index > era.index)
            .filter_map(|later| later.cutoff)
            .min()
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
                Err(self.rival(entry))
            };
        }
        if entry.index > held {
            return Err(LogError::Gap { expected: held });
        }
        let (_, prev) = self.head();
        if entry.prev != prev {
            return Err(self.rival(entry));
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

    /// What an entry that is not the one this log holds at its place, and does not follow its head, is.
    /// It is a fork only if the administrators signed it: an entry nobody signed (a hub rewrote one
    /// and took the signatures off, or invented one) proves nothing, and a host that took it for a
    /// fork would be frozen by anyone who can write to it. So the entry is judged as the log would
    /// have judged it at its place, and it is a fork when it holds there; when it does not, it is
    /// refused for the reason it does not, and the log is as it was.
    fn rival(&self, entry: &Entry) -> LogError {
        // An entry no key that was ever an administrator's signed is cheap to turn away: nothing
        // is replayed for it.
        let ever: Vec<PublicKey> = self
            .entries
            .iter()
            .flat_map(Self::administrators_named)
            .collect();
        if !entry.signatures.iter().any(|(key, _)| ever.contains(key)) {
            return LogError::NotEnoughSignatures { needed: 1, had: 0 };
        }
        let held = self.entries.len() as u64;
        if entry.index < held {
            // What the log was when the entry's place came: the entries before it, judged again.
            let mut before = Self::empty(&self.list, self.expected_genesis);
            for earlier in &self.entries[..entry.index as usize] {
                if let Err(error) = before.append(earlier) {
                    return error;
                }
            }
            return match before.append(entry) {
                Ok(_) => LogError::Fork { index: entry.index },
                Err(error) => error,
            };
        }
        // It names an entry before it that this log does not hold: it can be judged only by the
        // administrators this log has, and when they signed it, there are two histories.
        let needed = entry.body.required(self.quorum);
        match Self::verified_signers(entry, &self.administrators) {
            Ok(had) if had >= needed => LogError::Fork { index: entry.index },
            Ok(had) => LogError::NotEnoughSignatures { needed, had },
            Err(error) => error,
        }
    }

    /// The administrators an entry names: the genesis's, and those an entry that changes them adds.
    fn administrators_named(entry: &Entry) -> Vec<PublicKey> {
        match &entry.body {
            Body::Genesis { administrators, .. } => administrators.clone(),
            Body::Administrators { add, .. } => add.clone(),
            _ => Vec::new(),
        }
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

    /// Raise the last operation accepted of the device of `node`, which the latest grant of it took
    /// off, to `final_seq`. The evidence is checkpoints of other writers, made under an entry before
    /// that grant, each naming the device's chain in full to at least `final_seq`; one that holds is
    /// enough, and the administrator who signs the entry answers for choosing it.
    fn amend(
        &mut self,
        node: u64,
        final_seq: u64,
        evidence: &[Checkpoint],
    ) -> Result<(), LogError> {
        let (index, era) = self
            .members
            .get(&node)
            .and_then(|eras| eras.iter().next_back())
            .map(|(index, era)| (*index, *era))
            .ok_or(LogError::Invalid("only a device taken off is amended"))?;
        let Some(cutoff) = era.cutoff.filter(|_| era.role == 0) else {
            return Err(LogError::Invalid("only a device taken off is amended"));
        };
        if final_seq <= cutoff {
            return Err(LogError::Invalid("an amendment raises the last accepted"));
        }
        let held = |checkpoint: &Checkpoint| -> bool {
            if checkpoint.list != self.list
                || checkpoint.device == node
                || checkpoint.log >= index
                || self.hashes.get(checkpoint.log as usize) != Some(&checkpoint.log_hash)
                || !checkpoint.is_canonical()
            {
                return false;
            }
            let Some(signer) = self.era_for(checkpoint.device, checkpoint.log) else {
                return false;
            };
            signer.role >= WRITER
                && signer
                    .key
                    .verifies(&checkpoint.signing_bytes(), &checkpoint.signature)
                && checkpoint
                    .heads
                    .iter()
                    .any(|head| head.node == node && head.seq >= final_seq)
        };
        if !evidence.iter().any(held) {
            return Err(LogError::Invalid(
                "an amendment is shown by a checkpoint of another writer, made before the removal",
            ));
        }
        if let Some(era) = self
            .members
            .get_mut(&node)
            .and_then(|eras| eras.get_mut(&index))
        {
            era.cutoff = Some(final_seq);
        }
        Ok(())
    }

    /// Begin epoch `epoch`, which must follow the one before it and be of the kind the list's
    /// epochs are.
    fn next_epoch(&mut self, epoch: u32, mls: bool, position: u64) -> Result<(), LogError> {
        if epoch
            != self
                .epoch
                .checked_add(1)
                .ok_or(LogError::Invalid("epochs run out"))?
        {
            return Err(LogError::Invalid("an epoch follows the one before it"));
        }
        if self.mls.is_some_and(|kind| kind != mls) {
            return Err(LogError::Invalid(
                "a list's epochs are all MLS's or all a drawn key's",
            ));
        }
        self.mls = Some(mls);
        self.epoch = epoch;
        self.epoch_began.push(position);
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
                final_seq,
            } => {
                if *role > 3 {
                    return Err(LogError::Invalid("a role is 0 to 3"));
                }
                // A grant that takes writing away from a device that could write, or gives it
                // another key, says where its chain ends: without it the device could go on
                // writing under the entry before, which no host could tell from a late arrival.
                let takes_writing_away = self
                    .members
                    .get(node)
                    .and_then(|eras| eras.values().next_back())
                    .is_some_and(|latest| {
                        latest.role >= WRITER && (*role < WRITER || latest.key != *key)
                    });
                if takes_writing_away && final_seq.is_none() {
                    return Err(LogError::Invalid(
                        "a grant that takes writing away carries the last operation accepted",
                    ));
                }
                self.members.entry(*node).or_default().insert(
                    position,
                    Era {
                        index: position,
                        key: *key,
                        role: *role,
                        cutoff: *final_seq,
                    },
                );
            }
            Body::LinkedGrant {
                node,
                role,
                key,
                parent,
                parent_signature,
                signature,
            } => {
                if *role == 0 || *role > 3 {
                    return Err(LogError::Invalid("a linked device has a role of 1 to 3"));
                }
                if self.members.contains_key(node) {
                    return Err(LogError::Invalid(
                        "a linked device is a node the log has not held before",
                    ));
                }
                let linking = self
                    .members
                    .get(parent)
                    .and_then(|eras| eras.values().next_back())
                    .copied()
                    .filter(|era| era.role >= 1)
                    .ok_or(LogError::Invalid("a device is linked by a member"))?;
                if *role > linking.role {
                    return Err(LogError::Invalid(
                        "a linked device has no more than its parent's role",
                    ));
                }
                let statement = link_bytes(&self.list, *parent, *node, key);
                if !linking.key.verifies(&statement, parent_signature)
                    || !key.verifies(&statement, signature)
                {
                    return Err(LogError::BadSignature);
                }
                self.members.entry(*node).or_default().insert(
                    position,
                    Era {
                        index: position,
                        key: *key,
                        role: *role,
                        cutoff: None,
                    },
                );
                self.parents.insert(*node, *parent);
            }
            Body::Amend {
                node,
                final_seq,
                evidence,
            } => {
                self.amend(*node, *final_seq, evidence)?;
            }
            Body::Shred { epoch } => {
                if *epoch == 0 || *epoch >= self.epoch {
                    return Err(LogError::Invalid(
                        "an epoch is destroyed after the list has moved past it",
                    ));
                }
                if !self.shredded.insert(*epoch) {
                    return Err(LogError::Invalid("an epoch is destroyed once"));
                }
            }
            Body::Epoch { epoch } => {
                self.next_epoch(*epoch, false, position)?;
            }
            Body::MlsEpoch { epoch, commit } => {
                self.next_epoch(*epoch, true, position)?;
                self.commits.insert(*epoch, *commit);
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

    fn grant(log: &Log, node: u64, role: u8, by: &Identity) -> Entry {
        grant_cut(log, node, role, None, by)
    }

    fn grant_cut(log: &Log, node: u64, role: u8, final_seq: Option<u64>, by: &Identity) -> Entry {
        log.draft(Body::Grant {
            node,
            role,
            key: admin(50 + node as u8).public(),
            final_seq,
        })
        .sign(by)
        .finish()
    }

    #[test]
    fn an_entry_is_known_when_it_is_the_one_held_and_a_fork_when_it_is_not() {
        let mut log = started(2);
        let first = grant(&log, 1, 2, &admin(1));
        assert_eq!(log.append(&first), Ok(true));
        assert_eq!(log.append(&first), Ok(false), "taken in once");

        let other = log
            .draft(Body::Grant {
                node: 1,
                role: 3,
                key: admin(51).public(),
                final_seq: None,
            })
            .sign(&admin(1))
            .finish();
        // `other` was drafted after `first`, so it is at index 2; make one at index 1 instead.
        let mut forked = other.clone();
        forked.index = 1;
        forked.prev = log.hashes[0];
        forked.signatures.clear();
        // A rival the administrators did not sign (nobody, or a stranger) is no fork: it proves
        // nothing, and whoever can write to a host could else freeze it.
        let by_a_stranger = Draft {
            entry: forked.clone(),
        }
        .sign(&admin(77))
        .finish();
        assert!(matches!(
            log.append(&by_a_stranger),
            Err(LogError::NotEnoughSignatures { .. })
        ));
        let mut unsigned = by_a_stranger.clone();
        unsigned.signatures.clear();
        assert!(matches!(
            log.append(&unsigned),
            Err(LogError::NotEnoughSignatures { .. })
        ));
        // One the administrators signed is two histories.
        let forked = Draft { entry: forked }
            .sign(&admin(1))
            .sign(&admin(2))
            .finish();
        assert_eq!(log.append(&forked), Err(LogError::Fork { index: 1 }));
        assert_eq!(log.entries().len(), 2, "the log did not change");
        // The same changed after the administrators signed it does not hold at its place, and a
        // rival that does not hold is no fork.
        let mut changed = forked.clone();
        changed.body = Body::Epoch { epoch: 1 };
        assert_eq!(log.append(&changed), Err(LogError::BadSignature));
    }

    #[test]
    fn an_entry_that_skips_one_or_does_not_name_the_one_before_is_refused() {
        let mut log = started(1);
        let mut skipping = grant(&log, 1, 2, &admin(1));
        skipping.index = 5;
        assert_eq!(log.append(&skipping), Err(LogError::Gap { expected: 1 }));

        // One that names another entry before it, signed by the administrators, is a history this log
        // does not hold; made by hand, signed after it was changed.
        let mut draft = log.draft(Body::Epoch { epoch: 1 });
        draft.entry.prev = [9; 32];
        let wrong_prev = draft.sign(&admin(1)).finish();
        assert_eq!(log.append(&wrong_prev), Err(LogError::Fork { index: 1 }));
        // The same changed after it was signed holds no signature, and is no fork.
        let mut changed = log.draft(Body::Epoch { epoch: 1 }).sign(&admin(1)).finish();
        changed.prev = [9; 32];
        assert_eq!(log.append(&changed), Err(LogError::BadSignature));

        // One that names another entry before it and has fewer of the administrators' signatures than
        // its kind needs is no fork either: a change of administrators needs the quorum of two.
        let mut log = started(2);
        let mut draft = log.draft(Body::Administrators {
            add: vec![admin(4).public()],
            remove: vec![],
            quorum: 2,
        });
        draft.entry.prev = [9; 32];
        let under_signed = draft.sign(&admin(1)).finish();
        assert_eq!(
            log.append(&under_signed),
            Err(LogError::NotEnoughSignatures { needed: 2, had: 1 })
        );
    }

    #[test]
    fn an_entry_needs_the_signatures_its_kind_needs() {
        let mut log = started(2);
        // A member's role needs one administrator.
        let by_one = grant(&log, 1, 2, &admin(2));
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
        let mut entry = grant(&log, 1, 2, &admin(1));
        entry.signatures[0].1[0] ^= 1;
        assert_eq!(log.append(&entry), Err(LogError::BadSignature));
        let mut edited = grant(&log, 1, 2, &admin(1));
        edited.body = Body::Grant {
            node: 1,
            role: 3,
            key: admin(51).public(),
            final_seq: None,
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
    fn what_a_grant_says_holds_from_its_entry_and_an_operation_is_judged_by_the_one_it_names() {
        let mut log = started(1);
        let on = grant(&log, 1, 2, &admin(1));
        log.append(&on).unwrap();
        let off = grant_cut(&log, 1, 0, Some(4), &admin(1));
        log.append(&off).unwrap();
        // Entry 0 is the genesis, 1 the grant, 2 the removal.
        assert_eq!(
            log.era_for(1, 0),
            None,
            "before the first grant is no member"
        );
        assert_eq!(log.era_for(1, 1).map(|era| era.role), Some(2));
        assert_eq!(log.era_for(1, 2).map(|era| era.role), Some(0));
        assert_eq!(log.era_for(7, 2).map(|era| era.role), None);
        assert!(log.was_a_writer(&admin(51).public()));
        let era = *log.era_for(1, 1).unwrap();
        assert_eq!(
            log.cutoff_after(1, &era),
            Some(4),
            "an operation under entry 1 past number 4 is refused, whatever entry it names"
        );
        assert_eq!(log.cutoff_after(1, log.era_for(1, 2).unwrap()), None);
    }

    #[test]
    fn a_grant_that_takes_writing_away_says_where_the_devices_chain_ends() {
        let mut log = started(1);
        let on = grant(&log, 1, 2, &admin(1));
        log.append(&on).unwrap();
        let removal = grant(&log, 1, 0, &admin(1));
        assert_eq!(
            log.append(&removal),
            Err(LogError::Invalid(
                "a grant that takes writing away carries the last operation accepted"
            ))
        );
        let new_key = log
            .draft(Body::Grant {
                node: 1,
                role: 2,
                key: admin(77).public(),
                final_seq: None,
            })
            .sign(&admin(1))
            .finish();
        assert_eq!(
            log.append(&new_key),
            Err(LogError::Invalid(
                "a grant that takes writing away carries the last operation accepted"
            )),
            "another key is the same: the old one's chain has to end somewhere"
        );
        // A reader may be made a writer, and a writer a writer of more, with no cutoff.
        let reader = grant(&log, 2, 1, &admin(1));
        log.append(&reader).unwrap();
        let writer = grant(&log, 2, 2, &admin(1));
        assert_eq!(log.append(&writer), Ok(true));
    }

    #[test]
    fn an_mls_epoch_names_its_commit_and_a_list_does_not_mix_kinds_of_epoch() {
        let mut log = started(1);
        let first = log
            .draft(Body::MlsEpoch {
                epoch: 1,
                commit: [7; 32],
            })
            .sign(&admin(1))
            .finish();
        assert_eq!(log.append(&first), Ok(true));
        assert!(log.is_mls());
        assert_eq!(log.commit_of(1), Some([7; 32]));
        assert_eq!(log.commit_of(2), None);
        assert_eq!(Entry::from_json(&first.to_json()), Some(first.clone()));

        // The commit is part of what the administrators sign.
        let other = Entry {
            body: Body::MlsEpoch {
                epoch: 1,
                commit: [8; 32],
            },
            ..first.clone()
        };
        assert_ne!(other.signing_bytes(), first.signing_bytes());

        let mixed = log.draft(Body::Epoch { epoch: 2 }).sign(&admin(1)).finish();
        assert_eq!(
            log.append(&mixed),
            Err(LogError::Invalid(
                "a list's epochs are all MLS's or all a drawn key's"
            ))
        );
        let skipping = log
            .draft(Body::MlsEpoch {
                epoch: 3,
                commit: [9; 32],
            })
            .sign(&admin(1))
            .finish();
        assert_eq!(
            log.append(&skipping),
            Err(LogError::Invalid("an epoch follows the one before it"))
        );
        assert_eq!(log.epoch(), 1, "a refused entry changes nothing");

        let mut drawn = started(1);
        let epoch = drawn
            .draft(Body::Epoch { epoch: 1 })
            .sign(&admin(1))
            .finish();
        drawn.append(&epoch).unwrap();
        assert!(!drawn.is_mls());
        let mls = drawn
            .draft(Body::MlsEpoch {
                epoch: 2,
                commit: [1; 32],
            })
            .sign(&admin(1))
            .finish();
        assert!(drawn.append(&mls).is_err());
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
                final_seq: Some(u64::MAX),
            },
            Body::Grant {
                node: 3,
                role: 1,
                key: admin(52).public(),
                final_seq: None,
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
