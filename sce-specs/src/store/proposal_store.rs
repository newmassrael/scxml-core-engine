// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Import proposals in a spec's folder (ADR 0025, 0037).
//!
//! ```text
//! <spec-id>/
//!   proposals/<proposal-id>.json   one file per proposal, replaced whole (atomically)
//!   decisions/<proposal-id>.log    the owner's decisions, one JSON line each, never edited
//!   conversions/<digest>.json      a conversion, named by its own SHA-256
//!   reports/<digest>.json          SCE's check of a conversion, named the same way
//! ```
//!
//! A proposal changes in place (it is accepted, rejected, replaced), so its file is the truth and is
//! replaced whole by an atomic rename. Its decisions are a log and never change. Every change is made
//! under the lock a save of the spec's text takes, which is what lets accepting be one step: the
//! proposal's life is asked, the text head is compared and moved, and the proposal ends, with no other
//! save of the spec between them. Reads take no lock.
//!
//! **What the store does not do.** It does not run the check. The authoring tools make the extraction,
//! the conversion and the report (`check-conversion --json`) and the host hands them over; the store
//! checks that the three name each other and that the questions the report asks are about blocks that
//! exist, and keeps them. It does not check that the text an accept carries is the assembly of the
//! decisions either, as `save_source` does not check that a text is what its owner meant; it checks
//! that the text was assembled from as many decisions as the log holds now.

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::original_store::{append_line, keep, read_log};
use super::request_store::valid_name;
use super::{
    atomic_write, is_removed, read_head, removed_spec, Artifact, Saved, SpecId, SpecStore,
    LOCK_FILE, LOCK_WAIT,
};
use crate::clock::{utc_timestamp, Clock};
use crate::error::StoreError;
use crate::import_life::{self, Event, Verdict};
use crate::lock;
use crate::proposals::{
    Ask, Converter, Decision, DecisionKind, Proposal, ProposalState, MAX_ASKS,
    MAX_CONVERSION_BYTES, MAX_DECISIONS, MAX_DECISION_TEXT_BYTES, MAX_REPORT_BYTES,
};
use crate::revision::Revision;

const PROPOSALS_DIR: &str = "proposals";
const DECISIONS_DIR: &str = "decisions";
const CONVERSIONS_DIR: &str = "conversions";
const REPORTS_DIR: &str = "reports";
const ID_PREFIX: &str = "imp-";
const ID_HEX: usize = 12;
const KEY_MAX: usize = 200;

static ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// What a caller says to make a proposal.
#[derive(Debug, Clone)]
pub struct ProposalRegistration<'a> {
    /// What makes a repeated registration the proposal it repeats: a button pressed twice, a call
    /// sent again after a connection dropped.
    pub key: &'a str,
    /// Where it was asked from: `gui`, or the name of a client.
    pub origin: &'a str,
    /// The original the conversion is of (ADR 0032).
    pub original: Revision,
    /// The extraction the conversion was made of: one saved for that original.
    pub extraction: Revision,
    /// The conversion, as the authoring tools wrote it.
    pub conversion: &'a str,
    /// SCE's check of that conversion against that extraction, as `check-conversion --json` wrote it.
    pub report: &'a str,
    /// The text head the caller read and is asking about; `None` for a spec that has no text. The
    /// proposal is refused as `moved` when the spec is no longer at it.
    pub expect: Option<Revision>,
    /// Replace the open proposal of the spec, if there is one. Without it an open proposal is a
    /// refusal: whether a review under way is thrown away is the person's to say.
    pub supersede: bool,
}

/// What registering a proposal made.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProposalRegistered {
    pub proposal: Proposal,
    /// `false` when the key was one that registered a proposal before, and this is that proposal.
    pub created: bool,
}

/// A proposal with what the screen asks of it: its decisions, what is still asked, and whether the
/// spec's text is still where it was when the proposal was made.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProposalView {
    pub proposal: Proposal,
    pub decisions: Vec<Decision>,
    /// The questions that have no decision, in the order they were asked.
    pub undecided: Vec<Ask>,
    /// The spec's text head now.
    pub head: Option<Revision>,
    /// Whether that is the head the proposal was made against.
    pub head_matches: bool,
}

/// The texts a proposal is made of, for the screen that shows them beside the original.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProposalDocuments {
    pub conversion: String,
    pub report: String,
}

/// What accepting made.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Accepted {
    pub proposal: Proposal,
    /// The text revision that is the spec's now.
    pub saved: Revision,
    /// `false` when the proposal was accepted before, and this is that acceptance said again.
    pub created: bool,
}

fn bad(message: impl Into<String>, detail: Value) -> StoreError {
    StoreError::refused("bad-proposal", message, detail)
}

fn valid_proposal_id(id: &str) -> bool {
    id.strip_prefix(ID_PREFIX).is_some_and(|rest| {
        (1..=40).contains(&rest.len())
            && rest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}

fn proposal_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(PROPOSALS_DIR).join(format!("{id}.json"))
}

fn decisions_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(DECISIONS_DIR).join(format!("{id}.log"))
}

fn conversion_path(dir: &Path, digest: &Revision) -> PathBuf {
    dir.join(CONVERSIONS_DIR)
        .join(format!("{}.json", digest.as_str()))
}

fn report_path(dir: &Path, digest: &Revision) -> PathBuf {
    dir.join(REPORTS_DIR)
        .join(format!("{}.json", digest.as_str()))
}

fn new_proposal_id(spec: &SpecId, key: &str, epoch: u64) -> String {
    let counter = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut hasher = Sha256::new();
    hasher.update(spec.as_str().as_bytes());
    hasher.update([0]);
    hasher.update(key.as_bytes());
    hasher.update([0]);
    hasher.update(epoch.to_le_bytes());
    hasher.update(std::process::id().to_le_bytes());
    hasher.update(counter.to_le_bytes());
    let digest = hasher.finalize();
    let mut id = String::from(ID_PREFIX);
    for byte in digest.iter().take(ID_HEX / 2) {
        id.push_str(&format!("{byte:02x}"));
    }
    id
}

fn load_proposal(dir: &Path, id: &str) -> Result<Option<Proposal>, StoreError> {
    let path = proposal_path(dir, id);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(StoreError::io(&path, e)),
    };
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|e| StoreError::corrupt(&path, format!("not a proposal: {e}")))
}

/// Every proposal of the spec, in the order they were made.
fn load_proposals(dir: &Path) -> Result<Vec<Proposal>, StoreError> {
    let folder = dir.join(PROPOSALS_DIR);
    let entries = match fs::read_dir(&folder) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(StoreError::io(&folder, e)),
    };
    let mut proposals = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| StoreError::io(&folder, e))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        // A temporary file of a write in progress, or anything that is not a proposal's file.
        let Some(id) = name.strip_suffix(".json") else {
            continue;
        };
        if !valid_proposal_id(id) {
            continue;
        }
        if let Some(proposal) = load_proposal(dir, id)? {
            proposals.push(proposal);
        }
    }
    proposals.sort_by_key(|p| p.seq);
    Ok(proposals)
}

fn write_proposal(dir: &Path, proposal: &Proposal) -> Result<(), StoreError> {
    let folder = dir.join(PROPOSALS_DIR);
    fs::create_dir_all(&folder).map_err(|e| StoreError::io(&folder, e))?;
    let text = serde_json::to_string_pretty(proposal)
        .map_err(|e| StoreError::corrupt(proposal_path(dir, &proposal.id), e.to_string()))?;
    atomic_write(&proposal_path(dir, &proposal.id), text.as_bytes())
}

fn not_found(id: &SpecId, proposal: &str) -> StoreError {
    StoreError::NotFound {
        what: format!("proposal `{proposal}` of spec `{}`", id.as_str()),
    }
}

/// The ids of the blocks a conversion or an extraction names, or why its text is not one.
fn block_ids(text: &str, what: &str) -> Result<BTreeSet<String>, StoreError> {
    let value: Value = serde_json::from_str(text)
        .map_err(|e| bad(format!("the {what} is not JSON ({e})"), Value::Null))?;
    let blocks = value
        .get("blocks")
        .and_then(Value::as_array)
        .ok_or_else(|| bad(format!("the {what} has no list of blocks"), Value::Null))?;
    blocks
        .iter()
        .map(|block| {
            block
                .get("id")
                .and_then(Value::as_str)
                .filter(|id| !id.is_empty())
                .map(str::to_string)
                .ok_or_else(|| bad(format!("a block of the {what} has no id"), Value::Null))
        })
        .collect()
}

fn text_field<'a>(value: &'a Value, field: &str, what: &str) -> Result<&'a str, StoreError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| {
            bad(
                format!("the {what} says `{field}`: non-empty text"),
                json!({ "field": field }),
            )
        })
}

/// Read the conversion's own words about who made it and what it is of.
fn read_conversion(text: &str) -> Result<(String, Converter), StoreError> {
    let value: Value = serde_json::from_str(text)
        .map_err(|e| bad(format!("the conversion is not JSON ({e})"), Value::Null))?;
    let extraction = text_field(&value, "extraction", "conversion")?.to_string();
    let converter = value
        .get("converter")
        .filter(|c| c.is_object())
        .ok_or_else(|| bad("the conversion does not say who made it", Value::Null))?;
    Ok((
        extraction,
        Converter {
            kind: text_field(converter, "kind", "converter")?.to_string(),
            name: text_field(converter, "name", "converter")?.to_string(),
            version: text_field(converter, "version", "converter")?.to_string(),
        },
    ))
}

/// Read the report's own words about what it is of and what a person is asked about.
fn read_report(text: &str) -> Result<(String, String, Vec<Ask>), StoreError> {
    let value: Value = serde_json::from_str(text)
        .map_err(|e| bad(format!("the report is not JSON ({e})"), Value::Null))?;
    let extraction = text_field(&value, "extraction", "report")?.to_string();
    let conversion = text_field(&value, "conversion", "report")?.to_string();
    let queue = value
        .get("queue")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            bad(
                "the report has no list of what a person is asked",
                Value::Null,
            )
        })?;
    if queue.len() > MAX_ASKS {
        return Err(bad(
            format!(
                "the report asks {} questions, past the {MAX_ASKS} a person is asked in one proposal",
                queue.len()
            ),
            json!({ "limit": MAX_ASKS }),
        ));
    }
    let asks = queue
        .iter()
        .map(|ask| {
            Ok(Ask {
                why: text_field(ask, "why", "question")?.to_string(),
                id: text_field(ask, "id", "question")?.to_string(),
            })
        })
        .collect::<Result<Vec<Ask>, StoreError>>()?;
    Ok((extraction, conversion, asks))
}

impl<C: Clock> SpecStore<C> {
    fn stamp(&self) -> String {
        utc_timestamp(self.clock.epoch())
    }

    /// A proposal that ends is written down as it ended, and says why.
    fn ended_as(&self, proposal: &Proposal, state: ProposalState, note: Option<&str>) -> Proposal {
        Proposal {
            state,
            ended_at: Some(self.stamp()),
            note: note.map(str::to_string),
            ..proposal.clone()
        }
    }

    /// Keep what an import was made into as a proposal of the spec, to be reviewed.
    ///
    /// The proposal is made against the text head the caller read (`expect`) and is refused as
    /// `moved` when the spec is no longer at it, as a request is. The spec has one open proposal at
    /// most; a second is refused (`active-proposal`) unless it says it replaces the first.
    pub fn propose_import(
        &self,
        id: &SpecId,
        registration: ProposalRegistration<'_>,
    ) -> Result<ProposalRegistered, StoreError> {
        if registration.key.is_empty() || registration.key.len() > KEY_MAX {
            return Err(bad(
                "a registration key is 1 to 200 characters",
                json!({ "limit": KEY_MAX }),
            ));
        }
        if !valid_name(registration.origin) {
            return Err(bad(
                "an origin is 1 to 64 letters, digits and `._:-`",
                Value::Null,
            ));
        }
        for (what, bytes, limit) in [
            (
                "a conversion",
                registration.conversion.len(),
                MAX_CONVERSION_BYTES,
            ),
            (
                "a check report",
                registration.report.len(),
                MAX_REPORT_BYTES,
            ),
        ] {
            if bytes > limit {
                return Err(StoreError::TooLarge { what, bytes, limit });
            }
        }
        let conversion = Revision::of(registration.conversion.as_bytes());
        let report = Revision::of(registration.report.as_bytes());

        let dir = self.existing(id)?;
        let _held = lock::exclusive(&dir.join(LOCK_FILE), LOCK_WAIT)?;
        if is_removed(&dir) {
            return Err(removed_spec(id));
        }
        let proposals = load_proposals(&dir)?;
        if let Some(same) = proposals.iter().find(|p| p.key == registration.key) {
            let same_inputs = same.original == registration.original
                && same.extraction == registration.extraction
                && same.conversion == conversion
                && same.report == report;
            if !same_inputs {
                return Err(StoreError::refused(
                    "key-reused",
                    format!(
                        "the key was used for another proposal ({}), made of another original, \
                         conversion or check",
                        same.id
                    ),
                    json!({ "proposal": same.id }),
                ));
            }
            return Ok(ProposalRegistered {
                proposal: same.clone(),
                created: false,
            });
        }

        let head = read_head(&dir, Artifact::Source)?;
        if head != registration.expect {
            return Err(StoreError::refused(
                "moved",
                "the text changed after the proposal was asked about it, so nothing was \
                 registered; read the spec again",
                json!({ "moved": ["source"], "current": { "source": head } }),
            ));
        }

        // What the proposal names has to be there, and has to name each other.
        self.read_original(id, &registration.original)?;
        let saved = self.list_extractions(id, Some(&registration.original))?;
        if !saved.iter().any(|e| e.digest == registration.extraction) {
            return Err(bad(
                "the extraction is not one saved for this original",
                json!({ "extraction": registration.extraction }),
            ));
        }
        let extraction_ids = block_ids(
            &self.read_extraction(id, &registration.extraction)?,
            "extraction",
        )?;
        let (of_extraction, converter) = read_conversion(registration.conversion)?;
        if of_extraction != registration.extraction.as_str() {
            return Err(bad(
                "the conversion is of another extraction than the one named",
                json!({ "conversion_of": of_extraction, "named": registration.extraction }),
            ));
        }
        let conversion_ids = block_ids(registration.conversion, "conversion")?;
        let (report_extraction, report_conversion, asks) = read_report(registration.report)?;
        if report_extraction != registration.extraction.as_str()
            || report_conversion != conversion.as_str()
        {
            return Err(bad(
                "the check report is of another extraction or conversion than the ones named",
                json!({ "report_of": { "extraction": report_extraction, "conversion": report_conversion } }),
            ));
        }
        if let Some(stranger) = asks
            .iter()
            .find(|ask| !conversion_ids.contains(&ask.id) && !extraction_ids.contains(&ask.id))
        {
            return Err(bad(
                format!(
                    "the check asks about the block `{}`, which neither the conversion nor the \
                     original has",
                    stranger.id
                ),
                json!({ "block": stranger.id }),
            ));
        }

        let now = self.stamp();
        let open: Vec<&Proposal> = proposals.iter().filter(|p| p.state.is_open()).collect();
        if let Some(first) = open.first() {
            if !registration.supersede {
                return Err(StoreError::refused(
                    "active-proposal",
                    format!(
                        "the spec already has an open proposal ({}); ask to replace it, or \
                         accept or reject it",
                        first.id
                    ),
                    json!({ "proposal": first.id }),
                ));
            }
        }
        // Everything this proposal needs is written before anything else changes, so that a
        // failure leaves the open proposal as it was.
        keep(
            &dir.join(CONVERSIONS_DIR),
            &conversion_path(&dir, &conversion),
            &conversion,
            registration.conversion.as_bytes(),
        )?;
        keep(
            &dir.join(REPORTS_DIR),
            &report_path(&dir, &report),
            &report,
            registration.report.as_bytes(),
        )?;
        for old in open {
            let answer = import_life::decide(old.state, &Event::Supersede);
            if answer.verdict == Verdict::Ok {
                write_proposal(
                    &dir,
                    &self.ended_as(old, answer.state, Some("a newer proposal replaced it")),
                )?;
            }
        }
        let proposal = Proposal {
            id: new_proposal_id(id, registration.key, self.clock.epoch()),
            seq: proposals.iter().map(|p| p.seq).max().unwrap_or(0) + 1,
            key: registration.key.to_string(),
            origin: registration.origin.to_string(),
            original: registration.original,
            extraction: registration.extraction,
            conversion,
            report,
            converter,
            against: head,
            asks,
            created_at: now,
            state: ProposalState::Pending,
            ended_at: None,
            note: None,
            saved: None,
            rebased_from: None,
        };
        write_proposal(&dir, &proposal)?;
        Ok(ProposalRegistered {
            proposal,
            created: true,
        })
    }

    fn view_of(&self, dir: &Path, proposal: Proposal) -> Result<ProposalView, StoreError> {
        let decisions = read_log::<Decision>(&decisions_path(dir, &proposal.id))?;
        let head = read_head(dir, Artifact::Source)?;
        let undecided = proposal
            .undecided(&decisions)
            .into_iter()
            .cloned()
            .collect();
        Ok(ProposalView {
            head_matches: head == proposal.against,
            undecided,
            decisions,
            head,
            proposal,
        })
    }

    /// One proposal of the spec, with its decisions and what is still asked.
    pub fn read_proposal(&self, id: &SpecId, proposal: &str) -> Result<ProposalView, StoreError> {
        let dir = self.existing(id)?;
        let found = if valid_proposal_id(proposal) {
            load_proposal(&dir, proposal)?
        } else {
            None
        };
        let Some(found) = found else {
            return Err(not_found(id, proposal));
        };
        self.view_of(&dir, found)
    }

    /// Every proposal of the spec, the newest first.
    pub fn list_proposals(&self, id: &SpecId) -> Result<Vec<Proposal>, StoreError> {
        let dir = self.existing(id)?;
        let mut proposals = load_proposals(&dir)?;
        proposals.reverse();
        Ok(proposals)
    }

    /// The conversion and the check report a proposal is made of, as they were kept.
    pub fn read_proposal_documents(
        &self,
        id: &SpecId,
        proposal: &str,
    ) -> Result<ProposalDocuments, StoreError> {
        let dir = self.existing(id)?;
        let found = if valid_proposal_id(proposal) {
            load_proposal(&dir, proposal)?
        } else {
            None
        };
        let Some(found) = found else {
            return Err(not_found(id, proposal));
        };
        let read = |path: PathBuf, digest: &Revision| -> Result<String, StoreError> {
            let text = fs::read_to_string(&path).map_err(|e| {
                if e.kind() == io::ErrorKind::NotFound {
                    StoreError::corrupt(&path, "the proposal names a text that is not held")
                } else {
                    StoreError::io(&path, e)
                }
            })?;
            if Revision::of(text.as_bytes()) != *digest {
                return Err(StoreError::corrupt(
                    &path,
                    "it is named by a hash its text does not have",
                ));
            }
            Ok(text)
        };
        Ok(ProposalDocuments {
            conversion: read(conversion_path(&dir, &found.conversion), &found.conversion)?,
            report: read(report_path(&dir, &found.report), &found.report)?,
        })
    }

    /// The refusal of a proposal that ended, saying how.
    fn proposal_ended(proposal: &Proposal) -> StoreError {
        StoreError::refused(
            "proposal-ended",
            format!(
                "the proposal ended ({}), so it takes no other word",
                proposal.state.word()
            ),
            json!({ "proposal": proposal.id, "state": proposal.state.word() }),
        )
    }

    /// Record what the owner decided about a block.
    ///
    /// A decision names a block of the conversion, or of the original (the questions of the check
    /// are about both: a block the conversion left out, a block read from an image). `add` is the
    /// original's block put into the spec, so it names one the original has. Taken while the proposal
    /// is open and refused once it ended; the log only grows.
    pub fn decide_block(
        &self,
        id: &SpecId,
        proposal: &str,
        block: &str,
        what: DecisionKind,
        text: Option<&str>,
    ) -> Result<Decision, StoreError> {
        match (what, text) {
            (DecisionKind::Correct, Some(words)) if !words.trim().is_empty() => {}
            (DecisionKind::Correct, _) => {
                return Err(bad(
                    "a correction carries the owner's words",
                    json!({ "what": "correct" }),
                ));
            }
            (_, Some(_)) => {
                return Err(bad(
                    format!(
                        "only a correction carries words, and this is `{}`",
                        what.word()
                    ),
                    json!({ "what": what.word() }),
                ));
            }
            (_, None) => {}
        }
        if text.is_some_and(|words| words.len() > MAX_DECISION_TEXT_BYTES) {
            return Err(StoreError::TooLarge {
                what: "a decision's words",
                bytes: text.map_or(0, str::len),
                limit: MAX_DECISION_TEXT_BYTES,
            });
        }
        let dir = self.existing(id)?;
        let _held = lock::exclusive(&dir.join(LOCK_FILE), LOCK_WAIT)?;
        if is_removed(&dir) {
            return Err(removed_spec(id));
        }
        let found = if valid_proposal_id(proposal) {
            load_proposal(&dir, proposal)?
        } else {
            None
        };
        let Some(found) = found else {
            return Err(not_found(id, proposal));
        };
        if import_life::decide(found.state, &Event::Decide).verdict != Verdict::Ok {
            return Err(Self::proposal_ended(&found));
        }
        let documents = self.read_proposal_documents(id, proposal)?;
        let conversion_ids = block_ids(&documents.conversion, "conversion")?;
        let extraction_ids =
            block_ids(&self.read_extraction(id, &found.extraction)?, "extraction")?;
        let known = if what == DecisionKind::Add {
            extraction_ids.contains(block)
        } else {
            conversion_ids.contains(block) || extraction_ids.contains(block)
        };
        if !known {
            return Err(bad(
                if what == DecisionKind::Add {
                    format!("the original has no block `{block}` to add")
                } else {
                    format!("neither the conversion nor the original has a block `{block}`")
                },
                json!({ "block": block }),
            ));
        }
        let path = decisions_path(&dir, proposal);
        let held = read_log::<Decision>(&path)?;
        if held.len() >= MAX_DECISIONS {
            return Err(StoreError::TooLarge {
                what: "the decisions of a proposal",
                bytes: held.len(),
                limit: MAX_DECISIONS,
            });
        }
        let decision = Decision {
            n: held.len() as u64 + 1,
            at: self.stamp(),
            block: block.to_string(),
            what,
            text: text.map(str::to_string),
        };
        fs::create_dir_all(dir.join(DECISIONS_DIR))
            .map_err(|e| StoreError::io(dir.join(DECISIONS_DIR), e))?;
        append_line(&path, &decision)?;
        Ok(decision)
    }

    /// Accept the proposal: `text` is the spec's text from now.
    ///
    /// One step under the spec's lock. The proposal's life is asked whether it may be accepted (the
    /// text must still be the head it was made against, and every question must have a decision),
    /// and when it may, the text is saved from that head and the proposal ends as accepted. When the
    /// text moved the refusal is `moved`, naming both revisions, and nothing is written: the proposal
    /// stays, and the owner is shown what the spec has become (`rebase_proposal` carries it forward).
    ///
    /// `decisions` is how many decisions the text was assembled from. The log may have grown since
    /// (somebody decided one more block), and a text assembled before that is not saved as if it
    /// answered the log: `decisions-moved`. The same accept said again by an accepted proposal is
    /// the same acceptance and writes nothing.
    pub fn accept_proposal(
        &self,
        id: &SpecId,
        proposal: &str,
        text: &str,
        decisions: u64,
    ) -> Result<Accepted, StoreError> {
        let max = Artifact::Source.max_bytes();
        if text.len() > max {
            return Err(StoreError::TooLarge {
                what: Artifact::Source.noun(),
                bytes: text.len(),
                limit: max,
            });
        }
        let dir = self.existing(id)?;
        let _held = lock::exclusive(&dir.join(LOCK_FILE), LOCK_WAIT)?;
        if is_removed(&dir) {
            return Err(removed_spec(id));
        }
        let found = if valid_proposal_id(proposal) {
            load_proposal(&dir, proposal)?
        } else {
            None
        };
        let Some(found) = found else {
            return Err(not_found(id, proposal));
        };
        let log = read_log::<Decision>(&decisions_path(&dir, proposal))?;
        let head = read_head(&dir, Artifact::Source)?;
        let open = found.undecided(&log);
        let answer = import_life::decide(
            found.state,
            &Event::Accept {
                head_matches: head == found.against,
                undecided: u32::try_from(open.len()).unwrap_or(u32::MAX),
            },
        );
        match answer.verdict {
            Verdict::Moved => Err(StoreError::refused(
                "moved",
                "the text changed after the proposal was made, so nothing was written; look at \
                 what the spec has become, or carry the proposal forward",
                json!({
                    "moved": ["source"],
                    "current": { "source": head },
                    "against": found.against,
                    "proposal": found.id,
                }),
            )),
            Verdict::Undecided => Err(StoreError::refused(
                "undecided",
                format!(
                    "{} question(s) have no decision yet, so nothing was written",
                    open.len()
                ),
                json!({
                    "proposal": found.id,
                    "blocks": open.iter().map(|ask| ask.id.as_str()).collect::<Vec<_>>(),
                }),
            )),
            Verdict::Ended | Verdict::Nothing => Err(Self::proposal_ended(&found)),
            Verdict::Ok if found.state == ProposalState::Accepted => {
                let saved = found.saved.clone().ok_or_else(|| {
                    StoreError::corrupt(
                        proposal_path(&dir, &found.id),
                        "an accepted proposal does not say what it saved",
                    )
                })?;
                Ok(Accepted {
                    proposal: found,
                    saved,
                    created: false,
                })
            }
            Verdict::Ok => {
                if decisions != log.len() as u64 {
                    return Err(StoreError::refused(
                        "decisions-moved",
                        "decisions were taken after the text was assembled, so nothing was \
                         written; assemble it again",
                        json!({ "assembled_from": decisions, "decisions": log.len() }),
                    ));
                }
                let revision = match self.save_text_locked(
                    &dir,
                    id,
                    Artifact::Source,
                    text,
                    found.against.as_ref(),
                    None,
                )? {
                    Saved::Saved { revision, .. } | Saved::Unchanged { revision } => revision,
                };
                let ended = Proposal {
                    saved: Some(revision.clone()),
                    ..self.ended_as(&found, answer.state, None)
                };
                write_proposal(&dir, &ended)?;
                Ok(Accepted {
                    proposal: ended,
                    saved: revision,
                    created: true,
                })
            }
        }
    }

    /// Decline the proposal. Declining a declined proposal is the same word said again.
    pub fn reject_proposal(
        &self,
        id: &SpecId,
        proposal: &str,
        note: Option<&str>,
    ) -> Result<Proposal, StoreError> {
        if note.is_some_and(|words| words.len() > MAX_DECISION_TEXT_BYTES) {
            return Err(StoreError::TooLarge {
                what: "a note",
                bytes: note.map_or(0, str::len),
                limit: MAX_DECISION_TEXT_BYTES,
            });
        }
        let dir = self.existing(id)?;
        let _held = lock::exclusive(&dir.join(LOCK_FILE), LOCK_WAIT)?;
        if is_removed(&dir) {
            return Err(removed_spec(id));
        }
        let found = if valid_proposal_id(proposal) {
            load_proposal(&dir, proposal)?
        } else {
            None
        };
        let Some(found) = found else {
            return Err(not_found(id, proposal));
        };
        let answer = import_life::decide(found.state, &Event::Reject);
        match answer.verdict {
            Verdict::Ok if found.state == answer.state => Ok(found),
            Verdict::Ok => {
                let ended = self.ended_as(&found, answer.state, note);
                write_proposal(&dir, &ended)?;
                Ok(ended)
            }
            _ => Err(Self::proposal_ended(&found)),
        }
    }

    /// Carry a proposal whose text moved forward: a new proposal against the head the spec has now,
    /// of the same original, extraction and conversion, with the decisions of the old one copied in
    /// order, and the old one superseded, in one step.
    ///
    /// The original did not move when the spec did, so every block of the conversion is as it was and
    /// every decision still stands; what moved is what accepting would overwrite. Refused for a
    /// proposal that ended, and for one whose head still matches (there is nothing to carry). The
    /// same call again is the same proposal: the key is made of the proposal and the head.
    pub fn rebase_proposal(
        &self,
        id: &SpecId,
        proposal: &str,
    ) -> Result<ProposalRegistered, StoreError> {
        let dir = self.existing(id)?;
        let _held = lock::exclusive(&dir.join(LOCK_FILE), LOCK_WAIT)?;
        if is_removed(&dir) {
            return Err(removed_spec(id));
        }
        let found = if valid_proposal_id(proposal) {
            load_proposal(&dir, proposal)?
        } else {
            None
        };
        let Some(found) = found else {
            return Err(not_found(id, proposal));
        };
        let head = read_head(&dir, Artifact::Source)?;
        let key = format!(
            "rebase:{}:{}",
            found.id,
            head.as_ref().map_or("none", Revision::as_str)
        );
        let proposals = load_proposals(&dir)?;
        if let Some(same) = proposals.iter().find(|p| p.key == key) {
            return Ok(ProposalRegistered {
                proposal: same.clone(),
                created: false,
            });
        }
        if !found.state.is_open() {
            return Err(Self::proposal_ended(&found));
        }
        if head == found.against {
            return Err(StoreError::refused(
                "nothing-to-carry",
                "the text is where the proposal was made against it, so there is nothing to carry \
                 forward",
                json!({ "proposal": found.id }),
            ));
        }
        let answer = import_life::decide(found.state, &Event::Supersede);
        let decisions = read_log::<Decision>(&decisions_path(&dir, &found.id))?;
        let carried = Proposal {
            id: new_proposal_id(id, &key, self.clock.epoch()),
            seq: proposals.iter().map(|p| p.seq).max().unwrap_or(0) + 1,
            key,
            created_at: self.stamp(),
            state: ProposalState::Pending,
            against: head,
            ended_at: None,
            note: None,
            saved: None,
            rebased_from: Some(found.id.clone()),
            ..found.clone()
        };
        // The new proposal and its decisions are in place before the old one is let go of, so that
        // a failure leaves the owner with both, and never with neither.
        if !decisions.is_empty() {
            fs::create_dir_all(dir.join(DECISIONS_DIR))
                .map_err(|e| StoreError::io(dir.join(DECISIONS_DIR), e))?;
            for decision in &decisions {
                append_line(&decisions_path(&dir, &carried.id), decision)?;
            }
        }
        write_proposal(&dir, &carried)?;
        write_proposal(
            &dir,
            &self.ended_as(
                &found,
                answer.state,
                Some(&format!("carried forward as {}", carried.id)),
            ),
        )?;
        Ok(ProposalRegistered {
            proposal: carried,
            created: true,
        })
    }
}
