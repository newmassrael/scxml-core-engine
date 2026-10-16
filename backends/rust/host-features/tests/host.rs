// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What the reference host holds apart from the scenarios the lanes share
// (docs/SCE_HOST_FEATURES.md): its system clock, what it refuses, and what it does
// not serve.

use sce_rust_host_features::{
    HttpRequest, HttpResponse, Outcome, Params, ReferenceHost, ScriptedClock, SystemClock,
    Transport, TransportFailure,
};

struct Unreachable;

impl Transport for Unreachable {
    fn exchange(&mut self, _: &HttpRequest) -> Result<Option<HttpResponse>, TransportFailure> {
        Err(TransportFailure)
    }
}

fn done(outcome: Option<Outcome>) -> serde_json::Value {
    match outcome.expect("a standard type is served") {
        Outcome::Done(data) => serde_json::from_str(&data).expect("a result is JSON"),
        other => panic!("the host did not complete: {other:?}"),
    }
}

/// The system clock's monotonic reading never goes below one already answered.
#[test]
fn the_system_clock_answers_a_monotonic_reading_that_does_not_decrease() {
    let mut host = ReferenceHost::new(Box::new(SystemClock::new()), Unreachable);
    let mut last = -1i64;
    for _ in 0..50 {
        let fields = done(host.start("sce-host-clock-now", "now", &Params::new()));
        assert_eq!(fields["error"], "");
        let mono: i64 = fields["monotonic_ms"]
            .as_str()
            .expect("digits")
            .parse()
            .expect("a number");
        assert!(
            mono >= last,
            "the monotonic reading went from {last} to {mono}"
        );
        last = mono;
    }
    #[cfg(unix)]
    assert_eq!(
        done(host.start("sce-host-clock-zone", "zone", &Params::new()))["error"],
        ""
    );
}

#[test]
fn an_operation_outside_the_standard_set_is_not_served() {
    let mut host = ReferenceHost::new(Box::new(ScriptedClock::new()), Unreachable);
    assert!(host.start("x-app-host", "id", &Params::new()).is_none());
}

#[test]
fn a_key_with_a_nul_is_refused() {
    let mut host = ReferenceHost::new(Box::new(ScriptedClock::new()), Unreachable);
    let params: Params = [("key".to_string(), "a\0b".to_string())].into();
    assert!(matches!(
        host.start("sce-host-kv-get", "id", &params),
        Some(Outcome::Refuse(_))
    ));
}

/// Keys are ordered by their UTF-8 bytes.
#[test]
fn keys_are_ordered_by_their_utf8_bytes() {
    let mut host = ReferenceHost::new(Box::new(ScriptedClock::new()), Unreachable);
    for key in ["\u{FF5E}", "\u{1F600}"] {
        let params: Params = [
            ("key".to_string(), key.to_string()),
            ("value".to_string(), "v".to_string()),
        ]
        .into();
        host.start("sce-host-kv-put", "put", &params);
    }
    let walk: Params = [
        ("after".to_string(), String::new()),
        ("prefix".to_string(), String::new()),
    ]
    .into();
    assert_eq!(
        done(host.start("sce-host-kv-next-key", "walk", &walk))["key"],
        "\u{FF5E}"
    );
}
