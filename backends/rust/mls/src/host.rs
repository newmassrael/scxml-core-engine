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

use sce_rust_relay::Record;
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

/// A device and its log.
pub struct Member {
    device: Device,
    trust: Trust,
    node: u64,
    commits: BTreeMap<u32, Vec<u8>>,
    welcomes: BTreeMap<u32, Vec<Vec<u8>>>,
    packages: BTreeMap<u64, Vec<Vec<u8>>>,
    /// The Welcomes and commits that were refused, by their hash, so that they are not tried again.
    refused: BTreeSet<[u8; 32]>,
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
                        self.commits.entry(*epoch).or_insert(bytes);
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
            let Some(commit) = self.commits.get(&epoch).cloned() else {
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
        let Some(commit) = self.commits.get(&next).cloned() else {
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
