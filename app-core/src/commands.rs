// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The commands, as JSON in and JSON out.
//!
//! The desktop shell, the browser shell and the `sce-work` command all reach the
//! store through [`call`]. That is the point of having it: three entrances that
//! each decode their own arguments and encode their own errors would be three
//! chances to disagree about what a save is, and the screen is the one that
//! cannot be reasoned about from the command line.
//!
//! A command takes one JSON object and answers one JSON object. Unknown fields
//! are refused, not ignored: an argument that nothing reads is a caller that
//! thinks it said something.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::clock::Clock;
use crate::error::StoreError;
use crate::revision::Revision;
use crate::store::{WorkId, WorkStore};

/// Every command, in the order a person would meet them.
pub const COMMANDS: &[&str] = &[
    "describe",
    "list_works",
    "create_work",
    "read_work",
    "read_source",
    "save_source",
    "history",
];

/// The version of this command set. It moves when a command's arguments or
/// answer change in a way a caller written against the last one would misread.
pub const COMMAND_SET_VERSION: u32 = 1;

/// A command that did not do what was asked, in a shape every shell can pass on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CommandError {
    /// What a program branches on (see [`StoreError::kind`]); `bad-request` and
    /// `unknown-command` are this layer's own.
    pub kind: String,
    /// For a person.
    pub message: String,
    /// Facts a caller acts on, when there are any. For `conflict`, the base the
    /// caller wrote from and the revision that is current now.
    #[serde(skip_serializing_if = "Value::is_null")]
    pub detail: Value,
}

impl From<StoreError> for CommandError {
    fn from(error: StoreError) -> Self {
        let detail = match &error {
            StoreError::Conflict { base, current } => json!({ "base": base, "current": current }),
            _ => Value::Null,
        };
        CommandError {
            kind: error.kind().to_string(),
            message: error.to_string(),
            detail,
        }
    }
}

impl CommandError {
    fn bad_request(message: impl Into<String>) -> Self {
        CommandError {
            kind: "bad-request".to_string(),
            message: message.into(),
            detail: Value::Null,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Empty {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateWork {
    title: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OneWork {
    id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadSource {
    id: String,
    #[serde(default)]
    revision: Option<Revision>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SaveSource {
    id: String,
    text: String,
    /// Absent or `null` means "this is the work's first text".
    #[serde(default)]
    base: Option<Revision>,
}

fn arguments<T: for<'de> Deserialize<'de>>(args: Value) -> Result<T, CommandError> {
    // A caller that sends nothing means an empty object.
    let args = if args.is_null() { json!({}) } else { args };
    serde_json::from_value(args).map_err(|e| CommandError::bad_request(e.to_string()))
}

fn answer<T: Serialize>(value: &T) -> Result<Value, CommandError> {
    serde_json::to_value(value).map_err(|e| CommandError::bad_request(e.to_string()))
}

fn work_id(text: &str) -> Result<WorkId, CommandError> {
    WorkId::parse(text).map_err(CommandError::from)
}

/// Run the command `name` with `args` against `store`.
pub fn call<C: Clock>(
    store: &WorkStore<C>,
    name: &str,
    args: Value,
) -> Result<Value, CommandError> {
    match name {
        "describe" => {
            arguments::<Empty>(args)?;
            Ok(json!({
                "command_set_version": COMMAND_SET_VERSION,
                "commands": COMMANDS,
                "root": store.root().display().to_string(),
            }))
        }
        "list_works" => {
            arguments::<Empty>(args)?;
            answer(&store.list_works()?)
        }
        "create_work" => {
            let CreateWork { title } = arguments(args)?;
            answer(&store.create_work(&title)?)
        }
        "read_work" => {
            let OneWork { id } = arguments(args)?;
            let id = work_id(&id)?;
            let work = store.read_work(&id)?;
            let head = store.head(&id)?;
            Ok(json!({ "work": work, "head": head }))
        }
        "read_source" => {
            let ReadSource { id, revision } = arguments(args)?;
            let source = store.read_source(&work_id(&id)?, revision.as_ref())?;
            Ok(json!({ "source": source }))
        }
        "save_source" => {
            let SaveSource { id, text, base } = arguments(args)?;
            answer(&store.save_source(&work_id(&id)?, &text, base.as_ref())?)
        }
        "history" => {
            let OneWork { id } = arguments(args)?;
            Ok(json!({ "entries": store.history(&work_id(&id)?)? }))
        }
        other => Err(CommandError {
            kind: "unknown-command".to_string(),
            message: format!(
                "`{other}` is not a command; the commands are {}",
                COMMANDS.join(", ")
            ),
            detail: Value::Null,
        }),
    }
}
