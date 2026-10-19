// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A document being divided among specs, and the ledger that says where each paragraph of it went (ADR 0040).
//!
//! ```text
//! <root>/.sce-intakes/<intake-id>/    a folder of the shape of a spec's, and held by the same code
//!   spec.json, members/, originals/, originals.log, extractions/, extractions.log, .lock
//!   intake.json                       what the intake is: its key, who asked, the original and extraction
//!   ledgers/<digest>.json             every ledger saved, named by its own SHA-256, never rewritten
//!   reports/<digest>.json             SCE's count of a ledger, named the same way
//!   ledger.log                        one JSON line per save: number, digest, parent, report, complete
//!   ledger.head                       the digest of the current ledger
//!   bindings.log                      which spec a spec to be made was made as
//! ```
//!
//! An intake is about several specs, so it is kept in none of them: in the specs folder's root, in a folder that
//! begins with a dot, which no spec id does, so it is never listed as a spec. It is a fact and not soft state (the
//! ledger is the owner's work), so its head moves by a save that names its base, as every head of the folder does.
//!
//! **The original and the extraction are held by the code that holds a spec's** (`add_original`, `save_extraction`),
//! because the intake's folder has the shape of a spec's and is read as one through a store rooted at `.sce-intakes`.
//! They are copied into each destination spec when the division is confirmed (`give_intake_to_spec`), by hash, so a
//! copy that was made before is the same and is not made twice.
//!
//! **The store never runs the tools.** The ledger and SCE's count of it are made by the authoring tools and handed
//! over; the store holds the ledger to its form (`crate::ledger`) and the count to naming the ledger it is of, and
//! admits a proposal as a spec's part of the division only when its scope is exactly what the ledger gives that spec
//! (`SpecStore::admit_intake`). Confirming is the host's walk (it runs the tools for each spec); what the store
//! holds is that nothing is admitted that the ledger does not say.
//!
//! **No two locks are held at once.** The intake's lock is taken to start one, to save a ledger and to bind a spec,
//! and a spec's lock by the calls that write into the spec; none of them takes both, so there is no order to
//! deadlock in. A confirm that stops partway leaves some specs proposed and some not, and `read_intake` says which.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::original_store::{append_line, keep, read_log};
use super::proposal_store::{block_id_list, KEY_MAX};
use super::request_store::valid_name;
use super::{atomic_write, SpecId, SpecStore, LOCK_FILE, LOCK_WAIT};
use crate::clock::{utc_timestamp, Clock};
use crate::error::StoreError;
use crate::ledger::{Destination, Ledger};
use crate::lock;
use crate::original::{MemberDraft, OriginalDraft};
use crate::proposals::{IntakeLink, MAX_REPORT_BYTES};
use crate::revision::Revision;

const INTAKES_DIR: &str = ".sce-intakes";
const INTAKE_FILE: &str = "intake.json";
const LEDGERS_DIR: &str = "ledgers";
const REPORTS_DIR: &str = "reports";
const LEDGER_LOG: &str = "ledger.log";
const LEDGER_HEAD: &str = "ledger.head";
const BINDINGS_LOG: &str = "bindings.log";

/// The most a ledger may hold.
pub const MAX_LEDGER_BYTES: usize = 4 * 1024 * 1024;

/// What a caller says to start an intake.
#[derive(Debug, Clone)]
pub struct IntakeRegistration<'a> {
    /// What makes a repeated start the intake it repeats: a button pressed twice, a call sent again.
    pub key: &'a str,
    /// Where it was asked from: `gui`, or the name of a client.
    pub origin: &'a str,
    /// What the document is called, which the intake is called by.
    pub title: &'a str,
    /// The document, as an original (ADR 0032).
    pub draft: OriginalDraft,
    /// The path of the member of the original the extraction was read from.
    pub member: &'a str,
    /// The extraction, as the authoring tools wrote it, kept exactly as given.
    pub extraction: &'a str,
}

/// An intake as it is kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Intake {
    pub id: String,
    pub key: String,
    pub origin: String,
    pub title: String,
    pub created_at: String,
    /// The document, by the digest of its manifest.
    pub original: Revision,
    /// The member of it the extraction was read from.
    pub member: String,
    /// The extraction, by the hash of its text.
    pub extraction: Revision,
}

/// What starting an intake made.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IntakeStarted {
    pub intake: Intake,
    /// `false` when the key was one that started an intake before, and this is that intake.
    pub created: bool,
}

/// One save of a ledger, a line of the log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub n: u64,
    pub digest: Revision,
    pub parent: Option<Revision>,
    /// SCE's count of the ledger, by the hash of its text.
    pub report: Revision,
    /// What the count said: whether the ledger leaves nothing unplaced.
    pub complete: bool,
    pub at: String,
}

/// What saving a ledger made.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LedgerSaved {
    pub entry: LedgerEntry,
    /// `false` when the ledger was the head already and nothing was written.
    pub created: bool,
}

/// A spec to be made, and the spec it was made as.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Binding {
    pub title: String,
    pub spec: String,
    pub at: String,
}

/// What binding a spec to be made recorded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Bound {
    pub binding: Binding,
    /// `false` when the same binding was recorded before.
    pub created: bool,
}

/// The ledger the intake is at, with the texts it is made of.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LedgerHead {
    pub entry: LedgerEntry,
    pub text: String,
    pub report: String,
}

/// The proposal a spec of the division has, as the intake says it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IntakeProposal {
    pub id: String,
    pub state: String,
}

/// One allotment of the head ledger, and how far its spec has come.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IntakeDestination {
    pub to: Value,
    pub reason: String,
    pub blocks: usize,
    /// The spec it goes to: the one named, or the one a spec to be made was made as. Absent for what is left
    /// out, and for a spec to be made that has not been.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec: Option<String>,
    /// Whether that spec is gone (removed, or never made).
    pub spec_missing: bool,
    /// The newest proposal made under this ledger for the spec.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proposal: Option<IntakeProposal>,
}

/// An intake with what the screen asks of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IntakeView {
    pub intake: Intake,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ledger: Option<LedgerHead>,
    pub destinations: Vec<IntakeDestination>,
    pub bindings: Vec<Binding>,
}

/// What copying the document into a spec made.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IntakeGiven {
    pub original: Revision,
    pub extraction: Revision,
}

fn bad(message: impl Into<String>, detail: Value) -> StoreError {
    StoreError::refused("bad-ledger", message, detail)
}

fn read_file<T: for<'de> Deserialize<'de>>(path: &Path, what: &str) -> Result<T, StoreError> {
    let text = fs::read_to_string(path).map_err(|e| {
        if e.kind() == io::ErrorKind::NotFound {
            StoreError::corrupt(path, format!("the intake is missing its {what}"))
        } else {
            StoreError::io(path, e)
        }
    })?;
    serde_json::from_str(&text).map_err(|e| StoreError::corrupt(path, format!("not {what}: {e}")))
}

fn read_head(dir: &Path) -> Result<Option<Revision>, StoreError> {
    let path = dir.join(LEDGER_HEAD);
    match fs::read_to_string(&path) {
        Ok(text) => Revision::parse(text.trim())
            .map(Some)
            .map_err(|_| StoreError::corrupt(&path, "it does not hold a revision")),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(StoreError::io(&path, e)),
    }
}

/// An original a store holds, as a draft that adds the same original to another store: every member's bytes,
/// and a rendition naming the member it was made from by path, which is how a draft says it (a manifest says it
/// by hash). The same members under the same paths with the same roles and label are the same original, so
/// what is copied has the digest it had.
fn draft_of<C: Clock>(
    store: &SpecStore<C>,
    spec: &SpecId,
    original: &Revision,
) -> Result<OriginalDraft, StoreError> {
    let manifest = store.read_original(spec, original)?;
    let mut members = Vec::with_capacity(manifest.members.len());
    for member in &manifest.members {
        let bytes = store.read_original_member(spec, original, &member.path)?;
        let of = member.of.as_ref().and_then(|hash| {
            manifest
                .members
                .iter()
                .find(|other| other.hash == *hash)
                .map(|other| other.path.clone())
        });
        members.push(MemberDraft {
            path: member.path.clone(),
            role: member.role,
            bytes,
            of,
            renderer: member.renderer.clone(),
        });
    }
    Ok(OriginalDraft {
        label: manifest.label.clone(),
        members,
        missing: manifest.missing.clone(),
    })
}

/// A text kept under the hash of its bytes, read back and checked to be what it is named.
fn read_kept(folder: &Path, digest: &Revision, what: &str) -> Result<String, StoreError> {
    let path = folder.join(format!("{}.json", digest.as_str()));
    let text = fs::read_to_string(&path).map_err(|e| {
        if e.kind() == io::ErrorKind::NotFound {
            StoreError::corrupt(&path, format!("the intake names {what} that is not held"))
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
}

/// What a count of a ledger says of itself: what it is of, and whether it leaves nothing unplaced.
fn read_report(text: &str) -> Result<(String, String, bool), StoreError> {
    let value: Value = serde_json::from_str(text).map_err(|e| {
        bad(
            format!("the count of a ledger is not JSON ({e})"),
            Value::Null,
        )
    })?;
    let word = |field: &str| -> Result<String, StoreError> {
        value
            .get(field)
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| {
                bad(
                    format!("the count of a ledger says `{field}`: text"),
                    json!({ "field": field }),
                )
            })
    };
    let complete = value
        .get("complete")
        .and_then(Value::as_bool)
        .ok_or_else(|| {
            bad(
                "the count of a ledger says whether it is complete",
                Value::Null,
            )
        })?;
    Ok((word("extraction")?, word("ledger")?, complete))
}

impl<C: Clock> SpecStore<C> {
    /// The store the intakes are kept in: a specs folder of their own, rooted at `.sce-intakes`.
    fn intakes(&self) -> SpecStore<&C> {
        SpecStore::with_clock(self.root().join(INTAKES_DIR), &self.clock)
    }

    /// The folder of an intake that exists, and its id read as the spec id of the folder it is held as.
    fn intake_folder(&self, id: &str) -> Result<(SpecId, PathBuf), StoreError> {
        let spec = SpecId::parse(id)?;
        let dir = self.root().join(INTAKES_DIR).join(spec.as_str());
        // The intake exists when its record does: a folder made and not finished is not one.
        if !dir.join(INTAKE_FILE).is_file() {
            return Err(StoreError::NotFound {
                what: format!("intake `{id}`"),
            });
        }
        Ok((spec, dir))
    }

    fn read_intake_file(dir: &Path) -> Result<Intake, StoreError> {
        read_file(&dir.join(INTAKE_FILE), "an intake")
    }

    /// Start dividing a document: keep it as an original and its extraction, and make the intake that holds
    /// the ledger. The same key with the same document is the same intake.
    pub fn start_intake(
        &self,
        registration: IntakeRegistration<'_>,
    ) -> Result<IntakeStarted, StoreError> {
        if registration.key.is_empty() || registration.key.len() > KEY_MAX {
            return Err(StoreError::refused(
                "bad-proposal",
                "a registration key is 1 to 200 characters",
                json!({ "limit": KEY_MAX }),
            ));
        }
        if !valid_name(registration.origin) {
            return Err(StoreError::refused(
                "bad-proposal",
                "an origin is 1 to 64 letters, digits and `._:-`",
                Value::Null,
            ));
        }
        let original = registration.draft.clone().build()?.manifest.revision();
        let extraction = Revision::of(registration.extraction.as_bytes());

        let root = self.root().join(INTAKES_DIR);
        fs::create_dir_all(&root).map_err(|e| StoreError::io(&root, e))?;
        // One at a time across the folder, so that two presses of one key make one intake.
        let _held = lock::exclusive(&root.join(LOCK_FILE), LOCK_WAIT)?;
        for found in self.list_intakes()? {
            if found.key != registration.key {
                continue;
            }
            if found.original == original
                && found.extraction == extraction
                && found.member == registration.member
            {
                return Ok(IntakeStarted {
                    intake: found,
                    created: false,
                });
            }
            return Err(StoreError::refused(
                "key-reused",
                format!(
                    "the key was used for another intake ({}), made of another document",
                    found.id
                ),
                json!({ "intake": found.id }),
            ));
        }

        let held = self.intakes();
        let made = held.create_spec(registration.title)?;
        let dir = root.join(made.id.as_str());
        let built = (|| -> Result<Intake, StoreError> {
            let added = held.add_original(&made.id, registration.draft)?;
            let saved = held.save_extraction(
                &made.id,
                &added.digest,
                registration.member,
                registration.extraction,
            )?;
            let intake = Intake {
                id: made.id.as_str().to_string(),
                key: registration.key.to_string(),
                origin: registration.origin.to_string(),
                title: made.title.clone(),
                created_at: utc_timestamp(self.clock.epoch()),
                original: added.digest,
                member: registration.member.to_string(),
                extraction: saved.digest,
            };
            let mut bytes = serde_json::to_vec_pretty(&intake)
                .map_err(|e| StoreError::corrupt(&dir, e.to_string()))?;
            bytes.push(b'\n');
            // The record is written last: an intake exists once it does.
            atomic_write(&dir.join(INTAKE_FILE), &bytes)?;
            Ok(intake)
        })();
        match built {
            Ok(intake) => Ok(IntakeStarted {
                intake,
                created: true,
            }),
            Err(error) => {
                // A refused document leaves no folder behind.
                let _ = fs::remove_dir_all(&dir);
                Err(error)
            }
        }
    }

    /// Every intake, the newest first.
    pub fn list_intakes(&self) -> Result<Vec<Intake>, StoreError> {
        let root = self.root().join(INTAKES_DIR);
        let entries = match fs::read_dir(&root) {
            Ok(entries) => entries,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(StoreError::io(&root, e)),
        };
        let mut found = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| StoreError::io(&root, e))?;
            let path = entry.path().join(INTAKE_FILE);
            if path.is_file() {
                found.push(Self::read_intake_file(&entry.path())?);
            }
        }
        found.sort_by(|a, b| (&b.created_at, &b.id).cmp(&(&a.created_at, &a.id)));
        Ok(found)
    }

    /// The ledger of an intake as it is now, and how far each spec it gives paragraphs to has come.
    pub fn read_intake(&self, id: &str) -> Result<IntakeView, StoreError> {
        let (spec, dir) = self.intake_folder(id)?;
        let intake = Self::read_intake_file(&dir)?;
        let bindings = read_log::<Binding>(&dir.join(BINDINGS_LOG))?;
        let head = read_head(&dir)?;
        let Some(head) = head else {
            return Ok(IntakeView {
                intake,
                ledger: None,
                destinations: Vec::new(),
                bindings,
            });
        };
        let entry = read_log::<LedgerEntry>(&dir.join(LEDGER_LOG))?
            .into_iter()
            .rev()
            .find(|entry| entry.digest == head)
            .ok_or_else(|| {
                StoreError::corrupt(
                    dir.join(LEDGER_HEAD),
                    "the head names a ledger the log has no line of",
                )
            })?;
        let text = read_kept(&dir.join(LEDGERS_DIR), &entry.digest, "a ledger")?;
        let report = read_kept(&dir.join(REPORTS_DIR), &entry.report, "a count")?;
        let extraction_text = self.intakes().read_extraction(&spec, &intake.extraction)?;
        let order = block_id_list(&extraction_text, "extraction")?;
        let ledger = Ledger::read(&text, &intake.extraction, &order)?;
        let bound: Vec<(String, String)> = bindings
            .iter()
            .map(|b| (b.title.clone(), b.spec.clone()))
            .collect();
        let mut destinations = Vec::with_capacity(ledger.allotments.len());
        for allotment in &ledger.allotments {
            let spec_id = match &allotment.to {
                Destination::Spec(id) => Some(id.clone()),
                Destination::New(title) => bound
                    .iter()
                    .find(|(bound_title, _)| bound_title == title)
                    .map(|(_, spec)| spec.clone()),
                Destination::LeftOut => None,
            };
            let mut spec_missing = false;
            let mut proposal = None;
            if let Some(found) = &spec_id {
                match SpecId::parse(found).and_then(|sid| self.list_proposals(&sid)) {
                    Ok(proposals) => {
                        proposal = proposals
                            .into_iter()
                            .find(|p| {
                                p.intake
                                    .as_ref()
                                    .is_some_and(|link| link.id == id && link.ledger == head)
                            })
                            .map(|p| IntakeProposal {
                                id: p.id,
                                state: p.state.word().to_string(),
                            });
                    }
                    Err(StoreError::NotFound { .. }) => spec_missing = true,
                    Err(other) => return Err(other),
                }
            }
            destinations.push(IntakeDestination {
                to: allotment.to.to_value(),
                reason: allotment.reason.clone(),
                blocks: allotment.blocks.len(),
                spec: spec_id,
                spec_missing,
                proposal,
            });
        }
        Ok(IntakeView {
            intake,
            ledger: Some(LedgerHead {
                entry,
                text,
                report,
            }),
            destinations,
            bindings,
        })
    }

    /// Keep a ledger as the intake's head, with SCE's count of it.
    ///
    /// `base` is the ledger the caller read and is changing (none for the first): a head that has moved is the
    /// refusal every head gives (`conflict`), and nothing is written. A ledger that is the head already is
    /// said so and writes nothing. The ledger is held to its form and to the intake's extraction; the count
    /// is held to naming this ledger and this extraction, and its word on whether the ledger is complete is
    /// kept and not recomputed.
    pub fn save_ledger(
        &self,
        id: &str,
        ledger: &str,
        report: &str,
        base: Option<&Revision>,
    ) -> Result<LedgerSaved, StoreError> {
        for (what, bytes, limit) in [
            ("a ledger", ledger.len(), MAX_LEDGER_BYTES),
            ("a count of a ledger", report.len(), MAX_REPORT_BYTES),
        ] {
            if bytes > limit {
                return Err(StoreError::TooLarge { what, bytes, limit });
            }
        }
        let (spec, dir) = self.intake_folder(id)?;
        let intake = Self::read_intake_file(&dir)?;
        let extraction_text = self.intakes().read_extraction(&spec, &intake.extraction)?;
        let order = block_id_list(&extraction_text, "extraction")?;
        Ledger::read(ledger, &intake.extraction, &order)?;
        let digest = Revision::of(ledger.as_bytes());
        let (of_extraction, of_ledger, complete) = read_report(report)?;
        if of_extraction != intake.extraction.as_str() || of_ledger != digest.as_str() {
            return Err(bad(
                "the count is of another ledger or extraction than the ones given",
                json!({ "count_of": { "extraction": of_extraction, "ledger": of_ledger } }),
            ));
        }
        let report_digest = Revision::of(report.as_bytes());

        let _held = lock::exclusive(&dir.join(LOCK_FILE), LOCK_WAIT)?;
        let head = read_head(&dir)?;
        if head.as_ref() != base {
            return Err(StoreError::Conflict {
                base: base.cloned(),
                current: head,
            });
        }
        let log = read_log::<LedgerEntry>(&dir.join(LEDGER_LOG))?;
        if head.as_ref() == Some(&digest) {
            if let Some(entry) = log.iter().rev().find(|entry| entry.digest == digest) {
                return Ok(LedgerSaved {
                    entry: entry.clone(),
                    created: false,
                });
            }
        }
        keep(
            &dir.join(LEDGERS_DIR),
            &dir.join(LEDGERS_DIR)
                .join(format!("{}.json", digest.as_str())),
            &digest,
            ledger.as_bytes(),
        )?;
        keep(
            &dir.join(REPORTS_DIR),
            &dir.join(REPORTS_DIR)
                .join(format!("{}.json", report_digest.as_str())),
            &report_digest,
            report.as_bytes(),
        )?;
        let entry = LedgerEntry {
            n: log.len() as u64 + 1,
            digest: digest.clone(),
            parent: head,
            report: report_digest,
            complete,
            at: utc_timestamp(self.clock.epoch()),
        };
        append_line(&dir.join(LEDGER_LOG), &entry)?;
        atomic_write(
            &dir.join(LEDGER_HEAD),
            format!("{}\n", digest.as_str()).as_bytes(),
        )?;
        Ok(LedgerSaved {
            entry,
            created: true,
        })
    }

    /// Say which spec a spec to be made was made as, so that confirming again finds it and makes no second.
    ///
    /// The ledger the intake is at has to give a spec by that title, and the spec has to exist. A title is
    /// bound to one spec and a spec to one title: the same binding said again is the same, and another is
    /// refused (`already-bound`).
    pub fn bind_new_spec(&self, id: &str, title: &str, spec: &SpecId) -> Result<Bound, StoreError> {
        let (intake_spec, dir) = self.intake_folder(id)?;
        let intake = Self::read_intake_file(&dir)?;
        // The spec is a spec of the folder, and not removed.
        self.existing(spec)?;
        let _held = lock::exclusive(&dir.join(LOCK_FILE), LOCK_WAIT)?;
        let head = read_head(&dir)?.ok_or_else(|| {
            bad(
                "the intake has no ledger, so it gives no spec to be made",
                Value::Null,
            )
        })?;
        let text = read_kept(&dir.join(LEDGERS_DIR), &head, "a ledger")?;
        let extraction_text = self
            .intakes()
            .read_extraction(&intake_spec, &intake.extraction)?;
        let order = block_id_list(&extraction_text, "extraction")?;
        let ledger = Ledger::read(&text, &intake.extraction, &order)?;
        if !ledger
            .allotments
            .iter()
            .any(|a| a.to == Destination::New(title.to_string()))
        {
            return Err(bad(
                "the ledger gives no spec to be made by that title",
                json!({ "title": title }),
            ));
        }
        let bindings = read_log::<Binding>(&dir.join(BINDINGS_LOG))?;
        for held in &bindings {
            if held.title == title && held.spec == spec.as_str() {
                return Ok(Bound {
                    binding: held.clone(),
                    created: false,
                });
            }
            if held.title == title || held.spec == spec.as_str() {
                return Err(StoreError::refused(
                    "already-bound",
                    format!(
                        "`{}` was made as `{}` already, so it is not bound again",
                        held.title, held.spec
                    ),
                    json!({ "title": held.title, "spec": held.spec }),
                ));
            }
        }
        let binding = Binding {
            title: title.to_string(),
            spec: spec.as_str().to_string(),
            at: utc_timestamp(self.clock.epoch()),
        };
        append_line(&dir.join(BINDINGS_LOG), &binding)?;
        Ok(Bound {
            binding,
            created: true,
        })
    }

    /// Copy the document into a spec: its original and its extraction, by hash, so that what was copied before
    /// is the same and is not copied twice. The spec's own proposal is then made of them.
    pub fn give_intake_to_spec(&self, id: &str, spec: &SpecId) -> Result<IntakeGiven, StoreError> {
        let (intake_spec, dir) = self.intake_folder(id)?;
        let intake = Self::read_intake_file(&dir)?;
        let held = self.intakes();
        let added = self.add_original(spec, draft_of(&held, &intake_spec, &intake.original)?)?;
        if added.digest != intake.original {
            return Err(StoreError::corrupt(
                &dir,
                "the document copied into the spec is not the original the intake holds",
            ));
        }
        let text = held.read_extraction(&intake_spec, &intake.extraction)?;
        let saved = self.save_extraction(spec, &added.digest, &intake.member, &text)?;
        Ok(IntakeGiven {
            original: added.digest,
            extraction: saved.digest,
        })
    }

    /// Start dividing a document a spec already holds: the original and the extraction it kept, which is what a
    /// screen has when a person reading a document for one spec decides it holds several. Nothing is read again
    /// and no bytes are sent: the intake is made of what the spec has, and is the same intake the same press
    /// made before.
    pub fn start_intake_from_spec(
        &self,
        spec: &SpecId,
        original: &Revision,
        extraction: &Revision,
        key: &str,
        origin: &str,
        title: &str,
    ) -> Result<IntakeStarted, StoreError> {
        let entry = self
            .list_extractions(spec, Some(original))?
            .into_iter()
            .find(|entry| entry.digest == *extraction)
            .ok_or_else(|| {
                bad(
                    "the extraction is not one saved for this original",
                    json!({ "extraction": extraction }),
                )
            })?;
        let text = self.read_extraction(spec, extraction)?;
        let draft = draft_of(self, spec, original)?;
        self.start_intake(IntakeRegistration {
            key,
            origin,
            title,
            draft,
            member: &entry.member,
            extraction: &text,
        })
    }

    /// The extraction an intake holds, as the text it was kept as: what a screen divides by its outline.
    pub fn read_intake_extraction(&self, id: &str) -> Result<(Revision, String), StoreError> {
        let (spec, dir) = self.intake_folder(id)?;
        let intake = Self::read_intake_file(&dir)?;
        let text = self.intakes().read_extraction(&spec, &intake.extraction)?;
        Ok((intake.extraction, text))
    }

    /// Whether a proposal may be admitted as `spec`'s part of an intake's division, and why not.
    ///
    /// The proposal has to be of the intake's document, made under the ledger the intake is at, of a ledger SCE
    /// counted complete, and its scope has to be exactly the blocks the ledger gives the spec (none, when the
    /// ledger gives it every block, which is no scope). A proposal that says it is a part of a division and is not
    /// the part the ledger says would make the accounting a claim and not a count.
    pub(super) fn admit_intake(
        &self,
        link: &IntakeLink,
        spec: &SpecId,
        original: &Revision,
        extraction: &Revision,
        order: &[String],
        scope: Option<&[String]>,
    ) -> Result<(), StoreError> {
        let (_, dir) = self.intake_folder(&link.id)?;
        let intake = Self::read_intake_file(&dir)?;
        if intake.original != *original || intake.extraction != *extraction {
            return Err(StoreError::refused(
                "bad-proposal",
                "the proposal is not of the document the intake holds",
                json!({ "intake": link.id }),
            ));
        }
        let head = read_head(&dir)?;
        if head.as_ref() != Some(&link.ledger) {
            return Err(StoreError::refused(
                "ledger-moved",
                "the intake's ledger is not the one the proposal was made under; read the intake again",
                json!({ "intake": link.id, "current": head }),
            ));
        }
        let entry = read_log::<LedgerEntry>(&dir.join(LEDGER_LOG))?
            .into_iter()
            .rev()
            .find(|entry| entry.digest == link.ledger)
            .ok_or_else(|| {
                StoreError::corrupt(
                    dir.join(LEDGER_HEAD),
                    "the head names a ledger the log has no line of",
                )
            })?;
        if !entry.complete {
            return Err(StoreError::refused(
                "ledger-incomplete",
                "SCE did not count the intake's ledger complete, so nothing is proposed from it",
                json!({ "intake": link.id }),
            ));
        }
        let text = read_kept(&dir.join(LEDGERS_DIR), &link.ledger, "a ledger")?;
        let ledger = Ledger::read(&text, &intake.extraction, order)?;
        let bound: Vec<(String, String)> = read_log::<Binding>(&dir.join(BINDINGS_LOG))?
            .into_iter()
            .map(|b| (b.title, b.spec))
            .collect();
        let Some(allotment) = ledger.allotment_of(spec.as_str(), &bound) else {
            return Err(StoreError::refused(
                "bad-proposal",
                "the intake's ledger gives this spec nothing",
                json!({ "intake": link.id, "spec": spec.as_str() }),
            ));
        };
        let whole = allotment.blocks.len() == order.len();
        let fits = match scope {
            None => whole,
            Some(blocks) => !whole && blocks == allotment.blocks.as_slice(),
        };
        if !fits {
            return Err(StoreError::refused(
                "bad-proposal",
                "the proposal's scope is not what the intake's ledger gives this spec",
                json!({ "intake": link.id, "spec": spec.as_str(), "blocks": allotment.blocks.len() }),
            ));
        }
        Ok(())
    }
}
