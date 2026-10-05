// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Whether a shell hosts an executor, and if not, why: a record each, beside the works.
//!
//! ```text
//! <root>/.sce-hosts/<name>.json   what the shell said of its executor, and when
//! ```
//!
//! A shell (the desktop application, the browser shell) hosts an executor when it can find what
//! the executor needs, and says so, or why it cannot, the way an adapter says it is there: by
//! reporting, and it counts for as long as its last report is recent ([`ADAPTER_LIVE_SECONDS`]).
//! The screen reads it to tell the owner what to install or set where they look, which a message
//! on the standard error of a program that was started from a menu does not. Like an adapter's
//! it is soft state and not a fact about any work (see `soft_state`).

use serde::{Deserialize, Serialize};
use serde_json::json;

use super::adapter_store::ADAPTER_LIVE_SECONDS;
use super::request_store::valid_name;
use super::soft_state::{read_records, write_record, Named};
use super::{Unreadable, WorkStore};
use crate::clock::{utc_timestamp, Clock};
use crate::error::StoreError;

const HOSTS_DIR: &str = ".sce-hosts";
/// The longest reason a shell gives for hosting nothing: a sentence or two of what to do.
const REASON_MAX: usize = 2_000;
const VERSION_MAX: usize = 64;

/// What a shell says of its executor.
#[derive(Debug, Clone)]
pub struct HostReport<'a> {
    /// Which shell it is (`desktop`, `web-shell`): the name its executor's requests are taken under.
    pub name: &'a str,
    /// Whether an executor is running in it.
    pub hosting: bool,
    /// Why not, in words the owner can act on; `None` when it is hosting.
    pub reason: Option<&'a str>,
    /// The version the client says it is, when one is running.
    pub client_version: Option<&'a str>,
}

/// A shell's word as it is kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Host {
    pub name: String,
    pub hosting: bool,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub client_version: Option<String>,
    pub seen_at: String,
    /// The same moment, as seconds since the Unix epoch.
    pub seen_epoch: u64,
}

impl Named for Host {
    fn name(&self) -> &str {
        &self.name
    }
}

/// A shell's word, and whether the shell is still saying it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostStatus {
    pub host: Host,
    pub live: bool,
}

/// Every shell that ever reported, by name, and every record that could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostListing {
    pub hosts: Vec<HostStatus>,
    pub unreadable: Vec<Unreadable>,
}

fn bad(what: &str) -> StoreError {
    StoreError::refused("bad-host", what.to_string(), json!(null))
}

impl<C: Clock> WorkStore<C> {
    /// A shell says whether it hosts an executor, and why not. Replaces what it said before.
    pub fn report_host(&self, report: HostReport<'_>) -> Result<HostStatus, StoreError> {
        if !valid_name(report.name) {
            return Err(bad(
                "a shell is named by 1 to 64 letters, digits and `._:-`, starting with a letter or a digit",
            ));
        }
        if let Some(reason) = report.reason {
            if reason.trim().is_empty() || reason.chars().count() > REASON_MAX {
                return Err(bad(
                    "a reason is a sentence or two of what to do, up to 2000 characters",
                ));
            }
        }
        if report
            .client_version
            .is_some_and(|v| v.is_empty() || v.len() > VERSION_MAX)
        {
            return Err(bad("a client's version is 1 to 64 characters"));
        }
        let epoch = self.clock.epoch();
        let host = Host {
            name: report.name.to_string(),
            hosting: report.hosting,
            reason: report.reason.map(str::to_string),
            client_version: report.client_version.map(str::to_string),
            seen_at: utc_timestamp(epoch),
            seen_epoch: epoch,
        };
        write_record(&self.root.join(HOSTS_DIR), &host)?;
        Ok(HostStatus { host, live: true })
    }

    /// Every shell that reported, by name, each with whether it is still saying it.
    pub fn host_status(&self) -> Result<HostListing, StoreError> {
        let (hosts, unreadable) = read_records::<Host>(&self.root.join(HOSTS_DIR), "shell")?;
        let now = self.clock.epoch();
        Ok(HostListing {
            hosts: hosts
                .into_iter()
                .map(|host| HostStatus {
                    live: now < host.seen_epoch + ADAPTER_LIVE_SECONDS,
                    host,
                })
                .collect(),
            unreadable,
        })
    }
}
