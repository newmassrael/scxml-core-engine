// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The works folder.
//!
//! ```text
//! <root>/
//!   <work-id>/
//!     work.json          the work's identity and title
//!     source/<digest>.txt   every saved text, named by its own SHA-256, never rewritten
//!     source.head        the digest of the current text, one line
//!     source.log         one JSON line per save: digest, parent digest, time
//!     model/<digest>.scxml  the same, for the SCXML model written from the text
//!     model.head         the digest of the current model, one line
//!     model.log          one JSON line per save, and the source revision it was written for
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
//! The text and the model are two chains kept by ONE implementation
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
}

impl Artifact {
    /// The folder holding every revision, each in a file named by its digest.
    fn dir(self) -> &'static str {
        match self {
            Artifact::Source => "source",
            Artifact::Model => "model",
        }
    }

    fn extension(self) -> &'static str {
        match self {
            Artifact::Source => "txt",
            Artifact::Model => "scxml",
        }
    }

    /// The file holding the digest of the current revision.
    fn head_file(self) -> &'static str {
        match self {
            Artifact::Source => "source.head",
            Artifact::Model => "model.head",
        }
    }

    /// The file holding one line for each save.
    fn log_file(self) -> &'static str {
        match self {
            Artifact::Source => "source.log",
            Artifact::Model => "model.log",
        }
    }

    fn max_bytes(self) -> usize {
        match self {
            Artifact::Source => MAX_SOURCE_BYTES,
            Artifact::Model => MAX_MODEL_BYTES,
        }
    }

    /// What it is called in a message.
    fn noun(self) -> &'static str {
        match self {
            Artifact::Source => "source",
            Artifact::Model => "model",
        }
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
        let Some((revision, text)) = self.read_text(Artifact::Model, id, revision)? else {
            return Ok(None);
        };
        // What the writer said it read is in the log line of the save that
        // made this revision current; a revision no save on the chain names
        // (one a failed save left behind) says nothing.
        let chain = self.history_of(Artifact::Model, id)?;
        let written_for = chain
            .iter()
            .rev()
            .find(|entry| entry.revision == revision)
            .and_then(|entry| entry.written_for.clone());
        Ok(Some(ModelText {
            revision,
            written_for,
            text,
        }))
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
        let path = revision_path(&dir, artifact, &wanted);
        let bytes = fs::read(&path).map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                if revision.is_some() {
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
        Ok(Some((wanted, text)))
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
                        "the source revision {} of `{}` that the model names as written_for",
                        source.short(),
                        id.as_str()
                    ),
                });
            }
        }
        let current = read_head(&dir, artifact)?;
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
            // another text is the writer's news (see `save_model`).
            let same_claim = match artifact {
                Artifact::Source => true,
                Artifact::Model => {
                    history_in(&dir, artifact)?
                        .pop()
                        .and_then(|entry| entry.written_for)
                        .as_ref()
                        == written_for
                }
            };
            if same_claim {
                return Ok(Saved::Unchanged { revision });
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
        let entry = HistoryEntry {
            revision: revision.clone(),
            parent: current.clone(),
            saved_at: self.clock.now(),
            written_for: written_for.cloned(),
        };
        append_log(&dir.join(artifact.log_file()), &entry)?;
        atomic_write(
            &dir.join(artifact.head_file()),
            format!("{revision}\n").as_bytes(),
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
    // The pointer is read before the log. A save logs, then moves the
    // pointer, so a pointer read first can only name a revision whose line
    // the log read after it already holds.
    let head = read_head(dir, artifact)?;
    let path = dir.join(artifact.log_file());
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(StoreError::io(&path, e)),
    };
    let ends_cleanly = text.ends_with('\n');
    let lines: Vec<&str> = text.lines().collect();
    let mut entries = Vec::with_capacity(lines.len());
    for (index, line) in lines.iter().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<HistoryEntry>(line) {
            Ok(entry) => entries.push(entry),
            Err(_) if !ends_cleanly && index + 1 == lines.len() => {}
            Err(e) => {
                return Err(StoreError::corrupt(
                    &path,
                    format!("line {} is not a save record: {e}", index + 1),
                ))
            }
        }
    }

    // Nothing has taken effect until the pointer exists, whatever the log holds.
    let Some(head) = head else {
        return Ok(Vec::new());
    };
    let mut chain = Vec::new();
    let mut before = entries.len();
    let mut wanted = Some(head);
    while let Some(revision) = wanted {
        let Some(at) = entries[..before]
            .iter()
            .rposition(|entry| entry.revision == revision)
        else {
            return Err(StoreError::corrupt(
                    &path,
                    format!(
                        "revision {} is on the way back from the current one and no save record names it",
                        revision.short()
                    ),
                ));
        };
        wanted = entries[at].parent.clone();
        chain.push(entries[at].clone());
        before = at;
    }
    chain.reverse();
    Ok(chain)
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

fn read_head(dir: &Path, artifact: Artifact) -> Result<Option<Revision>, StoreError> {
    let path = dir.join(artifact.head_file());
    match fs::read_to_string(&path) {
        Ok(text) => Revision::parse(text.trim())
            .map(Some)
            .map_err(|e| StoreError::corrupt(&path, e.to_string())),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(StoreError::io(&path, e)),
    }
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

fn append_log(path: &Path, entry: &HistoryEntry) -> Result<(), StoreError> {
    let mut line =
        serde_json::to_string(entry).map_err(|e| StoreError::corrupt(path, e.to_string()))?;
    line.push('\n');
    drop_torn_tail(path).map_err(|e| StoreError::io(path, e))?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| StoreError::io(path, e))?;
    file.write_all(line.as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(|e| StoreError::io(path, e))
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
