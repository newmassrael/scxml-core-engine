// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The works folder.
//!
//! ```text
//! <root>/
//!   <work-id>/
//!     work.json          the work's identity and title
//!     source/<digest>.txt   every saved text, named by its own SHA-256, never rewritten
//!     source.head        the digest of the current text, and the place of the save that
//!                        made it current in source.log (`log <n>`); an older pointer is
//!                        the digest alone, one line, and is read as it always was
//!     source.log         one JSON line per save: digest, parent digest, time, and the
//!                        place of the line of the save it followed (`parent_at`)
//!     model/<digest>.scxml  the same, for the SCXML model written from the text
//!     model.head         the digest of the current model, one line
//!     model.log          one JSON line per save, and the source revision it was written for
//!     answers/<digest>.json  the owner's answers to the model's open questions, the same way
//!     answers.head       the digest of the current answers
//!     answers.log        one JSON line per save
//!     requirements/<digest>.json  the requirement list the owner's text was read into, the same way
//!     requirements.head  the digest of the current list
//!     requirements.log   one JSON line per save, and the source revision it was written for
//!     acceptances/<digest>.json   what the owner accepted, one revision per acceptance
//!     acceptances.head   the digest of the current acceptance
//!     acceptances.log    one JSON line per acceptance
//!     .lock              what a save holds while it checks and moves a pointer
//!     removed.json       present only for a removed work (see [`WorkStore::remove_work`])
//! ```
//!
//! The files are the truth. Nothing else (an index, a database) may hold a fact
//! that the folder does not, because a folder that two processes write and a
//! second place that one of them updates will, sooner or later, disagree.
//!
//! A save is an optimistic compare-and-swap on the current digest. The caller
//! says which revision its text was written from (`base`), and the store refuses
//! a base that is no longer current rather than overwrite what it has not seen.
//!
//! A digest does not say which SAVE made it current. The same model kept for a later
//! source is saved again under the same digest, and a save that logged and then could
//! not move the pointer leaves a line that never took effect. So the pointer names the
//! save by its place in the log, in the one atomic write that makes it current; every
//! read (the history, the source a model was written for) starts from that line and
//! follows each save's `parent_at`, and a line off that path was never current.
//!
//! A pointer from before places were kept names no save, and a save that failed leaves the
//! same line a save that worked does, so nothing in such a folder says which of several
//! saves of one digest it meant. A read takes the last line of the digest, as it always
//! did, UNLESS those saves disagree about the text they were written for: then the claim is
//! nobody's to vouch for and is left unstated (a model reads as `behind`, never `current`).
//! A save into such a folder writes the place first and logs afterwards, so a failure of that
//! write stops it before the log holds a line, and the first save that works leaves the
//! folder in the form this build writes. A read takes the pointer, the log, and the pointer
//! again, and starts over if it moved, so a save landing between the two reads is not read
//! against the pointer it replaced. Where the walk had to pick one of such saves by position,
//! the entry it lists says so (`HistoryEntry::unconfirmed`), because a history is read as a
//! record of saves that took effect and cannot be given back for these.
//!
//! The text, the model, the owner's answers, the requirement list and the acceptances are
//! five chains kept by ONE implementation
//! ([`Artifact`] says where each lives and how large it may be), because two
//! implementations of "what a save is" would be two chances to disagree about
//! it. A model's save also records the source revision the writer read
//! (`written_for`), so that "this model is about the text on screen" is a
//! comparison of two digests and not a guess: when the text moves on, the model
//! is the model of the previous one, and the screen can say so.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::clock::{Clock, SystemClock};
use crate::error::StoreError;
use crate::lock;
use crate::revision::Revision;

/// The most a single source text may hold.
pub const MAX_SOURCE_BYTES: usize = 8 * 1024 * 1024;

/// The most a single model may hold.
pub const MAX_MODEL_BYTES: usize = 8 * 1024 * 1024;

/// The most one set of answers may hold.
pub const MAX_ANSWERS_BYTES: usize = 1024 * 1024;

/// The most one requirement list (its manifest and the sentences quoted) may hold.
pub const MAX_REQUIREMENTS_BYTES: usize = 4 * 1024 * 1024;

/// The most one acceptance (the product's record and what it was taken from) may hold.
pub const MAX_ACCEPTANCE_BYTES: usize = 4 * 1024 * 1024;

/// How long a save waits for another save of the same work.
pub const LOCK_WAIT: Duration = Duration::from_secs(30);

const WORK_FILE: &str = "work.json";
const LOCK_FILE: &str = ".lock";
const REMOVED_FILE: &str = "removed.json";
const WORK_FORMAT: &str = "sce-work";
const REMOVED_FORMAT: &str = "sce-work-removed";
const WORK_VERSION: u32 = 1;
const ID_SUFFIX_HEX: usize = 8;
const ID_SLUG_MAX: usize = 40;
const TITLE_MAX_CHARS: usize = 200;

/// The name of a work's folder: ASCII on purpose, so a title in any script and a
/// path on any filesystem never have to agree about encoding.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct WorkId(String);

impl WorkId {
    /// Accept an id only if it names a folder directly under the root: no
    /// separators, no dots, nothing a path could climb out through.
    pub fn parse(text: &str) -> Result<Self, StoreError> {
        let mut chars = text.chars();
        let first_ok = chars
            .next()
            .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
        let rest_ok = chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        if first_ok && rest_ok && text.len() <= 80 {
            Ok(WorkId(text.to_string()))
        } else {
            Err(StoreError::InvalidId {
                id: text.to_string(),
            })
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The readable part of the id: what a title became, without the eight
    /// hexadecimal characters that keep two works of one title apart. An id of
    /// another shape (one made by hand) is its own slug.
    pub fn slug(&self) -> &str {
        match self.0.rsplit_once('-') {
            Some((slug, suffix))
                if suffix.len() == ID_SUFFIX_HEX
                    && suffix.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f'))
                    && !slug.is_empty() =>
            {
                slug
            }
            _ => &self.0,
        }
    }
}

impl std::fmt::Display for WorkId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for WorkId {
    type Error = StoreError;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        WorkId::parse(&text)
    }
}

impl From<WorkId> for String {
    fn from(id: WorkId) -> String {
        id.0
    }
}

/// What a work keeps a chain of revisions of: where each lives in the work's
/// folder, and how large one may be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Artifact {
    /// The specification, in prose.
    Source,
    /// The SCXML model an authoring client wrote from it.
    Model,
    /// What the owner answered to the questions the model leaves open.
    Answers,
    /// The requirements the text states, as the authoring client read them out of it.
    Requirements,
    /// What the owner accepted: the product's record and what it was taken from.
    Acceptances,
}

impl Artifact {
    /// The folder holding every revision, each in a file named by its digest.
    fn dir(self) -> &'static str {
        match self {
            Artifact::Source => "source",
            Artifact::Model => "model",
            Artifact::Answers => "answers",
            Artifact::Requirements => "requirements",
            Artifact::Acceptances => "acceptances",
        }
    }

    fn extension(self) -> &'static str {
        match self {
            Artifact::Source => "txt",
            Artifact::Model => "scxml",
            Artifact::Answers | Artifact::Requirements | Artifact::Acceptances => "json",
        }
    }

    /// The file holding the digest of the current revision.
    fn head_file(self) -> &'static str {
        match self {
            Artifact::Source => "source.head",
            Artifact::Model => "model.head",
            Artifact::Answers => "answers.head",
            Artifact::Requirements => "requirements.head",
            Artifact::Acceptances => "acceptances.head",
        }
    }

    /// The file holding one line for each save.
    fn log_file(self) -> &'static str {
        match self {
            Artifact::Source => "source.log",
            Artifact::Model => "model.log",
            Artifact::Answers => "answers.log",
            Artifact::Requirements => "requirements.log",
            Artifact::Acceptances => "acceptances.log",
        }
    }

    fn max_bytes(self) -> usize {
        match self {
            Artifact::Source => MAX_SOURCE_BYTES,
            Artifact::Model => MAX_MODEL_BYTES,
            Artifact::Answers => MAX_ANSWERS_BYTES,
            Artifact::Requirements => MAX_REQUIREMENTS_BYTES,
            Artifact::Acceptances => MAX_ACCEPTANCE_BYTES,
        }
    }

    /// What it is called in a message.
    fn noun(self) -> &'static str {
        match self {
            Artifact::Source => "source",
            Artifact::Model => "model",
            Artifact::Answers => "answers",
            Artifact::Requirements => "requirements",
            Artifact::Acceptances => "acceptance",
        }
    }

    /// Whether a save of this chain says which source revision it was written for,
    /// so that the same text written for another revision is news and not "unchanged".
    fn is_written_for_a_source(self) -> bool {
        matches!(self, Artifact::Model | Artifact::Requirements)
    }
}

/// A work: its identity and the name a person gave it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Work {
    pub id: WorkId,
    pub title: String,
    pub created_at: String,
}

/// `work.json` as written. Unknown fields are kept out of the reading on purpose
/// and allowed in the file: a later version may add to it.
#[derive(Debug, Serialize, Deserialize)]
struct WorkFile {
    format: String,
    v: u32,
    id: WorkId,
    title: String,
    created_at: String,
}

/// `removed.json` as written: who reads it is a person restoring a work by hand,
/// so it says what it is and when. The store itself reads only that the file exists.
#[derive(Debug, Serialize)]
struct RemovedFile {
    format: &'static str,
    v: u32,
    removed_at: String,
}

/// A folder under the root that looked like a work and could not be read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Unreadable {
    pub id: String,
    pub reason: String,
}

/// Every work, and every folder that should have been one and was not readable.
///
/// The second list is not an afterthought: a work that silently disappears from
/// the list because its `work.json` was damaged looks, to the person, like a work
/// that was deleted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Listing {
    pub works: Vec<Work>,
    pub unreadable: Vec<Unreadable>,
}

/// One saved text and the revision it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourceText {
    pub revision: Revision,
    pub text: String,
}

/// One saved model, the revision it is, and the text it was written for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModelText {
    pub revision: Revision,
    /// The source revision the writer said it read, when it said.
    pub written_for: Option<Revision>,
    pub text: String,
}

/// The owner's answers as saved, and the revision they are.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AnswersText {
    pub revision: Revision,
    pub text: String,
}

/// One saved requirement list, the revision it is, and the text it was written for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RequirementsText {
    pub revision: Revision,
    /// The source revision the writer said it read, when it said.
    pub written_for: Option<Revision>,
    pub text: String,
}

/// One acceptance as saved, and the revision it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AcceptanceText {
    pub revision: Revision,
    pub text: String,
}

/// A work as it stood at one moment: each of its five chains at its head, the heads
/// read so that they belong together (see [`WorkStore::read_work_snapshot`]).
///
/// What is here is what is kept and nothing the product says of it: whether an
/// acceptance still holds is the product's to answer, and asking it is not a read
/// of the folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WorkSnapshot {
    pub work: Work,
    pub source: Option<SourceText>,
    pub model: Option<ModelText>,
    pub answers: Option<AnswersText>,
    pub requirements: Option<RequirementsText>,
    pub acceptance: Option<AcceptanceText>,
}

/// One line of a work's history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub revision: Revision,
    pub parent: Option<Revision>,
    pub saved_at: String,
    /// For a model: the source revision the writer said it read. Absent for a
    /// source, and from a log written before models existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub written_for: Option<Revision>,
    /// Set only on an entry the log cannot vouch for. A folder written before places were
    /// kept cannot say which of two saves of one revision took effect (a save that failed
    /// leaves the same line a save that worked does), and where those saves disagree about
    /// the text they were written for, the one listed here may be a save that failed. It is
    /// a reading of the log and is never written into it, and it stays on the entry after a
    /// later save has made the model current: a save of today does not make one of the past
    /// knowable.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub unconfirmed: bool,
}

/// What a save did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "kebab-case")]
pub enum Saved {
    /// A new revision is now current.
    Saved {
        revision: Revision,
        parent: Option<Revision>,
    },
    /// The text is the current revision's own: nothing was written.
    Unchanged { revision: Revision },
}

/// A works folder.
#[derive(Debug, Clone)]
pub struct WorkStore<C: Clock = SystemClock> {
    root: PathBuf,
    clock: C,
}

impl WorkStore<SystemClock> {
    /// The works folder at `root`. The folder is created by the first work.
    pub fn at(root: impl Into<PathBuf>) -> Self {
        WorkStore {
            root: root.into(),
            clock: SystemClock,
        }
    }
}

impl<C: Clock> WorkStore<C> {
    /// The same, stamping saves with `clock`.
    pub fn with_clock(root: impl Into<PathBuf>, clock: C) -> Self {
        WorkStore {
            root: root.into(),
            clock,
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn work_dir(&self, id: &WorkId) -> PathBuf {
        self.root.join(id.as_str())
    }

    /// The folder of an existing work, or `NotFound`. A removed work is not an
    /// existing one: every read and every save refuses it the way it refuses a
    /// work that was never made.
    fn existing(&self, id: &WorkId) -> Result<PathBuf, StoreError> {
        let dir = self.work_dir(id);
        if !dir.join(WORK_FILE).is_file() {
            return Err(StoreError::NotFound {
                what: format!("work `{}`", id.as_str()),
            });
        }
        if is_removed(&dir) {
            return Err(removed_work(id));
        }
        Ok(dir)
    }

    /// Take a work out of the folder's list, and refuse every later read and
    /// save of it. Answers the work it removed, so the caller can say which.
    ///
    /// The files stay where they are, with a `removed.json` beside them. A
    /// specification is a person's writing, and the one command that can end it
    /// should not be the one that cannot be taken back: deleting that file brings
    /// the work back whole, history included, and deleting the folder is the
    /// removal that cannot be taken back, done by the person who means it.
    ///
    /// The marker is written under the work's lock, and a save re-checks for it
    /// once it holds the lock. Without that a save that passed [`Self::existing`]
    /// just before the removal would write into a work the person was told is gone.
    pub fn remove_work(&self, id: &WorkId) -> Result<Work, StoreError> {
        let dir = self.existing(id)?;
        let _held = lock::exclusive(&dir.join(LOCK_FILE), LOCK_WAIT)?;
        // A removal that waited on the lock for another removal finds it done.
        if is_removed(&dir) {
            return Err(removed_work(id));
        }
        let work = self.read_work_file(id)?;
        let mut bytes = serde_json::to_vec_pretty(&RemovedFile {
            format: REMOVED_FORMAT,
            v: WORK_VERSION,
            removed_at: self.clock.now(),
        })
        .map_err(|e| StoreError::corrupt(&dir, e.to_string()))?;
        bytes.push(b'\n');
        atomic_write(&dir.join(REMOVED_FILE), &bytes)?;
        Ok(work)
    }

    /// Start a work. Its folder is created exclusively, so two creations never
    /// share one.
    pub fn create_work(&self, title: &str) -> Result<Work, StoreError> {
        let title = validate_title(title)?;
        fs::create_dir_all(&self.root).map_err(|e| StoreError::io(&self.root, e))?;
        let slug = slug_of(&title);
        for attempt in 0..8u64 {
            let id = WorkId::parse(&format!("{slug}-{}", id_suffix(&title, attempt)))?;
            let dir = self.work_dir(&id);
            match fs::create_dir(&dir) {
                Ok(()) => {
                    let work = Work {
                        id: id.clone(),
                        title: title.clone(),
                        created_at: self.clock.now(),
                    };
                    let file = WorkFile {
                        format: WORK_FORMAT.to_string(),
                        v: WORK_VERSION,
                        id,
                        title: work.title.clone(),
                        created_at: work.created_at.clone(),
                    };
                    let mut bytes = serde_json::to_vec_pretty(&file)
                        .map_err(|e| StoreError::corrupt(&dir, e.to_string()))?;
                    bytes.push(b'\n');
                    atomic_write(&dir.join(WORK_FILE), &bytes)?;
                    return Ok(work);
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(StoreError::io(&dir, e)),
            }
        }
        Err(StoreError::corrupt(
            &self.root,
            "eight generated ids were all taken, which a random suffix does not do",
        ))
    }

    /// Every work, oldest first, and what could not be read.
    pub fn list_works(&self) -> Result<Listing, StoreError> {
        let mut listing = Listing {
            works: Vec::new(),
            unreadable: Vec::new(),
        };
        let entries = match fs::read_dir(&self.root) {
            Ok(entries) => entries,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(listing),
            Err(e) => return Err(StoreError::io(&self.root, e)),
        };
        for entry in entries {
            let entry = entry.map_err(|e| StoreError::io(&self.root, e))?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let Ok(id) = WorkId::parse(&name) else {
                continue;
            };
            if !entry.path().is_dir() || is_removed(&entry.path()) {
                continue;
            }
            match self.read_work_file(&id) {
                Ok(work) => listing.works.push(work),
                Err(e) => listing.unreadable.push(Unreadable {
                    id: name,
                    reason: e.to_string(),
                }),
            }
        }
        listing
            .works
            .sort_by(|a, b| (&a.created_at, &a.id).cmp(&(&b.created_at, &b.id)));
        listing.unreadable.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(listing)
    }

    /// One work.
    pub fn read_work(&self, id: &WorkId) -> Result<Work, StoreError> {
        self.existing(id)?;
        self.read_work_file(id)
    }

    /// The work and every chain at its head, as ONE state of the work.
    ///
    /// A screen that reads a work with a command for each chain can be handed a text
    /// and a model the work never held together: a save lands between two of the reads.
    /// Here the pointer of every chain is read, then what the pointers name, then the
    /// pointers again, and the read starts over when any of them moved. What is returned
    /// is the folder as the second look at the pointers found it.
    ///
    /// No lock is taken, so a screen that reads on every change never makes a save wait.
    /// That is sound for the reason [`read_chain`] is: a pointer is replaced by one atomic
    /// rename and a revision's file is never rewritten. A folder that is written to
    /// between every look cannot be read that way, and after [`READ_TRIES`] the read takes
    /// the lock a save takes, so the answer is one state in that case too.
    pub fn read_work_snapshot(&self, id: &WorkId) -> Result<WorkSnapshot, StoreError> {
        self.snapshot_between(id, || {})
    }

    /// [`Self::read_work_snapshot`], with `between_reads` run after the first look at the
    /// pointers and before what they name is read: a seam for the test that makes writers
    /// land exactly there. It never runs while the lock is held, because a save made from
    /// it would wait for the reader to finish.
    fn snapshot_between(
        &self,
        id: &WorkId,
        mut between_reads: impl FnMut(),
    ) -> Result<WorkSnapshot, StoreError> {
        let dir = self.existing(id)?;
        for _ in 0..READ_TRIES {
            let pointers = read_pointers(&dir)?;
            between_reads();
            let snapshot = self.read_state(&dir, id, &pointers)?;
            if read_pointers(&dir)? == pointers {
                return Ok(snapshot);
            }
        }
        let _held = lock::exclusive(&dir.join(LOCK_FILE), LOCK_WAIT)?;
        // A removal that came first, while this read waited for the lock.
        if is_removed(&dir) {
            return Err(removed_work(id));
        }
        let pointers = read_pointers(&dir)?;
        self.read_state(&dir, id, &pointers)
    }

    /// The work and what each of `pointers` names. The two chains whose saves say which
    /// source they were written for carry that claim, read from the chain the pointer
    /// ends, as [`Self::read_claimed`] reads it.
    fn read_state(
        &self,
        dir: &Path,
        id: &WorkId,
        pointers: &Pointers,
    ) -> Result<WorkSnapshot, StoreError> {
        let [source, model, answers, requirements, acceptance] = pointers;
        let text = |artifact: Artifact, pointer: &Option<Pointer>| {
            pointer
                .as_ref()
                .map(|p| self.read_revision(dir, artifact, id, p.revision.clone(), false))
                .transpose()
        };
        let claimed = |artifact: Artifact, pointer: &Option<Pointer>| {
            pointer
                .as_ref()
                .map(|p| {
                    let (revision, text) =
                        self.read_revision(dir, artifact, id, p.revision.clone(), false)?;
                    let written_for = chain_to(dir, artifact, Some(p))?.claim();
                    Ok::<_, StoreError>((revision, written_for, text))
                })
                .transpose()
        };
        Ok(WorkSnapshot {
            work: self.read_work_file(id)?,
            source: text(Artifact::Source, source)?
                .map(|(revision, text)| SourceText { revision, text }),
            model: claimed(Artifact::Model, model)?.map(|(revision, written_for, text)| {
                ModelText {
                    revision,
                    written_for,
                    text,
                }
            }),
            answers: text(Artifact::Answers, answers)?
                .map(|(revision, text)| AnswersText { revision, text }),
            requirements: claimed(Artifact::Requirements, requirements)?.map(
                |(revision, written_for, text)| RequirementsText {
                    revision,
                    written_for,
                    text,
                },
            ),
            acceptance: text(Artifact::Acceptances, acceptance)?
                .map(|(revision, text)| AcceptanceText { revision, text }),
        })
    }

    fn read_work_file(&self, id: &WorkId) -> Result<Work, StoreError> {
        let path = self.work_dir(id).join(WORK_FILE);
        let bytes = fs::read(&path).map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                StoreError::corrupt(&path, "the folder has no work.json")
            } else {
                StoreError::io(&path, e)
            }
        })?;
        let file: WorkFile = serde_json::from_slice(&bytes)
            .map_err(|e| StoreError::corrupt(&path, e.to_string()))?;
        if file.format != WORK_FORMAT || file.v != WORK_VERSION {
            return Err(StoreError::corrupt(
                &path,
                format!(
                    "format `{}` v{}, expected `{WORK_FORMAT}` v{WORK_VERSION}",
                    file.format, file.v
                ),
            ));
        }
        if file.id != *id {
            return Err(StoreError::corrupt(
                &path,
                format!(
                    "the file says it is `{}` and sits in the folder of `{}`",
                    file.id.as_str(),
                    id.as_str()
                ),
            ));
        }
        Ok(Work {
            id: file.id,
            title: file.title,
            created_at: file.created_at,
        })
    }

    /// The revision that is current, or `None` for a work nothing was saved to.
    pub fn head(&self, id: &WorkId) -> Result<Option<Revision>, StoreError> {
        let dir = self.existing(id)?;
        read_head(&dir, Artifact::Source)
    }

    /// The model that is current, or `None` for a work no model was saved to.
    pub fn model_head(&self, id: &WorkId) -> Result<Option<Revision>, StoreError> {
        let dir = self.existing(id)?;
        read_head(&dir, Artifact::Model)
    }

    /// The text of `revision`, or of the current one when none is named.
    /// `None` only for a work with no text yet.
    ///
    /// The bytes read are hashed and compared with the name they were stored
    /// under, so a file that was edited by hand, truncated by a crash or damaged
    /// by a disk is refused instead of being handed over as the revision it
    /// claims to be.
    pub fn read_source(
        &self,
        id: &WorkId,
        revision: Option<&Revision>,
    ) -> Result<Option<SourceText>, StoreError> {
        Ok(self
            .read_text(Artifact::Source, id, revision)?
            .map(|(revision, text)| SourceText { revision, text }))
    }

    /// The model `revision`, or the current one when none is named, with the
    /// source revision it was written for. `None` only for a work with no model.
    ///
    /// Read exactly as a source is: the bytes must hash to the name they are
    /// stored under.
    pub fn read_model(
        &self,
        id: &WorkId,
        revision: Option<&Revision>,
    ) -> Result<Option<ModelText>, StoreError> {
        Ok(self.read_claimed(Artifact::Model, id, revision)?.map(
            |(revision, written_for, text)| ModelText {
                revision,
                written_for,
                text,
            },
        ))
    }

    /// A revision of a chain whose saves say which source revision they were written
    /// for (the model's and the requirement list's), with that claim.
    ///
    /// What the writer said it read is in the log line of the save that made this
    /// revision current; a revision no save on the chain names (one a failed save
    /// left behind) says nothing.
    ///
    /// The pointer and the chain come from ONE snapshot (`read_chain`), and the text is the
    /// one that pointer names. Read separately, a save landing between the reads would hand
    /// over the text of one pointer with the claim of the next.
    ///
    /// The current revision's claim is the chain's (`Chain::claim`), which says nothing when
    /// an older folder cannot show which of two saves of it took effect; an earlier
    /// revision's is what the chain's save of it says.
    fn read_claimed(
        &self,
        artifact: Artifact,
        id: &WorkId,
        revision: Option<&Revision>,
    ) -> Result<Option<(Revision, Option<Revision>, String)>, StoreError> {
        let dir = self.existing(id)?;
        let Snapshot { pointer, chain } = read_chain(&dir, artifact, || {})?;
        let wanted = match (revision, pointer.as_ref()) {
            (Some(named), _) => named.clone(),
            (None, Some(pointer)) => pointer.revision.clone(),
            (None, None) => return Ok(None),
        };
        let (revision, text) =
            self.read_revision(&dir, artifact, id, wanted, revision.is_some())?;
        let written_for = if pointer.as_ref().is_some_and(|p| p.revision == revision) {
            chain.claim()
        } else {
            chain
                .entries
                .iter()
                .rev()
                .find(|entry| entry.revision == revision)
                .and_then(|entry| entry.written_for.clone())
        };
        Ok(Some((revision, written_for, text)))
    }

    fn read_text(
        &self,
        artifact: Artifact,
        id: &WorkId,
        revision: Option<&Revision>,
    ) -> Result<Option<(Revision, String)>, StoreError> {
        let dir = self.existing(id)?;
        let wanted = match revision {
            Some(revision) => revision.clone(),
            None => match read_head(&dir, artifact)? {
                Some(head) => head,
                None => return Ok(None),
            },
        };
        self.read_revision(&dir, artifact, id, wanted, revision.is_some())
            .map(Some)
    }

    /// The text of `wanted`, checked against its name. `named` says whether the caller
    /// asked for this revision (one that is missing is then not found) or was handed it
    /// as the current one (one that is missing is then a damaged folder).
    fn read_revision(
        &self,
        dir: &Path,
        artifact: Artifact,
        id: &WorkId,
        wanted: Revision,
        named: bool,
    ) -> Result<(Revision, String), StoreError> {
        let path = revision_path(dir, artifact, &wanted);
        let bytes = fs::read(&path).map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                if named {
                    StoreError::NotFound {
                        what: format!(
                            "{} revision {} of `{}`",
                            artifact.noun(),
                            wanted.short(),
                            id.as_str()
                        ),
                    }
                } else {
                    StoreError::corrupt(&path, "the current revision has no file")
                }
            } else {
                StoreError::io(&path, e)
            }
        })?;
        if Revision::of(&bytes) != wanted {
            return Err(StoreError::corrupt(
                &path,
                "its bytes do not hash to the name it is stored under",
            ));
        }
        let text = String::from_utf8(bytes)
            .map_err(|_| StoreError::corrupt(&path, "the text is not valid UTF-8"))?;
        Ok((wanted, text))
    }

    /// Save `text` as the work's next revision, from `base`.
    ///
    /// `base` is the revision the caller's text was written from: `None` for the
    /// first text of a work, otherwise the revision it read. It must be the
    /// current one. A `None` for a work that already has text is refused too,
    /// because a caller that never read the work has not seen what it would
    /// replace.
    pub fn save_source(
        &self,
        id: &WorkId,
        text: &str,
        base: Option<&Revision>,
    ) -> Result<Saved, StoreError> {
        self.save_text(Artifact::Source, id, text, base, None)
    }

    /// Save `model` as the work's next model, from `base`, written for the
    /// source revision `written_for`.
    ///
    /// `base` means what it means for a source. `written_for` is the source
    /// revision the writer read the specification at (`None` when it cannot
    /// say); it must be a revision of this work's text, because a model that
    /// claims to be about a text nobody saved is about nothing.
    ///
    /// The same model saved again for a DIFFERENT source revision is a new
    /// entry of the history and not an `Unchanged`: the text moved on, the
    /// writer read it again and kept the model, and that is a fact about the
    /// model the screen shows (it is no longer the model of the previous text).
    pub fn save_model(
        &self,
        id: &WorkId,
        model: &str,
        base: Option<&Revision>,
        written_for: Option<&Revision>,
    ) -> Result<Saved, StoreError> {
        self.save_text(Artifact::Model, id, model, base, written_for)
    }

    /// The answers `revision`, or the current ones when none is named. `None` only
    /// for a work the owner has answered nothing of. Read as a source is.
    pub fn read_answers(
        &self,
        id: &WorkId,
        revision: Option<&Revision>,
    ) -> Result<Option<AnswersText>, StoreError> {
        Ok(self
            .read_text(Artifact::Answers, id, revision)?
            .map(|(revision, text)| AnswersText { revision, text }))
    }

    /// Save `answers` as the work's next answers, from `base`, which means what it
    /// means for a source. What the text says (a question's id and the owner's
    /// words) is [`crate::answers`]'s to define; the store keeps text.
    pub fn save_answers(
        &self,
        id: &WorkId,
        answers: &str,
        base: Option<&Revision>,
    ) -> Result<Saved, StoreError> {
        self.save_text(Artifact::Answers, id, answers, base, None)
    }

    /// The requirement list `revision`, or the current one, with the source revision it
    /// was written for. `None` only for a work nobody has read requirements out of.
    pub fn read_requirements(
        &self,
        id: &WorkId,
        revision: Option<&Revision>,
    ) -> Result<Option<RequirementsText>, StoreError> {
        Ok(self
            .read_claimed(Artifact::Requirements, id, revision)?
            .map(|(revision, written_for, text)| RequirementsText {
                revision,
                written_for,
                text,
            }))
    }

    /// Save a requirement list as the work's next, from `base`, written for the source
    /// revision `written_for`. A list is quotes anchored in one text, so it is about
    /// that revision: the same list saved for another revision is news, as a model's is.
    /// What the text says is [`crate::requirements`]'s to define; the store keeps text.
    pub fn save_requirements(
        &self,
        id: &WorkId,
        requirements: &str,
        base: Option<&Revision>,
        written_for: Option<&Revision>,
    ) -> Result<Saved, StoreError> {
        self.save_text(Artifact::Requirements, id, requirements, base, written_for)
    }

    /// The acceptance `revision`, or the current one. `None` only for a work the
    /// owner has accepted nothing of.
    pub fn read_acceptance(
        &self,
        id: &WorkId,
        revision: Option<&Revision>,
    ) -> Result<Option<AcceptanceText>, StoreError> {
        Ok(self
            .read_text(Artifact::Acceptances, id, revision)?
            .map(|(revision, text)| AcceptanceText { revision, text }))
    }

    /// Record an acceptance as the work's next, from `base`. An acceptance is an
    /// event: the same record taken twice (it names the time) is two acceptances,
    /// and a `base` that is not the latest is refused like any other.
    pub fn save_acceptance(
        &self,
        id: &WorkId,
        acceptance: &str,
        base: Option<&Revision>,
    ) -> Result<Saved, StoreError> {
        self.save_text(Artifact::Acceptances, id, acceptance, base, None)
    }

    /// Every acceptance, oldest first, read as [`Self::history`] reads a text's.
    pub fn acceptance_history(&self, id: &WorkId) -> Result<Vec<HistoryEntry>, StoreError> {
        self.history_of(Artifact::Acceptances, id)
    }

    /// The time the store would stamp a save with, so a caller that stamps what it
    /// keeps inside a text uses the same clock the log does.
    pub fn now(&self) -> String {
        self.clock.now()
    }

    fn save_text(
        &self,
        artifact: Artifact,
        id: &WorkId,
        text: &str,
        base: Option<&Revision>,
        written_for: Option<&Revision>,
    ) -> Result<Saved, StoreError> {
        if text.len() > artifact.max_bytes() {
            return Err(StoreError::TooLarge {
                what: artifact.noun(),
                bytes: text.len(),
                limit: artifact.max_bytes(),
            });
        }
        let dir = self.existing(id)?;
        // The check and the move of the pointer are one step under this lock.
        // Without it two saves from one base both pass the check and the later
        // silently replaces the earlier.
        let _held = lock::exclusive(&dir.join(LOCK_FILE), LOCK_WAIT)?;
        // A removal that came first, while this save waited for the lock.
        if is_removed(&dir) {
            return Err(removed_work(id));
        }

        if let Some(source) = written_for {
            if !revision_path(&dir, Artifact::Source, source).is_file() {
                return Err(StoreError::NotFound {
                    what: format!(
                        "the source revision {} of `{}` that the {} names as written_for",
                        source.short(),
                        id.as_str(),
                        artifact.noun()
                    ),
                });
            }
        }
        let mut pointer = read_pointer(&dir, artifact)?;
        let current = pointer.as_ref().map(|p| p.revision.clone());
        if current.as_ref() != base {
            return Err(StoreError::Conflict {
                base: base.cloned(),
                current,
            });
        }
        let revision = Revision::of(text.as_bytes());
        if current.as_ref() == Some(&revision) {
            // A text is unchanged when it is the current one. A model is too
            // only if it is also written for the same text: the same model for
            // another text is the writer's news (see `save_model`). A claim an
            // older folder cannot vouch for is no claim, so it is never "the same".
            let same_claim = !artifact.is_written_for_a_source()
                || chain_to(&dir, artifact, pointer.as_ref())?.claim().as_ref() == written_for;
            if same_claim {
                return Ok(Saved::Unchanged { revision });
            }
        }

        // An older folder's pointer is the digest alone. Before this save is logged,
        // say which save of that digest the pointer meant -- so a save that then fails
        // leaves a line the pointer visibly does not name, and a failure of THIS write
        // stops the save before anything is logged. Left unsaid when the log cannot
        // say it either (two saves that disagree, or none): a guess pinned into the
        // pointer would turn a save that failed into the one that took effect.
        if let Some(older) = pointer.as_ref().filter(|p| p.entry.is_none()) {
            let pinned =
                chain_to(&dir, artifact, Some(older)).is_ok_and(|chain| !chain.head_unconfirmed());
            if pinned {
                pointer = Some(name_the_place(&dir, artifact, older)?);
            }
        }

        // The text first, then the log, then the pointer. The pointer is what
        // makes a save take effect, so a save that stops anywhere before it
        // (a crash, or the pointer's own write failing) leaves the previous
        // revision current. It can leave an unreferenced file, or a log line
        // for a revision that never became current; `history` follows the
        // pointer back through the log and does not list that line.
        let revisions = dir.join(artifact.dir());
        fs::create_dir_all(&revisions).map_err(|e| StoreError::io(&revisions, e))?;
        let path = revision_path(&dir, artifact, &revision);
        // A file already stored under this digest is reused only if it still
        // hashes to it. One that does not (damaged since it was written) is
        // replaced, because the pointer is about to name it.
        let intact = fs::read(&path).is_ok_and(|bytes| Revision::of(&bytes) == revision);
        if !intact {
            atomic_write(&path, text.as_bytes())?;
        }
        let line = LogLine {
            entry: HistoryEntry {
                revision: revision.clone(),
                parent: current.clone(),
                saved_at: self.clock.now(),
                written_for: written_for.cloned(),
                unconfirmed: false,
            },
            // The save this one follows is the one the pointer names, which is not
            // always the last line of its revision.
            parent_at: pointer.as_ref().and_then(|p| p.entry),
        };
        let place = append_log(&dir.join(artifact.log_file()), &line)?;
        // The pointer and the place of the save it makes current move together.
        atomic_write(
            &dir.join(artifact.head_file()),
            pointer_file_text(&revision, place).as_bytes(),
        )?;
        Ok(Saved::Saved {
            revision,
            parent: current,
        })
    }

    /// Every save that took effect, oldest first.
    ///
    /// A save takes effect when the pointer moves to it, which happens after it
    /// is logged. So the log can hold a line for a save that did not take effect
    /// (the pointer's write failed, or the process stopped between the two), and
    /// listing every line would show a save the caller was told had failed. The
    /// history is therefore the chain from the current revision back through
    /// each save's `parent`, which is exactly the saves the pointer moved
    /// through; a line off that chain is left out.
    ///
    /// The chain is followed by position in the log and not by revision alone,
    /// because a revision can recur (the text saved, changed, then saved again),
    /// and a failed save of a text that is later saved successfully leaves two
    /// lines for it, of which the later one is the save that took effect.
    ///
    /// In a folder written before places were kept that cannot be told, and where the saves of
    /// one revision disagree about the text they were written for, the one picked is marked
    /// `unconfirmed` and stays marked.
    ///
    /// A final line without its newline that does not parse is a save that was
    /// interrupted while it was being logged and is left out. Any other line that
    /// does not parse is damage, and says so; so does a current revision the
    /// log never recorded.
    pub fn history(&self, id: &WorkId) -> Result<Vec<HistoryEntry>, StoreError> {
        self.history_of(Artifact::Source, id)
    }

    /// Every model save that took effect, oldest first, read as [`Self::history`]
    /// reads a text's; each entry also says the source revision the model was
    /// written for.
    pub fn model_history(&self, id: &WorkId) -> Result<Vec<HistoryEntry>, StoreError> {
        self.history_of(Artifact::Model, id)
    }

    fn history_of(&self, artifact: Artifact, id: &WorkId) -> Result<Vec<HistoryEntry>, StoreError> {
        history_in(&self.existing(id)?, artifact)
    }
}

/// Whether the work folder `dir` carries a removal marker.
fn is_removed(dir: &Path) -> bool {
    dir.join(REMOVED_FILE).is_file()
}

fn removed_work(id: &WorkId) -> StoreError {
    StoreError::NotFound {
        what: format!("work `{}` (it was removed)", id.as_str()),
    }
}

/// The history of `artifact` in the work's folder `dir`.
fn history_in(dir: &Path, artifact: Artifact) -> Result<Vec<HistoryEntry>, StoreError> {
    Ok(read_chain(dir, artifact, || {})?.chain.entries)
}

/// How many times a read starts over because the pointer moved under it. A pointer that
/// keeps moving is a folder under constant writing; the last read stands.
const READ_TRIES: usize = 5;

/// A pointer and the chain it names, read so that they belong together.
struct Snapshot {
    pointer: Option<Pointer>,
    chain: Chain,
}

/// Read the pointer, then the log, then the pointer again, and start over if the pointer
/// moved: what is returned is the chain of a pointer that was still the pointer after the
/// log was read. Without the second look, a save that landed between the two reads
/// (or one that logged and failed) is read against a pointer that has already been
/// replaced -- and for a pointer that names no place (an older folder's) nothing in the
/// log says which of several saves of one digest it meant.
///
/// `between_reads` runs after the first pointer and before the log: a seam for the test that
/// makes a writer land exactly there, which cannot be arranged from outside the process.
fn read_chain(
    dir: &Path,
    artifact: Artifact,
    mut between_reads: impl FnMut(),
) -> Result<Snapshot, StoreError> {
    let mut tries = 0;
    loop {
        let pointer = read_pointer(dir, artifact)?;
        between_reads();
        let chain = chain_to(dir, artifact, pointer.as_ref())?;
        let after = read_pointer(dir, artifact)?;
        tries += 1;
        if after == pointer || tries >= READ_TRIES {
            return Ok(Snapshot { pointer, chain });
        }
    }
}

/// The saves that took effect up to the one a pointer names, oldest first.
struct Chain {
    /// Each entry says whether the log can vouch for it (`HistoryEntry::unconfirmed`).
    entries: Vec<HistoryEntry>,
}

impl Chain {
    /// Whether the save the chain ends at is one the log cannot vouch for.
    fn head_unconfirmed(&self) -> bool {
        self.entries.last().is_some_and(|entry| entry.unconfirmed)
    }

    /// The source revision the save the chain ends at was written for, or `None` when it
    /// says none or when it cannot be vouched for: what the last of two saves that
    /// disagree says is a claim nobody can vouch for.
    fn claim(&self) -> Option<Revision> {
        if self.head_unconfirmed() {
            return None;
        }
        self.entries
            .last()
            .and_then(|entry| entry.written_for.clone())
    }
}

/// Whether the saves of `revision` among `lines` disagree about the text they were written
/// for. Where the log names no place (an older folder), they are candidates for one save the
/// walk has to pick by position, and a pick between saves that disagree is a guess.
fn saves_disagree(lines: &[LogLine], revision: &Revision) -> bool {
    let mut claims = lines
        .iter()
        .filter(|l| l.entry.revision == *revision)
        .map(|l| &l.entry.written_for);
    claims
        .next()
        .is_some_and(|first| claims.any(|other| other != first))
}

/// The log's saves in the order they were appended. A final line cut short is left out.
fn read_log(dir: &Path, artifact: Artifact) -> Result<Vec<LogLine>, StoreError> {
    let path = dir.join(artifact.log_file());
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(StoreError::io(&path, e)),
    };
    let ends_cleanly = text.ends_with('\n');
    let raw: Vec<&str> = text.lines().collect();
    let mut lines = Vec::with_capacity(raw.len());
    for (index, line) in raw.iter().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<LogLine>(line) {
            Ok(parsed) => lines.push(parsed),
            Err(_) if !ends_cleanly && index + 1 == raw.len() => {}
            Err(e) => {
                return Err(StoreError::corrupt(
                    &path,
                    format!("line {} is not a save record: {e}", index + 1),
                ))
            }
        }
    }
    Ok(lines)
}

/// The saves that took effect up to the one `pointer` names, oldest first.
///
/// `pointer` is the one the caller read, and the answer is the chain AS OF that
/// pointer: the log only grows, and a line it gained after the pointer was read (a
/// save that landed, or one that logged and failed) is not part of it, because the
/// walk starts at the line the pointer names and goes back from there.
fn chain_to(
    dir: &Path,
    artifact: Artifact,
    pointer: Option<&Pointer>,
) -> Result<Chain, StoreError> {
    let path = dir.join(artifact.log_file());
    let lines = read_log(dir, artifact)?;

    // Nothing has taken effect until the pointer exists, whatever the log holds.
    let Some(pointer) = pointer else {
        return Ok(Chain {
            entries: Vec::new(),
        });
    };
    let missing = |revision: &Revision| {
        StoreError::corrupt(
            &path,
            format!(
                "revision {} is on the way back from the current one and no save record names it",
                revision.short()
            ),
        )
    };
    // The save the pointer names, by its place; a pointer without one (or one that
    // does not lead to its own revision) is read as before, by the last line of it --
    // and then every other save of that revision is a candidate for the one it meant.
    let named_by_place = match pointer.entry {
        Some(place)
            if lines
                .get(place)
                .is_some_and(|l| l.entry.revision == pointer.revision) =>
        {
            Some(place)
        }
        _ => None,
    };
    let mut at = match named_by_place {
        Some(place) => place,
        None => lines
            .iter()
            .rposition(|l| l.entry.revision == pointer.revision)
            .ok_or_else(|| missing(&pointer.revision))?,
    };
    // A save picked by position among several that disagree is one the log cannot vouch for;
    // one named by a place is vouched for by the pointer or the save that followed it.
    let mut unconfirmed = named_by_place.is_none() && saves_disagree(&lines, &pointer.revision);
    let mut chain = Vec::new();
    loop {
        let line = &lines[at];
        chain.push(HistoryEntry {
            unconfirmed,
            ..line.entry.clone()
        });
        let Some(parent) = line.entry.parent.as_ref() else {
            break;
        };
        // The save this one followed, by its place; failing that, the last line of
        // that revision before this one, as a log from before places were kept needs.
        (at, unconfirmed) = match line.parent_at {
            Some(place) if place < at && lines[place].entry.revision == *parent => (place, false),
            _ => (
                lines[..at]
                    .iter()
                    .rposition(|l| l.entry.revision == *parent)
                    .ok_or_else(|| missing(parent))?,
                saves_disagree(&lines[..at], parent),
            ),
        };
    }
    chain.reverse();
    Ok(Chain { entries: chain })
}

/// Where the default works folder is: `SCE_WORKS_DIR` when set, otherwise the
/// platform's per-user data directory.
///
/// One definition, read by every process that opens the folder, so the app and
/// the MCP do not each guess a different place.
pub fn default_root() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("SCE_WORKS_DIR").filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(dir));
    }
    let data = if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support"))
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
    };
    data.map(|dir| dir.join("sce-workbench").join("works"))
}

fn validate_title(title: &str) -> Result<String, StoreError> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return Err(StoreError::InvalidTitle {
            reason: "it is empty".to_string(),
        });
    }
    if trimmed.chars().count() > TITLE_MAX_CHARS {
        return Err(StoreError::InvalidTitle {
            reason: format!("it is longer than {TITLE_MAX_CHARS} characters"),
        });
    }
    if trimmed.chars().any(char::is_control) {
        return Err(StoreError::InvalidTitle {
            reason: "it holds a control character".to_string(),
        });
    }
    Ok(trimmed.to_string())
}

/// The readable part of a work's folder name: the ASCII letters and digits of
/// the title, hyphen-joined, or `work` when the title has none (a title in
/// another script keeps its own name; only the folder is plain).
fn slug_of(title: &str) -> String {
    let mut slug = String::new();
    let mut pending_hyphen = false;
    for c in title.chars() {
        if c.is_ascii_alphanumeric() {
            if pending_hyphen && !slug.is_empty() {
                slug.push('-');
            }
            pending_hyphen = false;
            slug.push(c.to_ascii_lowercase());
        } else {
            pending_hyphen = true;
        }
        if slug.len() >= ID_SLUG_MAX {
            break;
        }
    }
    if slug.is_empty() {
        "work".to_string()
    } else {
        slug
    }
}

static ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Eight hexadecimal characters that differ between creations of the same title.
fn id_suffix(title: &str, attempt: u64) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let counter = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut hasher = Sha256::new();
    hasher.update(title.as_bytes());
    hasher.update([0]);
    hasher.update(nanos.to_le_bytes());
    hasher.update(std::process::id().to_le_bytes());
    hasher.update(counter.to_le_bytes());
    hasher.update(attempt.to_le_bytes());
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(ID_SUFFIX_HEX);
    for byte in digest.iter().take(ID_SUFFIX_HEX / 2) {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

fn revision_path(dir: &Path, artifact: Artifact, revision: &Revision) -> PathBuf {
    dir.join(artifact.dir())
        .join(format!("{revision}.{}", artifact.extension()))
}

/// What a pointer file names: the current revision and the save that made it current.
///
/// A revision alone cannot say which save that was. The same text can be saved more
/// than once (a model kept for a later source is one), and a save that logged and
/// then failed to move the pointer leaves a line for a revision the pointer may
/// already name. Every reader has to know WHICH line is the one that took effect, and
/// the pointer knows: it is moved in one atomic write together with the position it
/// carries, so a position is only ever written by a save that is taking effect.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Pointer {
    revision: Revision,
    /// The place of that save's line among the log's saves. `None` for a pointer
    /// written before places were kept, which is read as it always was: the last
    /// line of the revision.
    entry: Option<usize>,
}

/// The second line of a pointer file: `log <n>`, the save's place in the log.
const POINTER_ENTRY_PREFIX: &str = "log ";

/// The pointer of each chain, in the order a [`WorkSnapshot`] lists them: the text, the
/// model, the answers, the requirement list, the acceptances.
type Pointers = [Option<Pointer>; 5];

fn read_pointers(dir: &Path) -> Result<Pointers, StoreError> {
    Ok([
        read_pointer(dir, Artifact::Source)?,
        read_pointer(dir, Artifact::Model)?,
        read_pointer(dir, Artifact::Answers)?,
        read_pointer(dir, Artifact::Requirements)?,
        read_pointer(dir, Artifact::Acceptances)?,
    ])
}

fn read_pointer(dir: &Path, artifact: Artifact) -> Result<Option<Pointer>, StoreError> {
    let path = dir.join(artifact.head_file());
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(StoreError::io(&path, e)),
    };
    let corrupt = |why: String| StoreError::corrupt(&path, why);
    let mut lines = text.lines();
    let revision =
        Revision::parse(lines.next().unwrap_or("").trim()).map_err(|e| corrupt(e.to_string()))?;
    let entry = match lines.next().map(str::trim) {
        None | Some("") => None,
        Some(line) => Some(
            line.strip_prefix(POINTER_ENTRY_PREFIX)
                .and_then(|n| n.parse::<usize>().ok())
                .ok_or_else(|| {
                    corrupt(format!("`{line}` is not the place of a save in the log"))
                })?,
        ),
    };
    if lines.any(|rest| !rest.trim().is_empty()) {
        return Err(corrupt(
            "the pointer has more lines than a revision and a place".to_string(),
        ));
    }
    Ok(Some(Pointer { revision, entry }))
}

/// What a pointer file holds: the revision, and the place of the save that made it current.
fn pointer_file_text(revision: &Revision, place: usize) -> String {
    format!("{revision}\n{POINTER_ENTRY_PREFIX}{place}\n")
}

/// An older folder's pointer written again with the place of the save it meant: the last
/// line the log holds for its revision, which is the line it has always been read as.
///
/// Only for a pointer whose revision the log leaves no doubt about (`Chain::head_unconfirmed`):
/// a place pinned for a line that may be a save which failed would make that save the one
/// that took effect.
fn name_the_place(dir: &Path, artifact: Artifact, older: &Pointer) -> Result<Pointer, StoreError> {
    let Some(place) = read_log(dir, artifact)?
        .iter()
        .rposition(|line| line.entry.revision == older.revision)
    else {
        return Ok(older.clone());
    };
    atomic_write(
        &dir.join(artifact.head_file()),
        pointer_file_text(&older.revision, place).as_bytes(),
    )?;
    Ok(Pointer {
        revision: older.revision.clone(),
        entry: Some(place),
    })
}

fn read_head(dir: &Path, artifact: Artifact) -> Result<Option<Revision>, StoreError> {
    Ok(read_pointer(dir, artifact)?.map(|pointer| pointer.revision))
}

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Write `bytes` to `path` so a reader sees the old file or the whole new one,
/// never a part: a temporary file beside it, flushed to disk, renamed over it.
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let temporary = parent.join(format!(
        ".{name}.{}.{}.tmp",
        std::process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let written = (|| -> io::Result<()> {
        let mut file = File::create(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temporary, path)
    })();
    if let Err(e) = written {
        let _ = fs::remove_file(&temporary);
        return Err(StoreError::io(path, e));
    }
    sync_directory(parent);
    Ok(())
}

/// Make the rename itself durable. Unix only, and best effort: a directory
/// cannot be opened as a file on Windows, whose rename is already durable
/// enough for a pointer the next save re-checks.
fn sync_directory(dir: &Path) {
    #[cfg(unix)]
    if let Ok(handle) = File::open(dir) {
        let _ = handle.sync_all();
    }
    #[cfg(not(unix))]
    let _ = dir;
}

/// One line of a log: the save, and the place of the line of the save it followed.
///
/// The parent is named in the log by its revision, and a revision can have several
/// lines. A line that never took effect and a line that did can share one (a model
/// kept for a later source, saved and failed), so "the last line of the parent's
/// revision" can be the one that failed. The place says which one the save followed.
/// A line from before places were kept has none, and is followed as it always was.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct LogLine {
    #[serde(flatten)]
    entry: HistoryEntry,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    parent_at: Option<usize>,
}

/// Append `line` to the log and say its place: how many saves the log held before it.
/// Under the save's lock, so no other save moves the count between the two.
fn append_log(path: &Path, line: &LogLine) -> Result<usize, StoreError> {
    let mut text =
        serde_json::to_string(line).map_err(|e| StoreError::corrupt(path, e.to_string()))?;
    text.push('\n');
    drop_torn_tail(path).map_err(|e| StoreError::io(path, e))?;
    let place = match fs::read_to_string(path) {
        Ok(held) => held.lines().filter(|l| !l.trim().is_empty()).count(),
        Err(e) if e.kind() == io::ErrorKind::NotFound => 0,
        Err(e) => return Err(StoreError::io(path, e)),
    };
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| StoreError::io(path, e))?;
    file.write_all(text.as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(|e| StoreError::io(path, e))?;
    Ok(place)
}

/// Cut a log back to its last complete line.
///
/// A save interrupted while it was logging leaves a final line without its
/// newline. That save never took effect (the pointer moves after the log), so
/// the fragment says nothing, and left in place the next save would be glued to
/// it and both lines would be lost.
fn drop_torn_tail(path: &Path) -> io::Result<()> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    if bytes.is_empty() || bytes.ends_with(b"\n") {
        return Ok(());
    }
    let keep = bytes.iter().rposition(|&b| b == b'\n').map_or(0, |i| i + 1);
    let file = OpenOptions::new().write(true).open(path)?;
    file.set_len(keep as u64)?;
    file.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A store in a folder of its own, removed when the test ends.
    struct Scratch {
        root: PathBuf,
    }

    impl Scratch {
        fn new(label: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "sce-store-{label}-{}-{}",
                std::process::id(),
                TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).expect("create the scratch folder");
            Self { root }
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    /// An older build's pointer is the digest alone.
    fn as_the_digest_alone(dir: &Path, revision: &Revision) {
        fs::write(
            dir.join(Artifact::Source.head_file()),
            format!("{revision}\n"),
        )
        .unwrap();
    }

    /// A save that landed between the reader's look at the pointer and its read of the log,
    /// and a save after it that logged and never took effect. The pointer the reader holds is
    /// an older folder's: the digest, with no place, so the log alone would be asked which of
    /// the lines of that digest it meant -- and the last one is the save that failed.
    /// Starting over when the pointer has moved reads the chain of the pointer that stood.
    #[test]
    fn a_read_that_a_save_landed_in_the_middle_of_starts_over() {
        let scratch = Scratch::new("seam");
        let store = WorkStore::at(&scratch.root);
        let id = store.create_work("Seam").unwrap().id;
        let dir = store.existing(&id).unwrap();
        let a = Revision::of(b"A");
        let b = Revision::of(b"B");
        store.save_source(&id, "A", None).unwrap();
        as_the_digest_alone(&dir, &a);

        let mut landed = false;
        let snapshot = read_chain(&dir, Artifact::Source, || {
            if std::mem::replace(&mut landed, true) {
                return;
            }
            store.save_source(&id, "B", Some(&a)).unwrap();
            // The revert to A that logged and could not move the pointer.
            let failed = LogLine {
                entry: HistoryEntry {
                    revision: a.clone(),
                    parent: Some(b.clone()),
                    saved_at: store.now(),
                    written_for: None,
                    unconfirmed: false,
                },
                parent_at: Some(1),
            };
            append_log(&dir.join(Artifact::Source.log_file()), &failed).unwrap();
        })
        .unwrap();

        assert_eq!(snapshot.pointer.map(|p| p.revision), Some(b.clone()));
        let chain: Vec<Revision> = snapshot
            .chain
            .entries
            .into_iter()
            .map(|e| e.revision)
            .collect();
        assert_eq!(chain, vec![a, b], "the save that failed was listed");
    }

    /// A pointer that moves on every look is a folder under constant writing; a read of it
    /// ends, with the last look.
    #[test]
    fn a_read_of_a_pointer_that_never_holds_still_ends() {
        let scratch = Scratch::new("seam-bound");
        let store = WorkStore::at(&scratch.root);
        let id = store.create_work("Seam").unwrap().id;
        let dir = store.existing(&id).unwrap();
        store.save_source(&id, "T0", None).unwrap();

        let mut looks = 0;
        let snapshot = read_chain(&dir, Artifact::Source, || {
            looks += 1;
            let base = read_head(&dir, Artifact::Source).unwrap();
            store
                .save_source(&id, &format!("T{looks}"), base.as_ref())
                .unwrap();
        })
        .unwrap();

        assert_eq!(looks, READ_TRIES);
        assert!(snapshot.pointer.is_some());
    }

    /// A work nothing was saved to is a work, not a failure: each chain is absent.
    #[test]
    fn a_snapshot_of_a_work_nothing_was_saved_to_holds_no_chain() {
        let scratch = Scratch::new("snapshot-empty");
        let store = WorkStore::at(&scratch.root);
        let work = store.create_work("Empty").unwrap();

        let snapshot = store.read_work_snapshot(&work.id).unwrap();

        assert_eq!(snapshot.work, work);
        assert!(snapshot.source.is_none());
        assert!(snapshot.model.is_none());
        assert!(snapshot.answers.is_none());
        assert!(snapshot.requirements.is_none());
        assert!(snapshot.acceptance.is_none());
    }

    /// Every chain at its head, and the source each of the two that are written for a
    /// source says it was written for: the text moved on, so they are of the one before.
    #[test]
    fn a_snapshot_holds_each_chain_at_its_head_with_what_the_model_and_the_list_were_written_for() {
        let scratch = Scratch::new("snapshot-all");
        let store = WorkStore::at(&scratch.root);
        let id = store.create_work("All").unwrap().id;
        let t1 = Revision::of(b"T1");
        let t2 = Revision::of(b"T2");
        store.save_source(&id, "T1", None).unwrap();
        store.save_model(&id, "M1", None, Some(&t1)).unwrap();
        store.save_requirements(&id, "R1", None, Some(&t1)).unwrap();
        store.save_answers(&id, "A1", None).unwrap();
        store.save_acceptance(&id, "C1", None).unwrap();
        store.save_source(&id, "T2", Some(&t1)).unwrap();

        let snapshot = store.read_work_snapshot(&id).unwrap();

        let source = snapshot.source.unwrap();
        assert_eq!(source.revision, t2);
        assert_eq!(source.text, "T2");
        let model = snapshot.model.unwrap();
        assert_eq!(model.text, "M1");
        assert_eq!(model.written_for, Some(t1.clone()));
        let list = snapshot.requirements.unwrap();
        assert_eq!(list.text, "R1");
        assert_eq!(list.written_for, Some(t1));
        assert_eq!(snapshot.answers.unwrap().text, "A1");
        assert_eq!(snapshot.acceptance.unwrap().text, "C1");
    }

    /// Two saves that landed after the reader looked at the pointers: the source it
    /// holds is the old one, and a read that stopped there would answer a state the
    /// folder has left. The pointers are looked at again, found moved, and the read
    /// starts over, so what is answered is the folder as the second look found it.
    #[test]
    fn a_snapshot_whose_pointers_moved_while_it_was_read_answers_the_state_after_the_saves() {
        let scratch = Scratch::new("snapshot-seam");
        let store = WorkStore::at(&scratch.root);
        let id = store.create_work("Seam").unwrap().id;
        let t1 = Revision::of(b"T1");
        let t2 = Revision::of(b"T2");
        let m1 = Revision::of(b"M1");
        store.save_source(&id, "T1", None).unwrap();
        store.save_model(&id, "M1", None, Some(&t1)).unwrap();

        let mut looks = 0;
        let snapshot = store
            .snapshot_between(&id, || {
                looks += 1;
                if looks == 1 {
                    store.save_source(&id, "T2", Some(&t1)).unwrap();
                    store.save_model(&id, "M2", Some(&m1), Some(&t2)).unwrap();
                }
            })
            .unwrap();

        assert_eq!(
            looks, 2,
            "the read starts over once, and the second look holds"
        );
        assert_eq!(snapshot.source.unwrap().revision, t2);
        let model = snapshot.model.unwrap();
        assert_eq!(model.text, "M2");
        assert_eq!(model.written_for, Some(t2));
    }

    /// A folder written to between every look cannot be read without a lock, and the
    /// answer is still one state: after the tries are spent the read takes the lock a
    /// save takes, and no save lands while it reads. The seam runs only while the reader
    /// holds no lock, because a save made under it would wait for the reader to finish.
    #[test]
    fn a_snapshot_of_a_folder_that_never_holds_still_is_read_under_the_lock() {
        let scratch = Scratch::new("snapshot-bound");
        let store = WorkStore::at(&scratch.root);
        let id = store.create_work("Seam").unwrap().id;
        let dir = store.existing(&id).unwrap();
        store.save_source(&id, "T0", None).unwrap();

        let mut looks = 0;
        let snapshot = store
            .snapshot_between(&id, || {
                looks += 1;
                let base = read_head(&dir, Artifact::Source).unwrap();
                store
                    .save_source(&id, &format!("T{looks}"), base.as_ref())
                    .unwrap();
            })
            .unwrap();

        assert_eq!(looks, READ_TRIES);
        assert_eq!(snapshot.source.unwrap().text, format!("T{READ_TRIES}"));
    }

    #[test]
    fn a_snapshot_of_a_removed_work_is_refused_as_every_other_read_is() {
        let scratch = Scratch::new("snapshot-removed");
        let store = WorkStore::at(&scratch.root);
        let id = store.create_work("Gone").unwrap().id;
        store.remove_work(&id).unwrap();

        let refused = store.read_work_snapshot(&id).unwrap_err();

        assert_eq!(refused.kind(), "not-found");
    }
}
