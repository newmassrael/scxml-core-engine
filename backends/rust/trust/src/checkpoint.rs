// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What a member's device says it holds, and what the members say together.
//!
//! A **checkpoint** is one device's signed statement: under which entry of the membership log it
//! speaks, the instant `before` and the digest its copy held of everything up to it, and the head
//! of every device's chain it has admitted in full (`heads`). Nothing in it is a claim about time
//! that is believed: `before` and `digest` are what the machines of the settle protocol compute,
//! and the heads are numbers and ids of operations the other hosts can compare with their own.
//!
//! A **settlement** is the checkpoints of every device that was a writer at the entry they name,
//! agreeing on `before` and `digest`. A host that holds the log checks it alone: an administrator
//! is not asked for anything, and a hub that holds none of the devices' keys cannot make one. The
//! tombstones up to `before` are forgotten on it.
//!
//! The heads also make a relay that **withholds** visible: a member whose checkpoint names the
//! operation 7 of a device when a host holds that device's chain only up to 5 has shown that 6 and
//! 7 exist, and the host asks for them from any other host that holds them.

use serde_json::{json, Value};

use crate::encoding;
use crate::membership::Hash;
use crate::message;

/// The last operation of one device's chain that a host holds with every number before it: its
/// number and its id (`message::operation_id`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Head {
    pub node: u64,
    pub seq: u64,
    pub id: Hash,
}

/// One device's signed statement of what its copy holds up to `before`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checkpoint {
    /// The list it is about: a checkpoint does not move to another list.
    pub list: String,
    /// The node of the device that made it.
    pub device: u64,
    /// The entry of the membership log it speaks under, and that entry's hash.
    pub log: u64,
    pub log_hash: Hash,
    /// The instant up to which the copy is settled, and the digest it held of everything up to it
    /// (`todo.settle` and `todo.floor` of the machine).
    pub before: i64,
    pub digest: u32,
    /// The head of each device's chain the host holds, lowest node first, one per node.
    pub heads: Vec<Head>,
    pub signature: [u8; 64],
}

impl Checkpoint {
    /// The bytes the device signs: every field but the signature, each written with its length,
    /// the heads in the order of their nodes.
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        message::part(&mut bytes, CHECKPOINT_DOMAIN);
        message::part(&mut bytes, &self.list);
        message::part(&mut bytes, &self.device.to_string());
        message::part(&mut bytes, &self.log.to_string());
        message::part(&mut bytes, &encoding::encode(&self.log_hash));
        message::part(&mut bytes, &self.before.to_string());
        message::part(&mut bytes, &self.digest.to_string());
        message::part(&mut bytes, &self.heads.len().to_string());
        for head in &self.heads {
            message::part(&mut bytes, &head.node.to_string());
            message::part(&mut bytes, &head.seq.to_string());
            message::part(&mut bytes, &encoding::encode(&head.id));
        }
        bytes
    }

    /// Whether the heads are in the one form a checkpoint is signed in: lowest node first, one
    /// per node, each from 1. A checkpoint that is not is refused, so that one statement has one
    /// byte form.
    pub fn is_canonical(&self) -> bool {
        self.heads.iter().all(|head| head.seq >= 1)
            && self
                .heads
                .windows(2)
                .all(|pair| pair[0].node < pair[1].node)
    }

    pub fn to_value(&self) -> Value {
        json!({
            "list": self.list,
            "device": self.device.to_string(),
            "mh": self.log.to_string(),
            "mhh": encoding::encode(&self.log_hash),
            "before": self.before.to_string(),
            "digest": self.digest.to_string(),
            "heads": self.heads.iter()
                .map(|head| json!([head.node.to_string(), head.seq.to_string(), encoding::encode(&head.id)]))
                .collect::<Vec<_>>(),
            "sig": encoding::encode(&self.signature),
        })
    }

    /// The checkpoint as the JSON a host sends to another.
    pub fn to_json(&self) -> String {
        self.to_value().to_string()
    }

    pub fn from_value(value: &Value) -> Option<Self> {
        let text = |name: &str| value.get(name)?.as_str();
        let hash = |text: &str| -> Option<Hash> { encoding::decode(text)?.try_into().ok() };
        let heads = value
            .get("heads")?
            .as_array()?
            .iter()
            .map(|row| {
                Some(Head {
                    node: row.get(0)?.as_str()?.parse().ok()?,
                    seq: row.get(1)?.as_str()?.parse().ok()?,
                    id: hash(row.get(2)?.as_str()?)?,
                })
            })
            .collect::<Option<Vec<_>>>()?;
        let checkpoint = Self {
            list: text("list")?.to_string(),
            device: text("device")?.parse().ok()?,
            log: text("mh")?.parse().ok()?,
            log_hash: hash(text("mhh")?)?,
            before: text("before")?.parse().ok()?,
            digest: text("digest")?.parse().ok()?,
            heads,
            signature: encoding::decode(text("sig")?)?.try_into().ok()?,
        };
        checkpoint.is_canonical().then_some(checkpoint)
    }

    pub fn from_json(text: &str) -> Option<Self> {
        Self::from_value(&serde_json::from_str(text).ok()?)
    }
}

const CHECKPOINT_DOMAIN: &str = "sce-trust/checkpoint/1";

/// The checkpoints that settle an instant: one from each device that was a writer at the entry of
/// the log they name, every one of them for the same `before` and `digest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settlement {
    pub checkpoints: Vec<Checkpoint>,
}

impl Settlement {
    pub fn to_value(&self) -> Value {
        Value::Array(self.checkpoints.iter().map(Checkpoint::to_value).collect())
    }

    pub fn from_value(value: &Value) -> Option<Self> {
        let checkpoints = value
            .as_array()?
            .iter()
            .map(Checkpoint::from_value)
            .collect::<Option<Vec<_>>>()?;
        Some(Self { checkpoints })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checkpoint() -> Checkpoint {
        Checkpoint {
            list: "home".to_string(),
            device: 2,
            log: 3,
            log_hash: [4; 32],
            before: 30_000,
            digest: 7,
            heads: vec![
                Head {
                    node: 1,
                    seq: 5,
                    id: [6; 32],
                },
                Head {
                    node: 2,
                    seq: 2,
                    id: [8; 32],
                },
            ],
            signature: [9; 64],
        }
    }

    #[test]
    fn a_checkpoint_reads_back_as_it_was_written() {
        let original = checkpoint();
        assert_eq!(Checkpoint::from_json(&original.to_json()), Some(original));
    }

    #[test]
    fn what_is_signed_covers_every_field() {
        let base = checkpoint().signing_bytes();
        let mut changes: Vec<Checkpoint> = Vec::new();
        for edit in [
            |c: &mut Checkpoint| c.list = "work".to_string(),
            |c: &mut Checkpoint| c.device = 3,
            |c: &mut Checkpoint| c.log = 4,
            |c: &mut Checkpoint| c.log_hash = [5; 32],
            |c: &mut Checkpoint| c.before = 30_001,
            |c: &mut Checkpoint| c.digest = 8,
            |c: &mut Checkpoint| c.heads[0].seq = 6,
            |c: &mut Checkpoint| c.heads[1].id = [1; 32],
            |c: &mut Checkpoint| {
                c.heads.pop();
            },
        ] {
            let mut moved = checkpoint();
            edit(&mut moved);
            changes.push(moved);
        }
        for moved in changes {
            assert_ne!(moved.signing_bytes(), base, "{moved:?}");
        }
    }

    #[test]
    fn heads_out_of_order_or_repeated_are_not_a_checkpoint() {
        let mut swapped = checkpoint();
        swapped.heads.reverse();
        assert_eq!(Checkpoint::from_json(&swapped.to_json()), None);
        let mut repeated = checkpoint();
        repeated.heads[1].node = 1;
        assert_eq!(Checkpoint::from_json(&repeated.to_json()), None);
        let mut empty_chain = checkpoint();
        empty_chain.heads[0].seq = 0;
        assert_eq!(Checkpoint::from_json(&empty_chain.to_json()), None);
    }

    #[test]
    fn text_that_is_not_a_checkpoint_is_refused() {
        assert_eq!(Checkpoint::from_json("not json"), None);
        assert_eq!(Checkpoint::from_json("[]"), None);
        let mut value = checkpoint().to_value();
        value["before"] = json!("soon");
        assert_eq!(Checkpoint::from_value(&value), None);
        let mut value = checkpoint().to_value();
        value["sig"] = json!("AA");
        assert_eq!(Checkpoint::from_value(&value), None);
    }
}
