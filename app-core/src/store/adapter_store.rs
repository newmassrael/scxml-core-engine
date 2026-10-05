// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Which AI adapters are there: a record each, beside the works.
//!
//! ```text
//! <root>/.sce-adapters/<name>.json   what the adapter said it is and what it can do, and when
//! ```
//!
//! An adapter says it is there by reporting, and is there for as long as its last report is
//! recent ([`ADAPTER_LIVE_SECONDS`]). The record outlives the report on purpose: a screen
//! says "the desktop adapter was last seen at ..." and not nothing. It is soft state and
//! not a fact about any work, so it takes no work's lock: one adapter writes its own file, by
//! an atomic rename, and the files of two adapters are two files.
//!
//! The folder begins with a dot, which no work id does, so it is never listed as a work.

use std::fs;
use std::io;

use serde::{Deserialize, Serialize};
use serde_json::json;

use super::request_store::valid_name;
use super::{atomic_write, Unreadable, WorkStore};
use crate::clock::{utc_timestamp, Clock};
use crate::error::StoreError;

const ADAPTERS_DIR: &str = ".sce-adapters";
const CAPABILITIES_MAX: usize = 16;

/// How long after its last report an adapter is still there, in seconds: three of the
/// minute-long reports an adapter is asked to send, so one that is late is not gone.
pub const ADAPTER_LIVE_SECONDS: u64 = 90;

/// What an adapter says when it reports.
#[derive(Debug, Clone)]
pub struct AdapterReport<'a> {
    /// What it is called: the name it takes requests under (`desktop`).
    pub name: &'a str,
    /// Which kind of AI client it runs (`claude-code`).
    pub kind: &'a str,
    /// What it can do (`generate`, `progress`, `cancel`, `resume`). A client that cannot
    /// cancel says so, and the screen does not offer to.
    pub capabilities: Vec<String>,
    pub version: Option<&'a str>,
}

/// An adapter as it is kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Adapter {
    pub name: String,
    pub kind: String,
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub version: Option<String>,
    pub seen_at: String,
    /// The same moment, as seconds since the Unix epoch.
    pub seen_epoch: u64,
}

/// An adapter, and whether it is there at this moment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterStatus {
    pub adapter: Adapter,
    pub live: bool,
}

/// Every adapter that ever reported, by name, and every record that could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterListing {
    pub adapters: Vec<AdapterStatus>,
    pub unreadable: Vec<Unreadable>,
}

fn bad(what: &str) -> StoreError {
    StoreError::refused("bad-adapter", what.to_string(), json!(null))
}

impl<C: Clock> WorkStore<C> {
    /// An adapter says it is there, and what it can do. Replaces what it said before.
    pub fn report_adapter(&self, report: AdapterReport<'_>) -> Result<AdapterStatus, StoreError> {
        if !valid_name(report.name) {
            return Err(bad(
                "an adapter is named by 1 to 64 letters, digits and `._:-`, starting with a letter or a digit",
            ));
        }
        if !valid_name(report.kind) {
            return Err(bad(
                "an adapter's kind is 1 to 64 letters, digits and `._:-`, starting with a letter or a digit",
            ));
        }
        if report.capabilities.len() > CAPABILITIES_MAX
            || report.capabilities.iter().any(|c| !valid_name(c))
        {
            return Err(bad(
                "an adapter reports at most 16 capabilities, each 1 to 64 letters, digits and `._:-`",
            ));
        }
        let epoch = self.clock.epoch();
        let adapter = Adapter {
            name: report.name.to_string(),
            kind: report.kind.to_string(),
            capabilities: report.capabilities,
            version: report.version.map(str::to_string),
            seen_at: utc_timestamp(epoch),
            seen_epoch: epoch,
        };
        let folder = self.root.join(ADAPTERS_DIR);
        fs::create_dir_all(&folder).map_err(|e| StoreError::io(&folder, e))?;
        let path = folder.join(format!("{}.json", adapter.name));
        let mut bytes = serde_json::to_vec_pretty(&adapter)
            .map_err(|e| StoreError::corrupt(&path, e.to_string()))?;
        bytes.push(b'\n');
        atomic_write(&path, &bytes)?;
        Ok(AdapterStatus {
            adapter,
            live: true,
        })
    }

    /// Every adapter that reported, by name, each with whether it is there now.
    pub fn adapter_status(&self) -> Result<AdapterListing, StoreError> {
        let folder = self.root.join(ADAPTERS_DIR);
        let entries = match fs::read_dir(&folder) {
            Ok(entries) => entries,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return Ok(AdapterListing {
                    adapters: Vec::new(),
                    unreadable: Vec::new(),
                })
            }
            Err(e) => return Err(StoreError::io(&folder, e)),
        };
        let now = self.clock.epoch();
        let mut listing = AdapterListing {
            adapters: Vec::new(),
            unreadable: Vec::new(),
        };
        for entry in entries {
            let entry = entry.map_err(|e| StoreError::io(&folder, e))?;
            let file = entry.file_name().to_string_lossy().into_owned();
            // What a half-written replacement leaves beside a record is not a record.
            let Some(name) = file.strip_suffix(".json") else {
                continue;
            };
            let read = fs::read(entry.path())
                .map_err(|e| e.to_string())
                .and_then(|bytes| {
                    serde_json::from_slice::<Adapter>(&bytes).map_err(|e| e.to_string())
                });
            match read {
                Ok(adapter) if adapter.name == name => listing.adapters.push(AdapterStatus {
                    live: now < adapter.seen_epoch + ADAPTER_LIVE_SECONDS,
                    adapter,
                }),
                Ok(adapter) => listing.unreadable.push(Unreadable {
                    id: name.to_string(),
                    reason: format!("the record says it is the adapter `{}`", adapter.name),
                }),
                Err(reason) => listing.unreadable.push(Unreadable {
                    id: name.to_string(),
                    reason,
                }),
            }
        }
        listing
            .adapters
            .sort_by(|a, b| a.adapter.name.cmp(&b.adapter.name));
        listing.unreadable.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(listing)
    }
}
