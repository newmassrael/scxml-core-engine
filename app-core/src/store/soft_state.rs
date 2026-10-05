// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Records a process keeps about itself beside the works: one file each, in a folder of its own.
//!
//! An adapter says it is there, a shell says whether it hosts an executor: both are what a
//! process says of itself, and neither is a fact about any work. So they take no work's lock;
//! one process writes its own file by an atomic rename, and the files of two processes are two
//! files. The folder begins with a dot, which no work id does, so it is never listed as a work.
//! Defined once, so that the two cannot come to different words about what a record is.

use std::fs;
use std::io;
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::Serialize;

use super::{atomic_write, Unreadable};
use crate::error::StoreError;

/// A record that is called by a name, which is also its file's name.
pub(super) trait Named {
    fn name(&self) -> &str;
}

/// Keep `record` as the file `<name>.json` of `folder`, in place of what was there.
pub(super) fn write_record<T: Serialize + Named>(
    folder: &Path,
    record: &T,
) -> Result<(), StoreError> {
    fs::create_dir_all(folder).map_err(|e| StoreError::io(folder, e))?;
    let path = folder.join(format!("{}.json", record.name()));
    let mut bytes =
        serde_json::to_vec_pretty(record).map_err(|e| StoreError::corrupt(&path, e.to_string()))?;
    bytes.push(b'\n');
    atomic_write(&path, &bytes)
}

/// Every record of `folder`, by name, and every file that is not a record that can be read. A
/// folder that is not there holds none. `what` names the kind of record, for the words said of
/// a record that names another.
pub(super) fn read_records<T: DeserializeOwned + Named>(
    folder: &Path,
    what: &str,
) -> Result<(Vec<T>, Vec<Unreadable>), StoreError> {
    let entries = match fs::read_dir(folder) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok((Vec::new(), Vec::new())),
        Err(e) => return Err(StoreError::io(folder, e)),
    };
    let mut records = Vec::new();
    let mut unreadable = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| StoreError::io(folder, e))?;
        let file = entry.file_name().to_string_lossy().into_owned();
        // What a half-written replacement leaves beside a record is not a record.
        let Some(name) = file.strip_suffix(".json") else {
            continue;
        };
        let read = fs::read(entry.path())
            .map_err(|e| e.to_string())
            .and_then(|bytes| serde_json::from_slice::<T>(&bytes).map_err(|e| e.to_string()));
        match read {
            Ok(record) if record.name() == name => records.push(record),
            Ok(record) => unreadable.push(Unreadable {
                id: name.to_string(),
                reason: format!("the record says it is the {what} `{}`", record.name()),
            }),
            Err(reason) => unreadable.push(Unreadable {
                id: name.to_string(),
                reason,
            }),
        }
    }
    records.sort_by(|a, b| a.name().cmp(b.name()));
    unreadable.sort_by(|a, b| a.id.cmp(&b.id));
    Ok((records, unreadable))
}
