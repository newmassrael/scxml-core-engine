// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! An import proposal: a document brought in, converted, checked, and waiting for the owner (ADR
//! 0025, 0037).
//!
//! This module is the record alone, with no folder and no clock of its own. What a proposal is made of
//! is kept here; what an event does to it is the machine's (`machines/import_life`, reached through
//! `import_life`); the store (`store/proposal_store.rs`) keeps proposals under the spec's lock, and it
//! is where accepting meets the text head.
//!
//! **What the record names, it names by hash.** The original, the extraction read out of it, the
//! conversion made of the extraction and the check report SCE made of the conversion are each a
//! digest, and the texts live where every text of the folder does. A proposal says what it was made of
//! and does not hold a copy of it.
//!
//! **The questions are asked once.** The check names what a person is asked about (`asks`: the reason
//! and the block). They are kept with the proposal and are not recomputed: a question that appears
//! later is another check, which is another proposal.

use serde::{Deserialize, Serialize};

use crate::revision::Revision;

/// The most a conversion may hold.
pub const MAX_CONVERSION_BYTES: usize = 16 * 1024 * 1024;
/// The most a check report may hold.
pub const MAX_REPORT_BYTES: usize = 4 * 1024 * 1024;
/// The most a decision's words may hold.
pub const MAX_DECISION_TEXT_BYTES: usize = 256 * 1024;
/// The most questions a proposal asks. A conversion that asks more is not one a person reviews.
pub const MAX_ASKS: usize = 20_000;
/// The most decisions a proposal keeps. A decision is a line of a log that is read whole.
pub const MAX_DECISIONS: usize = 100_000;

/// Where a proposal is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProposalState {
    /// Waiting for the owner. Nothing is locked meanwhile.
    Pending,
    /// The owner accepted it and the text is the spec's.
    Accepted,
    /// The owner declined it.
    Rejected,
    /// Another proposal replaced it, or the original it was made of was replaced.
    Superseded,
}

impl ProposalState {
    /// Whether the proposal can still be decided and accepted.
    pub fn is_open(self) -> bool {
        matches!(self, ProposalState::Pending)
    }

    /// The word a program branches on.
    pub fn word(self) -> &'static str {
        match self {
            ProposalState::Pending => "pending",
            ProposalState::Accepted => "accepted",
            ProposalState::Rejected => "rejected",
            ProposalState::Superseded => "superseded",
        }
    }
}

/// Who converted: a rule of the authoring tools, or a model, by the name it reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Converter {
    pub kind: String,
    pub name: String,
    pub version: String,
}

/// One thing a person is asked about: why, and which block (of the conversion or of the original).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ask {
    pub why: String,
    pub id: String,
}

/// One proposal, as it is kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Proposal {
    pub id: String,
    /// The order proposals of a spec were made in, from 1.
    pub seq: u64,
    /// What the caller said to make a repeated registration the proposal it repeats.
    pub key: String,
    /// Where it was asked from: `gui`, or the name of a client.
    pub origin: String,
    /// The original (ADR 0032) the proposal is made of, by the digest of its manifest.
    pub original: Revision,
    /// The extraction read out of a member of that original.
    pub extraction: Revision,
    /// The conversion made of the extraction.
    pub conversion: Revision,
    /// SCE's check of the conversion against the extraction.
    pub report: Revision,
    pub converter: Converter,
    /// The text head the proposal was made against; `None` for a spec that had no text.
    pub against: Option<Revision>,
    /// What a person is asked about, in the order they are asked.
    pub asks: Vec<Ask>,
    pub created_at: String,
    pub state: ProposalState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<String>,
    /// Why it ended or was let go of, in a person's words.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// The text revision accepting made; `None` until it is accepted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub saved: Option<Revision>,
    /// The proposal this one carries forward (`rebase`), if it does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rebased_from: Option<String>,
}

/// What the owner did with a block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DecisionKind {
    /// The block is right as the conversion has it.
    Keep,
    /// The block says something else: the owner's words stand for it.
    Correct,
    /// The block is not part of the spec.
    Drop,
    /// The original's block is part of the spec and the conversion left it out.
    Add,
}

impl DecisionKind {
    pub fn word(self) -> &'static str {
        match self {
            DecisionKind::Keep => "keep",
            DecisionKind::Correct => "correct",
            DecisionKind::Drop => "drop",
            DecisionKind::Add => "add",
        }
    }
}

/// One decision of the owner's, as a line of the proposal's log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Decision {
    /// The place of the line in the log, from 1.
    pub n: u64,
    pub at: String,
    /// The block decided about: of the conversion, or of the original for `add`.
    pub block: String,
    pub what: DecisionKind,
    /// The owner's words, for `correct`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

impl Proposal {
    /// The questions that have no decision yet, in the order they were asked. A question is answered
    /// when any decision names the block it asks about.
    pub fn undecided<'a>(&'a self, decisions: &[Decision]) -> Vec<&'a Ask> {
        self.asks
            .iter()
            .filter(|ask| !decisions.iter().any(|decision| decision.block == ask.id))
            .collect()
    }
}

/// The decisions that stand: the latest on each block, in the order the blocks were first decided.
/// An earlier one is kept in the log as history and does not count.
pub fn standing(decisions: &[Decision]) -> Vec<&Decision> {
    let mut blocks: Vec<&str> = Vec::new();
    for decision in decisions {
        if !blocks.contains(&decision.block.as_str()) {
            blocks.push(&decision.block);
        }
    }
    blocks
        .into_iter()
        .filter_map(|block| decisions.iter().rev().find(|d| d.block == block))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decision(n: u64, block: &str, what: DecisionKind) -> Decision {
        Decision {
            n,
            at: "2026-10-19T00:00:00Z".to_string(),
            block: block.to_string(),
            what,
            text: None,
        }
    }

    fn proposal(asks: &[&str]) -> Proposal {
        let digest = Revision::of(b"x");
        Proposal {
            id: "imp-0123456789ab".to_string(),
            seq: 1,
            key: "k".to_string(),
            origin: "gui".to_string(),
            original: digest.clone(),
            extraction: digest.clone(),
            conversion: digest.clone(),
            report: digest,
            converter: Converter {
                kind: "rules".to_string(),
                name: "blocks-to-markdown".to_string(),
                version: "1".to_string(),
            },
            against: None,
            asks: asks
                .iter()
                .map(|id| Ask {
                    why: "check".to_string(),
                    id: id.to_string(),
                })
                .collect(),
            created_at: "2026-10-19T00:00:00Z".to_string(),
            state: ProposalState::Pending,
            ended_at: None,
            note: None,
            saved: None,
            rebased_from: None,
        }
    }

    #[test]
    fn a_question_is_answered_by_any_decision_that_names_its_block_and_by_no_other() {
        let p = proposal(&["m1", "m2", "b3"]);
        let decided = [decision(1, "m2", DecisionKind::Keep)];

        let open: Vec<&str> = p
            .undecided(&decided)
            .iter()
            .map(|a| a.id.as_str())
            .collect();

        assert_eq!(open, ["m1", "b3"]);
        assert_eq!(p.undecided(&[]).len(), 3);
    }

    #[test]
    fn a_decision_on_a_block_nobody_asked_about_answers_nothing() {
        let p = proposal(&["m1"]);

        assert_eq!(
            p.undecided(&[decision(1, "m9", DecisionKind::Drop)]).len(),
            1
        );
    }

    #[test]
    fn every_kind_of_decision_answers_a_question() {
        for kind in [
            DecisionKind::Keep,
            DecisionKind::Correct,
            DecisionKind::Drop,
            DecisionKind::Add,
        ] {
            assert!(proposal(&["m1"])
                .undecided(&[decision(1, "m1", kind)])
                .is_empty());
        }
    }

    #[test]
    fn the_latest_decision_on_a_block_stands_and_blocks_keep_the_order_they_were_first_decided() {
        let log = [
            decision(1, "m2", DecisionKind::Keep),
            decision(2, "m1", DecisionKind::Drop),
            decision(3, "m2", DecisionKind::Drop),
        ];

        let standing = standing(&log);

        assert_eq!(
            standing
                .iter()
                .map(|d| (d.block.as_str(), d.what))
                .collect::<Vec<_>>(),
            [("m2", DecisionKind::Drop), ("m1", DecisionKind::Drop)]
        );
    }

    #[test]
    fn a_record_without_the_optional_fields_reads_and_writes_as_it_did() {
        let p = proposal(&[]);

        let text = serde_json::to_string(&p).unwrap();

        for field in ["ended_at", "note", "saved", "rebased_from"] {
            assert!(
                !text.contains(field),
                "{field} is written when it is absent"
            );
        }
        assert_eq!(serde_json::from_str::<Proposal>(&text).unwrap(), p);
    }

    #[test]
    fn the_states_say_whether_they_are_open_and_their_words() {
        assert!(ProposalState::Pending.is_open());
        for state in [
            ProposalState::Accepted,
            ProposalState::Rejected,
            ProposalState::Superseded,
        ] {
            assert!(!state.is_open());
        }
        assert_eq!(ProposalState::Superseded.word(), "superseded");
        assert_eq!(
            serde_json::to_string(&ProposalState::Pending).unwrap(),
            "\"pending\""
        );
    }
}
