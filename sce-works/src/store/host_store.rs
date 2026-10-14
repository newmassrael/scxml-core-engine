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
/// How many requests a shell says it could not run. More than this is a shell with something else
/// the matter, and a record of them would be a log; the oldest are the ones said.
pub const WAITING_MAX: usize = 20;
/// The longest sentence of why one request waits.
const WAITING_REASON_MAX: usize = 500;

/// A request a shell's executor left queued because it could not run it, and why.
///
/// This is in the works folder, which is shared and moved, so what it holds is what a person
/// is told and nothing of the computer it was said on: an id of the work, of the request and of
/// the connection, and a sentence. No path, no address and nothing a client printed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostWaiting {
    pub work: String,
    pub request: String,
    pub connection: String,
    pub reason: String,
}

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
    /// The requests it left queued because it could not run them, and why; at most [`WAITING_MAX`].
    pub waiting: &'a [HostWaiting],
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
    /// What it left queued, and why. A record written before a shell said this has none.
    #[serde(default)]
    pub waiting: Vec<HostWaiting>,
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
        if report.waiting.len() > WAITING_MAX
            || report.waiting.iter().any(|w| {
                !valid_name(&w.work)
                    || !valid_name(&w.request)
                    || !valid_name(&w.connection)
                    || w.reason.trim().is_empty()
                    || w.reason.chars().count() > WAITING_REASON_MAX
            })
        {
            return Err(bad(
                "what a shell waits for is at most 20 requests, each named by its work, request and \
                 connection and told in a sentence of up to 500 characters",
            ));
        }
        let epoch = self.clock.epoch();
        let host = Host {
            name: report.name.to_string(),
            hosting: report.hosting,
            reason: report.reason.map(str::to_string),
            client_version: report.client_version.map(str::to_string),
            waiting: report.waiting.to_vec(),
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
