// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The connections a person has made, in their own settings folder and not in the works folder.
//!
//! ```text
//! <root>/.lock                        taken by every writer, for the whole folder
//! <root>/default                      the id of the connection a new request uses
//! <root>/connections/<id>/<rev>.json  one revision: the bytes its name is the digest of
//! <root>/connections/<id>/head        the revision that is current, or no file once deleted
//! ```
//!
//! A works folder is shared and moved; whose account a person reaches a model with is nobody
//! else's business, so it is not part of any work, and a folder of works carries none of it.
//! A request pins a connection by id and revision, so a revision is never rewritten: a save
//! writes a new file and then moves `head` by an atomic rename, and a reader that sees the new
//! head finds the file it names. Deleting removes `head` and keeps the revisions, so a request
//! made while the connection existed can still say what it was made with.
//!
//! One lock covers the folder. These are a few small files that change when a person changes a
//! setting, so the cost of one lock is nothing, and it is what lets a save, a delete and a
//! change of default each be checked against what they replace and done before anything else
//! is.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::json;

use super::{atomic_write, Saved, Unreadable, LOCK_WAIT};
use crate::connection::{Connection, ConnectionId};
use crate::error::StoreError;
use crate::lock;
use crate::revision::Revision;

const CONNECTIONS_DIR: &str = "connections";
const HEAD_FILE: &str = "head";
const DEFAULT_FILE: &str = "default";
const LOCK_FILE: &str = ".lock";

/// Where the settings of the person who runs this program are kept: `SCE_SETTINGS_DIR`, else the
/// per-user configuration directory of the platform. The works are kept in the data directory
/// (`default_root`) and these in the configuration directory, so that a person who copies,
/// shares or backs up their works does not carry their accounts with them.
pub fn default_settings_root() -> Option<PathBuf> {
    settings_root_from(|name| std::env::var_os(name))
}

fn settings_root_from(var: impl Fn(&str) -> Option<std::ffi::OsString>) -> Option<PathBuf> {
    let set = |name: &str| var(name).filter(|v| !v.is_empty());
    if let Some(dir) = set("SCE_SETTINGS_DIR") {
        return Some(PathBuf::from(dir));
    }
    let config = if cfg!(windows) {
        set("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        set("HOME").map(|h| PathBuf::from(h).join("Library/Application Support"))
    } else {
        set("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| set("HOME").map(|h| PathBuf::from(h).join(".config")))
    };
    config.map(|dir| dir.join("sce-workbench").join("settings"))
}

/// A connection as it is kept, with the revision it is kept under.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StoredConnection {
    pub connection: Connection,
    pub revision: Revision,
}

/// Every connection that is current, by id, and every folder that should have been one and
/// could not be read: a connection that vanishes from the list because a file was damaged
/// looks, to the person, like a connection that was never made.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConnectionListing {
    pub connections: Vec<StoredConnection>,
    pub unreadable: Vec<Unreadable>,
}

/// The settings folder at one root.
#[derive(Debug, Clone)]
pub struct ConnectionStore {
    root: PathBuf,
}

impl ConnectionStore {
    /// The settings folder at `root`. The folder is made by the first write.
    pub fn at(root: impl Into<PathBuf>) -> Self {
        ConnectionStore { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn connection_dir(&self, id: &ConnectionId) -> PathBuf {
        self.root.join(CONNECTIONS_DIR).join(id.as_str())
    }

    fn lock(&self) -> Result<lock::Held, StoreError> {
        fs::create_dir_all(&self.root).map_err(|e| StoreError::io(&self.root, e))?;
        lock::exclusive(&self.root.join(LOCK_FILE), LOCK_WAIT)
    }

    /// Keep `connection` as the current one under its id, replacing the revision `base` names.
    ///
    /// `base` is what the caller read before it changed anything: `None` for a connection it
    /// believes is not there. A save that does not name what it replaces is a conflict, as a
    /// save of a work's text is, so a second window cannot quietly undo the first.
    pub fn save(
        &self,
        connection: &Connection,
        base: Option<&Revision>,
    ) -> Result<Saved, StoreError> {
        connection.validate()?;
        let bytes = connection.stored_bytes();
        let revision = Revision::of(&bytes);
        let _held = self.lock()?;
        let current = self.head(&connection.id)?;
        if current.as_ref() != base {
            return Err(StoreError::Conflict {
                base: base.cloned(),
                current,
            });
        }
        if current.as_ref() == Some(&revision) {
            return Ok(Saved::Unchanged { revision });
        }
        let dir = self.connection_dir(&connection.id);
        fs::create_dir_all(&dir).map_err(|e| StoreError::io(&dir, e))?;
        // The file first and the head after: whoever sees the head finds what it names. A
        // revision that is saved again after a change back is written again, with the same
        // bytes, which also puts right a file that was damaged.
        atomic_write(&dir.join(format!("{revision}.json")), &bytes)?;
        atomic_write(&dir.join(HEAD_FILE), format!("{revision}\n").as_bytes())?;
        Ok(Saved::Saved {
            revision,
            parent: current,
        })
    }

    /// The connection `id` at `revision`, or at its head when no revision is named. `None` for a
    /// connection that has none (never made, or deleted). A named revision that is not kept is
    /// `not-found`, and one whose bytes are not what its name says is `corrupt`.
    pub fn read(
        &self,
        id: &ConnectionId,
        revision: Option<&Revision>,
    ) -> Result<Option<StoredConnection>, StoreError> {
        let revision = match revision {
            Some(revision) => revision.clone(),
            None => match self.head(id)? {
                Some(head) => head,
                None => return Ok(None),
            },
        };
        let path = self.connection_dir(id).join(format!("{revision}.json"));
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return Err(StoreError::NotFound {
                    what: format!("revision {} of connection `{id}`", revision.short()),
                });
            }
            Err(e) => return Err(StoreError::io(&path, e)),
        };
        if Revision::of(&bytes) != revision {
            return Err(StoreError::corrupt(
                &path,
                "its bytes are not the revision it is named for",
            ));
        }
        // What a person or a program wrote into the folder by hand is held to the rules a save
        // is: a field this type does not have, a server address with a user in it, a credential
        // source the adapter has none of.
        let connection: Connection = serde_json::from_slice(&bytes)
            .map_err(|e| StoreError::corrupt(&path, e.to_string()))?;
        connection
            .validate()
            .map_err(|e| StoreError::corrupt(&path, e.to_string()))?;
        if connection.id != *id {
            return Err(StoreError::corrupt(
                &path,
                format!("the file says it is connection `{}`", connection.id),
            ));
        }
        Ok(Some(StoredConnection {
            connection,
            revision,
        }))
    }

    /// Every connection that is current, by id.
    pub fn list(&self) -> Result<ConnectionListing, StoreError> {
        let folder = self.root.join(CONNECTIONS_DIR);
        let entries = match fs::read_dir(&folder) {
            Ok(entries) => entries,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return Ok(ConnectionListing {
                    connections: Vec::new(),
                    unreadable: Vec::new(),
                });
            }
            Err(e) => return Err(StoreError::io(&folder, e)),
        };
        let mut connections = Vec::new();
        let mut unreadable = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| StoreError::io(&folder, e))?;
            if !entry.path().is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            let id = match ConnectionId::parse(&name) {
                Ok(id) => id,
                Err(e) => {
                    unreadable.push(Unreadable {
                        id: name,
                        reason: e.to_string(),
                    });
                    continue;
                }
            };
            match self.head(&id).and_then(|head| match head {
                None => Ok(None),
                Some(_) => self.read(&id, None),
            }) {
                Ok(Some(stored)) => connections.push(stored),
                // No head: deleted, and what a delete leaves behind is not a connection.
                Ok(None) => {}
                Err(e) => unreadable.push(Unreadable {
                    id: name,
                    reason: e.to_string(),
                }),
            }
        }
        connections.sort_by(|a, b| a.connection.id.cmp(&b.connection.id));
        unreadable.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(ConnectionListing {
            connections,
            unreadable,
        })
    }

    /// Take a connection out of the list. Its revisions stay, so that a request made while it
    /// existed can still say what it was made with. `base` is the revision the caller deletes.
    pub fn delete(&self, id: &ConnectionId, base: &Revision) -> Result<(), StoreError> {
        let _held = self.lock()?;
        let Some(current) = self.head(id)? else {
            return Err(StoreError::NotFound {
                what: format!("connection `{id}`"),
            });
        };
        if &current != base {
            return Err(StoreError::Conflict {
                base: Some(base.clone()),
                current: Some(current),
            });
        }
        let head = self.connection_dir(id).join(HEAD_FILE);
        fs::remove_file(&head).map_err(|e| StoreError::io(&head, e))?;
        if self.default_connection()?.as_ref() == Some(id) {
            self.remove_default()?;
        }
        Ok(())
    }

    /// The connection a new request uses when the person does not choose one.
    pub fn default_connection(&self) -> Result<Option<ConnectionId>, StoreError> {
        let path = self.root.join(DEFAULT_FILE);
        match fs::read_to_string(&path) {
            Ok(text) => ConnectionId::parse(text.trim())
                .map(Some)
                .map_err(|e| StoreError::corrupt(&path, e.to_string())),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(StoreError::io(&path, e)),
        }
    }

    /// Make `id` the default, or no connection when it is `None`. `expect` is the default the
    /// caller saw; another window that changed it first makes this `moved`.
    pub fn set_default(
        &self,
        id: Option<&ConnectionId>,
        expect: Option<&ConnectionId>,
    ) -> Result<(), StoreError> {
        let _held = self.lock()?;
        let current = self.default_connection()?;
        if current.as_ref() != expect {
            return Err(StoreError::refused(
                "moved",
                "the default connection was changed after it was read, so nothing was changed; \
                 read it again",
                json!({ "expected": expect, "current": current }),
            ));
        }
        match id {
            Some(id) => {
                if self.head(id)?.is_none() {
                    return Err(StoreError::NotFound {
                        what: format!("connection `{id}`"),
                    });
                }
                atomic_write(&self.root.join(DEFAULT_FILE), format!("{id}\n").as_bytes())
            }
            None => self.remove_default(),
        }
    }

    fn remove_default(&self) -> Result<(), StoreError> {
        let path = self.root.join(DEFAULT_FILE);
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(StoreError::io(&path, e)),
        }
    }

    /// The revision a connection is at, or `None` when it has none.
    fn head(&self, id: &ConnectionId) -> Result<Option<Revision>, StoreError> {
        let path = self.connection_dir(id).join(HEAD_FILE);
        match fs::read_to_string(&path) {
            Ok(text) => Revision::parse(text.trim())
                .map(Some)
                .map_err(|e| StoreError::corrupt(&path, e.to_string())),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(StoreError::io(&path, e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::*;

    fn environment(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
        let pairs: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |name| {
            pairs
                .iter()
                .find(|(k, _)| k == name)
                .map(|(_, v)| OsString::from(v))
        }
    }

    #[test]
    fn the_settings_folder_is_named_by_the_environment_before_the_platform() {
        let root = settings_root_from(environment(&[
            ("SCE_SETTINGS_DIR", "/somewhere/settings"),
            ("XDG_CONFIG_HOME", "/ignored"),
            ("HOME", "/ignored"),
        ]));

        assert_eq!(root, Some(PathBuf::from("/somewhere/settings")));
    }

    #[test]
    fn an_empty_variable_names_nothing() {
        let root = settings_root_from(environment(&[("SCE_SETTINGS_DIR", ""), ("HOME", "")]));

        assert_eq!(root, None);
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn the_settings_are_in_the_configuration_directory_and_not_beside_the_works() {
        let from_xdg = settings_root_from(environment(&[
            ("XDG_CONFIG_HOME", "/xdg/config"),
            ("HOME", "/home/coin"),
        ]));
        let from_home = settings_root_from(environment(&[("HOME", "/home/coin")]));

        assert_eq!(
            from_xdg,
            Some(PathBuf::from("/xdg/config/sce-workbench/settings"))
        );
        assert_eq!(
            from_home,
            Some(PathBuf::from("/home/coin/.config/sce-workbench/settings"))
        );
        // The works live under the data directory: a copy of them carries no settings.
        assert_ne!(
            from_home.unwrap().parent(),
            Some(Path::new("/home/coin/.local/share/sce-workbench"))
        );
    }
}
