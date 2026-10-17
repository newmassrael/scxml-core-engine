// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The group key agreement of a shared list: MLS (RFC 9420) through `openmls`, bound to the list's
//! membership log.
//!
//! A list's content is sealed under a key for each epoch (`sce-rust-trust`, `secrecy`). With MLS the
//! key is the group's: every membership change is a commit that moves the group to a new epoch whose
//! secrets a removed member cannot derive, and the key of the epoch is what the group's exporter
//! gives. What makes the group the list's group is the log:
//!
//! * A device's leaf signs with **the key the log gave it** (one key for a person), and its
//!   credential names the list and the node. A device the log has no grant for is not accepted.
//! * A commit is applied only when **the log names it**: an administrator-signed entry
//!   (`Body::MlsEpoch`) carries the epoch and the SHA-256 of the commit message, so the order of the
//!   group's epochs is the log's and a relay that reorders or makes up a commit is found out. The log
//!   epoch is the group's epoch.
//! * After a commit the group's roster **must be the log's members** (the nodes and keys of every
//!   grant of role 1 or more); a commit that leaves it otherwise is rolled back.
//! * A device that joins from a Welcome checks it against the log too: the commit that made the epoch
//!   (named by the log) carries the confirmation tag the group the Welcome made must hold, so a
//!   Welcome of another group with the same members is not the list's.
//!
//! Handshake messages are plaintext, since the log already says who is added and removed; what MLS is
//! used for is the secrets. Content is not sent as MLS application messages: [`Device::epoch_key`]
//! gives the exporter's key of the epoch and sealing is `sce_rust_trust::secrecy`'s. A section of
//! the list has a key of its own derived from the epoch's ([`Device::section_key`]).
//!
//! The suite is the mandatory one: X25519, AES-128-GCM, SHA-256, Ed25519.

#![forbid(unsafe_code)]

pub mod host;

use std::collections::BTreeMap;

use openmls::prelude::tls_codec::{Deserialize as _, Serialize as _};
use openmls::prelude::*;
use openmls_rust_crypto::OpenMlsRustCrypto;
use openmls_traits::signatures::{Signer, SignerError};
use openmls_traits::types::SignatureScheme;
use openmls_traits::OpenMlsProvider;
use sce_rust_trust::membership::Hash;
use sce_rust_trust::{Identity, Trust};
use sha2::{Digest, Sha256};

/// The cipher suite of every list: MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519.
pub const SUITE: Ciphersuite = Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519;

/// What the exporter is asked for, so that no other use of the group's secret gives the same bytes.
const EPOCH_LABEL: &str = "sce-trust/epoch-key/1";
const SECTION_LABEL: &str = "sce-trust/section-key/1";
const CREDENTIAL_PREFIX: &str = "sce-trust/mls/1";

/// The key of one epoch (or of one section of it), as `sce_rust_trust::secrecy` takes it.
pub type EpochKey = [u8; 32];

/// What the group needs to know of the membership log: who the members are now and with which key,
/// which commit the log names for each epoch, and which epoch the log is in. A host that holds the
/// log in [`Trust`] gives it as it is; a lane that does not (the C ABI) gives it as data
/// ([`Roster`]).
pub trait LogView {
    /// The nodes the log makes members now (a role of 1 or more), each with its Ed25519 key.
    fn members(&self) -> Vec<(u64, [u8; 32])>;
    /// The SHA-256 of the commit message the log names as the one that begins `epoch`.
    fn commit_of(&self, epoch: u32) -> Option<Hash>;
    /// The latest epoch the log begins.
    fn epoch(&self) -> u32;
}

impl LogView for Trust {
    fn members(&self) -> Vec<(u64, [u8; 32])> {
        self.current_members()
            .into_iter()
            .map(|(node, key)| (node, key.to_bytes()))
            .collect()
    }

    fn commit_of(&self, epoch: u32) -> Option<Hash> {
        Trust::commit_of(self, epoch)
    }

    fn epoch(&self) -> u32 {
        Trust::epoch(self)
    }
}

/// The log as data: what a lane that does not hold the membership log itself tells the group of
/// it. The lane is responsible for it being what the log verified to.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Roster {
    members: BTreeMap<u64, [u8; 32]>,
    commits: BTreeMap<u32, Hash>,
    epoch: u32,
}

impl Roster {
    pub fn new(epoch: u32) -> Self {
        Self {
            members: BTreeMap::new(),
            commits: BTreeMap::new(),
            epoch,
        }
    }

    pub fn add_member(&mut self, node: u64, key: [u8; 32]) {
        self.members.insert(node, key);
    }

    pub fn name_commit(&mut self, epoch: u32, commit: Hash) {
        self.commits.insert(epoch, commit);
    }
}

impl LogView for Roster {
    fn members(&self) -> Vec<(u64, [u8; 32])> {
        self.members
            .iter()
            .map(|(node, key)| (*node, *key))
            .collect()
    }

    fn commit_of(&self, epoch: u32) -> Option<Hash> {
        self.commits.get(&epoch).copied()
    }

    fn epoch(&self) -> u32 {
        self.epoch
    }
}

/// Why an MLS operation was not done.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MlsError {
    /// openmls refused it, with its words.
    Library(String),
    /// The device is in no group yet.
    NoGroup,
    /// The device is in a group already.
    InGroup,
    /// A commit is waiting to be merged or cleared.
    PendingCommit,
    /// There is no commit waiting.
    NoPendingCommit,
    /// The log does not name this commit as the one that begins the epoch.
    CommitNotNamed { epoch: u64 },
    /// A device is not a member the log holds, or holds with another key.
    NotInLog(String),
    /// A member is still a member of the log, and a commit does not remove it yet.
    StillAMember(u64),
    /// The group's members are not the log's.
    RosterDiffers(String),
    /// The Welcome is not the log's: its group does not hold the confirmation tag of the commit the
    /// log names.
    WelcomeNotTheLogs,
    /// A message is not the kind it was given as.
    Malformed(&'static str),
    /// The device was removed from the group: it has no key of any later epoch.
    Removed,
}

fn lib<E: std::fmt::Debug>(error: E) -> MlsError {
    MlsError::Library(format!("{error:?}"))
}

/// What applying a commit did to this device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Applied {
    /// The group is in this epoch now.
    Epoch(u64),
    /// The commit removed this device: the group is closed to it, and no key of a later epoch is
    /// derivable.
    Removed,
}

/// A commit this device made and has not merged: the entry of the log names `commit_hash`, and the
/// device merges when the log holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
    /// The commit message, as it goes to the others.
    pub commit: Vec<u8>,
    /// The Welcome for the members added, when it adds any.
    pub welcome: Option<Vec<u8>>,
    /// SHA-256 of `commit`: what the administrator's `Body::MlsEpoch` carries.
    pub commit_hash: Hash,
    /// The epoch the commit makes, which is the epoch the entry begins.
    pub epoch: u64,
}

/// The hash of a commit message, which the log names it by.
pub fn commit_hash(commit: &[u8]) -> Hash {
    Sha256::digest(commit).into()
}

fn identity_bytes(list: &str, node: u64) -> Vec<u8> {
    format!("{CREDENTIAL_PREFIX}\0{list}\0{node}").into_bytes()
}

/// The node a credential names, when it is this list's.
fn node_of(list: &str, identity: &[u8]) -> Option<u64> {
    let text = std::str::from_utf8(identity).ok()?;
    let rest = text.strip_prefix(CREDENTIAL_PREFIX)?.strip_prefix('\0')?;
    let (named, node) = rest.rsplit_once('\0')?;
    (named == list).then(|| node.parse().ok()).flatten()
}

/// The device's key as openmls signs with it: the log's key, which the device holds.
struct DeviceSigner(Identity);

impl Signer for DeviceSigner {
    fn sign(&self, payload: &[u8]) -> Result<Vec<u8>, SignerError> {
        Ok(self.0.sign(payload).to_vec())
    }

    fn signature_scheme(&self) -> SignatureScheme {
        SignatureScheme::ED25519
    }
}

/// How long a leaf node and a key package are valid for: ten years. MLS's default is three months,
/// which is how long a key package waits for an administrator in an ordinary service; here the
/// log decides who is a member and for how long, an invitation may wait a long time, and a group
/// whose leaves expire by the clock would stop working for reasons that are no one's decision.
pub const LIFETIME_SECONDS: u64 = 10 * 365 * 24 * 3600;

fn create_config() -> MlsGroupCreateConfig {
    MlsGroupCreateConfig::builder()
        .lifetime(Lifetime::new(LIFETIME_SECONDS))
        .ciphersuite(SUITE)
        .use_ratchet_tree_extension(true)
        .wire_format_policy(PURE_PLAINTEXT_WIRE_FORMAT_POLICY)
        .build()
}

fn join_config() -> MlsGroupJoinConfig {
    MlsGroupJoinConfig::builder()
        .use_ratchet_tree_extension(true)
        .wire_format_policy(PURE_PLAINTEXT_WIRE_FORMAT_POLICY)
        .build()
}

/// One device's place in a list's group.
pub struct Device {
    provider: OpenMlsRustCrypto,
    signer: DeviceSigner,
    list: String,
    node: u64,
    group: Option<MlsGroup>,
    pending: Option<Pending>,
}

impl Device {
    /// The MLS side of the device of `node` of the list `list`, signing with `identity`, which is
    /// the key the log gave it.
    pub fn new(list: &str, node: u64, identity: Identity) -> Self {
        Self {
            provider: OpenMlsRustCrypto::default(),
            signer: DeviceSigner(identity),
            list: list.to_string(),
            node,
            group: None,
            pending: None,
        }
    }

    fn group_id(&self) -> GroupId {
        GroupId::from_slice(self.list.as_bytes())
    }

    fn credential(&self) -> CredentialWithKey {
        CredentialWithKey {
            credential: BasicCredential::new(identity_bytes(&self.list, self.node)).into(),
            signature_key: self.signer.0.public().to_bytes().to_vec().into(),
        }
    }

    fn group(&self) -> Result<&MlsGroup, MlsError> {
        self.group.as_ref().ok_or(MlsError::NoGroup)
    }

    /// A key package for this device: what an administrator adds it to the group with. It holds the
    /// public keys, and the private ones stay in this device's storage; it is used once.
    pub fn key_package(&mut self) -> Result<Vec<u8>, MlsError> {
        let bundle = KeyPackage::builder()
            .key_package_lifetime(Lifetime::new(LIFETIME_SECONDS))
            .build(SUITE, &self.provider, &self.signer, self.credential())
            .map_err(lib)?;
        bundle.key_package().tls_serialize_detached().map_err(lib)
    }

    /// Begin the list's group, with this device alone in it at epoch 0. The device of the
    /// administrator who makes the list does this.
    pub fn create_group(&mut self) -> Result<(), MlsError> {
        if self.group.is_some() {
            return Err(MlsError::InGroup);
        }
        let group = MlsGroup::new_with_group_id(
            &self.provider,
            &self.signer,
            &create_config(),
            self.group_id(),
            self.credential(),
        )
        .map_err(lib)?;
        self.group = Some(group);
        Ok(())
    }

    /// The epoch the group is in.
    pub fn epoch(&self) -> Result<u64, MlsError> {
        Ok(self.group()?.epoch().as_u64())
    }

    /// Whether this device is in the group still (a removed one is not).
    pub fn is_active(&self) -> bool {
        self.group.as_ref().is_some_and(MlsGroup::is_active)
    }

    /// The members of the group, as the node and the key each signs with.
    pub fn members(&self) -> Result<BTreeMap<u64, [u8; 32]>, MlsError> {
        let group = self.group()?;
        let mut members = BTreeMap::new();
        for member in group.members() {
            let node =
                node_of(&self.list, member.credential.serialized_content()).ok_or_else(|| {
                    MlsError::RosterDiffers(
                        "a leaf has a credential that is not this list's".into(),
                    )
                })?;
            let key: [u8; 32] = member.signature_key.as_slice().try_into().map_err(|_| {
                MlsError::RosterDiffers("a leaf has a key that is not an Ed25519 key".into())
            })?;
            if members.insert(node, key).is_some() {
                return Err(MlsError::RosterDiffers(format!(
                    "node {node} is in the group twice"
                )));
            }
        }
        Ok(members)
    }

    /// Whether the group's members are the log's: the same nodes, each with the key the log gave it.
    pub fn verify_roster(&self, log: &impl LogView) -> Result<(), MlsError> {
        let have = self.members()?;
        let want: BTreeMap<u64, [u8; 32]> = log.members().into_iter().collect();
        if have == want {
            return Ok(());
        }
        let nodes = |members: &BTreeMap<u64, [u8; 32]>| members.keys().copied().collect::<Vec<_>>();
        Err(MlsError::RosterDiffers(format!(
            "the group holds nodes {:?} and the log's members are {:?}",
            nodes(&have),
            nodes(&want)
        )))
    }

    /// The commit that adds the devices of `key_packages` to the group, made here and not merged:
    /// the administrator appends the entry that names `commit_hash` to the log, then
    /// [`Self::merge_pending`]. Each device must be a member the log holds, with the key its key
    /// package is signed by.
    pub fn stage_add(
        &mut self,
        key_packages: &[Vec<u8>],
        log: &impl LogView,
    ) -> Result<Pending, MlsError> {
        if self.pending.is_some() {
            return Err(MlsError::PendingCommit);
        }
        let members = log.members();
        let mut packages = Vec::new();
        for bytes in key_packages {
            let package = KeyPackageIn::tls_deserialize_exact(bytes)
                .map_err(lib)?
                .validate(self.provider.crypto(), ProtocolVersion::Mls10)
                .map_err(lib)?;
            let leaf = package.leaf_node();
            let node = node_of(&self.list, leaf.credential().serialized_content())
                .ok_or_else(|| MlsError::NotInLog("a key package of another list".into()))?;
            let listed = members
                .iter()
                .any(|(held, key)| *held == node && key == leaf.signature_key().as_slice());
            if !listed {
                return Err(MlsError::NotInLog(format!(
                    "node {node} is not a member of the log with this key"
                )));
            }
            packages.push(package);
        }
        let group = self.group.as_mut().ok_or(MlsError::NoGroup)?;
        let (commit, welcome, _) = group
            .add_members(&self.provider, &self.signer, &packages)
            .map_err(lib)?;
        let pending = Pending {
            commit: commit.to_bytes().map_err(lib)?,
            welcome: Some(welcome.to_bytes().map_err(lib)?),
            commit_hash: [0; 32],
            epoch: group.epoch().as_u64() + 1,
        };
        self.stage(pending)
    }

    /// The commit that removes the devices of `nodes` from the group, made here and not merged. A
    /// node must already be off the log's roster: the entry that takes it off comes first.
    pub fn stage_remove(&mut self, nodes: &[u64], log: &impl LogView) -> Result<Pending, MlsError> {
        if self.pending.is_some() {
            return Err(MlsError::PendingCommit);
        }
        let members = log.members();
        if let Some((node, _)) = members.iter().find(|(node, _)| nodes.contains(node)) {
            return Err(MlsError::StillAMember(*node));
        }
        let group = self.group.as_mut().ok_or(MlsError::NoGroup)?;
        let indexes: Vec<LeafNodeIndex> = group
            .members()
            .filter(|member| {
                node_of(&self.list, member.credential.serialized_content())
                    .is_some_and(|node| nodes.contains(&node))
            })
            .map(|member| member.index)
            .collect();
        if indexes.is_empty() {
            return Err(MlsError::Malformed("none of these nodes is in the group"));
        }
        let (commit, welcome, _) = group
            .remove_members(&self.provider, &self.signer, &indexes)
            .map_err(lib)?;
        let pending = Pending {
            commit: commit.to_bytes().map_err(lib)?,
            welcome: match welcome {
                Some(welcome) => Some(welcome.to_bytes().map_err(lib)?),
                None => None,
            },
            commit_hash: [0; 32],
            epoch: group.epoch().as_u64() + 1,
        };
        self.stage(pending)
    }

    fn stage(&mut self, mut pending: Pending) -> Result<Pending, MlsError> {
        pending.commit_hash = commit_hash(&pending.commit);
        self.pending = Some(pending.clone());
        Ok(pending)
    }

    /// Merge the commit made here, now that the log names it. The group's roster must then be the
    /// log's, and when it is not the commit is undone.
    pub fn merge_pending(&mut self, log: &impl LogView) -> Result<u64, MlsError> {
        let pending = self.pending.clone().ok_or(MlsError::NoPendingCommit)?;
        let named = u32::try_from(pending.epoch)
            .ok()
            .and_then(|epoch| log.commit_of(epoch));
        if named != Some(pending.commit_hash) {
            return Err(MlsError::CommitNotNamed {
                epoch: pending.epoch,
            });
        }
        let snapshot = self.snapshot();
        let merged = {
            let group = self.group.as_mut().ok_or(MlsError::NoGroup)?;
            group.merge_pending_commit(&self.provider).map_err(lib)
        };
        let checked = merged.and_then(|()| self.verify_roster(log));
        if let Err(error) = checked {
            self.restore_snapshot(snapshot);
            return Err(error);
        }
        self.pending = None;
        self.epoch()
    }

    /// Give up the commit made here and not merged.
    pub fn clear_pending(&mut self) -> Result<(), MlsError> {
        let group = self.group.as_mut().ok_or(MlsError::NoGroup)?;
        group
            .clear_pending_commit(self.provider.storage())
            .map_err(lib)?;
        self.pending = None;
        Ok(())
    }

    /// Apply a commit another device made, when the log names it as the one that begins the next
    /// epoch. A commit the log did not name, one that fails, and one after which the group's roster
    /// is not the log's leave this device where it was.
    pub fn apply_commit(&mut self, commit: &[u8], log: &impl LogView) -> Result<Applied, MlsError> {
        let next = self.epoch()? + 1;
        let named = u32::try_from(next)
            .ok()
            .and_then(|epoch| log.commit_of(epoch));
        if named != Some(commit_hash(commit)) {
            return Err(MlsError::CommitNotNamed { epoch: next });
        }
        let snapshot = self.snapshot();
        let applied = self.process(commit);
        let outcome = match applied {
            Ok(()) if !self.is_active() => Ok(Applied::Removed),
            Ok(()) => {
                if next == u64::from(log.epoch()) {
                    self.verify_roster(log).map(|()| Applied::Epoch(next))
                } else {
                    Ok(Applied::Epoch(next))
                }
            }
            Err(error) => Err(error),
        };
        if outcome.is_err() {
            self.restore_snapshot(snapshot);
        }
        outcome
    }

    fn process(&mut self, commit: &[u8]) -> Result<(), MlsError> {
        let message = MlsMessageIn::tls_deserialize_exact(commit).map_err(lib)?;
        let protocol = message.try_into_protocol_message().map_err(lib)?;
        let group = self.group.as_mut().ok_or(MlsError::NoGroup)?;
        let processed = group
            .process_message(&self.provider, protocol)
            .map_err(lib)?;
        match processed.into_content() {
            ProcessedMessageContent::StagedCommitMessage(staged) => group
                .merge_staged_commit(&self.provider, *staged)
                .map_err(lib),
            _ => Err(MlsError::Malformed("it is not a commit")),
        }
    }

    /// Join the group from a Welcome. `commit` is the commit message that made the epoch the
    /// Welcome is for: the log names it, and the group the Welcome made must hold its confirmation
    /// tag, which is what ties the Welcome to the list and not to another group with the same
    /// members. When the epoch is the log's latest the roster must be the log's.
    ///
    /// A Welcome that is refused leaves the device as it was, its key package still unused: the
    /// group a refused Welcome made is not kept.
    pub fn join(
        &mut self,
        welcome: &[u8],
        commit: &[u8],
        log: &impl LogView,
    ) -> Result<u64, MlsError> {
        if self.group.is_some() {
            return Err(MlsError::InGroup);
        }
        let snapshot = self.snapshot();
        let joined = self.try_join(welcome, commit, log);
        if joined.is_err() {
            self.restore_snapshot(snapshot);
        }
        joined
    }

    fn try_join(
        &mut self,
        welcome: &[u8],
        commit: &[u8],
        log: &impl LogView,
    ) -> Result<u64, MlsError> {
        let MlsMessageBodyIn::Welcome(welcome) = MlsMessageIn::tls_deserialize_exact(welcome)
            .map_err(lib)?
            .extract()
        else {
            return Err(MlsError::Malformed("it is not a Welcome"));
        };
        let staged = StagedWelcome::new_from_welcome(&self.provider, &join_config(), welcome, None)
            .map_err(lib)?;
        let group = staged.into_group(&self.provider).map_err(lib)?;
        let epoch = group.epoch().as_u64();
        let named = u32::try_from(epoch)
            .ok()
            .and_then(|epoch| log.commit_of(epoch));
        if named != Some(commit_hash(commit)) {
            return Err(MlsError::CommitNotNamed { epoch });
        }
        let MlsMessageBodyIn::PublicMessage(public) = MlsMessageIn::tls_deserialize_exact(commit)
            .map_err(lib)?
            .extract()
        else {
            return Err(MlsError::Malformed("the commit is not a plaintext message"));
        };
        if public.confirmation_tag() != Some(group.confirmation_tag()) {
            return Err(MlsError::WelcomeNotTheLogs);
        }
        self.group = Some(group);
        if epoch == u64::from(log.epoch()) {
            self.verify_roster(log)?;
        }
        Ok(epoch)
    }

    /// The key of the group's current epoch for sealing the list's content: what the exporter gives
    /// for this list. Every member of the epoch derives the same, and a device removed from the group
    /// derives none of any epoch after its removal.
    pub fn epoch_key(&self) -> Result<EpochKey, MlsError> {
        self.export(EPOCH_LABEL, self.list.as_bytes())
    }

    /// The key of the section `name` of the list in the current epoch, derived from the epoch's
    /// secret with a label of its own: a device that is given this key and not the epoch's opens that
    /// section and no other.
    pub fn section_key(&self, name: &str) -> Result<EpochKey, MlsError> {
        let mut context = self.list.as_bytes().to_vec();
        context.push(0);
        context.extend_from_slice(name.as_bytes());
        self.export(SECTION_LABEL, &context)
    }

    fn export(&self, label: &str, context: &[u8]) -> Result<EpochKey, MlsError> {
        let group = self.group()?;
        if !group.is_active() {
            return Err(MlsError::Removed);
        }
        let key = group
            .export_secret(self.provider.crypto(), label, context, 32)
            .map_err(lib)?;
        key.try_into()
            .map_err(|_| MlsError::Malformed("the exporter gave a key of another length"))
    }

    /// Give `trust` the key of the group's current epoch, so that the host seals and opens the
    /// list's content under it.
    pub fn publish_epoch_key(&self, trust: &Trust) -> Result<u32, MlsError> {
        let epoch = u32::try_from(self.epoch()?)
            .map_err(|_| MlsError::Malformed("the epoch is past what the log counts"))?;
        trust.set_epoch_key(epoch, self.epoch_key()?);
        Ok(epoch)
    }

    fn snapshot(&self) -> std::collections::HashMap<Vec<u8>, Vec<u8>> {
        self.provider
            .storage()
            .values
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// Put the storage back as it was and reload the group from it.
    fn restore_snapshot(&mut self, snapshot: std::collections::HashMap<Vec<u8>, Vec<u8>>) {
        *self
            .provider
            .storage()
            .values
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = snapshot;
        self.group = MlsGroup::load(self.provider.storage(), &self.group_id())
            .ok()
            .flatten();
        self.pending = None;
    }

    /// What this device keeps of the group, for its store: the group, its secrets and the key
    /// packages' private keys. It is secret, and is kept where the host keeps its person's secret.
    pub fn to_bytes(&self) -> Result<Vec<u8>, MlsError> {
        let values = self.snapshot();
        let mut rows: Vec<(&Vec<u8>, &Vec<u8>)> = values.iter().collect();
        rows.sort();
        let mut out = Vec::new();
        out.extend_from_slice(&(rows.len() as u64).to_be_bytes());
        for (key, value) in rows {
            for part in [key, value] {
                out.extend_from_slice(&(part.len() as u64).to_be_bytes());
                out.extend_from_slice(part);
            }
        }
        Ok(out)
    }

    /// The device as [`Self::to_bytes`] kept it.
    pub fn from_bytes(
        list: &str,
        node: u64,
        identity: Identity,
        bytes: &[u8],
    ) -> Result<Self, MlsError> {
        let mut device = Self::new(list, node, identity);
        device.restore_snapshot(read_rows(bytes)?);
        device.group =
            MlsGroup::load(device.provider.storage(), &device.group_id()).map_err(lib)?;
        Ok(device)
    }
}

/// The rows [`Device::to_bytes`] wrote, or `Malformed` when the bytes are not that.
fn read_rows(bytes: &[u8]) -> Result<std::collections::HashMap<Vec<u8>, Vec<u8>>, MlsError> {
    const MALFORMED: MlsError = MlsError::Malformed("the kept state is not what a device writes");
    fn take<'a>(rest: &mut &'a [u8], length: usize) -> Result<&'a [u8], MlsError> {
        if rest.len() < length {
            return Err(MALFORMED);
        }
        let (head, tail) = rest.split_at(length);
        *rest = tail;
        Ok(head)
    }
    fn number(rest: &mut &[u8]) -> Result<usize, MlsError> {
        let bytes: [u8; 8] = take(rest, 8)?.try_into().map_err(|_| MALFORMED)?;
        usize::try_from(u64::from_be_bytes(bytes)).map_err(|_| MALFORMED)
    }
    let mut rest = bytes;
    let count = number(&mut rest)?;
    let mut rows = std::collections::HashMap::new();
    for _ in 0..count {
        let key_length = number(&mut rest)?;
        let key = take(&mut rest, key_length)?.to_vec();
        let value_length = number(&mut rest)?;
        let value = take(&mut rest, value_length)?.to_vec();
        if rows.insert(key, value).is_some() {
            return Err(MALFORMED);
        }
    }
    if rest.is_empty() {
        Ok(rows)
    } else {
        Err(MALFORMED)
    }
}
