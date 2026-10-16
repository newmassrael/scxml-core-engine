// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The reference host of the standard host features (docs/SCE_HOST_FEATURES.md).
//!
//! A [`ReferenceHost`] is the whole of the four features' semantics as one
//! object: it is told the start of an operation (its `type` and the request's
//! text parameters) and answers with an [`Outcome`] — a completion, a failure,
//! a refusal, or nothing yet. It knows no engine, so the same object serves a
//! generated machine through [`serve`], a test, or another embedding.
//!
//! Time comes from a [`ClockSource`] and the network from a [`Transport`], both
//! the embedding's to supply: the host's semantics are fixed and what it reads
//! is not, which is what lets a scenario judge it.

mod clock;
mod http;
mod kv;
mod notify;
mod serve;

use std::collections::BTreeMap;

pub use clock::{ClockSource, Reading, ScriptedClock, SystemClock, Zone};
pub use http::{HttpRequest, HttpResponse, Transport, TransportFailure};
pub use serve::{serve, Shared};

/// The request of an operation as the engine hands it to a host: each
/// `<param>` by name, as text. A 64-bit integer is its decimal digits.
pub type Params = BTreeMap<String, String>;

/// Every operation type of the standard host features, in the order the
/// specification lists them.
pub const TYPES: [&str; 11] = [
    "sce-host-clock-now",
    "sce-host-clock-zone",
    "sce-host-kv-get",
    "sce-host-kv-put",
    "sce-host-kv-delete",
    "sce-host-kv-next-key",
    "sce-host-notify-permission",
    "sce-host-notify-request-permission",
    "sce-host-notify-post",
    "sce-host-notify-cancel",
    "sce-host-http-fetch",
];

/// What a host answers a start with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The operation ended: the result record, as the JSON a completion
    /// carries. A failure is such a record whose `error` is a code of the
    /// operation's closed set ([`failure`]); the machine reads it as a field.
    Done(String),
    /// The host would not start it; the reason, as the JSON `error.execution`
    /// carries.
    Refuse(&'static str),
    /// The host has started it and answers later, or never.
    Hold,
}

/// The refusal the specification gives an argument the operation does not
/// accept.
const INVALID_ARGUMENT: &str = r#"{"reason":"invalid_argument"}"#;

/// The standard host features' semantics, over a clock and a transport.
pub struct ReferenceHost<T: Transport> {
    pub(crate) clock: Box<dyn ClockSource + Send>,
    pub(crate) transport: T,
    pub(crate) store: kv::Store,
    pub(crate) notifications: notify::Notifications,
    /// Failures to inject: the operation type to the code it fails with.
    pub(crate) faults: BTreeMap<String, String>,
    /// The invocations held and not yet answered, by id.
    pub(crate) held: Vec<String>,
    /// The invocations the host was told to stop, in the order told.
    pub(crate) cancelled: Vec<String>,
    /// The highest monotonic reading answered, which no answer goes below.
    pub(crate) monotonic_floor: i64,
    /// Every start the host was given, refused ones included, until taken.
    pub(crate) starts: Vec<Start>,
}

/// A start the host was given: the operation, the invocation's id and the
/// request's parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Start {
    pub operation: String,
    pub id: String,
    pub params: Params,
}

impl<T: Transport> ReferenceHost<T> {
    /// A host reading `clock` and carrying HTTP over `transport`, with an empty
    /// store, nothing posted, and the notification permission undetermined.
    pub fn new(clock: Box<dyn ClockSource + Send>, transport: T) -> Self {
        Self {
            clock,
            transport,
            store: kv::Store::default(),
            notifications: notify::Notifications::default(),
            faults: BTreeMap::new(),
            held: Vec::new(),
            cancelled: Vec::new(),
            monotonic_floor: i64::MIN,
            starts: Vec::new(),
        }
    }

    /// The starts the host was given since this was last called, oldest first.
    pub fn take_starts(&mut self) -> Vec<Start> {
        std::mem::take(&mut self.starts)
    }

    /// The transport, for an embedding that scripts or reads it.
    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }

    /// Inject failures: each operation type named fails `unavailable` or with
    /// the code it maps to, once the request has been accepted. An empty map
    /// ends them.
    pub fn set_faults(&mut self, faults: BTreeMap<String, String>) {
        self.faults = faults;
    }

    /// Bound the store to `keys` keys; a write of a new key past it fails
    /// `quota_exceeded`.
    pub fn set_quota(&mut self, keys: usize) {
        self.store.quota = Some(keys);
    }

    /// The notification permission, as the platform says it:
    /// `granted`, `denied` or `undetermined`.
    pub fn set_permission(&mut self, state: &str) {
        self.notifications.permission = state.to_string();
    }

    /// What the user chooses when asked: `granted`, `denied`, or `none` for a
    /// host with no user to ask.
    pub fn set_user_decision(&mut self, decision: &str) {
        self.notifications.user_decision = decision.to_string();
    }

    /// The store's keys and values, in key order.
    pub fn store_entries(&self) -> Vec<(String, String)> {
        self.store.entries()
    }

    /// The ids of the waiting notifications, in the order posted.
    pub fn waiting(&self) -> Vec<String> {
        self.notifications
            .waiting
            .iter()
            .map(|n| n.id.clone())
            .collect()
    }

    /// The ids of the shown notifications, in the order posted.
    pub fn shown(&self) -> Vec<String> {
        self.notifications
            .shown
            .iter()
            .map(|n| n.id.clone())
            .collect()
    }

    /// The invocations the host was told to stop, in the order told.
    pub fn cancelled(&self) -> &[String] {
        &self.cancelled
    }

    /// Start the operation `operation` of invocation `id` with `params`.
    ///
    /// `None` for a type that is not one of the standard host features: the host
    /// serves none of it. Otherwise the arguments are judged first (a refusal
    /// starts nothing), then a failure the embedding injected, then the
    /// operation itself.
    pub fn start(&mut self, operation: &str, id: &str, params: &Params) -> Option<Outcome> {
        if !TYPES.contains(&operation) {
            return None;
        }
        self.starts.push(Start {
            operation: operation.to_string(),
            id: id.to_string(),
            params: params.clone(),
        });
        let argument = |name: &str| params.get(name).map(String::as_str).unwrap_or("");
        let refused = match operation {
            "sce-host-kv-get" | "sce-host-kv-delete" => !kv::valid_key(argument("key")),
            "sce-host-kv-put" => !kv::valid_key(argument("key")),
            "sce-host-notify-post" | "sce-host-notify-cancel" => argument("id").is_empty(),
            "sce-host-http-fetch" => {
                !http::acceptable(argument("method"), argument("url"), argument("headers"))
            }
            _ => false,
        };
        if refused {
            return Some(Outcome::Refuse(INVALID_ARGUMENT));
        }
        if let Some(code) = self.faults.get(operation) {
            return Some(failure(operation, code));
        }
        Some(match operation {
            "sce-host-clock-now" => clock::now(self),
            "sce-host-clock-zone" => clock::zone(self),
            "sce-host-kv-get" => kv::get(self, argument("key")),
            "sce-host-kv-put" => kv::put(self, argument("key"), argument("value")),
            "sce-host-kv-delete" => kv::delete(self, argument("key")),
            "sce-host-kv-next-key" => kv::next_key(self, argument("after"), argument("prefix")),
            "sce-host-notify-permission" => notify::permission(self),
            "sce-host-notify-request-permission" => notify::request_permission(self),
            "sce-host-notify-post" => notify::post(
                self,
                argument("id"),
                argument("title"),
                argument("body"),
                argument("at_ms"),
            ),
            "sce-host-notify-cancel" => notify::cancel(self, argument("id")),
            "sce-host-http-fetch" => http::fetch(
                self,
                id,
                argument("method"),
                argument("url"),
                argument("headers"),
                argument("body"),
            ),
            _ => unreachable!("the type was checked against TYPES"),
        })
    }

    /// The host is told to stop invocation `id`, because its state exited or its
    /// deadline passed. What it held for it is dropped.
    pub fn cancel(&mut self, id: &str) {
        self.held.retain(|held| held != id);
        self.cancelled.push(id.to_string());
    }
}

/// A result record as the JSON a completion carries, its `error` empty. A
/// 64-bit integer is written as the text of its digits, as a payload's reader
/// takes it.
pub(crate) fn record(fields: serde_json::Value) -> Outcome {
    let mut object = serde_json::Map::new();
    object.insert(
        "error".to_string(),
        serde_json::Value::String(String::new()),
    );
    if let serde_json::Value::Object(fields) = fields {
        object.extend(fields);
    }
    Outcome::Done(serde_json::Value::Object(object).to_string())
}

/// The failure `code` of `operation`: its result record with the fields at their
/// zero and `error` the code, so a machine reads the failure as a typed field.
pub(crate) fn failure(operation: &str, code: &str) -> Outcome {
    let zero = match operation {
        "sce-host-clock-now" => serde_json::json!({ "utc_ms": "0", "monotonic_ms": "0" }),
        "sce-host-clock-zone" => serde_json::json!({ "zone_id": "", "utc_offset_s": 0 }),
        "sce-host-kv-get" => serde_json::json!({ "found": false, "value": "" }),
        "sce-host-kv-put" => serde_json::json!({ "replaced": false }),
        "sce-host-kv-delete" | "sce-host-notify-cancel" => serde_json::json!({ "existed": false }),
        "sce-host-kv-next-key" => serde_json::json!({ "found": false, "key": "" }),
        "sce-host-notify-permission" | "sce-host-notify-request-permission" => {
            serde_json::json!({ "state": "" })
        }
        "sce-host-notify-post" => serde_json::json!({ "status": "" }),
        "sce-host-http-fetch" => serde_json::json!({ "status": 0, "headers": "", "body": "" }),
        other => unreachable!("`{other}` is not a standard host operation"),
    };
    let Outcome::Done(text) = record(zero) else {
        unreachable!("a record is a completion")
    };
    let mut object: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&text).expect("a record is JSON");
    object.insert(
        "error".to_string(),
        serde_json::Value::String(code.to_string()),
    );
    Outcome::Done(serde_json::Value::Object(object).to_string())
}
