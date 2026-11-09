// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Where a relay keeps what it holds.
//!
//! A relay's state is the records it was given, in the order it was given them, and nothing else:
//! the log, the chains and the heads are what verifying those records again makes. So a store is an
//! append-only list of `(list, record)`, and a relay that starts reads it back through the same
//! checks that took each record in, which is why a store that was tampered with cannot make a relay
//! hold what its members never signed: it fails to verify and the relay does not start.
//!
//! Beside its records, a store keeps a mark of each list a device created on a connection with no
//! token (ADR 0080 section 3, [`Openly`]): when it was created and when a login last touched it. A
//! list no login has touched for long enough lapses, and is removed whole ([`Store::remove`]); the
//! devices that hold it create it again. A list with no mark (one created with a token of the
//! authority) never lapses. A mark is written before the first record of its list, and removed
//! after the list's records, so that a crash between the two leaves a mark of a list the store holds
//! no record of, which the relay removes as it starts, and never a list created openly with no mark,
//! which would never lapse.

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::protocol::Record;

/// What a store keeps of a list a device created with no token of the authority: when it was
/// created, and when a login last touched it, in milliseconds since the Unix epoch by the relay's
/// clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Openly {
    pub created_at_ms: u64,
    pub touched_at_ms: u64,
}

/// An append-only record of what a relay was given.
pub trait Store {
    /// Every record kept, each list's in the order it was appended.
    fn load(&mut self) -> io::Result<Vec<(String, Record)>>;

    /// Keep one more record, durably: when this returns `Ok` the record survives the process.
    fn append(&mut self, list: &str, record: &Record) -> io::Result<()>;

    /// The mark of each list created openly the store keeps one for, in the order of the lists'
    /// names.
    fn marks(&mut self) -> io::Result<Vec<(String, Openly)>>;

    /// Keep `mark` as the mark of `list`, in place of the one it had, durably: when this returns
    /// `Ok` the mark survives the process, and a crash leaves the old mark or the new one whole.
    fn mark(&mut self, list: &str, mark: &Openly) -> io::Result<()>;

    /// Remove `list` whole, durably: its records, and then its mark. Removing a list the store
    /// holds nothing of does nothing. A store read again after a crash, or after this failed, holds
    /// the list's records whole or none of them; a mark it holds of no record is the relay's to
    /// remove.
    fn remove(&mut self, list: &str) -> io::Result<()>;
}

/// A store that keeps nothing past the process: for tests, and for a relay whose lists can be
/// rebuilt from its members.
#[derive(Default)]
pub struct MemoryStore {
    rows: Vec<(String, Record)>,
    marks: BTreeMap<String, Openly>,
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

    fn marks(&mut self) -> io::Result<Vec<(String, Openly)>> {
        Ok(self
            .marks
            .iter()
            .map(|(list, mark)| (list.clone(), *mark))
            .collect())
    }

    fn mark(&mut self, list: &str, mark: &Openly) -> io::Result<()> {
        self.marks.insert(list.to_string(), *mark);
        Ok(())
    }

    fn remove(&mut self, list: &str) -> io::Result<()> {
        self.rows.retain(|(held, _)| held != list);
        self.marks.remove(list);
        Ok(())
    }
}

/// A store in a directory: one file for each list, a line for each record, flushed to the disk
/// before an append returns. The file is named by the hexadecimal of the list's name, so no name a
/// device sends reaches a path of its own making.
///
/// What a crash leaves is read as what was acknowledged: a file a list's first record made is
/// written into the directory durably before that record is, and a line a crash cut off is cut from
/// the end of its file before anything is appended after it, so that it never becomes the front of
/// a whole line that is not a record. A store keeps at most [`OPEN_FILES`] files open, whatever the
/// number of lists it holds.
///
/// The mark of a list created openly is a file of its own beside the list's, named by the same
/// hexadecimal with the extension `open`, of one line: written whole to a file beside it and
/// renamed over it, so that a crash leaves the old mark or the new one. Removing a list closes its
/// file, deletes it and then its mark, each deletion written into the directory durably.
pub struct FileStore {
    dir: PathBuf,
    /// The files open for appending, by list, each with the append it was last written by.
    files: BTreeMap<String, (File, u64)>,
    /// How many appends the store has made, which orders the open files by when they were used.
    appends: u64,
}

const EXTENSION: &str = "jsonl";

/// The extension of the file of a list's mark, and of the file a new mark is written to before it
/// is renamed over it.
const MARK_EXTENSION: &str = "open";
const MARK_DRAFT_EXTENSION: &str = "open.draft";

/// The most files a [`FileStore`] keeps open at once: the one written longest ago is closed to open
/// another, so that a relay of many lists does not run out of the descriptors its connections need.
pub const OPEN_FILES: usize = 64;

fn hex(text: &str) -> String {
    text.bytes().map(|byte| format!("{byte:02x}")).collect()
}

/// The name whose hexadecimal [`hex`] wrote, when it is that.
fn unhex(text: &str) -> Option<String> {
    fn digit(character: u8) -> Option<u8> {
        match character {
            b'0'..=b'9' => Some(character - b'0'),
            b'a'..=b'f' => Some(character - b'a' + 10),
            _ => None,
        }
    }
    let bytes = text
        .as_bytes()
        .chunks(2)
        .map(|pair| match pair {
            [high, low] => Some((digit(*high)? << 4) | digit(*low)?),
            _ => None,
        })
        .collect::<Option<Vec<u8>>>()?;
    String::from_utf8(bytes).ok()
}

impl FileStore {
    /// A store in `dir`, which is made when it is not there.
    pub fn open(dir: impl AsRef<Path>) -> io::Result<Self> {
        let dir = dir.as_ref();
        if !dir.is_dir() {
            fs::create_dir_all(dir)?;
            if let Some(parent) = dir.parent().filter(|parent| !parent.as_os_str().is_empty()) {
                sync_directory(parent)?;
            }
        }
        Ok(Self {
            dir: dir.to_path_buf(),
            files: BTreeMap::new(),
            appends: 0,
        })
    }

    fn path_of(&self, list: &str) -> PathBuf {
        self.dir.join(format!("{}.{EXTENSION}", hex(list)))
    }

    fn mark_path_of(&self, list: &str) -> PathBuf {
        self.dir.join(format!("{}.{MARK_EXTENSION}", hex(list)))
    }

    /// The files of the lists the store holds, in the order of their names.
    fn paths(&self) -> io::Result<Vec<PathBuf>> {
        let mut paths: Vec<PathBuf> = fs::read_dir(&self.dir)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == EXTENSION))
            .collect();
        paths.sort();
        Ok(paths)
    }

    /// The names of the lists the store holds a file for, read from the files' names alone: what a
    /// host that places each list in one of several stores (a slot of [`crate::Shards`]) checks
    /// before it opens them. A file whose name is not the hexadecimal of a name is corruption.
    pub fn lists(&self) -> io::Result<Vec<String>> {
        self.paths()?
            .iter()
            .map(PathBuf::as_path)
            .map(list_of)
            .collect()
    }

    /// The file of `list`, open for appending: made, and written into the directory durably, when
    /// the list has none; cut back to its last whole line when a crash left part of one.
    fn open_for_append(&self, list: &str) -> io::Result<File> {
        let path = self.path_of(list);
        let made = OpenOptions::new()
            .read(true)
            .append(true)
            .create_new(true)
            .open(&path);
        match made {
            Ok(file) => {
                sync_directory(&self.dir)?;
                Ok(file)
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                let mut file = OpenOptions::new().read(true).append(true).open(&path)?;
                cut_torn_tail(&mut file)?;
                Ok(file)
            }
            Err(error) => Err(error),
        }
    }
}

/// The list whose file `path` is.
fn list_of(path: &Path) -> io::Result<String> {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .and_then(unhex)
        .ok_or_else(|| {
            invalid(format!(
                "{}: the name is not the hexadecimal of a list's name",
                path.display()
            ))
        })
}

/// Cut what follows the last line end of `file`: the part of a line a crash cut off, which was never
/// acknowledged. Read back from the end, a block at a time, so that a long file costs one block.
fn cut_torn_tail(file: &mut File) -> io::Result<()> {
    const BLOCK: u64 = 64 * 1024;
    let length = file.metadata()?.len();
    let mut whole = length;
    let mut block = Vec::new();
    while whole > 0 {
        let start = whole.saturating_sub(BLOCK);
        block.resize((whole - start) as usize, 0);
        file.seek(SeekFrom::Start(start))?;
        file.read_exact(&mut block)?;
        if let Some(end) = block.iter().rposition(|byte| *byte == b'\n') {
            whole = start + end as u64 + 1;
            break;
        }
        whole = start;
    }
    if whole < length {
        file.set_len(whole)?;
        file.sync_data()?;
    }
    Ok(())
}

/// Delete the file at `path`, when there is one.
fn remove_if_there(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        done => done,
    }
}

/// A 64-bit number of a mark, written as text.
fn number_in(value: &Value, name: &str) -> Option<u64> {
    value.get(name)?.as_str()?.parse().ok()
}

/// Make what was done to the entries of `dir` (a file made in it) survive the machine.
#[cfg(unix)]
fn sync_directory(dir: &Path) -> io::Result<()> {
    File::open(dir)?.sync_all()
}

/// A directory is not opened as a file on this system, and its entries are written through by the
/// file system's own journal.
#[cfg(not(unix))]
fn sync_directory(_dir: &Path) -> io::Result<()> {
    Ok(())
}

fn invalid(message: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

impl Store for FileStore {
    fn load(&mut self) -> io::Result<Vec<(String, Record)>> {
        let paths = self.paths()?;
        let mut rows = Vec::new();
        for path in paths {
            let named = list_of(&path)?;
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
                    (Some(list), Some(_)) if list != named => {
                        return Err(invalid(format!(
                            "{}: line {} is a record of another list",
                            path.display(),
                            number + 1
                        )))
                    }
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
        self.appends += 1;
        let now = self.appends;
        if !self.files.contains_key(list) {
            if self.files.len() >= OPEN_FILES {
                let oldest = self
                    .files
                    .iter()
                    .min_by_key(|(_, (_, used))| *used)
                    .map(|(held, _)| held.clone());
                if let Some(oldest) = oldest {
                    self.files.remove(&oldest);
                }
            }
            let file = self.open_for_append(list)?;
            self.files.insert(list.to_string(), (file, now));
        }
        let (file, used) = self
            .files
            .get_mut(list)
            .ok_or_else(|| io::Error::other("the file was just opened"))?;
        *used = now;
        let mut line = json!({"list": list, "record": record.to_value()}).to_string();
        line.push('\n');
        let written = file
            .write_all(line.as_bytes())
            .and_then(|()| file.sync_data());
        if written.is_err() {
            // What reached the file of a failed write is cut when the file is next opened.
            self.files.remove(list);
        }
        written
    }

    fn marks(&mut self) -> io::Result<Vec<(String, Openly)>> {
        let mut paths: Vec<PathBuf> = fs::read_dir(&self.dir)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == MARK_EXTENSION))
            .collect();
        paths.sort();
        let mut marks = Vec::new();
        for path in paths {
            let named = list_of(&path)?;
            let not_a_mark = || invalid(format!("{}: it is not a list's mark", path.display()));
            let text = fs::read_to_string(&path)?;
            let value: Value = serde_json::from_str(text.trim_end()).map_err(|_| not_a_mark())?;
            if value.get("list").and_then(Value::as_str) != Some(named.as_str()) {
                return Err(invalid(format!(
                    "{}: it is the mark of another list",
                    path.display()
                )));
            }
            let mark = Openly {
                created_at_ms: number_in(&value, "created_at_ms").ok_or_else(not_a_mark)?,
                touched_at_ms: number_in(&value, "touched_at_ms").ok_or_else(not_a_mark)?,
            };
            marks.push((named, mark));
        }
        Ok(marks)
    }

    fn mark(&mut self, list: &str, mark: &Openly) -> io::Result<()> {
        let path = self.mark_path_of(list);
        let draft = self
            .dir
            .join(format!("{}.{MARK_DRAFT_EXTENSION}", hex(list)));
        let mut line = json!({
            "list": list,
            "created_at_ms": mark.created_at_ms.to_string(),
            "touched_at_ms": mark.touched_at_ms.to_string(),
        })
        .to_string();
        line.push('\n');
        {
            let mut file = File::create(&draft)?;
            file.write_all(line.as_bytes())?;
            file.sync_all()?;
        }
        fs::rename(&draft, &path)?;
        sync_directory(&self.dir)
    }

    fn remove(&mut self, list: &str) -> io::Result<()> {
        // The file open for appending is closed before it is deleted.
        self.files.remove(list);
        remove_if_there(&self.path_of(list))?;
        sync_directory(&self.dir)?;
        remove_if_there(&self.mark_path_of(list))?;
        sync_directory(&self.dir)
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
            event: "record_list.change".to_string(),
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

    #[test]
    fn a_record_appended_after_a_crash_does_not_join_the_line_the_crash_cut_off() {
        let dir = scratch("after-crash");
        let mut store = FileStore::open(&dir).unwrap();
        store.append("home", &op(1)).unwrap();
        let path = store.path_of("home");
        drop(store);
        let mut file = OpenOptions::new().append(true).open(&path).unwrap();
        file.write_all(b"{\"list\":\"home\",\"rec").unwrap();
        drop(file);

        // The relay starts again, reads what was acknowledged, and takes one more record.
        let mut again = FileStore::open(&dir).unwrap();
        assert_eq!(again.load().unwrap().len(), 1);
        again.append("home", &op(2)).unwrap();
        let rows = FileStore::open(&dir).unwrap().load().unwrap();
        assert_eq!(
            rows,
            vec![("home".to_string(), op(1)), ("home".to_string(), op(2))]
        );
        assert!(fs::read_to_string(&path).unwrap().ends_with("}}\n"));

        // A file that is all one cut-off line is cut to nothing.
        fs::write(store_path(&dir, "work"), b"{\"list\":\"wo").unwrap();
        let mut third = FileStore::open(&dir).unwrap();
        third.append("work", &op(3)).unwrap();
        let rows = FileStore::open(&dir).unwrap().load().unwrap();
        assert_eq!(rows.len(), 3);
        assert!(rows.contains(&("work".to_string(), op(3))));
        fs::remove_dir_all(&dir).unwrap();
    }

    fn store_path(dir: &Path, list: &str) -> PathBuf {
        dir.join(format!("{}.{EXTENSION}", hex(list)))
    }

    #[test]
    fn a_store_of_many_lists_keeps_a_bounded_number_of_files_open() {
        let dir = scratch("many");
        let mut store = FileStore::open(&dir).unwrap();
        let lists = OPEN_FILES * 2 + 1;
        for n in 0..lists {
            store.append(&format!("list-{n}"), &op(n as u32)).unwrap();
            assert!(store.files.len() <= OPEN_FILES);
        }
        // The first list's file was closed long ago, and is opened again to take another record.
        assert!(!store.files.contains_key("list-0"));
        store.append("list-0", &op(1000)).unwrap();
        assert!(store.files.contains_key("list-0"));
        assert!(
            store.files.contains_key(&format!("list-{}", lists - 1)),
            "the file written last is kept"
        );
        let rows = FileStore::open(&dir).unwrap().load().unwrap();
        assert_eq!(rows.len(), lists + 1);
        assert_eq!(
            rows.iter()
                .filter(|(list, _)| list == "list-0")
                .map(|(_, record)| record.clone())
                .collect::<Vec<_>>(),
            vec![op(0), op(1000)]
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_store_names_its_lists_by_their_files_and_refuses_a_file_that_names_none() {
        let dir = scratch("lists");
        let mut store = FileStore::open(&dir).unwrap();
        for list in ["home", "../x", "\u{e9}t\u{e9}"] {
            store.append(list, &op(1)).unwrap();
        }
        let mut lists = store.lists().unwrap();
        lists.sort();
        assert_eq!(lists, vec!["../x", "home", "\u{e9}t\u{e9}"]);

        for stray in ["zz", "abc", "414A", "ff"] {
            let path = dir.join(format!("{stray}.{EXTENSION}"));
            fs::write(&path, b"").unwrap();
            let error = store.lists().unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::InvalidData, "{stray}");
            assert!(error.to_string().contains(stray), "{error}");
            assert!(store.load().is_err(), "{stray}");
            fs::remove_file(&path).unwrap();
        }
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_record_in_the_file_of_another_list_is_corruption() {
        let dir = scratch("misplaced");
        let mut store = FileStore::open(&dir).unwrap();
        store.append("work", &op(1)).unwrap();
        fs::rename(store_path(&dir, "work"), store_path(&dir, "home")).unwrap();
        let error = FileStore::open(&dir).unwrap().load().unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("another list"), "{error}");
        fs::remove_dir_all(&dir).unwrap();
    }

    fn openly(created_at_ms: u64, touched_at_ms: u64) -> Openly {
        Openly {
            created_at_ms,
            touched_at_ms,
        }
    }

    #[test]
    fn a_mark_is_kept_in_place_of_the_one_before_and_read_back_by_its_list() {
        let dir = scratch("marks");
        let mut store = FileStore::open(&dir).unwrap();
        store.append("../x", &op(1)).unwrap();
        store.append("home", &op(2)).unwrap();
        store.mark("home", &openly(10, 10)).unwrap();
        store.mark("home", &openly(10, 99)).unwrap();
        store.mark("../x", &openly(5, 6)).unwrap();
        let mut again = FileStore::open(&dir).unwrap();
        let mut marks = again.marks().unwrap();
        marks.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(
            marks,
            vec![
                ("../x".to_string(), openly(5, 6)),
                ("home".to_string(), openly(10, 99)),
            ]
        );
        // A mark is no list: the lists and their records are what they were.
        let mut lists = again.lists().unwrap();
        lists.sort();
        assert_eq!(lists, vec!["../x", "home"]);
        assert_eq!(again.load().unwrap().len(), 2);
        // A draft a crash left before its rename is not a mark.
        fs::write(
            dir.join(format!("{}.{MARK_DRAFT_EXTENSION}", hex("work"))),
            b"{\"list\":\"wo",
        )
        .unwrap();
        assert_eq!(again.marks().unwrap().len(), 2);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_mark_that_is_not_one_or_names_another_list_is_corruption() {
        let dir = scratch("bad-marks");
        let mut store = FileStore::open(&dir).unwrap();
        store.mark("home", &openly(1, 2)).unwrap();
        let path = store.mark_path_of("home");
        fs::write(
            &path,
            b"{\"list\":\"work\",\"created_at_ms\":\"1\",\"touched_at_ms\":\"2\"}\n",
        )
        .unwrap();
        let error = store.marks().unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("another list"), "{error}");
        fs::write(&path, b"{\"list\":\"home\",\"created_at_ms\":1}\n").unwrap();
        assert_eq!(
            store.marks().unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_list_removed_is_gone_whole_and_its_name_can_be_held_again() {
        let dir = scratch("remove");
        let mut store = FileStore::open(&dir).unwrap();
        store.mark("home", &openly(1, 1)).unwrap();
        store.append("home", &op(1)).unwrap();
        store.append("work", &op(2)).unwrap();
        assert!(store.files.contains_key("home"), "its file is open");
        store.remove("home").unwrap();
        assert!(!store.files.contains_key("home"), "its file was closed");
        assert!(!store.path_of("home").exists() && !store.mark_path_of("home").exists());
        let mut again = FileStore::open(&dir).unwrap();
        assert_eq!(again.load().unwrap(), vec![("work".to_string(), op(2))]);
        assert_eq!(again.marks().unwrap(), Vec::new());
        // Removing what the store holds nothing of does nothing.
        again.remove("home").unwrap();
        again.remove("never").unwrap();
        // The name is a new list's from its first record on.
        store.append("home", &op(3)).unwrap();
        let rows = FileStore::open(&dir).unwrap().load().unwrap();
        assert!(rows.contains(&("home".to_string(), op(3))));
        assert!(!rows.contains(&("home".to_string(), op(1))));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_memory_store_keeps_marks_and_removes_a_list_whole() {
        let mut store = MemoryStore::new();
        store.append("home", &op(1)).unwrap();
        store.append("work", &op(2)).unwrap();
        store.mark("home", &openly(1, 1)).unwrap();
        store.mark("home", &openly(1, 4)).unwrap();
        assert_eq!(
            store.marks().unwrap(),
            vec![("home".to_string(), openly(1, 4))]
        );
        store.remove("home").unwrap();
        assert_eq!(store.load().unwrap(), vec![("work".to_string(), op(2))]);
        assert!(store.marks().unwrap().is_empty());
    }

    #[test]
    fn a_store_whose_directory_is_new_is_made_with_its_parents() {
        let dir = scratch("nested").join("a").join("b");
        let mut store = FileStore::open(&dir).unwrap();
        store.append("home", &op(1)).unwrap();
        assert_eq!(FileStore::open(&dir).unwrap().load().unwrap().len(), 1);
        fs::remove_dir_all(dir.parent().unwrap().parent().unwrap()).unwrap();
    }
}
