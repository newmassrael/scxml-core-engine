// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The originals of a spec, and what was read out of them (ADR 0032, 0026).
//!
//! ```text
//! <spec-id>/
//!   members/<hash>.bin          the bytes of a member, named by their own SHA-256, never rewritten
//!   originals/<digest>.json     the manifest of an original; its digest is the original's name
//!   originals.log               one JSON line per original added: digest, time, label, size
//!   extractions/<digest>.json   what an extractor read out of a member, named by its own SHA-256
//!   extractions.log             one JSON line per extraction saved: what it is of, and who made it
//! ```
//!
//! An original is not a chain as a text is: there is no current one. A spec may be brought in from
//! several documents over time, and a proposal names the one it is made from by its digest (ADR 0025),
//! so what is kept is a set of immutable things named by hash and a log that says when each arrived.
//! Adding the same bundle again is the same original (`created: false`), and two originals that hold
//! one picture keep one copy of it: a member's bytes are named by their hash and shared.
//!
//! **A manifest names only bytes that are there.** The bytes are written before the manifest, and a
//! read checks each member is held at the size and hash the manifest says, so an original whose
//! members were lost is `corrupt` and not a bundle that reads clean with a hole in it.
//!
//! An extraction is kept as the text its maker gave. Its form belongs to the authoring tools
//! (`tools/authoring/schema/extraction.v1.schema.json`); the store checks that it is a JSON object, says
//! it is of a form and who made it, names an original it holds and a member of that original, and keeps
//! it under the hash of exactly those bytes. Two extractors reading one member keep two extractions.
//!
//! Every write is made under the lock a save takes, so a removal of the spec and an addition cannot
//! meet. Reads take no lock: every file is written whole under a name its content gives, and the log
//! is appended to after the file it names is in place.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{
    atomic_write, drop_torn_tail, is_removed, removed_spec, SpecId, SpecStore, LOCK_FILE, LOCK_WAIT,
};
use crate::clock::{utc_timestamp, Clock};
use crate::error::StoreError;
use crate::lock;
use crate::original::{Manifest, OriginalDraft, MAX_EXTRACTION_BYTES};
use crate::revision::Revision;

const MEMBERS_DIR: &str = "members";
const ORIGINALS_DIR: &str = "originals";
const ORIGINALS_LOG: &str = "originals.log";
const EXTRACTIONS_DIR: &str = "extractions";
const EXTRACTIONS_LOG: &str = "extractions.log";

/// What adding an original made.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OriginalAdded {
    /// The original's name: the hash of its manifest.
    pub digest: Revision,
    /// `false` when this bundle was added before, and this is that original.
    pub created: bool,
    pub members: usize,
    pub bytes: u64,
}

/// One line of the log of originals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OriginalEntry {
    pub digest: Revision,
    pub at: String,
    pub label: String,
    pub members: usize,
    pub bytes: u64,
}

/// An extraction as the log says it: what it is of, and who read it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtractionEntry {
    pub digest: Revision,
    pub at: String,
    pub original: Revision,
    /// The path of the member of the original it was read from.
    pub member: String,
    pub extractor: String,
    pub extractor_version: String,
}

/// What saving an extraction made.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExtractionSaved {
    pub digest: Revision,
    /// `false` when exactly these bytes were saved before for this member of this original.
    pub created: bool,
}

fn bad(message: impl Into<String>, detail: serde_json::Value) -> StoreError {
    StoreError::refused("bad-original", message, detail)
}

fn member_path(dir: &Path, hash: &Revision) -> PathBuf {
    dir.join(MEMBERS_DIR).join(format!("{}.bin", hash.as_str()))
}

fn original_path(dir: &Path, digest: &Revision) -> PathBuf {
    dir.join(ORIGINALS_DIR)
        .join(format!("{}.json", digest.as_str()))
}

fn extraction_path(dir: &Path, digest: &Revision) -> PathBuf {
    dir.join(EXTRACTIONS_DIR)
        .join(format!("{}.json", digest.as_str()))
}

/// The lines of a log, in the order they were appended. A line that is not one (a torn tail, a file
/// edited by hand) is skipped and not read as an entry: the log says what was saved, and a fragment
/// says nothing.
pub(super) fn read_log<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<Vec<T>, StoreError> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(StoreError::io(path, e)),
    };
    Ok(text
        .lines()
        .filter_map(|line| serde_json::from_str::<T>(line).ok())
        .collect())
}

pub(super) fn append_line<T: Serialize>(path: &Path, entry: &T) -> Result<(), StoreError> {
    let mut text =
        serde_json::to_string(entry).map_err(|e| StoreError::corrupt(path, e.to_string()))?;
    text.push('\n');
    drop_torn_tail(path).map_err(|e| StoreError::io(path, e))?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| StoreError::io(path, e))?;
    file.write_all(text.as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(|e| StoreError::io(path, e))
}

/// Keep `bytes` under `hash` in `folder`, unless the bytes are already there. A file that is there
/// under the name and is not those bytes is a store that was damaged, and is refused as such rather than
/// trusted because it has the right name.
pub(super) fn keep(
    folder: &Path,
    path: &Path,
    hash: &Revision,
    bytes: &[u8],
) -> Result<(), StoreError> {
    match fs::read(path) {
        Ok(held) => {
            if Revision::of(&held) == *hash {
                return Ok(());
            }
            return Err(StoreError::corrupt(
                path,
                format!("it is named {} and holds other bytes", hash.as_str()),
            ));
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(StoreError::io(path, e)),
    }
    fs::create_dir_all(folder).map_err(|e| StoreError::io(folder, e))?;
    atomic_write(path, bytes)
}

impl<C: Clock> SpecStore<C> {
    /// Add an original to the spec: keep the bytes of its members, then its manifest, and say its name.
    ///
    /// The same bundle (the same members under the same paths, with the same roles and label) is the
    /// same original and is not added twice.
    pub fn add_original(
        &self,
        id: &SpecId,
        draft: OriginalDraft,
    ) -> Result<OriginalAdded, StoreError> {
        let built = draft.build()?;
        let manifest = built.manifest;
        let digest = manifest.revision();
        let members = manifest.members.len();
        let bytes = manifest.bytes();

        let dir = self.existing(id)?;
        let _held = lock::exclusive(&dir.join(LOCK_FILE), LOCK_WAIT)?;
        if is_removed(&dir) {
            return Err(removed_spec(id));
        }
        // The bytes come first: a manifest that is in place names bytes that are.
        let folder = dir.join(MEMBERS_DIR);
        for (hash, blob) in &built.blobs {
            keep(&folder, &member_path(&dir, hash), hash, blob)?;
        }
        let manifest_path = original_path(&dir, &digest);
        let text = manifest.canonical_text();
        let known = read_log::<OriginalEntry>(&dir.join(ORIGINALS_LOG))?
            .iter()
            .any(|entry| entry.digest == digest);
        if !manifest_path.is_file() {
            fs::create_dir_all(dir.join(ORIGINALS_DIR))
                .map_err(|e| StoreError::io(dir.join(ORIGINALS_DIR), e))?;
            atomic_write(&manifest_path, text.as_bytes())?;
        }
        if !known {
            append_line(
                &dir.join(ORIGINALS_LOG),
                &OriginalEntry {
                    digest: digest.clone(),
                    at: utc_timestamp(self.clock.epoch()),
                    label: manifest.label.clone(),
                    members,
                    bytes,
                },
            )?;
        }
        Ok(OriginalAdded {
            digest,
            created: !known,
            members,
            bytes,
        })
    }

    /// The manifest of an original of the spec.
    ///
    /// Every member is checked to be held at the size and hash the manifest says. An original whose
    /// bytes are gone is `corrupt`; it is not given back with a hole in it.
    pub fn read_original(&self, id: &SpecId, digest: &Revision) -> Result<Manifest, StoreError> {
        let dir = self.existing(id)?;
        let manifest = self.load_manifest(&dir, id, digest)?;
        for member in &manifest.members {
            let path = member_path(&dir, &member.hash);
            let held = fs::metadata(&path).map_err(|e| {
                if e.kind() == io::ErrorKind::NotFound {
                    StoreError::corrupt(
                        &path,
                        format!(
                            "the original names the member `{}` and its bytes are not held",
                            member.path
                        ),
                    )
                } else {
                    StoreError::io(&path, e)
                }
            })?;
            if held.len() != member.size {
                return Err(StoreError::corrupt(
                    &path,
                    format!(
                        "the member `{}` is held at {} bytes and the original says {}",
                        member.path,
                        held.len(),
                        member.size
                    ),
                ));
            }
        }
        Ok(manifest)
    }

    fn load_manifest(
        &self,
        dir: &Path,
        id: &SpecId,
        digest: &Revision,
    ) -> Result<Manifest, StoreError> {
        let path = original_path(dir, digest);
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return Err(StoreError::NotFound {
                    what: format!(
                        "an original `{}` of spec `{}`",
                        digest.as_str(),
                        id.as_str()
                    ),
                });
            }
            Err(e) => return Err(StoreError::io(&path, e)),
        };
        // The name is the hash of the manifest's text: a file that holds anything else is not it.
        if Revision::of(text.as_bytes()) != *digest {
            return Err(StoreError::corrupt(
                &path,
                "it is named by a hash its text does not have",
            ));
        }
        Manifest::read(&text).map_err(|why| StoreError::corrupt(&path, why))
    }

    /// The originals added to the spec, the newest first.
    pub fn list_originals(&self, id: &SpecId) -> Result<Vec<OriginalEntry>, StoreError> {
        let dir = self.existing(id)?;
        let mut entries = read_log::<OriginalEntry>(&dir.join(ORIGINALS_LOG))?;
        entries.reverse();
        Ok(entries)
    }

    /// The bytes of one member of an original, checked against the hash the manifest names.
    pub fn read_original_member(
        &self,
        id: &SpecId,
        digest: &Revision,
        path: &str,
    ) -> Result<Vec<u8>, StoreError> {
        let dir = self.existing(id)?;
        let manifest = self.load_manifest(&dir, id, digest)?;
        let Some(member) = manifest.member(path) else {
            return Err(StoreError::NotFound {
                what: format!("a member `{path}` of the original `{}`", digest.as_str()),
            });
        };
        let held = member_path(&dir, &member.hash);
        let bytes = match fs::read(&held) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return Err(StoreError::corrupt(
                    &held,
                    format!("the original names the member `{path}` and its bytes are not held"),
                ));
            }
            Err(e) => return Err(StoreError::io(&held, e)),
        };
        if Revision::of(&bytes) != member.hash {
            return Err(StoreError::corrupt(
                &held,
                "it is named by a hash its bytes do not have",
            ));
        }
        Ok(bytes)
    }

    /// Keep an extraction of a member of an original.
    ///
    /// `text` is kept exactly as given, under the hash of those bytes. It has to be a JSON object that
    /// says what form it is (`v`) and which extractor made it (`extractor`, `extractor_version`); the
    /// rest of its form is the authoring tools', and a text that is not one of theirs is refused by them
    /// when it is read.
    pub fn save_extraction(
        &self,
        id: &SpecId,
        original: &Revision,
        member: &str,
        text: &str,
    ) -> Result<ExtractionSaved, StoreError> {
        if text.len() > MAX_EXTRACTION_BYTES {
            return Err(StoreError::TooLarge {
                what: "an extraction",
                bytes: text.len(),
                limit: MAX_EXTRACTION_BYTES,
            });
        }
        let value: serde_json::Value = serde_json::from_str(text).map_err(|e| {
            bad(
                format!("an extraction is JSON ({e})"),
                serde_json::Value::Null,
            )
        })?;
        let object = value
            .as_object()
            .ok_or_else(|| bad("an extraction is a JSON object", serde_json::Value::Null))?;
        if !object.get("v").is_some_and(serde_json::Value::is_u64) {
            return Err(bad(
                "an extraction says what form it is of (`v`, a whole number)",
                json!({ "field": "v" }),
            ));
        }
        let word = |field: &str| -> Result<String, StoreError> {
            object
                .get(field)
                .and_then(serde_json::Value::as_str)
                .filter(|s| !s.is_empty() && s.len() <= 200)
                .map(str::to_string)
                .ok_or_else(|| {
                    bad(
                        format!("an extraction says `{field}`: 1 to 200 characters of text"),
                        json!({ "field": field }),
                    )
                })
        };
        let extractor = word("extractor")?;
        let extractor_version = word("extractor_version")?;

        let dir = self.existing(id)?;
        let _held = lock::exclusive(&dir.join(LOCK_FILE), LOCK_WAIT)?;
        if is_removed(&dir) {
            return Err(removed_spec(id));
        }
        let manifest = self.load_manifest(&dir, id, original)?;
        if manifest.member(member).is_none() {
            return Err(bad(
                format!("the original has no member `{member}` for an extraction to be of"),
                json!({ "member": member }),
            ));
        }
        let digest = Revision::of(text.as_bytes());
        let path = extraction_path(&dir, &digest);
        keep(&dir.join(EXTRACTIONS_DIR), &path, &digest, text.as_bytes())?;
        let known = read_log::<ExtractionEntry>(&dir.join(EXTRACTIONS_LOG))?
            .iter()
            .any(|entry| {
                entry.digest == digest && entry.original == *original && entry.member == member
            });
        if !known {
            append_line(
                &dir.join(EXTRACTIONS_LOG),
                &ExtractionEntry {
                    digest: digest.clone(),
                    at: utc_timestamp(self.clock.epoch()),
                    original: original.clone(),
                    member: member.to_string(),
                    extractor,
                    extractor_version,
                },
            )?;
        }
        Ok(ExtractionSaved {
            digest,
            created: !known,
        })
    }

    /// The text of an extraction of the spec, as it was saved.
    pub fn read_extraction(&self, id: &SpecId, digest: &Revision) -> Result<String, StoreError> {
        let dir = self.existing(id)?;
        let path = extraction_path(&dir, digest);
        // Only what this spec saved: a hash that names a file nobody saved for this spec is not one.
        let saved = read_log::<ExtractionEntry>(&dir.join(EXTRACTIONS_LOG))?
            .iter()
            .any(|entry| entry.digest == *digest);
        let text = if saved {
            fs::read_to_string(&path)
        } else {
            Err(io::ErrorKind::NotFound.into())
        };
        match text {
            Ok(text) if Revision::of(text.as_bytes()) == *digest => Ok(text),
            Ok(_) => Err(StoreError::corrupt(
                &path,
                "it is named by a hash its text does not have",
            )),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Err(StoreError::NotFound {
                what: format!(
                    "an extraction `{}` of spec `{}`",
                    digest.as_str(),
                    id.as_str()
                ),
            }),
            Err(e) => Err(StoreError::io(&path, e)),
        }
    }

    /// The extractions saved for the spec, the newest first; of one original when `original` is given.
    pub fn list_extractions(
        &self,
        id: &SpecId,
        original: Option<&Revision>,
    ) -> Result<Vec<ExtractionEntry>, StoreError> {
        let dir = self.existing(id)?;
        let mut entries = read_log::<ExtractionEntry>(&dir.join(EXTRACTIONS_LOG))?;
        if let Some(original) = original {
            entries.retain(|entry| entry.original == *original);
        }
        entries.reverse();
        Ok(entries)
    }
}
