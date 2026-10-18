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

use std::collections::{BTreeMap, BTreeSet};

use sce_rust_relay::{Courier, Link, Record, Round};
use sce_rust_trust::encoding;
use sce_rust_trust::membership::{Body, Entry};
use sce_rust_trust::{Identity, PublicKey, Trust};
use sha2::{Digest, Sha256};

use crate::{Applied, Device, MlsError};

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
        self.absorb(&round.messages);
        let mut followed = Followed {
            steps: self.advance(),
            reread: None,
        };
        if self.commits_to_fetch().is_empty() {
            return followed;
        }
        if let Ok(again) = courier.reread() {
            self.absorb(&again.messages);
            followed.steps.extend(self.advance());
            followed.reread = Some(again);
        }
        followed
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
    /// `role`, from its key package. Writes the grant (when the log has none for it), makes the
    /// commit, writes the entry that names it, and merges. What it returns is for the relay.
    pub fn add_member(
        &mut self,
        administrator: &Identity,
        node: u64,
        role: u8,
        key: PublicKey,
        package: &[u8],
    ) -> Result<Issued, MlsError> {
        let mut entries = Vec::new();
        if !self
            .trust
            .current_members()
            .iter()
            .any(|(held, held_key)| *held == node && *held_key == key)
        {
            entries.push(self.append(
                administrator,
                Body::Grant {
                    node,
                    role,
                    key,
                    final_seq: None,
                },
            )?);
        }
        let pending = self.device.stage_add(&[package.to_vec()], &self.trust)?;
        self.finish(administrator, entries, pending)
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
        let entries = vec![self.append(
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
        let pending = self.device.stage_add(&[package.to_vec()], &self.trust)?;
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
    /// operation this host has seen of it, and move the group to an epoch without it.
    pub fn remove_member(
        &mut self,
        administrator: &Identity,
        node: u64,
    ) -> Result<Issued, MlsError> {
        let key = self
            .trust
            .current_members()
            .into_iter()
            .find(|(held, _)| *held == node)
            .map(|(_, key)| key)
            .ok_or_else(|| MlsError::NotInLog(format!("node {node} is no member")))?;
        let entries = vec![self.append(
            administrator,
            Body::Grant {
                node,
                role: 0,
                key,
                final_seq: Some(self.trust.last_seq(node)),
            },
        )?];
        let pending = self.device.stage_remove(&[node], &self.trust)?;
        self.finish(administrator, entries, pending)
    }

    fn append(&self, administrator: &Identity, body: Body) -> Result<Entry, MlsError> {
        let entry = self.trust.draft(body).sign(administrator).finish();
        self.trust
            .append(&entry)
            .map_err(|error| MlsError::NotInLog(format!("the log refused the entry: {error:?}")))?;
        Ok(entry)
    }

    fn finish(
        &mut self,
        administrator: &Identity,
        mut entries: Vec<Entry>,
        pending: crate::Pending,
    ) -> Result<Issued, MlsError> {
        let epoch = u32::try_from(pending.epoch)
            .map_err(|_| MlsError::Malformed("the epoch is past what the log counts"))?;
        entries.push(self.append(
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
