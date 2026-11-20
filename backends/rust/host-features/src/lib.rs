// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The reference host of the standard host features (docs/SCE_HOST_FEATURES.md).
//!
//! A [`ReferenceHost`] is the whole of the features' semantics as one
//! object: it is told the start of an operation (its `type`, the identity of
//! the invocation and the request's text parameters) and answers with an
//! [`Outcome`] — a completion, a failure, a refusal, or nothing yet. It knows
//! no engine, so the same object serves a generated machine through [`serve`],
//! a test, or another embedding.
//!
//! Time comes from a [`ClockSource`], the network from a [`Transport`] and
//! random bytes from a [`RandomSource`], all the embedding's to supply; the
//! keys of signing and sealing it configures as [`CoseKeys`], the keys the
//! host generates and derives are kept by a [`Keyring`] over a key store it
//! may supply, the bearer tokens it issues are kept, as their digests, in a
//! [`CredentialStore`] it may supply, and the values of the key-value feature
//! are kept in the [`KvStore`] it was built with
//! ([`ReferenceHost::with_kv_store`]): the host's semantics are fixed and what
//! it reads is not, which is what lets a scenario judge it.
//!
//! A transport that reaches the network for a server checks where each request
//! goes against a [`destination::DestinationPolicy`] (docs/adr/0109, D4): the
//! network transport of the `sce-rust-host-http` crate is built with one.

pub mod audit;
mod clock;
pub mod cose;
pub mod credentials;
pub mod destination;
mod http;
pub mod keys;
mod kv;
mod model;
mod notify;
mod serve;

use std::collections::BTreeMap;
use std::sync::Arc;

pub use audit::{AuditStreams, Personal, StreamPolicy};
pub use clock::{ClockSource, Reading, ScriptedClock, SystemClock, Zone};
pub use cose::{CoseKeys, NonceSource, ReservedKeyId, SystemNonces, TrustedKey};
pub use http::{HttpRequest, HttpResponse, Transport, TransportFailure, HTTP_BODY_MAX_BYTES};
pub use model::{
    ExhaustiveProvider, GenerateRequest, Generation, ModelProvider, OutcomeClass, ScriptedAnswer,
    ScriptedProvider, ANSWER_MAX_BYTES, TEMPERATURE_MAX_MILLI,
};
pub use sce_rust_crypto::audit::{AuditError, Verdict as AuditVerdict};
pub use sce_rust_crypto::keyring::KeyError;
pub use sce_rust_crypto::keyring::Keyring;
pub use sce_rust_crypto::random::{RandomSource, ScriptedRandom, SystemRandom};
pub use sce_rust_runtime::AnswerSchema;
pub use sce_rust_session_store::{
    AuditStore, CredentialStore, Failure as StoreFailure, KvStore, MemoryCredentials, MemoryStore,
    TokenRow, TokenStore,
};
pub use serve::{serve, Shared};

/// The operation that generates a text (docs/SCE_HOST_FEATURES.md, section 6).
pub const MODEL_GENERATE: &str = "sce-host-model-generate";

/// The most bytes `sce-host-random-bytes` draws, the bound of its result's
/// `bytes` (`random_bytes` of the result record).
pub const RANDOM_BYTES_MAX: usize = 256;

/// The bound of a keyed digest's `message`, in bytes (`mac_message` of the
/// request records).
pub const MAC_MESSAGE_MAX: usize = 4096;

/// The request of an operation as the engine hands it to a host: each
/// `<param>` by name, as text. A 64-bit integer is its decimal digits.
pub type Params = BTreeMap<String, String>;

/// The identity of a host-run invocation (docs/SCE_HOST_FEATURES.md, section
/// 1, "Identity"): the session that started it, the `<invoke>`'s id, and which
/// start of that `<invoke>` in that session it is. A restore starts a running
/// invocation again under the identity it had, so two starts with one identity
/// are one operation asked twice.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Invocation {
    /// The id of the session that started it (`_sessionid`).
    pub session_id: String,
    /// The `<invoke>`'s id, which its completion names.
    pub id: String,
    /// Which start of the `<invoke>` in the session it is, counted from 1.
    pub start: u64,
}

impl Invocation {
    /// The identity of start `start` of the `<invoke>` `id` in the session
    /// `session_id`.
    pub fn new(session_id: &str, id: &str, start: u64) -> Self {
        Self {
            session_id: session_id.to_string(),
            id: id.to_string(),
            start,
        }
    }
}

/// Every operation type of the standard host features, in the order the
/// specification lists them.
pub const TYPES: [&str; 27] = [
    "sce-host-clock-now",
    "sce-host-clock-zone",
    "sce-host-kv-get",
    "sce-host-kv-put",
    "sce-host-kv-put-if",
    "sce-host-kv-delete",
    "sce-host-kv-next-key",
    "sce-host-notify-permission",
    "sce-host-notify-request-permission",
    "sce-host-notify-post",
    "sce-host-notify-cancel",
    "sce-host-http-fetch",
    MODEL_GENERATE,
    cose::SIGN1,
    cose::VERIFY1,
    cose::ENCRYPT0,
    cose::DECRYPT0,
    cose::KEY_DERIVE,
    keys::RANDOM_BYTES,
    keys::MAC,
    keys::MAC_VERIFY,
    keys::KEY_GENERATE,
    keys::KEY_PUBLIC,
    keys::KEY_RETIRE,
    credentials::CREDENTIAL_ISSUE,
    credentials::CREDENTIAL_REVOKE,
    audit::AUDIT_APPEND,
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
pub(crate) const INVALID_ARGUMENT: &str = r#"{"reason":"invalid_argument"}"#;

/// The standard host features' semantics, over a clock and a transport.
pub struct ReferenceHost<T: Transport> {
    pub(crate) clock: Box<dyn ClockSource + Send>,
    pub(crate) transport: T,
    /// The store of the key-value feature, the first result of each put-if by
    /// the identity of its invocation with it: the one the host was built with
    /// (docs/adr/0109, D13).
    pub(crate) kv: Arc<dyn KvStore>,
    /// The most keys the key-value store holds, set by the embedding; `None`
    /// for no bound.
    pub(crate) quota: Option<u64>,
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
    /// Where a generation's text comes from: a scripted provider until the
    /// embedding gives another.
    pub(crate) provider: Box<dyn ModelProvider + Send>,
    /// The machine's table of answer schemas, which a generation's
    /// `answer_schema` is resolved through.
    pub(crate) answer_schemas: Vec<AnswerSchema>,
    /// The keys of signing, sealing and tagging the embedding configured.
    pub(crate) cose: cose::Keys,
    /// The keys the host generated and derived, in the key store it was given:
    /// in memory until the embedding gives another.
    pub(crate) keyring: Keyring,
    /// Where random bytes, a generated key and its id, the naming key of
    /// derived keys and each seal's nonce come from: the operating system
    /// until the embedding gives another.
    pub(crate) random: Box<dyn RandomSource + Send>,
    /// Where each seal's nonce comes from when the embedding gave a source of
    /// its own; none draws it from `random`.
    pub(crate) nonces: Option<Box<dyn NonceSource + Send>>,
    /// The pins and the bearer tokens the host keeps, each as its digest: in
    /// memory until the embedding gives another (docs/SCE_HOST_FEATURES.md,
    /// section 14).
    pub(crate) credentials: Arc<dyn CredentialStore>,
    /// The audit log the host appends to (docs/SCE_HOST_FEATURES.md, section
    /// 15): in memory until the embedding gives another.
    pub(crate) audit: Arc<dyn AuditStore>,
    /// The streams the host keeps, each with its policy; an append to another
    /// fails `unknown_stream`.
    pub(crate) audit_streams: audit::AuditStreams,
}

/// A start the host was given: the operation, the invocation's id, the
/// session that started it, which start of its `<invoke>` it is, and the
/// request's parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Start {
    pub operation: String,
    pub id: String,
    pub session_id: String,
    pub start: u64,
    pub params: Params,
}

impl<T: Transport> ReferenceHost<T> {
    /// A host reading `clock` and carrying HTTP over `transport`, with an empty
    /// key-value store in memory, nothing posted, the notification permission
    /// undetermined, and no key. A host that stops forgets what its store held.
    pub fn new(clock: Box<dyn ClockSource + Send>, transport: T) -> Self {
        Self::with_kv_store(clock, transport, Arc::new(MemoryStore::new()))
    }

    /// [`Self::new`], with `store` as its key-value store: the store the
    /// program opened, durable when it is one (the session store's SQLite file,
    /// `sce_rust_session_store::SqliteStore`), in which the host keeps each
    /// put-if's first result with its write (docs/SCE_HOST_FEATURES.md
    /// section 3; docs/adr/0109, D13).
    pub fn with_kv_store(
        clock: Box<dyn ClockSource + Send>,
        transport: T,
        store: Arc<dyn KvStore>,
    ) -> Self {
        Self {
            clock,
            transport,
            kv: store,
            quota: None,
            notifications: notify::Notifications::default(),
            faults: BTreeMap::new(),
            held: Vec::new(),
            cancelled: Vec::new(),
            monotonic_floor: i64::MIN,
            starts: Vec::new(),
            provider: Box::new(ScriptedProvider::default()),
            answer_schemas: Vec::new(),
            cose: cose::Keys::default(),
            keyring: Keyring::in_memory(),
            random: Box::new(SystemRandom),
            nonces: None,
            credentials: Arc::new(MemoryCredentials::new()),
            audit: Arc::new(MemoryStore::new()),
            audit_streams: audit::AuditStreams::new(),
        }
    }

    /// Append audit entries to `store` from here on: the store the program
    /// opened, durable when it is one (the session store's SQLite file), which
    /// keeps every row before the append answers (docs/SCE_HOST_FEATURES.md
    /// section 15). What the store held before is no longer the host's.
    pub fn set_audit_store(&mut self, store: Arc<dyn AuditStore>) {
        self.audit = store;
    }

    /// Keep the streams `streams` names, each with its policy, from here on, in
    /// place of those kept before; an append to another stream fails
    /// `unknown_stream`.
    pub fn set_audit_streams(&mut self, streams: audit::AuditStreams) {
        self.audit_streams = streams;
    }

    /// The audit log the host appends to, which a reader verifies
    /// (`sce_rust_crypto::audit::verify`).
    pub fn audit_store(&self) -> &Arc<dyn AuditStore> {
        &self.audit
    }

    /// The public key of the host's audit key, which a reader of its log
    /// verifies the checkpoints under: [`KeyError::Unknown`] until the host
    /// signed a first checkpoint, which draws the key.
    pub fn audit_public_key(&self) -> Result<[u8; 32], KeyError> {
        self.keyring
            .public_key(sce_rust_crypto::keyring::AUDIT_SIGNING_KEY_ID)
    }

    /// What a reader finds `stream` of the host's audit log to be, under the
    /// host's audit key (`sce_rust_crypto::audit::verify`). Before the host
    /// signed a first checkpoint it holds no audit key, and a stream with no
    /// checkpoint is held to its chain alone.
    pub fn audit_verify(&self, stream: &str) -> Result<AuditVerdict, StoreFailure> {
        let public_key = self.audit_public_key().unwrap_or([0; 32]);
        sce_rust_crypto::audit::verify(self.audit.as_ref(), stream, &public_key)
    }

    /// Sign the head of every stream the host keeps unless a checkpoint is kept
    /// at it, at `now_ms`: what a host does at its orderly stop, so a
    /// truncation of the rows after the last periodic checkpoint is told.
    /// Answers each stream it signed, with the sequence.
    pub fn audit_stop(&mut self, now_ms: u64) -> Result<Vec<(String, u64)>, AuditError> {
        let mut writer = sce_rust_crypto::audit::Writer {
            store: &*self.audit,
            keyring: &self.keyring,
            random: &mut *self.random,
            now_ms,
        };
        let mut signed = Vec::new();
        for stream in self.audit_streams.keys() {
            if let Some(sequence) = writer.checkpoint_head(stream)? {
                signed.push((stream.clone(), sequence));
            }
        }
        Ok(signed)
    }

    /// Delete, from every stream the host keeps, the segments older than the
    /// stream's `retain_days` before `now_ms`, each through a checkpoint that
    /// anchors what remains; answers how many rows it deleted.
    pub fn audit_retain(&self, now_ms: u64) -> Result<u64, StoreFailure> {
        const DAY_MS: u64 = 86_400_000;
        let mut deleted = 0u64;
        for (stream, policy) in &self.audit_streams {
            let before =
                now_ms.saturating_sub(u64::from(policy.retain_days).saturating_mul(DAY_MS));
            deleted = deleted.saturating_add(sce_rust_crypto::audit::retain(
                self.audit.as_ref(),
                stream,
                before,
            )?);
        }
        Ok(deleted)
    }

    /// Keep the tokens the host issues, and revoke tokens and pins, in `store`
    /// from here on: the credential store the program opened, which its `https`
    /// server's authenticator reads too (docs/SCE_HOST_FEATURES.md section 14).
    /// What the store held before is no longer the host's.
    pub fn set_credentials(&mut self, store: Arc<dyn CredentialStore>) {
        self.credentials = store;
    }

    /// Sign, seal, tag and verify with `keys` from here on, in place of the
    /// keys configured before. The keys the host generated and derived are
    /// kept, under the ids it gave them; a key configured under an id of their
    /// namespace (`generated:`, `derived:`) is refused.
    pub fn set_cose_keys(&mut self, keys: CoseKeys) -> Result<(), ReservedKeyId> {
        self.cose.configure(keys)
    }

    /// Keep the keys the host generates and derives in `keyring` from here on:
    /// a key store the program opened, with the key-encryption key it holds
    /// (docs/SCE_HOST_FEATURES.md section 13). The keys of the key ring held
    /// before are no longer the host's.
    pub fn set_keyring(&mut self, keyring: Keyring) {
        self.keyring = keyring;
    }

    /// Draw random bytes, keys, key ids, the naming key of derived keys and,
    /// unless a nonce source was given, the nonce of each seal from `source`
    /// from here on, in place of the operating system's random source. A
    /// source that is not random gives known keys and nonces: it is for a test
    /// that holds the host to its bytes.
    pub fn set_random_source(&mut self, source: Box<dyn RandomSource + Send>) {
        self.random = source;
    }

    /// Draw the nonce of each seal from `source` from here on, in place of the
    /// host's random source. A source that gives one nonce twice under one key
    /// gives both plaintexts away: one that is not random is for a test that
    /// holds a seal to its bytes.
    pub fn set_nonce_source(&mut self, source: Box<dyn NonceSource + Send>) {
        self.nonces = Some(source);
    }

    /// Generate texts with `provider` from here on.
    pub fn set_model_provider(&mut self, provider: Box<dyn ModelProvider + Send>) {
        self.provider = provider;
    }

    /// Resolve a generation's `answer_schema` through `schemas`, the table of
    /// the machine this host serves (its generated `ANSWER_SCHEMAS`). A request
    /// that names an id the table does not hold is refused.
    pub fn set_answer_schemas(&mut self, schemas: &[AnswerSchema]) {
        self.answer_schemas = schemas.to_vec();
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
        self.quota = Some(u64::try_from(keys).unwrap_or(u64::MAX));
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

    /// The store's keys and values, in key order, or why the store could not
    /// list them.
    pub fn store_entries(&self) -> Result<Vec<(String, String)>, StoreFailure> {
        kv::entries(self.kv.as_ref())
    }

    /// The key-value store the host was built with.
    pub fn kv_store(&self) -> &Arc<dyn KvStore> {
        &self.kv
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

    /// Start the operation `operation` of `invocation` with `params`.
    ///
    /// `None` for a type that is not one of the standard host features: the host
    /// serves none of it. Otherwise the arguments are judged first (a refusal
    /// starts nothing), then a failure the embedding injected, then the
    /// operation itself.
    ///
    /// A put-if whose invocation was answered before is answered with that first
    /// result, a failure included, and changes nothing: the engine starts a
    /// running invocation again under the same identity when it restores a
    /// state saved before the answer was taken (docs/adr/0107, D4), and the
    /// write it asked for was made once. The store keeps the first result in
    /// the transaction of the write (docs/adr/0109, D13), so a durable store
    /// answers it after the host's process stopped, whether it stopped before
    /// or after the answer left.
    pub fn start(
        &mut self,
        operation: &str,
        invocation: &Invocation,
        params: &Params,
    ) -> Option<Outcome> {
        if !TYPES.contains(&operation) {
            return None;
        }
        let id = invocation.id.as_str();
        self.starts.push(Start {
            operation: operation.to_string(),
            id: id.to_string(),
            session_id: invocation.session_id.clone(),
            start: invocation.start,
            params: params.clone(),
        });
        if operation == kv::PUT_IF {
            if let Some(first) = kv::first(self, invocation) {
                sce_rust_runtime::sce_log_info!(
                    "{operation} {id}: start {} of session {} repeated, answered as first",
                    invocation.start,
                    invocation.session_id
                );
                return Some(first);
            }
        }
        Some(self.perform(operation, invocation, params))
    }

    /// The operation itself, as [`Self::start`] describes it, for a start that is
    /// not a repeat.
    fn perform(&mut self, operation: &str, invocation: &Invocation, params: &Params) -> Outcome {
        let id = invocation.id.as_str();
        let argument = |name: &str| params.get(name).map(String::as_str).unwrap_or("");
        let refused = match operation {
            "sce-host-kv-get" | "sce-host-kv-delete" => !kv::valid_key(argument("key")),
            "sce-host-kv-put" => !kv::valid_key(argument("key")),
            "sce-host-kv-put-if" => {
                !kv::valid_key(argument("key")) || !kv::valid_version(argument("expected_version"))
            }
            "sce-host-notify-post" | "sce-host-notify-cancel" => argument("id").is_empty(),
            "sce-host-http-fetch" => {
                !http::acceptable(argument("method"), argument("url"), argument("headers"))
            }
            MODEL_GENERATE => model::refuses(&self.answer_schemas, id, params),
            signing_or_sealing if cose::OPERATIONS.contains(&signing_or_sealing) => {
                cose::refuses(signing_or_sealing, params)
            }
            of_keys if keys::OPERATIONS.contains(&of_keys) => keys::refuses(of_keys, params),
            of_credentials if credentials::OPERATIONS.contains(&of_credentials) => {
                credentials::refuses(of_credentials, params)
            }
            audit::AUDIT_APPEND => audit::refuses(params),
            _ => false,
        };
        if refused {
            return Outcome::Refuse(INVALID_ARGUMENT);
        }
        if let Some(code) = self.faults.get(operation) {
            sce_rust_runtime::sce_log_info!("{operation} {id}: fails {code}, injected");
            // A put-if's failure is its first result too, which a repeat is
            // answered with (docs/SCE_HOST_FEATURES.md section 3).
            if operation == kv::PUT_IF {
                return kv::keep_failure(self, invocation, code);
            }
            return failure(operation, code);
        }
        match operation {
            "sce-host-clock-now" => clock::now(self),
            "sce-host-clock-zone" => clock::zone(self),
            "sce-host-kv-get" => kv::get(self, argument("key")),
            "sce-host-kv-put" => kv::put(self, argument("key"), argument("value")),
            kv::PUT_IF => kv::put_if(
                self,
                invocation,
                argument("key"),
                argument("value"),
                argument("expected_version"),
            ),
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
            MODEL_GENERATE => model::generate(self, id, params),
            signing_or_sealing if cose::OPERATIONS.contains(&signing_or_sealing) => {
                cose::perform(self, signing_or_sealing, params)
            }
            of_keys if keys::OPERATIONS.contains(&of_keys) => {
                keys::perform(self, of_keys, invocation, params)
            }
            of_credentials if credentials::OPERATIONS.contains(&of_credentials) => {
                credentials::perform(self, of_credentials, invocation, params)
            }
            audit::AUDIT_APPEND => audit::perform(self, invocation, params),
            _ => unreachable!("the type was checked against TYPES"),
        }
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
        "sce-host-kv-get" => serde_json::json!({ "found": false, "value": "", "version": "0" }),
        "sce-host-kv-put" => serde_json::json!({ "replaced": false }),
        "sce-host-kv-put-if" => serde_json::json!({ "written": false, "version": "0" }),
        "sce-host-kv-delete" | "sce-host-notify-cancel" => serde_json::json!({ "existed": false }),
        "sce-host-kv-next-key" => serde_json::json!({ "found": false, "key": "" }),
        "sce-host-notify-permission" | "sce-host-notify-request-permission" => {
            serde_json::json!({ "state": "" })
        }
        "sce-host-notify-post" => serde_json::json!({ "status": "" }),
        "sce-host-http-fetch" => serde_json::json!({ "status": 0, "headers": "", "body": "" }),
        MODEL_GENERATE => serde_json::json!({
            "text": "",
            "constrained": false,
            "model": "",
            "prompt_tokens": 0,
            "answer_tokens": 0,
            "finish": "",
        }),
        cose::SIGN1 | cose::ENCRYPT0 => serde_json::json!({ "cose": "" }),
        cose::VERIFY1 => serde_json::json!({ "valid": false, "key_id": "", "payload": "" }),
        cose::DECRYPT0 => serde_json::json!({ "opened": false, "plaintext": "" }),
        cose::KEY_DERIVE | keys::KEY_GENERATE => serde_json::json!({ "key_id": "" }),
        keys::RANDOM_BYTES => serde_json::json!({ "bytes": "" }),
        keys::MAC => serde_json::json!({ "tag": "" }),
        keys::MAC_VERIFY => serde_json::json!({ "valid": false }),
        keys::KEY_PUBLIC => serde_json::json!({ "cose_key": "" }),
        keys::KEY_RETIRE => serde_json::json!({ "retired": false }),
        credentials::CREDENTIAL_ISSUE => serde_json::json!({ "token": "" }),
        credentials::CREDENTIAL_REVOKE => serde_json::json!({ "revoked": 0 }),
        audit::AUDIT_APPEND => serde_json::json!({ "sequence": "0" }),
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
