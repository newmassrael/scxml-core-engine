// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The store directory of a relay: a directory `slot-<n>` for each slot of the relay
//! ([`Shards`]), each a [`FileStore`] of the lists whose names belong to that slot ([`slot_of`]).
//!
//! The number of slots is the deployment's, set when it begins: a list in a slot its name does not
//! belong to would be read by a relay that is never asked for it, and the list would seem empty to
//! its devices. So before anything is read the layout is checked against the number configured,
//! and a relay whose store was laid out for another number does not start. Changing the number is
//! moving each list's file into the slot its name belongs to while the relay is stopped
//! (`docs/SCE_RELAY_DEPLOY.md`), which this check then confirms.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use sce_rust_relay::{slot_of, Authority, FileStore, Limits, Shards, Store};

/// The most slots a relay is configured with.
pub const MOST_SLOTS: usize = 4096;

/// Why a store was not opened.
#[derive(Debug)]
pub struct StoreError {
    pub kind: StoreErrorKind,
    pub message: String,
}

/// What kind of wrong a [`StoreError`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreErrorKind {
    /// The directory cannot be made, read or written.
    Unreadable,
    /// What it holds is not a store laid out for this relay, or does not verify.
    Invalid,
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for StoreError {}

fn from_io(store: &Path, error: io::Error) -> StoreError {
    let kind = if error.kind() == io::ErrorKind::InvalidData {
        StoreErrorKind::Invalid
    } else {
        StoreErrorKind::Unreadable
    };
    StoreError {
        kind,
        message: format!("the store at {}: {error}", store.display()),
    }
}

fn invalid(message: String) -> StoreError {
    StoreError {
        kind: StoreErrorKind::Invalid,
        message,
    }
}

/// The directory of slot `slot` of the store at `store`.
pub fn slot_directory(store: &Path, slot: usize) -> PathBuf {
    store.join(format!("slot-{slot}"))
}

/// Whether the store at `store` is laid out for `slots` slots: no slot's directory past them, and
/// each list's file in the directory of the slot its name belongs to. A store that is not there yet
/// is one of a relay that has not begun, and is laid out for any number.
pub fn check_layout(store: &Path, slots: usize) -> Result<(), StoreError> {
    let entries = match fs::read_dir(store) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(from_io(store, error)),
    };
    for entry in entries {
        let entry = entry.map_err(|error| from_io(store, error))?;
        let name = entry.file_name();
        let Some(number) = name.to_str().and_then(|name| name.strip_prefix("slot-")) else {
            continue;
        };
        match number.parse::<usize>() {
            Ok(slot) if slot < slots => {}
            _ => {
                return Err(invalid(format!(
                    "the store at {} holds {}, and the relay is configured for {slots} \
                     slot(s): the number of slots is the store's, set when it began",
                    store.display(),
                    entry.path().display()
                )))
            }
        }
    }
    for slot in 0..slots {
        let directory = slot_directory(store, slot);
        if !directory.is_dir() {
            continue;
        }
        // A list's records and the mark of a list created openly both belong in its slot: a mark
        // left in another is read by a relay that holds no record of its list, and dropped.
        let lists = FileStore::open(&directory)
            .and_then(|mut held| {
                let mut lists = held.lists()?;
                lists.extend(held.marks()?.into_iter().map(|(list, _)| list));
                Ok(lists)
            })
            .map_err(|error| from_io(store, error))?;
        for list in lists {
            let belongs = slot_of(&list, slots);
            if belongs != slot {
                return Err(invalid(format!(
                    "{} holds a list whose name belongs in {}: it was laid out for another \
                     number of slots",
                    directory.display(),
                    slot_directory(store, belongs).display()
                )));
            }
        }
    }
    Ok(())
}

/// The relay of `slots` slots over the store at `store`, each slot's records read back through the
/// checks that took them in: made when it is not there, refused when it is laid out for another
/// number of slots or holds what does not verify.
pub fn open(
    store: &Path,
    slots: usize,
    authority: Authority,
    limits: Limits,
) -> Result<Shards<FileStore>, StoreError> {
    fs::create_dir_all(store).map_err(|error| from_io(store, error))?;
    check_layout(store, slots)?;
    Shards::open(
        slots,
        |slot| FileStore::open(slot_directory(store, slot)),
        authority,
        limits,
    )
    .map_err(|error| from_io(store, error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sce_rust_relay::{Openly, Record};

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "sce-relay-slots-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |elapsed| elapsed.as_nanos())
        ));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn record() -> Record {
        Record::Operation {
            event: "record_list.change".to_string(),
            data: "{}".to_string(),
        }
    }

    /// Put a record of `list` in the store of `slot`, whatever slot its name belongs to.
    fn place(store: &Path, slot: usize, list: &str) {
        FileStore::open(slot_directory(store, slot))
            .unwrap()
            .append(list, &record())
            .unwrap();
    }

    #[test]
    fn a_store_that_is_not_there_is_laid_out_for_any_number() {
        let store = scratch("absent");
        assert!(check_layout(&store, 1).is_ok());
        assert!(check_layout(&store, 64).is_ok());
    }

    #[test]
    fn each_list_in_the_slot_its_name_belongs_to_is_a_store_laid_out_for_that_number() {
        let store = scratch("laid-out");
        for list in ["home", "work", "a", "b"] {
            place(&store, slot_of(list, 4), list);
        }
        assert!(check_layout(&store, 4).is_ok());
        fs::remove_dir_all(&store).unwrap();
    }

    #[test]
    fn a_list_in_another_slot_is_refused_and_named_by_its_directory() {
        let store = scratch("misplaced");
        let (list, slots) = ("home", 4);
        let wrong = (slot_of(list, slots) + 1) % slots;
        place(&store, wrong, list);
        let refused = check_layout(&store, slots).unwrap_err();
        assert_eq!(refused.kind, StoreErrorKind::Invalid);
        assert!(
            refused.message.contains(&format!("slot-{wrong}"))
                && refused
                    .message
                    .contains(&format!("slot-{}", slot_of(list, slots))),
            "{refused}"
        );
        fs::remove_dir_all(&store).unwrap();
    }

    #[test]
    fn the_mark_of_a_list_in_another_slot_is_refused_as_its_records_are() {
        let store = scratch("misplaced-mark");
        let (list, slots) = ("home", 4);
        let wrong = (slot_of(list, slots) + 1) % slots;
        FileStore::open(slot_directory(&store, wrong))
            .unwrap()
            .mark(
                list,
                &Openly {
                    created_at_ms: 1,
                    touched_at_ms: 1,
                },
            )
            .unwrap();
        let refused = check_layout(&store, slots).unwrap_err();
        assert_eq!(refused.kind, StoreErrorKind::Invalid);
        assert!(
            refused.message.contains(&format!("slot-{wrong}")),
            "{refused}"
        );
        fs::remove_dir_all(&store).unwrap();
    }

    #[test]
    fn a_slot_past_the_number_configured_is_refused() {
        let store = scratch("past");
        place(&store, 0, "home");
        fs::create_dir_all(slot_directory(&store, 3)).unwrap();
        let refused = check_layout(&store, 2).unwrap_err();
        assert_eq!(refused.kind, StoreErrorKind::Invalid);
        assert!(refused.message.contains("slot-3"), "{refused}");
        fs::create_dir_all(store.join("slot-x")).unwrap();
        fs::remove_dir_all(slot_directory(&store, 3)).unwrap();
        assert!(
            check_layout(&store, 2).is_err(),
            "a slot that is not a number"
        );
        fs::remove_dir_all(store.join("slot-x")).unwrap();
        // Other entries are not the relay's, and are left to whoever put them there.
        fs::create_dir_all(store.join("lost+found")).unwrap();
        assert!(check_layout(&store, 2).is_ok());
        fs::remove_dir_all(&store).unwrap();
    }

    #[test]
    fn a_relay_opened_over_a_new_store_makes_its_directory() {
        let store = scratch("new").join("state");
        let shards = open(
            &store,
            2,
            Authority::new([1; 32], "relay"),
            Limits::default(),
        )
        .unwrap();
        assert_eq!(shards.count(), 2);
        assert!(slot_directory(&store, 0).is_dir() && slot_directory(&store, 1).is_dir());
        fs::remove_dir_all(store.parent().unwrap()).unwrap();
    }

    #[test]
    fn a_store_that_does_not_verify_is_invalid_and_says_so() {
        let store = scratch("forged");
        // An operation the relay never took: there is no log it could stand under.
        place(&store, slot_of("home", 1), "home");
        let refused = open(
            &store,
            1,
            Authority::new([1; 32], "relay"),
            Limits::default(),
        )
        .err()
        .expect("a store that does not verify does not open");
        assert_eq!(refused.kind, StoreErrorKind::Invalid);
        assert!(refused.message.contains("does not verify"), "{refused}");
        fs::remove_dir_all(&store).unwrap();
    }
}
