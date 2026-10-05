// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Candidates and bundles: what a generation request makes, and how it becomes the work's.
//!
//! ```text
//! <work-id>/
//!   model/<digest>.scxml          every model that was ever written, candidates included
//!   requirements/<digest>.json    the same, for requirement lists
//!   bundles/<digest>.json         one file per published bundle, never rewritten
//!   bundles.head, bundles.log     the work's current bundle, and one line per publication
//! ```
//!
//! What an executor writes for a request is a CANDIDATE: a model and a requirement list,
//! written to the place every revision of them is kept, named by what they hold, and named in
//! the request. Writing a candidate moves no pointer. The work's model is still the one it was.
//!
//! PUBLISHING is the one step that makes a candidate the work's: under the lock a save takes,
//! the request is checked once more (still held by this attempt, still about the text and the
//! answers the work has), a bundle naming the pair is written, `bundles.head` moves to it, and
//! the request is written down as completed. A reader takes one pointer and has the model and
//! the list together.
//!
//! The core's own check is part of publishing, not something a client says it did: a bundle
//! without a check of the model that the core ran and accepted is refused. A check only a
//! client can run (the authoring package's decision record) is reported, and kept as reported.

use std::fs;
use std::path::Path;

use serde_json::json;

use super::request_store::{self, not_found, persist, refusal_error};
use super::{
    atomic_write, is_removed, read_pointer, removed_work, revision_path, Artifact, HistoryEntry,
    Pointer, WorkId, WorkStore, LOCK_FILE, LOCK_WAIT,
};
use crate::bundle::{Bundle, BundleCheck, CheckedBy, Previous, BUNDLE_FORMAT, BUNDLE_VERSION};
use crate::clock::Clock;
use crate::error::StoreError;
use crate::lock;
use crate::requests::{Candidate, Moment};
use crate::revision::Revision;

/// What an executor writes for a request: the texts as the store keeps them. A half left out
/// is left as it was named before.
#[derive(Debug, Clone, Default)]
pub struct CandidateWrite {
    pub model: Option<String>,
    pub requirements: Option<String>,
}

/// The candidate's texts, by revision.
#[derive(Debug, Clone, Default)]
pub struct CandidateTexts {
    pub model: Option<(Revision, String)>,
    pub requirements: Option<(Revision, String)>,
}

/// What publishing made: the request as completed, and the bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Published {
    pub request: super::RequestView,
    pub bundle: Revision,
}

/// A published bundle and the revision it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleRead {
    pub revision: Revision,
    pub bundle: Bundle,
}

/// Read the bundle named by `revision` from its file, checked against its name and its form.
fn read_bundle_file(dir: &Path, revision: &Revision) -> Result<Bundle, StoreError> {
    let path = revision_path(dir, Artifact::Bundles, revision);
    let bytes = fs::read(&path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            StoreError::corrupt(&path, "the current bundle has no file")
        } else {
            StoreError::io(&path, e)
        }
    })?;
    if Revision::of(&bytes) != *revision {
        return Err(StoreError::corrupt(
            &path,
            "its bytes do not hash to the name it is stored under",
        ));
    }
    let text = String::from_utf8(bytes)
        .map_err(|_| StoreError::corrupt(&path, "the bundle is not valid UTF-8"))?;
    Bundle::parse(&text).map_err(|e| StoreError::corrupt(&path, e.to_string()))
}

/// The bundle a pointer names, read from its file; `None` for a work that keeps none.
pub(super) fn bundle_at(
    dir: &Path,
    pointer: &Option<Pointer>,
) -> Result<Option<(Revision, Bundle)>, StoreError> {
    pointer
        .as_ref()
        .map(|p| Ok((p.revision.clone(), read_bundle_file(dir, &p.revision)?)))
        .transpose()
}

/// The work's current bundle, or `None` for a work that keeps none.
pub(super) fn current_bundle(dir: &Path) -> Result<Option<(Revision, Bundle)>, StoreError> {
    bundle_at(dir, &read_pointer(dir, Artifact::Bundles)?)
}

/// The text a published bundle says the model or the list `revision` was written for: the
/// most recent bundle that names it, or `None` when none does.
pub(super) fn claim_of_a_bundle(
    dir: &Path,
    artifact: Artifact,
    revision: &Revision,
) -> Result<Option<Revision>, StoreError> {
    if !matches!(artifact, Artifact::Model | Artifact::Requirements) {
        return Ok(None);
    }
    let entries = super::history_in(dir, Artifact::Bundles)?;
    for entry in entries.iter().rev() {
        let bundle = read_bundle_file(dir, &entry.revision)?;
        let named = match artifact {
            Artifact::Model => &bundle.model,
            _ => &bundle.requirements,
        };
        if named == revision {
            return Ok(Some(bundle.source));
        }
    }
    Ok(None)
}

/// The model's history after the chain it began as: one entry for each bundle, oldest
/// first, saying which text the model was written for. The first follows the last model the
/// chain held (its bundle says which, when the work had one).
pub(super) fn model_entries(dir: &Path) -> Result<Vec<HistoryEntry>, StoreError> {
    let mut entries = Vec::new();
    let mut before: Option<Revision> = None;
    for entry in super::history_in(dir, Artifact::Bundles)? {
        let bundle = read_bundle_file(dir, &entry.revision)?;
        let parent = before
            .clone()
            .or_else(|| bundle.previous.as_ref().and_then(|p| p.model.clone()));
        before = Some(bundle.model.clone());
        entries.push(HistoryEntry {
            revision: bundle.model,
            parent,
            saved_at: bundle.published_at,
            written_for: Some(bundle.source),
            unconfirmed: false,
        });
    }
    Ok(entries)
}

/// Keep `text` as a revision of `artifact` without moving any pointer: the file every
/// revision is, named by what it holds. A file already there that still hashes to its name is
/// kept as it is.
fn store_revision(dir: &Path, artifact: Artifact, text: &str) -> Result<Revision, StoreError> {
    if text.len() > artifact.max_bytes() {
        return Err(StoreError::TooLarge {
            what: artifact.noun(),
            bytes: text.len(),
            limit: artifact.max_bytes(),
        });
    }
    let revision = Revision::of(text.as_bytes());
    let folder = dir.join(artifact.dir());
    fs::create_dir_all(&folder).map_err(|e| StoreError::io(&folder, e))?;
    let path = revision_path(dir, artifact, &revision);
    let intact = fs::read(&path).is_ok_and(|bytes| Revision::of(&bytes) == revision);
    if !intact {
        atomic_write(&path, text.as_bytes())?;
    }
    Ok(revision)
}

fn bad_candidate(what: &str) -> StoreError {
    StoreError::refused("bad-candidate", what.to_string(), json!(null))
}

impl<C: Clock> WorkStore<C> {
    /// The executor writes a model, a requirement list, or both for the request.
    ///
    /// What it writes is kept, and named in the request, and is not the work's model: a
    /// candidate becomes that only by [`Self::publish_candidate`]. The executor says it as it
    /// says everything about a request, so it is the executor of the current attempt.
    pub fn save_candidate(
        &self,
        id: &WorkId,
        request: &str,
        holder: &str,
        attempt: u32,
        write: CandidateWrite,
    ) -> Result<super::RequestView, StoreError> {
        request_store::checked_holder(holder)?;
        if write.model.is_none() && write.requirements.is_none() {
            return Err(bad_candidate(
                "a candidate is a model, a requirement list, or both: nothing was written",
            ));
        }
        let dir = self.existing(id)?;
        let _held = lock::exclusive(&dir.join(LOCK_FILE), LOCK_WAIT)?;
        if is_removed(&dir) {
            return Err(removed_work(id));
        }
        let Some(stored) = request_store::find_request(&dir, request)? else {
            return Err(not_found(id, request));
        };
        let now = Moment::of(&self.clock);
        let current = request_store::heal(&dir, stored, &now)?;
        // The revisions the texts will have, before anything is written: a word from an
        // executor that is not the executor writes nothing.
        let named = Candidate {
            model: write.model.as_ref().map(|t| Revision::of(t.as_bytes())),
            requirements: write
                .requirements
                .as_ref()
                .map(|t| Revision::of(t.as_bytes())),
        };
        let next = current
            .with_candidate(holder, attempt, named)
            .map_err(|refusal| refusal_error(&current, refusal))?;
        if let Some(text) = &write.model {
            store_revision(&dir, Artifact::Model, text)?;
        }
        if let Some(text) = &write.requirements {
            store_revision(&dir, Artifact::Requirements, text)?;
        }
        persist(&dir, Some(&current), &next, &now)?;
        self.view(&dir, next)
    }

    /// What the executor has written for the request, as texts.
    pub fn read_candidate(&self, id: &WorkId, request: &str) -> Result<CandidateTexts, StoreError> {
        let dir = self.existing(id)?;
        let Some(found) = request_store::find_request(&dir, request)? else {
            return Err(not_found(id, request));
        };
        let candidate = found.candidate.unwrap_or_default();
        let read = |artifact: Artifact, named: &Option<Revision>| {
            named
                .as_ref()
                .map(|r| self.read_revision(&dir, artifact, id, r.clone(), true))
                .transpose()
        };
        Ok(CandidateTexts {
            model: read(Artifact::Model, &candidate.model)?,
            requirements: read(Artifact::Requirements, &candidate.requirements)?,
        })
    }

    /// The executor says it is done: its candidate becomes the work's model and requirement
    /// list, as one bundle, and the request is completed.
    ///
    /// `checks` are what was checked of the candidate. The core's own check of the model is
    /// required and must have been accepted, and so must every check a client reports: a bundle
    /// is the work's model, and one that was refused is not published. Said again by the same
    /// attempt, it is the same publication.
    pub fn publish_candidate(
        &self,
        id: &WorkId,
        request: &str,
        holder: &str,
        attempt: u32,
        checks: Vec<BundleCheck>,
    ) -> Result<Published, StoreError> {
        request_store::checked_holder(holder)?;
        let dir = self.existing(id)?;
        let _held = lock::exclusive(&dir.join(LOCK_FILE), LOCK_WAIT)?;
        if is_removed(&dir) {
            return Err(removed_work(id));
        }
        let Some(stored) = request_store::find_request(&dir, request)? else {
            return Err(not_found(id, request));
        };
        let now = Moment::of(&self.clock);
        // A publication that was made and not written down is written down now, and said
        // again it is that publication: the bundle is the work's already.
        let current = request_store::heal(&dir, stored, &now)?;
        if let Some(done) = current
            .outcome
            .as_ref()
            .filter(|_| current.state == crate::requests::State::Completed)
        {
            let bundle = done.bundle.clone();
            let next = current
                .publish(holder, attempt, &bundle, &now)
                .map_err(|refusal| refusal_error(&current, refusal))?;
            return Ok(Published {
                request: self.view(&dir, next)?,
                bundle,
            });
        }

        // Who is speaking, before what is said: a request that is not theirs, or ended, is
        // refused as that, whatever else is wrong with the call.
        current
            .check_held_by(holder, attempt)
            .map_err(|refusal| refusal_error(&current, refusal))?;
        let candidate = current.candidate.clone().unwrap_or_default();
        let missing = candidate.missing();
        if !missing.is_empty() {
            let words: Vec<&str> = missing
                .iter()
                .map(|half| match *half {
                    "model" => "a model",
                    _ => "a requirement list",
                })
                .collect();
            return Err(StoreError::refused(
                "no-candidate",
                format!(
                    "the request {} has not written {}, and a bundle is both",
                    current.id,
                    words.join(" or ")
                ),
                json!({ "request": current.id, "missing": missing }),
            ));
        }
        if !checks.iter().any(|c| c.by == CheckedBy::Core) {
            return Err(StoreError::refused(
                "no-core-check",
                "a bundle is published only with the check of its model that the core ran: a \
                 client's report of its own check is not one"
                    .to_string(),
                json!({ "request": current.id }),
            ));
        }
        // A check says what it came to for one text. One of the model or the list that is not
        // the text the candidate names now was run on a text that has since been written over.
        for check in &checks {
            let named = match check.name.as_str() {
                "model" => &candidate.model,
                "requirements" => &candidate.requirements,
                _ => continue,
            };
            if let (Some(checked), Some(named)) = (&check.subject, named) {
                if checked != named {
                    return Err(StoreError::refused(
                        "candidate-moved",
                        format!(
                            "the {} check was run on {}, and the request names {} now: check \
                             the candidate as it is",
                            check.name,
                            checked.short(),
                            named.short()
                        ),
                        json!({
                            "request": current.id,
                            "checked": checked,
                            "candidate": named,
                        }),
                    ));
                }
            }
        }
        let refused: Vec<&str> = checks
            .iter()
            .filter(|c| !c.is_accepted())
            .map(|c| c.name.as_str())
            .collect();
        if !refused.is_empty() {
            return Err(StoreError::refused(
                "check-refused",
                format!(
                    "the candidate was refused by {}, so it is not the work's model",
                    refused.join(" and ")
                ),
                json!({ "request": current.id, "checks": refused }),
            ));
        }

        // What the request was asked about is what the work has: a save that moved it ended
        // the request in the same step, and this is for the save that could not.
        let source = read_pointer(&dir, Artifact::Source)?.map(|p| p.revision);
        let answers = read_pointer(&dir, Artifact::Answers)?.map(|p| p.revision);
        if source.as_ref() != Some(&current.inputs.source) || answers != current.inputs.answers {
            if let Some(ended) =
                current.supersede("the work moved after the request was made", &now)
            {
                persist(&dir, Some(&current), &ended, &now)?;
            }
            return Err(refusal_error(
                &current,
                crate::requests::Refusal::Ended {
                    state: crate::requests::State::Superseded,
                },
            ));
        }

        let head = read_pointer(&dir, Artifact::Bundles)?;
        let previous = if head.is_none() {
            let model = read_pointer(&dir, Artifact::Model)?.map(|p| p.revision);
            let requirements = read_pointer(&dir, Artifact::Requirements)?.map(|p| p.revision);
            (model.is_some() || requirements.is_some()).then_some(Previous {
                model,
                requirements,
            })
        } else {
            None
        };
        let bundle = Bundle {
            format: BUNDLE_FORMAT.to_string(),
            v: BUNDLE_VERSION,
            request: current.id.clone(),
            attempt,
            executor: holder.to_string(),
            source: current.inputs.source.clone(),
            answers: current.inputs.answers.clone(),
            model: candidate.model.clone().expect("whole"),
            requirements: candidate.requirements.clone().expect("whole"),
            previous,
            checks,
            published_at: now.text.clone(),
        };
        let saved = self.save_text_locked(
            &dir,
            id,
            Artifact::Bundles,
            &bundle.stored_text(),
            head.as_ref().map(|p| &p.revision),
            None,
        )?;
        let revision = match saved {
            super::Saved::Saved { revision, .. } | super::Saved::Unchanged { revision } => revision,
        };
        let done = current
            .publish(holder, attempt, &revision, &now)
            .map_err(|refusal| refusal_error(&current, refusal))?;
        persist(&dir, Some(&current), &done, &now)?;
        Ok(Published {
            request: self.view(&dir, done)?,
            bundle: revision,
        })
    }

    /// The bundle `revision`, or the work's current one when none is named; `None` for a work
    /// that keeps none.
    pub fn read_bundle(
        &self,
        id: &WorkId,
        revision: Option<&Revision>,
    ) -> Result<Option<BundleRead>, StoreError> {
        let dir = self.existing(id)?;
        match revision {
            Some(named) => {
                let path = revision_path(&dir, Artifact::Bundles, named);
                if !path.is_file() {
                    return Err(StoreError::NotFound {
                        what: format!("bundle {} of `{id}`", named.short()),
                    });
                }
                Ok(Some(BundleRead {
                    revision: named.clone(),
                    bundle: read_bundle_file(&dir, named)?,
                }))
            }
            None => {
                Ok(current_bundle(&dir)?.map(|(revision, bundle)| BundleRead { revision, bundle }))
            }
        }
    }

    /// Every bundle that was published, oldest first.
    pub fn bundle_history(&self, id: &WorkId) -> Result<Vec<HistoryEntry>, StoreError> {
        self.history_of(Artifact::Bundles, id)
    }
}
