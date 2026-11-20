// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What the reference host reports through the metrics port (docs/adr/0109,
// D16): how long each start of an operation took to answer, by its type, and
// the code of each failure and refusal, through a recording adapter.

use std::collections::BTreeMap;
use std::sync::Arc;

use sce_rust_host_features::{
    HttpRequest, HttpResponse, Invocation, Outcome, Params, ReferenceHost, ScriptedClock,
    Transport, TransportFailure, TYPES,
};
use sce_rust_metrics::recording::{Call, Recording};
use sce_rust_metrics::{Counter, Histogram, HostFailure, HostOperation, Metrics};

/// A transport whose every exchange gets no response.
struct Unreachable;

impl Transport for Unreachable {
    fn exchange(&mut self, _: &HttpRequest) -> Result<Option<HttpResponse>, TransportFailure> {
        Err(TransportFailure::Network)
    }
}

/// A transport that holds every exchange: the host answers later, or never.
struct Holding;

impl Transport for Holding {
    fn exchange(&mut self, _: &HttpRequest) -> Result<Option<HttpResponse>, TransportFailure> {
        Ok(None)
    }
}

fn params(pairs: &[(&str, &str)]) -> Params {
    pairs
        .iter()
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect()
}

fn reported<T: Transport>(transport: T) -> (ReferenceHost<T>, Arc<Recording>) {
    let mut host = ReferenceHost::new(Box::new(ScriptedClock::new()), transport);
    let recording = Arc::new(Recording::new());
    host.set_metrics(Arc::clone(&recording) as Arc<dyn Metrics>);
    (host, recording)
}

fn failures(recording: &Recording, operation: HostOperation, code: HostFailure) -> u64 {
    recording.counted(Counter::HostOperationFailures { operation, code })
}

/// Every type the host serves is a value of the registry's closed set of
/// operations, and the set names no other: a type the host serves is never
/// left out of what it reports, nor reported under a text of its own. Red if
/// a type is added to the host and not to the registry.
#[test]
fn every_type_the_host_serves_is_one_operation_of_the_registry() {
    for operation in TYPES {
        assert!(
            HostOperation::of(operation).is_some(),
            "{operation} is not in the registry"
        );
    }
    assert_eq!(HostOperation::ALL.len(), TYPES.len());
}

/// A start answered at once is measured once, under its type, and counts no
/// failure; a refusal counts as `invalid_argument`; an injected failure under
/// its own code, and one no operation names as `other`; a failure the
/// operation met under its code. Red if `ReferenceHost::report` reads the code
/// from anywhere but the answer, or measures what it does not answer.
#[test]
fn each_start_is_measured_and_each_failure_counted_by_its_code() {
    let (mut host, recording) = reported(Unreachable);
    let put = params(&[("key", "k"), ("value", "v")]);
    host.start("sce-host-kv-put", &Invocation::new("s", "put", 1), &put);
    let measured = recording.observed(Histogram::HostOperationDuration {
        operation: HostOperation::KvPut,
    });
    assert_eq!(measured.len(), 1);
    assert!(measured[0] >= 0.0);
    assert_eq!(
        recording.counted_in(&sce_rust_metrics::HOST_OPERATION_FAILURES),
        0,
        "a put that completed is no failure"
    );

    let refused = host.start(
        "sce-host-kv-get",
        &Invocation::new("s", "get", 1),
        &params(&[("key", "")]),
    );
    assert!(matches!(refused, Some(Outcome::Refuse(_))));
    assert_eq!(
        failures(
            &recording,
            HostOperation::KvGet,
            HostFailure::InvalidArgument
        ),
        1
    );

    host.set_faults(BTreeMap::from([
        ("sce-host-kv-get".to_string(), "unavailable".to_string()),
        (
            "sce-host-kv-delete".to_string(),
            "a code of its own".to_string(),
        ),
    ]));
    host.start(
        "sce-host-kv-get",
        &Invocation::new("s", "get", 2),
        &params(&[("key", "k")]),
    );
    host.start(
        "sce-host-kv-delete",
        &Invocation::new("s", "delete", 1),
        &params(&[("key", "k")]),
    );
    assert_eq!(
        failures(&recording, HostOperation::KvGet, HostFailure::Unavailable),
        1
    );
    assert_eq!(
        failures(&recording, HostOperation::KvDelete, HostFailure::Other),
        1
    );
    host.set_faults(BTreeMap::new());

    let fetch = params(&[
        ("method", "GET"),
        ("url", "https://example.com/"),
        ("headers", ""),
        ("body", ""),
    ]);
    host.start(
        "sce-host-http-fetch",
        &Invocation::new("s", "fetch", 1),
        &fetch,
    );
    assert_eq!(
        failures(&recording, HostOperation::HttpFetch, HostFailure::Network),
        1
    );
    assert_eq!(
        recording
            .calls()
            .iter()
            .filter(|call| matches!(call, Call::Observe(..)))
            .count(),
        5,
        "each start answered at once is measured once"
    );
}

/// A start the host holds is answered later, or never: it is not measured, and
/// counts no failure. A host given no metrics reports nothing and answers as
/// before. Red if `ReferenceHost::report` measures a held start.
#[test]
fn a_held_start_is_not_measured() {
    let (mut host, recording) = reported(Holding);
    let fetch = params(&[
        ("method", "GET"),
        ("url", "https://example.com/"),
        ("headers", ""),
        ("body", ""),
    ]);
    let held = host.start(
        "sce-host-http-fetch",
        &Invocation::new("s", "fetch", 1),
        &fetch,
    );
    assert_eq!(held, Some(Outcome::Hold));
    assert!(recording.calls().is_empty(), "{:?}", recording.calls());

    let mut unreported = ReferenceHost::new(Box::new(ScriptedClock::new()), Unreachable);
    assert!(matches!(
        unreported.start(
            "sce-host-kv-put",
            &Invocation::new("s", "put", 1),
            &params(&[("key", "k"), ("value", "v")]),
        ),
        Some(Outcome::Done(_))
    ));
}
