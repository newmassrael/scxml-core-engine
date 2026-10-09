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
//! 200  the command's answer, as `sce_app_core::call_in` returns it (as the browser entrance)
//! 4xx  a `CommandError` (`kind`, `message`, `detail`); 409 is `conflict`
//! ```

pub mod address;
pub mod assets;
pub mod server;
pub mod token;

use sce_app_core::{
    call_in, CommandError, ConnectionStore, Context, Entrance, Policy, Product, WorkStore,
    MAX_SOURCE_BYTES,
};
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
///
/// A refusal is the caller's situation and not a fault of the server: only a store that is
/// damaged or cannot be written (`corrupt`, `io`) and a kind nobody mapped are a 500, which is
/// what a monitor counts. The classes, by what the caller can do about it:
///
/// * 400: what was sent is not well formed (an id, a title, an address, a field).
/// * 403: this entrance is not where that is done; the desktop application is.
/// * 404: what is asked about does not exist.
/// * 409: the work, the request or the connection is not in the state the call assumes: the
///   caller read, or wrote from, something that has moved. Reading again and deciding is the
///   way out.
/// * 413, 422, 502, 503, 504: too large; understood and refused on its content; the product
///   failed, was busy or unavailable, or did not answer in time.
///
/// `tests::every_kind_the_core_gives_travels_under_a_chosen_status` holds every kind of the
/// contract file to a status written down on purpose, so a new kind is a choice and not a 500.
fn status_of(kind: &str) -> u16 {
    match kind {
        "not-found" | "unknown-command" | "no-settings" | "removed-work" => 404,
        "invalid-id" | "invalid-title" | "bad-request" | "bad-connection" | "bad-adapter"
        | "bad-lease" | "bad-server-address" | "bad-host" | "bad-instructions" => 400,
        // This shell is not where that is done: the desktop application is.
        "not-allowed-here"
        | "not-allowed-to-find-clients"
        | "not-allowed-to-read-server-status"
        | "not-allowed-to-start-a-program" => 403,
        // Something moved since the caller read it, or the call assumes a state the work, the
        // request or the connection is not in: a text saved from another revision, a model or a
        // list written for an earlier text (so nothing is compared or accepted until it is
        // written again), a request that is held, ended or pinned elsewhere, a lineage that does
        // not continue the one the work keeps.
        "conflict"
        | "model-conflict"
        | "connection-conflict"
        | "connection-moved"
        | "request-connection-moved"
        | "moved"
        | "candidate-moved"
        | "not-current"
        | "revision-not-current"
        | "active-request"
        | "key-reused"
        | "request-held"
        | "not-holder"
        | "request-ended"
        | "not-resuming"
        | "wrong-connection"
        | "no-candidate"
        | "no-core-check"
        | "bundled-work"
        | "lineage-dropped"
        | "lineage-not-continued" => 409,
        "too-large" => 413,
        // The request was understood and what it carries is refused on its content: a model, a
        // list, answers or a candidate that are not one, a reference to a text there is not, a
        // lineage that is not the list's, a list that says nothing of its words, a model SCE
        // refused.
        "invalid-answers"
        | "invalid-model"
        | "invalid-requirements"
        | "bad-candidate"
        | "check-refused"
        | "model-for-an-unknown-text"
        | "lineage-unusable"
        | "lineage-of-another-list"
        | "revision-not-judged"
        | "sce-refused" => 422,
        "busy" | "sce-unavailable" => 503,
        "sce-timeout" => 504,
        "sce-failed" => 502,
        // `corrupt` and `io` are the server's own; a kind nobody mapped is one too.
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
///
/// It may be given the person's settings folder to read ([`Shell::with_settings`]) and never to
/// change: what a token can reach over a network is not the person at the keyboard.
pub struct Shell {
    store: WorkStore,
    figures: Box<dyn Product>,
    token: String,
    assets: Option<Assets>,
    connections: Option<ConnectionStore>,
    policy: Policy,
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
            connections: None,
            policy: Policy::shipped(),
        }
    }

    /// The same, with the person's settings folder to read. Changing it stays the desktop
    /// application's: the commands that do are refused here (`not-allowed-here`).
    pub fn with_settings(mut self, connections: ConnectionStore) -> Self {
        self.connections = Some(connections);
        self
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
        let context = Context::new(
            &self.store,
            self.figures.as_ref(),
            &self.policy,
            Entrance::Browser,
        )
        .with_connections(self.connections.as_ref());
        match call_in(&context, &envelope.name, envelope.args) {
            Ok(answer) => Reply {
                status: 200,
                content_type: JSON,
                body: serde_json::to_vec(&answer).expect("a JSON value is always serialisable"),
            },
            Err(error) => Reply::command_error(status_of(&error.kind), &error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::status_of;

    /// Every kind the core gives, with the status chosen for it on purpose: the kinds of the
    /// contract file (`app-core/contract/replies.json`, written by running the commands) and the
    /// few the commands give that no reply of it shows.
    const CHOSEN: &[(&str, u16)] = &[
        ("active-request", 409),
        ("bad-adapter", 400),
        ("bad-candidate", 422),
        ("bad-connection", 400),
        ("bad-host", 400),
        ("bad-instructions", 400),
        ("bad-lease", 400),
        ("bad-request", 400),
        ("bad-server-address", 400),
        ("busy", 503),
        ("bundled-work", 409),
        ("candidate-moved", 409),
        ("check-refused", 422),
        ("conflict", 409),
        ("connection-conflict", 409),
        ("connection-moved", 409),
        // The server's own faults: a store that is damaged or cannot be written.
        ("corrupt", 500),
        ("invalid-answers", 422),
        ("invalid-id", 400),
        ("invalid-model", 422),
        ("invalid-requirements", 422),
        ("invalid-title", 400),
        ("io", 500),
        ("key-reused", 409),
        ("lineage-dropped", 409),
        ("lineage-not-continued", 409),
        ("lineage-of-another-list", 422),
        ("lineage-unusable", 422),
        ("model-conflict", 409),
        ("model-for-an-unknown-text", 422),
        ("moved", 409),
        ("no-candidate", 409),
        ("no-core-check", 409),
        ("no-settings", 404),
        ("not-allowed-here", 403),
        ("not-allowed-to-find-clients", 403),
        ("not-allowed-to-read-server-status", 403),
        ("not-allowed-to-start-a-program", 403),
        ("not-current", 409),
        ("not-found", 404),
        ("not-holder", 409),
        ("not-resuming", 409),
        ("removed-work", 404),
        ("request-connection-moved", 409),
        ("request-ended", 409),
        ("request-held", 409),
        ("revision-not-current", 409),
        ("revision-not-judged", 422),
        ("sce-failed", 502),
        ("sce-refused", 422),
        ("sce-timeout", 504),
        ("sce-unavailable", 503),
        ("too-large", 413),
        ("unknown-command", 404),
        ("wrong-connection", 409),
    ];

    /// A refusal is the caller's situation and not a fault of the server, so it does not travel as
    /// the 500 a monitor counts: a work whose model or list was not written again for its text,
    /// a request that is held, a lineage that does not continue, a model that is not one.
    #[test]
    fn each_kind_travels_under_the_status_chosen_for_it() {
        for (kind, status) in CHOSEN {
            assert_eq!(status_of(kind), *status, "{kind}");
        }
        // Only the server's own faults are 500, and a kind nobody mapped is still one.
        let faults: Vec<&str> = CHOSEN
            .iter()
            .filter(|(_, s)| *s == 500)
            .map(|(k, _)| *k)
            .collect();
        assert_eq!(faults, ["corrupt", "io"]);
        assert_eq!(status_of("a-kind-nobody-mapped"), 500);
    }

    /// A new kind in the contract file is a choice, not a 500: the contract file is written by
    /// running every command, so a refusal that is added there and has no status chosen above
    /// fails here, and the person who added it says which.
    #[test]
    fn every_kind_the_core_gives_travels_under_a_chosen_status() {
        let contract: serde_json::Value =
            serde_json::from_str(include_str!("../../../app-core/contract/replies.json"))
                .expect("the contract file is JSON");
        let refusals = contract["refusals"]
            .as_object()
            .expect("the contract has refusals");
        assert!(!refusals.is_empty());
        for kind in refusals.keys() {
            assert!(
                CHOSEN.iter().any(|(chosen, _)| chosen == kind),
                "`{kind}` is a kind the core gives and no status was chosen for it: add it to \
                 `CHOSEN` and to `status_of`"
            );
        }
    }
}
