// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What an SCE server reports of what it does, through a port (docs/adr/0109, D16).
//!
//! Observability through a port: a component reports named counters, gauges and histograms to an
//! interface its host implements, and the exposition format and the collector are the host's. The
//! `https` server, the keyed pool, the store's committer, the reference host of the standard host
//! features and the `https` client report to [`Metrics`]; a program gives them the adapter it
//! chooses, and one that gives none gets [`NoMetrics`], which reports nothing.
//!
//! **Fixed names, closed label sets.** Every series is a value of [`Counter`], [`Gauge`] or
//! [`Histogram`]: its family is one of [`FAMILIES`], whose name and label names are fixed, and
//! each label value is a variant of a closed enum of this crate ([`ResponseStatus`],
//! [`ServerShed`], ...), which holds only the values its family's events produce. A caller cannot hand a label a text of its own, so no series carries a
//! client's name, a session's key or anything else a client chose: the cardinality of every
//! family is bounded by its enums, and what an exposition shows is fixed by this crate. The
//! names are the registry of `docs/SCE_SERVER_DEPLOY.md` section 10, a wire surface of
//! SCE_WIRE_CONTRACTS.md named [`REGISTRY`].
//!
//! **Adapters.** [`openmetrics::OpenMetrics`] is the reference adapter: it keeps every series in
//! memory and renders the OpenMetrics text exposition, which the program serves on a route of its
//! own. [`recording::Recording`] keeps every call in order, for a test that asserts what a
//! component reported. A program on another metrics stack implements [`Metrics`] over it.

#![forbid(unsafe_code)]

pub mod openmetrics;
pub mod recording;

/// The name of the registry of names this crate holds: what the exposition's `sce_metrics_info`
/// series names, so that a dashboard or an alert written against one registry tells another.
/// A change that renames or removes a family, a label or a label value names a new registry.
pub const REGISTRY: &str = "sce-metrics-1";

/// What a component reports to: the port its host implements (docs/adr/0109, D16).
///
/// Called from any thread, often from many at once, on the paths that serve clients: an adapter
/// answers at once and never blocks on a collector.
pub trait Metrics: Send + Sync {
    /// `counter` moved by `by`.
    fn count(&self, counter: Counter, by: u64);

    /// `gauge` moved by `delta`, up or down: every gauge here is a level the components move by
    /// what they add and take away, so components that share an adapter add up.
    fn adjust(&self, gauge: Gauge, delta: i64);

    /// One observation of `histogram`: a duration in seconds, or a count, as its family says.
    fn observe(&self, histogram: Histogram, value: f64);
}

/// The adapter that reports nothing: what every component reports to when its program gives it
/// no adapter, so that every constructor that takes none keeps working.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NoMetrics;

impl Metrics for NoMetrics {
    fn count(&self, _: Counter, _: u64) {}

    fn adjust(&self, _: Gauge, _: i64) {}

    fn observe(&self, _: Histogram, _: f64) {}
}

/// [`NoMetrics`] as the shared handle a component keeps.
pub fn none() -> std::sync::Arc<dyn Metrics> {
    std::sync::Arc::new(NoMetrics)
}

/// The kind of a family, as the OpenMetrics exposition names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A total that only grows: its sample is the family's name with `_total`.
    Counter,
    /// A level that moves up and down.
    Gauge,
    /// Observations counted into fixed buckets, with their count and their sum.
    Histogram,
    /// A constant series whose labels say something of the exposition: its sample is the
    /// family's name with `_info`, and its value is 1.
    Info,
}

impl Kind {
    /// How the OpenMetrics exposition names the kind (`# TYPE`).
    pub const fn name(self) -> &'static str {
        match self {
            Kind::Counter => "counter",
            Kind::Gauge => "gauge",
            Kind::Histogram => "histogram",
            Kind::Info => "info",
        }
    }
}

/// One family of the registry: its fixed name, its kind, its unit (empty for none), what it
/// counts, the names of its labels, and, for a histogram, the upper bounds of its buckets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Family {
    pub name: &'static str,
    pub kind: Kind,
    pub unit: &'static str,
    pub help: &'static str,
    pub labels: &'static [&'static str],
    pub buckets: &'static [f64],
}

/// The buckets of a duration, in seconds: from half a millisecond to half a minute.
const DURATION_BUCKETS: &[f64] = &[
    0.0005, 0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0,
];

/// The buckets of a batch's size, in writes: powers of two up to 1024.
const BATCH_BUCKETS: &[f64] = &[
    1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0, 128.0, 256.0, 512.0, 1024.0,
];

/// `sce_metrics`: the registry of names the exposition follows ([`REGISTRY`]).
pub static METRICS_INFO: Family = Family {
    name: "sce_metrics",
    kind: Kind::Info,
    unit: "",
    help: "The registry of names these metrics follow.",
    labels: &["registry"],
    buckets: &[],
};

/// `sce_server_exchanges`: [`Counter::ServerExchanges`].
pub static SERVER_EXCHANGES: Family = Family {
    name: "sce_server_exchanges",
    kind: Kind::Counter,
    unit: "",
    help: "Exchanges the https server answered, by the status of the answer.",
    labels: &["status"],
    buckets: &[],
};

/// `sce_server_held_exchanges`: [`Gauge::HeldExchanges`].
pub static SERVER_HELD_EXCHANGES: Family = Family {
    name: "sce_server_held_exchanges",
    kind: Kind::Gauge,
    unit: "",
    help: "Exchanges the https server holds open while the mailbox of their client is empty.",
    labels: &[],
    buckets: &[],
};

/// `sce_server_sheds`: [`Counter::ServerSheds`].
pub static SERVER_SHEDS: Family = Family {
    name: "sce_server_sheds",
    kind: Kind::Counter,
    unit: "",
    help: "Exchanges the https server answered 429 or 503 as load shed, and connections it \
           closed at accept, by why.",
    labels: &["reason"],
    buckets: &[],
};

/// `sce_server_authentication_failures`: [`Counter::AuthenticationFailures`].
pub static SERVER_AUTHENTICATION_FAILURES: Family = Family {
    name: "sce_server_authentication_failures",
    kind: Kind::Counter,
    unit: "",
    help: "Exchanges the https server answered 401, by the authenticator that refused them.",
    labels: &["authenticator"],
    buckets: &[],
};

/// `sce_pool_live_sessions`: [`Gauge::LiveSessions`].
pub static POOL_LIVE_SESSIONS: Family = Family {
    name: "sce_pool_live_sessions",
    kind: Kind::Gauge,
    unit: "",
    help: "Sessions the keyed pool holds in memory.",
    labels: &[],
    buckets: &[],
};

/// `sce_pool_loads`: [`Counter::Loads`].
pub static POOL_LOADS: Family = Family {
    name: "sce_pool_loads",
    kind: Kind::Counter,
    unit: "",
    help: "Sessions the keyed pool brought into memory, started or restored.",
    labels: &[],
    buckets: &[],
};

/// `sce_pool_restores`: [`Counter::Restores`].
pub static POOL_RESTORES: Family = Family {
    name: "sce_pool_restores",
    kind: Kind::Counter,
    unit: "",
    help: "Sessions the keyed pool restored from their last durable checkpoint, by whether it \
           was clean; one that was not raises restored in its machine.",
    labels: &["checkpoint"],
    buckets: &[],
};

/// `sce_pool_evictions`: [`Counter::Evictions`].
pub static POOL_EVICTIONS: Family = Family {
    name: "sce_pool_evictions",
    kind: Kind::Counter,
    unit: "",
    help: "Sessions the keyed pool saved clean and dropped from memory, by why.",
    labels: &["reason"],
    buckets: &[],
};

/// `sce_pool_sheds`: [`Counter::PoolSheds`].
pub static POOL_SHEDS: Family = Family {
    name: "sce_pool_sheds",
    kind: Kind::Counter,
    unit: "",
    help: "Envelopes the keyed pool shed before a session took them in, by why.",
    labels: &["reason"],
    buckets: &[],
};

/// `sce_pool_lease_renewals_refused`: [`Counter::LeaseRenewalsRefused`].
pub static POOL_LEASE_RENEWALS_REFUSED: Family = Family {
    name: "sce_pool_lease_renewals_refused",
    kind: Kind::Counter,
    unit: "",
    help: "Lease renewals the session store refused: keys this process lost.",
    labels: &[],
    buckets: &[],
};

/// `sce_pool_outbox_depth`: [`Gauge::OutboxDepth`].
pub static POOL_OUTBOX_DEPTH: Family = Family {
    name: "sce_pool_outbox_depth",
    kind: Kind::Gauge,
    unit: "",
    help: "Envelopes the process holds of its durable clients' outboxes, unacknowledged.",
    labels: &[],
    buckets: &[],
};

/// `sce_pool_outbox_expiries`: [`Counter::OutboxExpiries`].
pub static POOL_OUTBOX_EXPIRIES: Family = Family {
    name: "sce_pool_outbox_expiries",
    kind: Kind::Counter,
    unit: "",
    help: "Outbox entries deleted unacknowledged past their binding's retain_ms.",
    labels: &[],
    buckets: &[],
};

/// `sce_store_commit_duration_seconds`: [`Histogram::CommitDuration`].
pub static STORE_COMMIT_DURATION: Family = Family {
    name: "sce_store_commit_duration_seconds",
    kind: Kind::Histogram,
    unit: "seconds",
    help: "How long each commit of the store's committer took, durable on return.",
    labels: &[],
    buckets: DURATION_BUCKETS,
};

/// `sce_store_commit_batch_size`: [`Histogram::CommitBatchSize`].
pub static STORE_COMMIT_BATCH_SIZE: Family = Family {
    name: "sce_store_commit_batch_size",
    kind: Kind::Histogram,
    unit: "",
    help: "How many writes and acknowledgements each commit of the store's committer held.",
    labels: &[],
    buckets: BATCH_BUCKETS,
};

/// `sce_host_operation_duration_seconds`: [`Histogram::HostOperationDuration`].
pub static HOST_OPERATION_DURATION: Family = Family {
    name: "sce_host_operation_duration_seconds",
    kind: Kind::Histogram,
    unit: "seconds",
    help: "How long the reference host took to answer a start of a standard host operation, \
           by its type; a start the host holds is not observed.",
    labels: &["type"],
    buckets: DURATION_BUCKETS,
};

/// `sce_host_operation_failures`: [`Counter::HostOperationFailures`].
pub static HOST_OPERATION_FAILURES: Family = Family {
    name: "sce_host_operation_failures",
    kind: Kind::Counter,
    unit: "",
    help: "Starts of a standard host operation the reference host answered with a failure code \
           or refused, by type and code.",
    labels: &["type", "code"],
    buckets: &[],
};

/// `sce_host_audit_appends`: [`Counter::AuditAppends`].
pub static HOST_AUDIT_APPENDS: Family = Family {
    name: "sce_host_audit_appends",
    kind: Kind::Counter,
    unit: "",
    help: "Entries the reference host appended to its audit log; a repeat appends none.",
    labels: &[],
    buckets: &[],
};

/// `sce_client_exchanges`: [`Counter::ClientExchanges`].
pub static CLIENT_EXCHANGES: Family = Family {
    name: "sce_client_exchanges",
    kind: Kind::Counter,
    unit: "",
    help: "Exchanges an https client made, by the status its server answered, none when no \
           response came.",
    labels: &["status"],
    buckets: &[],
};

/// Every family of the registry, in the order an exposition renders them.
pub static FAMILIES: [&Family; 19] = [
    &METRICS_INFO,
    &SERVER_EXCHANGES,
    &SERVER_HELD_EXCHANGES,
    &SERVER_SHEDS,
    &SERVER_AUTHENTICATION_FAILURES,
    &POOL_LIVE_SESSIONS,
    &POOL_LOADS,
    &POOL_RESTORES,
    &POOL_EVICTIONS,
    &POOL_SHEDS,
    &POOL_LEASE_RENEWALS_REFUSED,
    &POOL_OUTBOX_DEPTH,
    &POOL_OUTBOX_EXPIRIES,
    &STORE_COMMIT_DURATION,
    &STORE_COMMIT_BATCH_SIZE,
    &HOST_OPERATION_DURATION,
    &HOST_OPERATION_FAILURES,
    &HOST_AUDIT_APPENDS,
    &CLIENT_EXCHANGES,
];

/// A closed set of label values: an enum whose every variant is one value, the whole set
/// (`ALL`) and each variant's text (`value`).
macro_rules! label_set {
    (
        $(#[$meta:meta])*
        $name:ident {
            $( $(#[$variant_meta:meta])* $variant:ident => $value:literal, )+
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum $name {
            $( $(#[$variant_meta])* $variant, )+
        }

        impl $name {
            /// Every value of the set, in its order.
            pub const ALL: &'static [$name] = &[$($name::$variant,)+];

            /// The label value it is.
            pub const fn value(self) -> &'static str {
                match self {
                    $($name::$variant => $value,)+
                }
            }
        }
    };
}

label_set! {
    /// The status code of a response the `https` server answers an exchange with (RFC 9110
    /// section 15): exactly the codes it sends, so the server builds every response from one of
    /// them and no other can be reported.
    ResponseStatus {
        Ok => "200",
        BadRequest => "400",
        Unauthorized => "401",
        NotFound => "404",
        MethodNotAllowed => "405",
        RequestTimeout => "408",
        /// The fence of docs/adr/0113, D4: an exchange made under an incarnation of an instance's
        /// membership the server does not hold (RFC 9110 section 15.5.13).
        PreconditionFailed => "412",
        PayloadTooLarge => "413",
        UnsupportedMediaType => "415",
        TooManyRequests => "429",
        ServiceUnavailable => "503",
    }
}

impl ResponseStatus {
    /// The status code it is.
    pub const fn code(self) -> u16 {
        match self {
            ResponseStatus::Ok => 200,
            ResponseStatus::BadRequest => 400,
            ResponseStatus::Unauthorized => 401,
            ResponseStatus::NotFound => 404,
            ResponseStatus::MethodNotAllowed => 405,
            ResponseStatus::RequestTimeout => 408,
            ResponseStatus::PayloadTooLarge => 413,
            ResponseStatus::UnsupportedMediaType => 415,
            ResponseStatus::PreconditionFailed => 412,
            ResponseStatus::TooManyRequests => 429,
            ResponseStatus::ServiceUnavailable => 503,
        }
    }

    /// The status `code` is, when the server answers with it; `None` for any other.
    pub fn of(code: u16) -> Option<Self> {
        ResponseStatus::ALL
            .iter()
            .copied()
            .find(|status| status.code() == code)
    }
}

label_set! {
    /// What an exchange a client made came to: the status its server answered, `other` for a
    /// status no SCE server answers (an intermediary's), and `none` when no response came.
    ExchangeOutcome {
        Ok => "200",
        BadRequest => "400",
        Unauthorized => "401",
        NotFound => "404",
        MethodNotAllowed => "405",
        RequestTimeout => "408",
        PreconditionFailed => "412",
        PayloadTooLarge => "413",
        UnsupportedMediaType => "415",
        TooManyRequests => "429",
        ServiceUnavailable => "503",
        Other => "other",
        NoResponse => "none",
    }
}

impl ExchangeOutcome {
    /// The outcome of a response of status `code`: the server's status, or `other`.
    pub fn of_status(code: u16) -> Self {
        ResponseStatus::of(code).map_or(ExchangeOutcome::Other, ExchangeOutcome::from)
    }
}

impl From<ResponseStatus> for ExchangeOutcome {
    fn from(status: ResponseStatus) -> Self {
        match status {
            ResponseStatus::Ok => ExchangeOutcome::Ok,
            ResponseStatus::BadRequest => ExchangeOutcome::BadRequest,
            ResponseStatus::Unauthorized => ExchangeOutcome::Unauthorized,
            ResponseStatus::NotFound => ExchangeOutcome::NotFound,
            ResponseStatus::MethodNotAllowed => ExchangeOutcome::MethodNotAllowed,
            ResponseStatus::RequestTimeout => ExchangeOutcome::RequestTimeout,
            ResponseStatus::PayloadTooLarge => ExchangeOutcome::PayloadTooLarge,
            ResponseStatus::UnsupportedMediaType => ExchangeOutcome::UnsupportedMediaType,
            ResponseStatus::PreconditionFailed => ExchangeOutcome::PreconditionFailed,
            ResponseStatus::TooManyRequests => ExchangeOutcome::TooManyRequests,
            ResponseStatus::ServiceUnavailable => ExchangeOutcome::ServiceUnavailable,
        }
    }
}

label_set! {
    /// Why the `https` server shed an exchange or closed a connection at accept.
    ServerShed {
        /// A limiter of the edge refused it (`rate:`): `429`.
        Rate => "rate",
        /// `max_exchanges` in flight (`503`), or `max_exchanges_per_client` for its client
        /// (`429`).
        Concurrency => "concurrency",
        /// Its name would make an entry past `max_clients`: `503`.
        MaxClients => "max_clients",
        /// The server holds `max_connections` connections: closed at accept.
        MaxConnections => "max_connections",
        /// Its address holds `max_connections_per_address` connections: closed at accept.
        MaxConnectionsPerAddress => "max_connections_per_address",
        /// The envelopes it carried were not made durable, or not within the longest hold:
        /// `503`.
        Durability => "durability",
        /// A store the exchange reads could not be read or written (a pin store, a durable
        /// client's outbox): `503`.
        Store => "store",
        /// The server is stopping: `503`.
        Stopping => "stopping",
        /// No incarnation could be drawn from the operating system's random source: `503`.
        RandomSource => "random_source",
        /// Its instance's offer stands while as many members of its open machine stand as
        /// `max_instances` allows, none of whom is put out for it (docs/adr/0113, D6): `503`.
        MaxInstances => "max_instances",
        /// The port of proofs of a person could not judge the proof an exchange carried to
        /// confirm its instance's offer: the identity provider it asks did not answer
        /// (docs/adr/0116, D8): `503`.
        IdentityProvider => "identity_provider",
    }
}

label_set! {
    /// Why the keyed pool shed an envelope before a session took it in.
    PoolShed {
        /// Its key is not in memory, the pool holds `sessions.max_live`, and none is quiet.
        MaxLive => "max_live",
        /// Its session's inbox holds `sessions.max_inbox` envelopes.
        MaxInbox => "max_inbox",
        /// The pool is stopping.
        Stopping => "stopping",
    }
}

label_set! {
    /// The authenticator of a binding (SCE_MESH.md section 22.5).
    AuthenticatorKind {
        Bearer => "bearer",
        Install => "install",
        Mtls => "mtls",
        Key => "key",
    }
}

label_set! {
    /// Whether the checkpoint a session was restored from was clean.
    RestoredFrom {
        /// Saved idle or by an orderly stop: the restore raises nothing.
        Clean => "clean",
        /// Written while the session ran: the restore raises `restored` (SCE_MESH.md section
        /// 16.7 row 17).
        NotClean => "not_clean",
    }
}

label_set! {
    /// Why the keyed pool saved a session clean and dropped it.
    Eviction {
        /// It had no event for `sessions.idle_ms` and held nothing pending.
        Idle => "idle",
        /// The pool held `sessions.max_live` and another key needed the room.
        MaxLive => "max_live",
    }
}

label_set! {
    /// The type of a standard host operation (docs/SCE_HOST_FEATURES.md).
    HostOperation {
        ClockNow => "sce-host-clock-now",
        ClockZone => "sce-host-clock-zone",
        KvGet => "sce-host-kv-get",
        KvPut => "sce-host-kv-put",
        KvPutIf => "sce-host-kv-put-if",
        KvDelete => "sce-host-kv-delete",
        KvNextKey => "sce-host-kv-next-key",
        NotifyPermission => "sce-host-notify-permission",
        NotifyRequestPermission => "sce-host-notify-request-permission",
        NotifyPost => "sce-host-notify-post",
        NotifyCancel => "sce-host-notify-cancel",
        HttpFetch => "sce-host-http-fetch",
        ModelGenerate => "sce-host-model-generate",
        CoseSign1 => "sce-host-cose-sign1",
        CoseVerify1 => "sce-host-cose-verify1",
        CoseEncrypt0 => "sce-host-cose-encrypt0",
        CoseDecrypt0 => "sce-host-cose-decrypt0",
        KeyDerive => "sce-host-key-derive",
        RandomBytes => "sce-host-random-bytes",
        Mac => "sce-host-mac",
        MacVerify => "sce-host-mac-verify",
        KeyGenerate => "sce-host-key-generate",
        KeyPublic => "sce-host-key-public",
        KeyRetire => "sce-host-key-retire",
        CredentialIssue => "sce-host-credential-issue",
        CredentialRevoke => "sce-host-credential-revoke",
        AuditAppend => "sce-host-audit-append",
        JournalRead => "sce-host-journal-read",
    }
}

impl HostOperation {
    /// The operation whose type is `type_name`; `None` for a type that is no standard host
    /// operation.
    pub fn of(type_name: &str) -> Option<Self> {
        HostOperation::ALL
            .iter()
            .copied()
            .find(|operation| operation.value() == type_name)
    }
}

label_set! {
    /// The code a standard host operation failed with: the union of every operation's closed set
    /// (docs/SCE_HOST_FEATURES.md), `invalid_argument` for a start the host refused, and `other`
    /// for a code no operation names (one an embedding injected).
    HostFailure {
        Unavailable => "unavailable",
        QuotaExceeded => "quota_exceeded",
        Network => "network",
        ResponseTooLarge => "response_too_large",
        InvalidBody => "invalid_body",
        DestinationRefused => "destination_refused",
        Busy => "busy",
        ContextExceeded => "context_exceeded",
        AnswerTooLarge => "answer_too_large",
        Declined => "declined",
        UnknownKey => "unknown_key",
        KeyRetired => "key_retired",
        UnknownStream => "unknown_stream",
        InvalidArgument => "invalid_argument",
        Other => "other",
    }
}

impl HostFailure {
    /// The value of the failure code `code`: `other` for a code no operation names.
    pub fn of(code: &str) -> Self {
        HostFailure::ALL
            .iter()
            .copied()
            .find(|failure| *failure != HostFailure::Other && failure.value() == code)
            .unwrap_or(HostFailure::Other)
    }
}

/// The labels of one series: at most two, each a fixed name and a value of a closed set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Labels {
    pairs: [(&'static str, &'static str); 2],
    len: usize,
}

impl Labels {
    /// No label.
    pub const NONE: Labels = Labels {
        pairs: [("", ""); 2],
        len: 0,
    };

    const fn one(name: &'static str, value: &'static str) -> Labels {
        Labels {
            pairs: [(name, value), ("", "")],
            len: 1,
        }
    }

    const fn two(
        first: (&'static str, &'static str),
        second: (&'static str, &'static str),
    ) -> Labels {
        Labels {
            pairs: [first, second],
            len: 2,
        }
    }

    /// Each label's name and value, in the order the family names them.
    pub fn as_slice(&self) -> &[(&'static str, &'static str)] {
        &self.pairs[..self.len]
    }
}

/// A counter series: a family of kind [`Kind::Counter`] with its label values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Counter {
    /// An exchange the `https` server answered, by the status of the answer.
    ServerExchanges { status: ResponseStatus },
    /// An exchange the `https` server shed, or a connection it closed at accept.
    ServerSheds { reason: ServerShed },
    /// An exchange the `https` server answered `401`.
    AuthenticationFailures { authenticator: AuthenticatorKind },
    /// A session the keyed pool brought into memory, started or restored.
    Loads,
    /// A session the keyed pool restored from its last durable checkpoint.
    Restores { checkpoint: RestoredFrom },
    /// A session the keyed pool saved clean and dropped from memory.
    Evictions { reason: Eviction },
    /// An envelope the keyed pool shed before a session took it in.
    PoolSheds { reason: PoolShed },
    /// A lease renewal the session store refused.
    LeaseRenewalsRefused,
    /// An outbox entry deleted unacknowledged past its binding's `retain_ms`.
    OutboxExpiries,
    /// A start of a standard host operation answered with a failure code, or refused.
    HostOperationFailures {
        operation: HostOperation,
        code: HostFailure,
    },
    /// An entry appended to the audit log.
    AuditAppends,
    /// An exchange an `https` client made, by the status its server answered.
    ClientExchanges { status: ExchangeOutcome },
}

impl Counter {
    /// The family it is a series of.
    pub fn family(&self) -> &'static Family {
        match self {
            Counter::ServerExchanges { .. } => &SERVER_EXCHANGES,
            Counter::ServerSheds { .. } => &SERVER_SHEDS,
            Counter::AuthenticationFailures { .. } => &SERVER_AUTHENTICATION_FAILURES,
            Counter::Loads => &POOL_LOADS,
            Counter::Restores { .. } => &POOL_RESTORES,
            Counter::Evictions { .. } => &POOL_EVICTIONS,
            Counter::PoolSheds { .. } => &POOL_SHEDS,
            Counter::LeaseRenewalsRefused => &POOL_LEASE_RENEWALS_REFUSED,
            Counter::OutboxExpiries => &POOL_OUTBOX_EXPIRIES,
            Counter::HostOperationFailures { .. } => &HOST_OPERATION_FAILURES,
            Counter::AuditAppends => &HOST_AUDIT_APPENDS,
            Counter::ClientExchanges { .. } => &CLIENT_EXCHANGES,
        }
    }

    /// Its labels, in the order its family names them.
    pub fn labels(&self) -> Labels {
        match *self {
            Counter::ServerExchanges { status } => Labels::one("status", status.value()),
            Counter::ClientExchanges { status } => Labels::one("status", status.value()),
            Counter::ServerSheds { reason } => Labels::one("reason", reason.value()),
            Counter::AuthenticationFailures { authenticator } => {
                Labels::one("authenticator", authenticator.value())
            }
            Counter::Restores { checkpoint } => Labels::one("checkpoint", checkpoint.value()),
            Counter::Evictions { reason } => Labels::one("reason", reason.value()),
            Counter::PoolSheds { reason } => Labels::one("reason", reason.value()),
            Counter::HostOperationFailures { operation, code } => {
                Labels::two(("type", operation.value()), ("code", code.value()))
            }
            Counter::Loads
            | Counter::LeaseRenewalsRefused
            | Counter::OutboxExpiries
            | Counter::AuditAppends => Labels::NONE,
        }
    }

    /// The counters of no label, each its family's one series.
    pub const UNLABELLED: [Counter; 4] = [
        Counter::Loads,
        Counter::LeaseRenewalsRefused,
        Counter::OutboxExpiries,
        Counter::AuditAppends,
    ];
}

/// A gauge series: a family of kind [`Kind::Gauge`]. No gauge has a label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Gauge {
    /// Exchanges the `https` server holds open while their client's mailbox is empty.
    HeldExchanges,
    /// Sessions the keyed pool holds in memory.
    LiveSessions,
    /// Envelopes the process holds of its durable clients' outboxes.
    OutboxDepth,
}

impl Gauge {
    /// Every gauge.
    pub const ALL: [Gauge; 3] = [
        Gauge::HeldExchanges,
        Gauge::LiveSessions,
        Gauge::OutboxDepth,
    ];

    /// The family it is the series of.
    pub fn family(&self) -> &'static Family {
        match self {
            Gauge::HeldExchanges => &SERVER_HELD_EXCHANGES,
            Gauge::LiveSessions => &POOL_LIVE_SESSIONS,
            Gauge::OutboxDepth => &POOL_OUTBOX_DEPTH,
        }
    }

    /// Its labels: none.
    pub fn labels(&self) -> Labels {
        Labels::NONE
    }
}

/// A histogram series: a family of kind [`Kind::Histogram`] with its label values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Histogram {
    /// How long a commit of the store's committer took, in seconds.
    CommitDuration,
    /// How many items a commit of the store's committer held.
    CommitBatchSize,
    /// How long the reference host took to answer a start of `operation`, in seconds.
    HostOperationDuration { operation: HostOperation },
}

impl Histogram {
    /// The histograms of no label, each its family's one series.
    pub const UNLABELLED: [Histogram; 2] = [Histogram::CommitDuration, Histogram::CommitBatchSize];

    /// The family it is a series of.
    pub fn family(&self) -> &'static Family {
        match self {
            Histogram::CommitDuration => &STORE_COMMIT_DURATION,
            Histogram::CommitBatchSize => &STORE_COMMIT_BATCH_SIZE,
            Histogram::HostOperationDuration { .. } => &HOST_OPERATION_DURATION,
        }
    }

    /// Its labels, in the order its family names them.
    pub fn labels(&self) -> Labels {
        match *self {
            Histogram::HostOperationDuration { operation } => {
                Labels::one("type", operation.value())
            }
            Histogram::CommitDuration | Histogram::CommitBatchSize => Labels::NONE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every counter series of the registry, each label set taken whole.
    fn every_counter() -> Vec<Counter> {
        let mut all = Vec::new();
        for status in ResponseStatus::ALL {
            all.push(Counter::ServerExchanges { status: *status });
        }
        for status in ExchangeOutcome::ALL {
            all.push(Counter::ClientExchanges { status: *status });
        }
        all.extend(
            ServerShed::ALL
                .iter()
                .map(|reason| Counter::ServerSheds { reason: *reason }),
        );
        all.extend(AuthenticatorKind::ALL.iter().map(|authenticator| {
            Counter::AuthenticationFailures {
                authenticator: *authenticator,
            }
        }));
        all.extend(
            RestoredFrom::ALL
                .iter()
                .map(|checkpoint| Counter::Restores {
                    checkpoint: *checkpoint,
                }),
        );
        all.extend(
            Eviction::ALL
                .iter()
                .map(|reason| Counter::Evictions { reason: *reason }),
        );
        all.extend(
            PoolShed::ALL
                .iter()
                .map(|reason| Counter::PoolSheds { reason: *reason }),
        );
        for operation in HostOperation::ALL {
            for code in HostFailure::ALL {
                all.push(Counter::HostOperationFailures {
                    operation: *operation,
                    code: *code,
                });
            }
        }
        all.extend(Counter::UNLABELLED);
        all
    }

    /// A name the OpenMetrics exposition takes: `[a-zA-Z_][a-zA-Z0-9_]*`.
    fn is_name(text: &str) -> bool {
        let mut chars = text.chars();
        chars
            .next()
            .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
            && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
    }

    /// Every series names a family of the registry, of its own kind, with exactly the labels the
    /// family names, in order: a series cannot carry a label its family does not fix. Red if a
    /// variant's labels or family drift from the registry.
    #[test]
    fn every_series_carries_the_labels_its_family_fixes() {
        for counter in every_counter() {
            let family = counter.family();
            assert_eq!(family.kind, Kind::Counter, "{counter:?}");
            let names: Vec<&str> = counter
                .labels()
                .as_slice()
                .iter()
                .map(|(n, _)| *n)
                .collect();
            assert_eq!(names, family.labels, "{counter:?}");
        }
        for gauge in Gauge::ALL {
            assert_eq!(gauge.family().kind, Kind::Gauge, "{gauge:?}");
            assert!(gauge.labels().as_slice().is_empty());
            assert!(gauge.family().labels.is_empty());
        }
        let mut histograms: Vec<Histogram> = Histogram::UNLABELLED.to_vec();
        histograms.extend(HostOperation::ALL.iter().map(|operation| {
            Histogram::HostOperationDuration {
                operation: *operation,
            }
        }));
        for histogram in histograms {
            let family = histogram.family();
            assert_eq!(family.kind, Kind::Histogram, "{histogram:?}");
            let names: Vec<&str> = histogram
                .labels()
                .as_slice()
                .iter()
                .map(|(n, _)| *n)
                .collect();
            assert_eq!(names, family.labels, "{histogram:?}");
            assert!(!family.buckets.is_empty(), "{histogram:?}");
        }
    }

    /// The registry's names are OpenMetrics names, each once; a family with a unit ends with it,
    /// a counter does not end with `_total`, and a histogram's buckets rise. Red if a family is
    /// added under a name the exposition cannot carry.
    #[test]
    fn every_family_is_named_as_the_exposition_takes_it() {
        let mut seen: Vec<&str> = Vec::new();
        for family in FAMILIES {
            assert!(is_name(family.name), "{}", family.name);
            assert!(family.name.starts_with("sce_"), "{}", family.name);
            assert!(!seen.contains(&family.name), "{} twice", family.name);
            seen.push(family.name);
            if !family.unit.is_empty() {
                assert!(
                    family.name.ends_with(&format!("_{}", family.unit)),
                    "{}",
                    family.name
                );
            }
            if family.kind == Kind::Counter {
                assert!(!family.name.ends_with("_total"), "{}", family.name);
            }
            for label in family.labels {
                assert!(is_name(label) && !label.starts_with("__"), "{label}");
                assert_ne!(*label, "le");
            }
            assert!(family.buckets.windows(2).all(|pair| pair[0] < pair[1]));
            assert!(family.help.chars().all(|c| c != '\n'));
        }
        assert_eq!(seen.len(), FAMILIES.len());
    }

    /// Every family a series names is in [`FAMILIES`], so the exposition renders it. Red if a
    /// family is added and not listed.
    #[test]
    fn every_series_family_is_listed() {
        let listed = |family: &Family| FAMILIES.iter().any(|kept| kept.name == family.name);
        for counter in every_counter() {
            assert!(listed(counter.family()), "{counter:?}");
        }
        for gauge in Gauge::ALL {
            assert!(listed(gauge.family()), "{gauge:?}");
        }
        assert!(listed(
            Histogram::HostOperationDuration {
                operation: HostOperation::KvGet
            }
            .family()
        ));
        for histogram in Histogram::UNLABELLED {
            assert!(listed(histogram.family()), "{histogram:?}");
        }
    }

    /// Each closed set reads its own values back and refuses or folds what it does not name.
    #[test]
    fn a_value_is_read_back_into_its_closed_set() {
        for status in ResponseStatus::ALL {
            assert_eq!(status.value().parse::<u16>(), Ok(status.code()));
            assert_eq!(ResponseStatus::of(status.code()), Some(*status));
            assert_eq!(
                ExchangeOutcome::of_status(status.code()).value(),
                status.value()
            );
        }
        assert_eq!(ResponseStatus::of(502), None);
        assert_eq!(ExchangeOutcome::of_status(502), ExchangeOutcome::Other);
        for operation in HostOperation::ALL {
            assert_eq!(HostOperation::of(operation.value()), Some(*operation));
        }
        assert_eq!(HostOperation::of("phone"), None);
        for failure in HostFailure::ALL {
            assert_eq!(HostFailure::of(failure.value()), *failure);
        }
        assert_eq!(HostFailure::of("a client's name"), HostFailure::Other);
    }
}
