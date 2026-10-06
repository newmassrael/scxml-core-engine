// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What a connection is: the settings of one way to reach a model.
//!
//! A person connects the workbench to an AI client (Claude Code, Codex) or to a server they
//! run (a local model), picks a model, and says where the client finds its credentials. That
//! is a connection. It names no secret and holds none: a credential is the client's own
//! (its login, an environment variable the person chose) or the operating system's, and a
//! connection says only which of those to use. So the one field that could carry a secret by
//! accident, a server's address, takes no user, no query and no fragment, and a stored
//! connection with a field this type does not have is not read.
//!
//! The connection is kept apart from the works (see `store::connection_store`) and a request
//! pins it by its [`ConnectionId`] and the revision it had when the request was made.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::StoreError;

const ID_MAX: usize = 40;
const NAME_MAX: usize = 80;
const MODEL_MAX: usize = 200;
const EXECUTABLE_MAX: usize = 1_024;
const SERVER_MAX: usize = 300;
/// The most turns a connection may allow one generation.
pub const TURNS_MAX: u32 = 500;
/// The longest, in seconds, a connection may allow one generation.
pub const SECONDS_MAX: u32 = 86_400;

fn bad(message: impl Into<String>) -> StoreError {
    StoreError::refused("bad-connection", message, serde_json::Value::Null)
}

/// What names a connection: 1 to 40 lowercase letters, digits and hyphens, none at either end.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ConnectionId(String);

impl ConnectionId {
    pub fn parse(text: &str) -> Result<Self, StoreError> {
        let well_formed = (1..=ID_MAX).contains(&text.len())
            && !text.starts_with('-')
            && !text.ends_with('-')
            && text
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
        if well_formed {
            Ok(ConnectionId(text.to_string()))
        } else {
            Err(bad(format!(
                "`{text}` is not a connection id: 1 to {ID_MAX} lowercase letters, digits and hyphens, \
                 with no hyphen at either end"
            )))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ConnectionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for ConnectionId {
    type Error = StoreError;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        ConnectionId::parse(&text)
    }
}

impl From<ConnectionId> for String {
    fn from(id: ConnectionId) -> String {
        id.0
    }
}

/// Which kind of client or server a connection reaches. Each is its own adapter: they take
/// different arguments, answer in different shapes and hold different credentials.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AdapterKind {
    ClaudeCode,
    Codex,
    /// A server the person runs, spoken to as an OpenAI-compatible API.
    Local,
}

impl AdapterKind {
    /// The word it is stored and sent under.
    pub fn word(self) -> &'static str {
        match self {
            AdapterKind::ClaudeCode => "claude-code",
            AdapterKind::Codex => "codex",
            AdapterKind::Local => "local",
        }
    }

    /// Where this kind of connection may take its credential from.
    pub fn credential_sources(self) -> &'static [AuthSource] {
        match self {
            // The client keeps its own login; the workbench neither reads nor replaces it.
            AdapterKind::ClaudeCode => &[AuthSource::OfficialLogin],
            AdapterKind::Codex => &[
                AuthSource::AppStore,
                AuthSource::EnvApiKey,
                AuthSource::OfficialLogin,
            ],
            AdapterKind::Local => &[AuthSource::NoAuth, AuthSource::ServerKey],
        }
    }
}

/// Where a connection's credential comes from. The credential itself is never in a connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AuthSource {
    /// The login the person made with the client's own command, in the client's own folder.
    OfficialLogin,
    /// A login kept in a folder of the workbench's own, apart from the person's.
    AppStore,
    /// An environment variable the person chose (`CODEX_API_KEY`).
    EnvApiKey,
    /// A key the operating system's credential store holds for a local server.
    ServerKey,
    /// The server asks for none.
    #[serde(rename = "none")]
    NoAuth,
}

/// What one generation may spend, when the connection says so.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turns: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seconds: Option<u32>,
}

impl Limits {
    pub(crate) fn is_empty(&self) -> bool {
        self.turns.is_none() && self.seconds.is_none()
    }

    fn validate(&self) -> Result<(), StoreError> {
        if let Some(turns) = self.turns {
            if !(1..=TURNS_MAX).contains(&turns) {
                return Err(bad(format!(
                    "a turn limit of {turns} is not between 1 and {TURNS_MAX}"
                )));
            }
        }
        if let Some(seconds) = self.seconds {
            if !(1..=SECONDS_MAX).contains(&seconds) {
                return Err(bad(format!(
                    "a time limit of {seconds} seconds is not between 1 and {SECONDS_MAX}"
                )));
            }
        }
        Ok(())
    }
}

/// The settings of one way to reach a model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Connection {
    pub id: ConnectionId,
    pub adapter: AdapterKind,
    /// What the person calls it. A local connection must have one: it is what the screen shows
    /// beside the button, so that a person can tell where a specification is sent when the
    /// address says nothing (a tunnel is `127.0.0.1`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Where the client's program is, when the person chose one. Absolute.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executable: Option<String>,
    /// The model asked for, as the client or server names it. None leaves the choice to it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub auth: AuthSource,
    /// A local connection's server: `http(s)://host[:port][/path]`, nothing more.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_url: Option<String>,
    #[serde(default, skip_serializing_if = "Limits::is_empty")]
    pub limits: Limits,
}

impl Connection {
    /// Whether this is a connection a store keeps. The same rules apply when one is read back,
    /// so a file somebody else wrote is held to them too.
    pub fn validate(&self) -> Result<(), StoreError> {
        if !self.adapter.credential_sources().contains(&self.auth) {
            return Err(bad(format!(
                "a {} connection takes its credential from {}, and `{}` is not one of them",
                self.adapter.word(),
                self.adapter
                    .credential_sources()
                    .iter()
                    .map(|s| format!("`{}`", source_word(*s)))
                    .collect::<Vec<_>>()
                    .join(", "),
                source_word(self.auth),
            )));
        }
        match (self.adapter, self.server_url.as_deref()) {
            (AdapterKind::Local, None) => {
                return Err(bad("a local connection needs a server address"));
            }
            (AdapterKind::Local, Some(address)) => check_server_address(address)?,
            (_, Some(_)) => {
                return Err(bad("only a local connection has a server address"));
            }
            (_, None) => {}
        }
        match self.display_name.as_deref() {
            None if self.adapter == AdapterKind::Local => {
                return Err(bad(
                    "a local connection needs a display name, so a person can tell where a \
                     specification is sent",
                ));
            }
            Some(name) if !one_line(name, NAME_MAX) => {
                return Err(bad(format!(
                    "a display name is 1 to {NAME_MAX} characters on one line, with nothing \
                     blank at either end"
                )));
            }
            _ => {}
        }
        if let Some(model) = self.model.as_deref() {
            if !one_line(model, MODEL_MAX) {
                return Err(bad(format!(
                    "a model is 1 to {MODEL_MAX} characters on one line, with nothing blank at \
                     either end"
                )));
            }
        }
        if let Some(executable) = self.executable.as_deref() {
            if !valid_executable(executable) {
                return Err(bad(format!(
                    "an executable is an absolute path of 1 to {EXECUTABLE_MAX} characters on \
                     one line"
                )));
            }
        }
        self.limits.validate()
    }

    /// The bytes a store keeps, which are what its revision is the digest of.
    pub(crate) fn stored_bytes(&self) -> Vec<u8> {
        let mut bytes = serde_json::to_vec_pretty(self).expect("a connection is plain data");
        bytes.push(b'\n');
        bytes
    }
}

fn source_word(source: AuthSource) -> &'static str {
    match source {
        AuthSource::OfficialLogin => "official-login",
        AuthSource::AppStore => "app-store",
        AuthSource::EnvApiKey => "env-api-key",
        AuthSource::ServerKey => "server-key",
        AuthSource::NoAuth => "none",
    }
}

/// One line of 1 to `max` characters, with nothing blank at either end and no control character.
fn one_line(text: &str, max: usize) -> bool {
    let length = text.chars().count();
    (1..=max).contains(&length) && text == text.trim() && !text.chars().any(char::is_control)
}

/// Whether `address` is one a connection may keep for a server, and what is wrong with it when it
/// is not. The same rule holds an address that is only asked about (`server_status`), so that a
/// screen is not told that a server is there at an address it could not then save.
pub fn check_server_address(address: &str) -> Result<(), StoreError> {
    if valid_server_address(address) {
        Ok(())
    } else {
        Err(bad(format!(
            "`{}` is not a server address: it is http:// or https://, a host and an optional \
             port and path, with no user, query or fragment",
            address.escape_debug()
        )))
    }
}

fn valid_server_address(address: &str) -> bool {
    if address.len() > SERVER_MAX {
        return false;
    }
    let Some(rest) = address
        .strip_prefix("http://")
        .or_else(|| address.strip_prefix("https://"))
    else {
        return false;
    };
    // `@` is a user, `?` a query and `#` a fragment: each is where a token ends up by habit.
    let clean = !rest
        .chars()
        .any(|c| c.is_control() || c.is_whitespace() || matches!(c, '@' | '?' | '#' | '\\'));
    clean && !rest.split('/').next().unwrap_or("").is_empty()
}

/// An absolute path on any platform the person may be using, judged by its spelling: whether it
/// exists is the shell's business, and it is checked there against the program it names.
fn valid_executable(path: &str) -> bool {
    if path.is_empty() || path.len() > EXECUTABLE_MAX || path.chars().any(char::is_control) {
        return false;
    }
    let bytes = path.as_bytes();
    path.starts_with('/')
        || path.starts_with("\\\\")
        || (bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && matches!(bytes[2], b'\\' | b'/'))
}
