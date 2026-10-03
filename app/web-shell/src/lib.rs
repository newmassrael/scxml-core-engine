// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The workbench's commands over HTTP.
//!
//! The desktop application needs a display, and the machine the screen is
//! developed on has none: it is reached over SSH, from a terminal or a phone. This
//! program offers the same single entrance the desktop shell offers, `call`, as
//! `POST /api/call`, and serves the built screen beside it, so the screen is the
//! same code in a browser as in the window.
//!
//! It is a development and viewing tool, not a second product: it is a separate
//! binary, so a release build of the application cannot contain it, and it listens
//! only where [`address::check_bind`] allows. [`server`] holds the sockets, with a
//! bound on every wait on a client; [`Shell`] is the handler and has no sockets,
//! so it is tested without any.
//!
//! The wire, one request and one answer:
//!
//! ```text
//! POST /api/call   Authorization: Bearer <token>
//!   {"name": "<command>", "args": {...}}
//! 200  the command's answer, as `sce_app_core::call` returns it
//! 4xx  a `CommandError` (`kind`, `message`, `detail`); 409 is `conflict`
//! ```

pub mod address;
pub mod assets;
pub mod server;
pub mod token;

use sce_app_core::{call, CommandError, Product, WorkStore, MAX_SOURCE_BYTES};
use serde::Deserialize;
use serde_json::Value;

use assets::Assets;

/// The largest request body. A source of `MAX_SOURCE_BYTES` can grow sixfold when
/// JSON escapes every byte of it, and the envelope is small.
pub const MAX_BODY_BYTES: usize = MAX_SOURCE_BYTES * 6 + 4096;

const JSON: &str = "application/json";

/// One answer, before it is written to a socket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    pub status: u16,
    pub content_type: &'static str,
    pub body: Vec<u8>,
}

impl Reply {
    pub(crate) fn error(status: u16, kind: &str, message: impl Into<String>) -> Reply {
        let error = CommandError {
            kind: kind.to_string(),
            message: message.into(),
            detail: Value::Null,
        };
        Reply::command_error(status, &error)
    }

    fn command_error(status: u16, error: &CommandError) -> Reply {
        Reply {
            status,
            content_type: JSON,
            body: serde_json::to_vec(error).expect("a CommandError is always serialisable"),
        }
    }
}

/// The HTTP status a command's refusal travels under.
fn status_of(kind: &str) -> u16 {
    match kind {
        "not-found" | "unknown-command" => 404,
        "invalid-id" | "invalid-title" | "bad-request" => 400,
        "conflict" => 409,
        "too-large" => 413,
        "busy" | "sce-unavailable" => 503,
        // SCE answered and said no to this model: the request was understood.
        "sce-refused" => 422,
        "sce-timeout" => 504,
        "sce-failed" => 502,
        _ => 500,
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    name: String,
    #[serde(default)]
    args: Value,
}

/// The handler: the works folder it serves, the product that draws a model and
/// reads one for it, the token it requires, the screen it shows.
pub struct Shell {
    store: WorkStore,
    figures: Box<dyn Product>,
    token: String,
    assets: Option<Assets>,
}

impl Shell {
    pub fn new(
        store: WorkStore,
        figures: Box<dyn Product>,
        token: String,
        assets: Option<Assets>,
    ) -> Self {
        Shell {
            store,
            figures,
            token,
            assets,
        }
    }

    /// Whether this request is one [`Shell::handle`] will refuse for its headers
    /// alone: a call without the token. The server asks before it reads the body,
    /// so a caller that has not proved itself cannot make it wait for one.
    pub fn refuses_before_reading(
        &self,
        method: &str,
        url: &str,
        authorization: Option<&str>,
    ) -> bool {
        let path = url.split(['?', '#']).next().unwrap_or("");
        method == "POST" && path == "/api/call" && !self.authorized(authorization)
    }

    fn authorized(&self, authorization: Option<&str>) -> bool {
        authorization
            .and_then(token::bearer)
            .is_some_and(|t| token::matches(&self.token, t))
    }

    /// Answer one request. `authorization` is the `Authorization` header's value.
    pub fn handle(
        &self,
        method: &str,
        url: &str,
        authorization: Option<&str>,
        body: &[u8],
    ) -> Reply {
        let path = url.split(['?', '#']).next().unwrap_or("");
        if path == "/api/call" {
            return match method {
                "POST" => self.call(authorization, body),
                _ => Reply::error(405, "bad-request", "/api/call takes POST"),
            };
        }
        if path.starts_with("/api/") {
            return Reply::error(404, "unknown-command", format!("{path} is not an endpoint"));
        }
        match (method, &self.assets) {
            ("GET" | "HEAD", Some(assets)) => match assets.get(url) {
                Some((bytes, content_type)) => Reply {
                    status: 200,
                    content_type,
                    body: bytes,
                },
                None => Reply::error(404, "not-found", format!("{path} is not a file here")),
            },
            ("GET" | "HEAD", None) => Reply::error(
                404,
                "not-found",
                "no screen is being served; start with --ui <dir>, or use the dev server",
            ),
            _ => Reply::error(405, "bad-request", "only GET and POST are served"),
        }
    }

    fn call(&self, authorization: Option<&str>, body: &[u8]) -> Reply {
        if !self.authorized(authorization) {
            return Reply::error(
                401,
                "unauthorized",
                "the request carries no valid bearer token",
            );
        }
        let envelope: Envelope = match serde_json::from_slice(body) {
            Ok(envelope) => envelope,
            Err(e) => {
                return Reply::error(400, "bad-request", format!("the body is not a call: {e}"))
            }
        };
        match call(
            &self.store,
            self.figures.as_ref(),
            &envelope.name,
            envelope.args,
        ) {
            Ok(answer) => Reply {
                status: 200,
                content_type: JSON,
                body: serde_json::to_vec(&answer).expect("a JSON value is always serialisable"),
            },
            Err(error) => Reply::command_error(status_of(&error.kind), &error),
        }
    }
}
