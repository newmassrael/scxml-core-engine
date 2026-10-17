// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The record of a list that someone who is not in it can check.
//!
//! An auditor (a regulator, a court, a member's lawyer, the list's own people after an incident) is
//! given one text: the membership log, every operation the exporter holds as it travelled, and the
//! checkpoints. They need to be told one thing in a channel they trust, the hash of the genesis entry
//! (what a person is invited with), and they hold no key: the content of every write is sealed and
//! stays so. What they can then check is everything the members' hosts check, by the same code: that
//! each entry of the log is signed by the administrators it needs and names the one before, that each
//! operation was made by a device that was a writer under the entry it names, is numbered within what
//! the log accepted of the device, and is signed by its key, and that the checkpoints are genuine.
//!
//! [`verify`] returns what the record says: who the administrators and members were and from which
//! entry, which devices are one person's, how long each device's chain is and whether it is whole, which
//! epochs were destroyed, and what in the text does not hold. A record that has been changed fails
//! where it was changed: a changed entry stops the log at that entry, a changed operation or checkpoint
//! is listed as refused and counts for nothing.

use serde_json::{json, Value};

use crate::checkpoint::Checkpoint;
use crate::identity::PublicKey;
use crate::membership::{Entry, Era, Hash, LogError, Profile};
use crate::trust::{Divergence, Trust};

/// Why a record could not be checked at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditError {
    /// It is not what [`export`] writes.
    Malformed(&'static str),
    /// Its first entry is not the genesis the auditor was told.
    NotTheGenesis,
    /// An entry of the log does not hold, at this position, and nothing after it is read.
    Log { index: usize, error: LogError },
}

/// What a grant of the log said of a device, and from which entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EraReport {
    pub entry: u64,
    pub role: u8,
    pub key: PublicKey,
    /// The last operation accepted of the device before this grant took writing away or changed its
    /// key, when it did.
    pub cutoff: Option<u64>,
}

impl From<&Era> for EraReport {
    fn from(era: &Era) -> Self {
        Self {
            entry: era.index,
            role: era.role,
            key: era.key,
            cutoff: era.cutoff,
        }
    }
}

/// A device the log holds, and the person it is one of.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberReport {
    pub node: u64,
    /// The node of the person's first device: the same for the devices that are one person's.
    pub person: u64,
    pub eras: Vec<EraReport>,
}

/// What the record holds of one device's chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainReport {
    pub node: u64,
    pub operations: usize,
    /// The highest number held with every number before it.
    pub whole_to: u64,
    /// The highest number held.
    pub last: u64,
}

/// What a record says, as far as it holds.
#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    pub list: String,
    pub entries: usize,
    pub genesis: Hash,
    pub administrators: Vec<PublicKey>,
    pub profile: Profile,
    pub epoch: u32,
    pub mls: bool,
    pub shredded: Vec<u32>,
    pub members: Vec<MemberReport>,
    pub chains: Vec<ChainReport>,
    pub checkpoints: usize,
    /// Operations and checkpoints in the text that do not hold, and why: they count for nothing.
    pub refused: Vec<String>,
    /// Devices that signed two operations at one number, and the number.
    pub equivocating: Vec<(u64, u64)>,
    /// A device a checkpoint shows has operations the record lacks, or another than it holds.
    pub divergences: Vec<Divergence>,
}

/// The record `trust` holds, as one text: its log, its operations and its checkpoints.
pub fn export(trust: &Trust) -> String {
    json!({
        "v": 1,
        "list": trust.entries().first().map(|entry| entry.list.clone()).unwrap_or_default(),
        "entries": trust.entries().iter().map(Entry::to_json).collect::<Vec<_>>(),
        "operations": trust.all_operations().iter().map(|(event, data)| json!([event, data])).collect::<Vec<_>>(),
        "checkpoints": trust.checkpoints().iter().map(Checkpoint::to_json).collect::<Vec<_>>(),
    })
    .to_string()
}

/// Check a record against the genesis the auditor was told, with no key.
pub fn verify(genesis: Hash, text: &str) -> Result<Report, AuditError> {
    let value: Value =
        serde_json::from_str(text).map_err(|_| AuditError::Malformed("it is not JSON"))?;
    if value.get("v").and_then(Value::as_u64) != Some(1) {
        return Err(AuditError::Malformed("it is not version 1 of the record"));
    }
    let list = value
        .get("list")
        .and_then(Value::as_str)
        .ok_or(AuditError::Malformed("it names no list"))?;
    let rows = |name: &'static str| -> Result<&Vec<Value>, AuditError> {
        value
            .get(name)
            .and_then(Value::as_array)
            .ok_or(AuditError::Malformed("a part of the record is missing"))
    };
    let entries: Vec<Entry> = rows("entries")?
        .iter()
        .map(|row| row.as_str().and_then(Entry::from_json))
        .collect::<Option<_>>()
        .ok_or(AuditError::Malformed("an entry is not one"))?;
    let first = entries
        .first()
        .ok_or(AuditError::Malformed("the log has no entry"))?;
    if first.hash() != genesis {
        return Err(AuditError::NotTheGenesis);
    }
    let trust = Trust::invited(list, genesis);
    for (index, entry) in entries.iter().enumerate() {
        trust
            .append(entry)
            .map_err(|error| AuditError::Log { index, error })?;
    }

    let mut refused = Vec::new();
    for row in rows("operations")? {
        let (Some(event), Some(data)) = (
            row.get(0).and_then(Value::as_str),
            row.get(1).and_then(Value::as_str),
        ) else {
            refused.push("an operation is not an event and its data".to_string());
            continue;
        };
        if let Err(error) = trust.take_operation(event, data) {
            refused.push(format!("{event}: {error:?}"));
        }
    }
    let mut checkpoints = 0;
    for row in rows("checkpoints")? {
        match row.as_str().and_then(Checkpoint::from_json) {
            Some(checkpoint) => match trust.take_checkpoint(&checkpoint) {
                Ok(_) => checkpoints += 1,
                Err(error) => refused.push(format!("a checkpoint: {error:?}")),
            },
            None => refused.push("a checkpoint is not one".to_string()),
        }
    }

    let whole: std::collections::BTreeMap<u64, u64> = trust
        .heads()
        .into_iter()
        .map(|head| (head.node, head.seq))
        .collect();
    Ok(Report {
        list: list.to_string(),
        entries: entries.len(),
        genesis,
        administrators: trust.administrators(),
        profile: trust.profile(),
        epoch: trust.epoch(),
        mls: trust.is_mls(),
        shredded: trust.shredded(),
        members: trust
            .member_history()
            .into_iter()
            .map(|(node, eras)| MemberReport {
                node,
                person: trust.person_of(node),
                eras: eras.iter().map(EraReport::from).collect(),
            })
            .collect(),
        chains: trust
            .chains()
            .into_iter()
            .map(|(node, operations, last)| ChainReport {
                node,
                operations,
                whole_to: whole.get(&node).copied().unwrap_or(0),
                last,
            })
            .collect(),
        checkpoints,
        refused,
        equivocating: trust.equivocating(),
        divergences: trust.divergences(),
    })
}
