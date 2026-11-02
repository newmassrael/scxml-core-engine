// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What a host does with the MLS messages a relay carries.
//!
//! A [`Member`] holds a device's MLS state ([`Device`]) and its [`Trust`] (the log). What a courier
//! reads from a relay (key packages, commits, Welcomes) goes in with [`Member::absorb`], and
//! [`Member::advance`] does what the log allows: a device that is in no group yet joins from a Welcome
//! addressed to it, and a device in the group applies the commit the log names for the next epoch,
//! each in order, giving its [`Trust`] the key of each epoch it reaches. A commit the log does not
//! name yet waits; one the log names and that does not apply is refused and dropped.
//!
//! The administrator's device also makes the changes: [`Member::add_member`] and
//! [`Member::remove_member`] write the entries of the log (the grant, and the entry that names the
//! commit), make and merge the commit, and give back what is to go to the relay.
//!
//! The commit of a change is made before any entry of it is written. The log is append-only and an
//! entry signed and appended cannot be taken back, while a commit made and not merged can: so a change
//! the group refuses (a key package that is not the device's or not the list's, a node that is in no
//! group) leaves the log as it was, and does not leave a member of the log the group does not hold,
//! which would make the group's roster differ from the log's at every later commit.

use std::collections::{BTreeMap, BTreeSet};

use sce_rust_relay::{Courier, Link, Record, Round};
use sce_rust_trust::encoding;
use sce_rust_trust::membership::{Body, Entry, Hash};
use sce_rust_trust::{Identity, PublicKey, Trust};
use sha2::{Digest, Sha256};

use crate::{Applied, Device, LogView, MlsError};

/// What [`Member::advance`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// The device joined the group, at this epoch, from a Welcome.
    Joined(u64),
    /// The device applied the commit that began this epoch.
    Applied(u64),
    /// The commit removed the device: it holds no key of any later epoch.
    Removed,
    /// A Welcome or commit was refused, and why; it is not tried again.
    Refused(String),
}

/// What an administrator's change leaves to be sent: the entries of the log to give the relay (the
/// grant, when there is one, and the entry that names the commit), the commit, and the Welcome for the
/// member added.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issued {
    pub entries: Vec<Entry>,
    pub epoch: u32,
    pub commit: Vec<u8>,
    pub welcome: Option<Vec<u8>>,
}

/// The most commits a host keeps waiting for an epoch the log has not yet named a commit for. A relay
/// that is not trusted can give as many as it likes, and the host does not keep them all: once the log
/// names a commit the true one is kept whatever the number given before, but one given while the
/// host waited and over the limit was not kept. The host does not lose it for good: a log that names a
/// commit the host does not hold, for an epoch where it dropped one at the limit, is told by
/// [`Member::commits_to_fetch`], and [`Member::follow`] reads the relay again (`Courier::reread`),
/// which keeps the commit the log names.
const MAX_CANDIDATES: usize = 64;

/// What [`Member::follow`] did with a round.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Followed {
    /// What the device did, in order, over the round and over the second read when there was one.
    pub steps: Vec<Step>,
    /// The second read of the relay, when the host had to make one for a commit it dropped. Its
    /// events (operations the host did not hold) are for the machine, as a round's are.
    pub reread: Option<Round>,
}

/// The log as it will be once the entry of a change is written: the device of `node` a member that
/// signs with `key`, or taken off when `key` is `None`. What an administrator's commit is made
/// against before that entry is written (the module's note).
struct Prospect<'a> {
    trust: &'a Trust,
    node: u64,
    key: Option<[u8; 32]>,
}

impl LogView for Prospect<'_> {
    fn members(&self) -> Vec<(u64, [u8; 32])> {
        let mut members: Vec<(u64, [u8; 32])> = LogView::members(self.trust)
            .into_iter()
            .filter(|(held, _)| *held != self.node)
            .collect();
        members.extend(self.key.map(|key| (self.node, key)));
        members
    }

    fn commit_of(&self, epoch: u32) -> Option<Hash> {
        self.trust.commit_of(epoch)
    }

    fn epoch(&self) -> u32 {
        self.trust.epoch()
    }
}

/// A device and its log.
pub struct Member {
    device: Device,
    trust: Trust,
    node: u64,
    /// The commits a relay gave for each epoch, by their hash. A relay can give any number of them
    /// and none of them is believed for being first: the one the log names is the one used, and
    /// until the log names one they wait (a bounded number, [`MAX_CANDIDATES`]).
    commits: BTreeMap<u32, BTreeMap<[u8; 32], Vec<u8>>>,
    welcomes: BTreeMap<u32, Vec<Vec<u8>>>,
    packages: BTreeMap<u64, Vec<Vec<u8>>>,
    /// The Welcomes and commits that were refused, by their hash, so that they are not tried again.
    refused: BTreeSet<[u8; 32]>,
    /// The epochs for which a commit was given and not kept, for the limit
    /// ([`MAX_CANDIDATES`]) was reached before the log named one.
    overflowed: BTreeSet<u32>,
}

impl Member {
    pub fn new(device: Device, trust: Trust, node: u64) -> Self {
        Self {
            device,
            trust,
            node,
            commits: BTreeMap::new(),
            welcomes: BTreeMap::new(),
            packages: BTreeMap::new(),
            refused: BTreeSet::new(),
            overflowed: BTreeSet::new(),
        }
    }

    pub fn device(&self) -> &Device {
        &self.device
    }

    pub fn device_mut(&mut self) -> &mut Device {
        &mut self.device
    }

    pub fn trust(&self) -> &Trust {
        &self.trust
    }

    /// The key packages the relay holds of the device of `node`, the newest last.
    pub fn key_packages_of(&self, node: u64) -> &[Vec<u8>] {
        self.packages.get(&node).map_or(&[], Vec::as_slice)
    }

    /// Take in what a courier read: the group's messages, kept until the log lets them be used. A
    /// Welcome for another device is not kept.
    pub fn absorb(&mut self, records: &[Record]) {
        for record in records {
            match record {
                Record::KeyPackage { node, data } => {
                    if let Some(bytes) = encoding::decode(data) {
                        let held = self.packages.entry(*node).or_default();
                        if !held.contains(&bytes) {
                            held.push(bytes);
                        }
                    }
                }
                Record::Commit { epoch, data } => {
                    if let Some(bytes) = encoding::decode(data) {
                        self.keep_commit(*epoch, bytes);
                    }
                }
                Record::Welcome { epoch, node, data } if *node == self.node => {
                    if let Some(bytes) = encoding::decode(data) {
                        let held = self.welcomes.entry(*epoch).or_default();
                        if !held.contains(&bytes) {
                            held.push(bytes);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Keep a commit a relay gave for `epoch`, unless the log names another for it, or the host
    /// already waits on as many as it will for an epoch the log has not named a commit for. What the
    /// log names is kept whatever else was kept before it, and the rest of the epoch's are dropped.
    fn keep_commit(&mut self, epoch: u32, bytes: Vec<u8>) {
        let hash: [u8; 32] = Sha256::digest(&bytes).into();
        let held = self.commits.entry(epoch).or_default();
        match self.trust.commit_of(epoch) {
            Some(named) if named != hash => {}
            Some(_) => {
                held.retain(|kept, _| *kept == hash);
                held.insert(hash, bytes);
            }
            None if held.len() < MAX_CANDIDATES => {
                held.insert(hash, bytes);
            }
            None => {
                self.overflowed.insert(epoch);
            }
        }
    }

    /// The epochs whose commit the log names and this host does not hold, among the ones it would use
    /// next (the epoch after its own, or, for a device in no group yet, the epoch of each Welcome it
    /// holds) and for which it dropped a commit at the limit ([`MAX_CANDIDATES`]) while the log named
    /// none. A relay's feed does not give a record twice, so [`Self::advance`] has nothing to do for
    /// these epochs and would stay quiet; [`Self::follow`] reads the relay again from the start, and the
    /// log names the commit by then, so the true one is kept. A commit the relay has not given yet (the
    /// log can name it before the relay holds it) is not in this list: that is waiting, and the next
    /// round brings it. Empty when nothing was lost.
    pub fn commits_to_fetch(&self) -> Vec<u32> {
        let wanted: Vec<u32> = match self.device.epoch() {
            Err(_) => self.welcomes.keys().copied().collect(),
            Ok(epoch) => u32::try_from(epoch + 1).into_iter().collect(),
        };
        wanted
            .into_iter()
            .filter(|epoch| {
                self.overflowed.contains(epoch)
                    && self
                        .trust
                        .commit_of(*epoch)
                        .is_some_and(|named| !self.has_commit(*epoch, &named))
            })
            .collect()
    }

    /// Take in what a courier's round read and do what the log allows with it. When that leaves the
    /// host without a commit it dropped at the limit ([`Self::commits_to_fetch`]), read the relay again
    /// from the start once and do what the log allows with that too, so that a relay that gave false
    /// commits before the log named the true one cannot keep a device at its epoch. A failure of the
    /// second read is left to the next round: the commit is still named and still lacking, and the
    /// host asks again.
    pub fn follow<L: Link>(&mut self, courier: &mut Courier<L>, round: &Round) -> Followed {
        let mut followed = Followed {
            steps: self.take_round(round),
            reread: None,
        };
        if !self.owes_reread() {
            return followed;
        }
        if let Ok(again) = courier.reread() {
            followed.steps.extend(self.take_round(&again));
            followed.reread = Some(again);
        }
        followed
    }

    /// Take in what a round read and do what the log allows with it: the half of [`Self::follow`]
    /// that needs no relay, for a host that drives its rounds itself.
    pub fn take_round(&mut self, round: &Round) -> Vec<Step> {
        self.absorb(&round.messages);
        self.advance()
    }

    /// Whether the host owes the relay a second read from the start, once, for a commit the log
    /// names that it dropped at the limit ([`Self::commits_to_fetch`]): the other half of
    /// [`Self::follow`].
    pub fn owes_reread(&self) -> bool {
        !self.commits_to_fetch().is_empty()
    }

    fn has_commit(&self, epoch: u32, hash: &[u8; 32]) -> bool {
        self.commits
            .get(&epoch)
            .is_some_and(|held| held.contains_key(hash))
    }

    /// The commit of `epoch` the log names, when this host has been given it.
    fn named_commit(&self, epoch: u32) -> Option<Vec<u8>> {
        let named = self.trust.commit_of(epoch)?;
        self.commits.get(&epoch)?.get(&named).cloned()
    }

    /// Do what the log lets this device do with what it holds, in order, and say what was done.
    pub fn advance(&mut self) -> Vec<Step> {
        let mut steps = Vec::new();
        loop {
            let progressed = if self.device.epoch().is_err() {
                self.try_join(&mut steps)
            } else if self.device.is_active() {
                self.try_apply(&mut steps)
            } else {
                false
            };
            if !progressed {
                return steps;
            }
        }
    }

    fn try_join(&mut self, steps: &mut Vec<Step>) -> bool {
        let epochs: Vec<u32> = self.welcomes.keys().copied().collect();
        for epoch in epochs {
            let Some(commit) = self.named_commit(epoch) else {
                continue;
            };
            for welcome in self.welcomes.get(&epoch).cloned().unwrap_or_default() {
                let id: [u8; 32] = Sha256::digest(&welcome).into();
                if self.refused.contains(&id) {
                    continue;
                }
                match self.device.join(&welcome, &commit, &self.trust) {
                    Ok(joined) => {
                        let _ = self.device.publish_epoch_key(&self.trust);
                        steps.push(Step::Joined(joined));
                        self.welcomes.remove(&epoch);
                        return true;
                    }
                    // The log does not name the commit yet: it may be given to this host later.
                    Err(MlsError::CommitNotNamed { .. }) => {}
                    Err(error) => {
                        self.refused.insert(id);
                        steps.push(Step::Refused(format!(
                            "a Welcome for epoch {epoch}: {error:?}"
                        )));
                    }
                }
            }
        }
        false
    }

    fn try_apply(&mut self, steps: &mut Vec<Step>) -> bool {
        let Ok(next) = self.device.epoch().map(|epoch| epoch + 1) else {
            return false;
        };
        let Ok(next) = u32::try_from(next) else {
            return false;
        };
        let Some(commit) = self.named_commit(next) else {
            return false;
        };
        let id: [u8; 32] = Sha256::digest(&commit).into();
        if self.refused.contains(&id) {
            return false;
        }
        match self.device.apply_commit(&commit, &self.trust) {
            Ok(Applied::Epoch(epoch)) => {
                let _ = self.device.publish_epoch_key(&self.trust);
                steps.push(Step::Applied(epoch));
                true
            }
            Ok(Applied::Removed) => {
                steps.push(Step::Removed);
                false
            }
            // The log has not named the commit yet: wait for the entry.
            Err(MlsError::CommitNotNamed { .. }) => false,
            Err(error) => {
                self.refused.insert(id);
                steps.push(Step::Refused(format!(
                    "the commit of epoch {next}: {error:?}"
                )));
                false
            }
        }
    }

    /// The administrator's change: add the device of `node` (which signs with `key`) with the role
    /// `role`, from its key package. Makes the commit, writes the grant (when the log has none for
    /// it with this key), writes the entry that names the commit, and merges. What it returns is for
    /// the relay. A key package the group refuses leaves the log and the group as they were.
    pub fn add_member(
        &mut self,
        administrator: &Identity,
        node: u64,
        role: u8,
        key: PublicKey,
        package: &[u8],
    ) -> Result<Issued, MlsError> {
        let granted = self
            .trust
            .current_members()
            .iter()
            .any(|(held, held_key)| *held == node && *held_key == key);
        let pending = self.stage_add(node, key, package)?;
        let mut entries = Vec::new();
        if !granted {
            let grant = self.trust.grant_body(node, role, key);
            entries.push(self.append_staged(administrator, grant)?);
        }
        self.finish(administrator, entries, pending)
    }

    /// The commit that adds the device of `package`, made against the log as it will be once the
    /// device of `node` is a member that signs with `key` (the module's note).
    fn stage_add(
        &mut self,
        node: u64,
        key: PublicKey,
        package: &[u8],
    ) -> Result<crate::Pending, MlsError> {
        let prospect = Prospect {
            trust: &self.trust,
            node,
            key: Some(key.to_bytes()),
        };
        self.device.stage_add(&[package.to_vec()], &prospect)
    }

    /// The administrator's change: add a new device of a person, **linked** by the device of
    /// `parent`, which signed for it (`parent_signature`) as the new key did too (`signature`), both
    /// over `sce_rust_trust::membership::link_bytes`. As [`Self::add_member`], with the entry of the log
    /// that records who vouched.
    #[allow(clippy::too_many_arguments)]
    pub fn add_linked_member(
        &mut self,
        administrator: &Identity,
        node: u64,
        role: u8,
        key: PublicKey,
        parent: u64,
        parent_signature: [u8; 64],
        signature: [u8; 64],
        package: &[u8],
    ) -> Result<Issued, MlsError> {
        let pending = self.stage_add(node, key, package)?;
        let entries = vec![self.append_staged(
            administrator,
            Body::LinkedGrant {
                node,
                role,
                key,
                parent,
                parent_signature,
                signature,
            },
        )?];
        self.finish(administrator, entries, pending)
    }

    /// The administrator's change: take every device of the person whose device `node` is off, one
    /// removal each, since the log does not take a person's other devices off with one of them.
    /// What it returns is one [`Issued`] for each device that was a member.
    pub fn remove_person(
        &mut self,
        administrator: &Identity,
        node: u64,
    ) -> Result<Vec<Issued>, MlsError> {
        let mut issued = Vec::new();
        for device in self.trust.devices_of(node) {
            if self.trust.role(device) > 0 {
                issued.push(self.remove_member(administrator, device)?);
            }
        }
        Ok(issued)
    }

    /// The keys of `epochs` that this host holds, each given to the device whose key is `to` (a new
    /// device of the same person, or a new member the administrator means to give the history of the
    /// list to): a grant for that device's host to take in with `Trust::receive_grant`. A new member is
    /// given no history unless one of these is sent; an epoch this host holds no key of is left out.
    pub fn history_grants(&self, to: &PublicKey, epochs: &[u32]) -> Vec<(u32, String)> {
        epochs
            .iter()
            .filter_map(|epoch| Some((*epoch, self.trust.grant_key(*epoch, to)?)))
            .collect()
    }

    /// The administrator's change: raise the last operation accepted of the device of `node`, which
    /// was taken off, to `final_seq`, on the checkpoints this host holds that were made before the
    /// removal and name that much of its chain. The entry is for the relay and for every host.
    pub fn amend_removed(
        &mut self,
        administrator: &Identity,
        node: u64,
        final_seq: u64,
    ) -> Result<Entry, MlsError> {
        let evidence: Vec<_> = self
            .trust
            .checkpoints()
            .into_iter()
            .filter(|checkpoint| {
                checkpoint
                    .heads
                    .iter()
                    .any(|head| head.node == node && head.seq >= final_seq)
            })
            .collect();
        self.append(
            administrator,
            Body::Amend {
                node,
                final_seq,
                evidence,
            },
        )
    }

    /// The administrator's change: take the device of `node` off, ending its chain at the last
    /// operation this host has seen of it, and move the group to an epoch without it. The commit
    /// is made before the entry that takes the device off is written (the module's note).
    pub fn remove_member(
        &mut self,
        administrator: &Identity,
        node: u64,
    ) -> Result<Issued, MlsError> {
        let removal = self
            .trust
            .removal_body(node)
            .ok_or_else(|| MlsError::NotInLog(format!("node {node} is no member")))?;
        let prospect = Prospect {
            trust: &self.trust,
            node,
            key: None,
        };
        let pending = self.device.stage_remove(&[node], &prospect)?;
        let entries = vec![self.append_staged(administrator, removal)?];
        self.finish(administrator, entries, pending)
    }

    fn append(&self, administrator: &Identity, body: Body) -> Result<Entry, MlsError> {
        self.trust
            .administer(administrator, body)
            .map_err(|error| MlsError::NotInLog(format!("the log refused the entry: {error:?}")))
    }

    /// An entry of a change whose commit is made and not merged: when the log refuses it, the
    /// commit is given up, so that the device does not wait on a commit no entry will name.
    fn append_staged(&mut self, administrator: &Identity, body: Body) -> Result<Entry, MlsError> {
        let appended = self.append(administrator, body);
        if appended.is_err() {
            self.device.clear_pending()?;
        }
        appended
    }

    fn finish(
        &mut self,
        administrator: &Identity,
        mut entries: Vec<Entry>,
        pending: crate::Pending,
    ) -> Result<Issued, MlsError> {
        let epoch = u32::try_from(pending.epoch)
            .map_err(|_| MlsError::Malformed("the epoch is past what the log counts"))?;
        entries.push(self.append_staged(
            administrator,
            Body::MlsEpoch {
                epoch,
                commit: pending.commit_hash,
            },
        )?);
        self.device.merge_pending(&self.trust)?;
        self.device.publish_epoch_key(&self.trust)?;
        Ok(Issued {
            entries,
            epoch,
            commit: pending.commit,
            welcome: pending.welcome,
        })
    }
}
