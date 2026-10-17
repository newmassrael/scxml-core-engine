// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Where a relay keeps what it holds.
//!
//! A relay's state is the records it was given, in the order it was given them, and nothing else:
//! the log, the chains and the heads are what verifying those records again makes. So a store is an
//! append-only list of `(list, record)`, and a relay that starts reads it back through the same
//! checks that took each record in, which is why a store that was tampered with cannot make a relay
//! hold what its members never signed: it fails to verify and the relay does not start.

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::protocol::Record;

/// An append-only record of what a relay was given.
pub trait Store {
    /// Every record kept, each list's in the order it was appended.
    fn load(&mut self) -> io::Result<Vec<(String, Record)>>;

    /// Keep one more record, durably: when this returns `Ok` the record survives the process.
    fn append(&mut self, list: &str, record: &Record) -> io::Result<()>;
}

/// A store that keeps nothing past the process: for tests, and for a relay whose lists can be
/// rebuilt from its members.
#[derive(Default)]
pub struct MemoryStore {
    rows: Vec<(String, Record)>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Store for MemoryStore {
    fn load(&mut self) -> io::Result<Vec<(String, Record)>> {
        Ok(self.rows.clone())
    }

    fn append(&mut self, list: &str, record: &Record) -> io::Result<()> {
        self.rows.push((list.to_string(), record.clone()));
        Ok(())
    }
}

/// A store in a directory: one file for each list, a line for each record, flushed to the disk
/// before an append returns. The file is named by the hexadecimal of the list's name, so no name a
/// device sends reaches a path of its own making.
pub struct FileStore {
    dir: PathBuf,
    files: BTreeMap<String, File>,
}

const EXTENSION: &str = "jsonl";

fn hex(text: &str) -> String {
    text.bytes().map(|byte| format!("{byte:02x}")).collect()
}

impl FileStore {
    /// A store in `dir`, which is made when it is not there.
    pub fn open(dir: impl AsRef<Path>) -> io::Result<Self> {
        fs::create_dir_all(dir.as_ref())?;
        Ok(Self {
            dir: dir.as_ref().to_path_buf(),
            files: BTreeMap::new(),
        })
    }

    fn path_of(&self, list: &str) -> PathBuf {
        self.dir.join(format!("{}.{EXTENSION}", hex(list)))
    }
}

fn invalid(message: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

impl Store for FileStore {
    fn load(&mut self) -> io::Result<Vec<(String, Record)>> {
        let mut paths: Vec<PathBuf> = fs::read_dir(&self.dir)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == EXTENSION))
            .collect();
        paths.sort();
        let mut rows = Vec::new();
        for path in paths {
            let text = fs::read_to_string(&path)?;
            // A process that died in the middle of a line leaves a last line with no end: that
            // record was never acknowledged, and is dropped. A line that is whole and is not a
            // record is corruption, and is not read past.
            let complete = text.rfind('\n').map_or(0, |end| end + 1);
            for (number, line) in text[..complete].lines().enumerate() {
                let row: Value = serde_json::from_str(line).map_err(|_| {
                    invalid(format!(
                        "{}: line {} is not JSON",
                        path.display(),
                        number + 1
                    ))
                })?;
                let list = row.get("list").and_then(Value::as_str);
                let record = row.get("record").and_then(Record::from_value);
                match (list, record) {
                    (Some(list), Some(record)) => rows.push((list.to_string(), record)),
                    _ => {
                        return Err(invalid(format!(
                            "{}: line {} is not a record",
                            path.display(),
                            number + 1
                        )))
                    }
                }
            }
        }
        Ok(rows)
    }

    fn append(&mut self, list: &str, record: &Record) -> io::Result<()> {
        if !self.files.contains_key(list) {
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(self.path_of(list))?;
            self.files.insert(list.to_string(), file);
        }
        let file = self
            .files
            .get_mut(list)
            .ok_or_else(|| io::Error::other("the file was just opened"))?;
        let mut line = json!({"list": list, "record": record.to_value()}).to_string();
        line.push('\n');
        file.write_all(line.as_bytes())?;
        file.sync_data()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "sce-relay-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |elapsed| elapsed.as_nanos())
        ));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn op(n: u32) -> Record {
        Record::Operation {
            event: "todo.write".to_string(),
            data: format!("{{\"n\":{n}}}"),
        }
    }

    #[test]
    fn what_a_file_store_appended_is_read_back_in_order_by_list() {
        let dir = scratch("roundtrip");
        {
            let mut store = FileStore::open(&dir).unwrap();
            store.append("home", &op(1)).unwrap();
            store.append("work", &op(2)).unwrap();
            store
                .append("home", &Record::Entry("e".to_string()))
                .unwrap();
        }
        let mut again = FileStore::open(&dir).unwrap();
        let rows = again.load().unwrap();
        let of = |list: &str| -> Vec<Record> {
            rows.iter()
                .filter(|(held, _)| held == list)
                .map(|(_, record)| record.clone())
                .collect()
        };
        assert_eq!(of("home"), vec![op(1), Record::Entry("e".to_string())]);
        assert_eq!(of("work"), vec![op(2)]);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_list_name_that_is_a_path_makes_no_path() {
        let dir = scratch("hostile");
        let mut store = FileStore::open(&dir).unwrap();
        store.append("../../etc/passwd", &op(1)).unwrap();
        let names: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        assert_eq!(names.len(), 1);
        assert!(
            names[0].ends_with(".jsonl") && !names[0].contains('/'),
            "{names:?}"
        );
        assert_eq!(store.load().unwrap().len(), 1);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_line_cut_off_by_a_crash_is_dropped_and_a_corrupt_one_is_not_read_past() {
        let dir = scratch("crash");
        let mut store = FileStore::open(&dir).unwrap();
        store.append("home", &op(1)).unwrap();
        let path = store.path_of("home");
        let mut file = OpenOptions::new().append(true).open(&path).unwrap();
        file.write_all(b"{\"list\":\"home\",\"rec").unwrap();
        assert_eq!(FileStore::open(&dir).unwrap().load().unwrap().len(), 1);
        file.write_all(b"\n").unwrap();
        let error = FileStore::open(&dir).unwrap().load().unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        fs::remove_dir_all(&dir).unwrap();
    }
}
