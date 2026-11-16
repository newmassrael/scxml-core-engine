// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! W3C SCXML conformance test harness.
//!
//! Rust port of the C++ `SimpleAotTest<Derived, Num>` + `AotTestRegistrar<T>`
//! pattern documented in `CLAUDE.md` → "Adding W3C Tests". Uses the `linkme`
//! crate for compile-time test registration (Rust analog of C++ static
//! initializers).
//!
//! ## Usage from generated code
//!
//! Each generated test emits a struct + registrar:
//!
//! ```ignore
//! pub struct Test332;
//! impl SimpleAotTest for Test332 {
//!     type Policy = Test332Policy;
//!     const TEST_NUM: u32 = 332;
//!     const DESCRIPTION: &'static str = "W3C SCXML 5.10.1: event.type (332 AOT)";
//!     const EXPECTED_PASS: Test332State = Test332State::Pass;
//!     fn create_policy() -> Test332Policy { Test332Policy::new() }
//! }
//!
//! #[linkme::distributed_slice(crate::harness::AOT_TESTS)]
//! static REGISTRAR_TEST332: fn() -> TestResult = || run_simple_aot_test::<Test332>();
//! ```
//!
//! And an integration test in `backends/rust/tests/tests/test_332.rs`:
//!
//! ```ignore
//! #[test]
//! fn test_332() {
//!     use sce_rust_tests::{generated::test332::Test332, harness::{run_simple_aot_test, TestResult}};
//!     assert!(matches!(run_simple_aot_test::<Test332>(), TestResult::Pass));
//! }
//! ```

use std::fmt::Debug;
use std::time::Duration;

use sce_rust_runtime::{Engine, StatePolicy};

/// Result of running a W3C conformance test.
#[derive(Debug, Clone)]
pub enum TestResult {
    /// Final state matches expected pass state.
    Pass,
    /// Final state reached but does not match expected state.
    Fail {
        /// State the test expected as the pass state.
        expected: String,
        /// State the engine actually reached.
        actual: String,
    },
    /// Timed out before reaching any final state.
    Timeout,
}

/// The trait each W3C conformance test struct implements.
///
/// Ports C++ `SimpleAotTest<Derived, Num>` CRTP base class. The `const`
/// parameters bake the test number and description into the type; generated
/// code fills them from `tests/CMakeLists.txt` metadata.
pub trait SimpleAotTest: 'static {
    /// The generated policy type for this W3C test case.
    type Policy: StatePolicy;

    /// W3C test number (e.g., 332).
    const TEST_NUM: u32;

    /// Human-readable description (from `tests/CMakeLists.txt` comment).
    const DESCRIPTION: &'static str;

    /// The state the test expects the SM to reach for a pass.
    ///
    /// Typically `<Policy>::State::Pass`, but some tests have custom pass states.
    const EXPECTED_PASS: <Self::Policy as StatePolicy>::State;

    /// Construct a fresh policy instance for this test run.
    fn create_policy() -> Self::Policy;
}

/// Run a W3C SimpleAotTest: construct engine, initialize, poll to completion, assert final state.
///
/// Timeout is 3 seconds (matches C++ `SimpleAotTest::run()` default), poll
/// interval is 10ms. Matches C++ `SimpleAotTest::run()` signature and behavior.
pub fn run_simple_aot_test<T: SimpleAotTest>() -> TestResult
where
    <T::Policy as StatePolicy>::State: Debug,
{
    let policy = T::create_policy();
    let mut engine = Engine::<T::Policy>::new(policy);
    engine
        .initialize()
        .expect("the machine starts: a W3C test is run over a script session that is established");
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    if !completed {
        return TestResult::Timeout;
    }

    let actual = engine.get_current_state();
    let expected = T::EXPECTED_PASS;
    if actual == expected {
        TestResult::Pass
    } else {
        TestResult::Fail {
            expected: format!("{:?}", expected),
            actual: format!("{:?}", actual),
        }
    }
}

/// Distributed slice for compile-time test registration (C++ AotTestRegistrar equivalent).
///
/// Each generated test appends itself via
/// `#[linkme::distributed_slice(AOT_TESTS)] static REG: fn() -> TestResult = ...;`.
/// The slice is walked by the end-to-end `run_all_tests` helper.
#[linkme::distributed_slice]
pub static AOT_TESTS: [fn() -> TestResult] = [..];

/// Count of registered tests. Useful for diagnostics.
pub fn registered_test_count() -> usize {
    AOT_TESTS.len()
}

/// W3C SCXML C.2: Configure an engine for real HTTP tests against the shared
/// W3C test server (`standalone_http_server.js` on the port
/// [`http_test_endpoint_port`] reads).
///
/// The callback sends a real HTTP POST via `reqwest`, parses the JSON response,
/// and returns `HttpSendResponse` so the engine injects the echoed event.
///
/// On first call this also verifies the test server is reachable. A missing
/// server is an environment problem, not a code defect — without the check
/// every BasicHTTP-dependent W3C test (test_201, test_207, test_208, test_509,
/// test_510, test_513, test_518, test_519, test_520, test_522, test_531,
/// test_532, test_534, test_567, ...) silently routes to its `<transition
/// event="*"/>` fail branch with a state-mismatch assertion that does not
/// hint at the real cause. The fast-fail panic below tells the developer
/// exactly which process to start before re-running the test.
pub fn setup_http_test<P: StatePolicy>(engine: &mut Engine<P>) {
    assert_http_test_server_reachable();

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .expect("failed to create HTTP client");

    engine.set_http_send_callback(move |request| {
        // W3C SCXML C.2: Build form-encoded POST body
        let mut body_parts = Vec::new();
        if !request.event_name.is_empty() {
            body_parts.push(format!(
                "_scxmleventname={}",
                urlencoding(&request.event_name)
            ));
        }
        for (key, values) in &request.params {
            if key == "_scxmleventname" && !request.event_name.is_empty() {
                continue;
            }
            for value in values {
                body_parts.push(format!("{}={}", urlencoding(key), urlencoding(value)));
            }
        }

        let (body, content_type) = if !body_parts.is_empty() {
            (body_parts.join("&"), "application/x-www-form-urlencoded")
        } else if !request.content.is_empty() {
            (request.content.clone(), "text/plain")
        } else {
            (String::new(), "application/x-www-form-urlencoded")
        };

        // W3C SCXML C.2: Send real HTTP POST to shared test server. A POST
        // that does not reach the server, or that it refuses with a status
        // outside 2xx, is answered as not delivered, which the engine reports
        // as error.communication (W3C SCXML 6.2.4): it used to be answered as
        // a delivery with no reply, and the document then waited for an
        // answer that was never coming.
        let response = client
            .post(&request.target)
            .header("Content-Type", content_type)
            .body(body)
            .send()
            .map_err(|err| {
                sce_rust_runtime::HttpSendFailure::new(format!(
                    "the POST to {} was not delivered: {err}",
                    request.target
                ))
            })?;
        let status = response.status();
        if !status.is_success() {
            return Err(sce_rust_runtime::HttpSendFailure::new(format!(
                "the server answered HTTP {status}"
            )));
        }
        // Delivered from here on: a reply that cannot be read names no event.
        Ok(response.text().ok().and_then(|text| echoed_event(&text)))
    });
}

/// W3C SCXML C.2: the event a reply of the W3C test server names, with its
/// data, or `None` when the reply names none.
fn echoed_event(text: &str) -> Option<sce_rust_runtime::HttpSendResponse> {
    let json: serde_json::Value = serde_json::from_str(text).ok()?;
    let event_name = json.get("event")?.as_str()?.to_string();
    let event_data = json
        .get("data")
        .map(|d| {
            if d.is_string() {
                d.as_str().unwrap_or("").to_string()
            } else {
                d.to_string()
            }
        })
        .unwrap_or_default();
    Some(sce_rust_runtime::HttpSendResponse {
        event_name,
        event_data,
    })
}

/// §scxml-C-2-3: the fixture path, PINNED to
/// `tests/w3c/basic_http_test_endpoint.h`.
///
/// This file is not only compiled in this repository — `sce-build` embeds it
/// and writes it into the standalone suites it emits, where there is no
/// repository above it. So it cannot read the owning header, at compile time or
/// at run time: an earlier cut used `include_str!` and the emitted suite failed
/// with "couldn't read ../../../../tests/w3c/basic_http_test_endpoint.h".
///
/// A shipped copy is therefore structurally forced, and the answer to a forced
/// copy is not to let it drift: `scripts/gates/http-endpoint-ssot.sh` reads
/// both this value and the owner's and REFUSES a tree where they disagree.
/// One author, a mirror that cannot rot. The port has no such copy: the
/// endpoint has no default port.
const DEFAULT_ENDPOINT_PATH: &str = "/test";

/// §scxml-C-2-3: the port the out-of-process fixture listener bound, which this
/// harness addresses: the one the file `SCE_W3C_HTTP_PORT_FILE` names holds, or
/// the one `SCE_W3C_HTTP_PORT` names, as `basic_http_test_endpoint.h` reads it.
///
/// There is no default. Whoever starts the listener (a gate, the `verify`
/// action) starts it on port 0 and hands on the port its `[READY] port=<n>`
/// line names; a port assumed here would be one another program on the host
/// may hold, which made every run need that one port free.
pub fn http_test_endpoint_port() -> u16 {
    if let Ok(file) = std::env::var("SCE_W3C_HTTP_PORT_FILE") {
        if !file.is_empty() {
            let text = std::fs::read_to_string(&file).unwrap_or_else(|err| {
                panic!(
                    "SCE_W3C_HTTP_PORT_FILE names {file}, which cannot be read ({err}): the \
                     out-of-process W3C BasicHTTP listener did not write the port it bound there"
                )
            });
            return parse_endpoint_port(text.trim_end_matches('\n'), &file);
        }
    }
    match std::env::var("SCE_W3C_HTTP_PORT") {
        Ok(raw) if !raw.is_empty() => parse_endpoint_port(&raw, "SCE_W3C_HTTP_PORT"),
        _ => panic!(
            "neither SCE_W3C_HTTP_PORT_FILE nor SCE_W3C_HTTP_PORT says where the W3C BasicHTTP \
             fixture listener answers. Start it with `node tests/w3c/standalone_http_server.js 0` \
             and set SCE_W3C_HTTP_PORT to the port its `[READY] port=<n>` line names. It has no \
             default port: one assumed is one another program on this host may hold."
        ),
    }
}

/// `raw` read as a TCP port, or a panic naming `source`, where it came from.
fn parse_endpoint_port(raw: &str, source: &str) -> u16 {
    match raw.parse::<u16>() {
        Ok(port) if port != 0 => port,
        _ => panic!("{source} holds \"{raw}\", which is not a TCP port"),
    }
}

/// The path the fixture listener answers on.
pub fn http_test_endpoint_path() -> &'static str {
    DEFAULT_ENDPOINT_PATH
}

/// W3C BasicHTTP test server endpoint, referenced by codegen for
/// `conf:basicHTTPAccessURITarget=""` (see `tools/codegen/templates/rust/...` +
/// every `tests/generated/test*/test*_sm.rs` BasicHTTP fixture), so the
/// reachability assertion below uses the same address the SM publishes.
///
/// §scxml-C-2-3: where the harness's inbound BasicHTTP listener answers, and
/// therefore the address the generated tests declare as the machine's
/// published 'location'. Bind address and published address are one fact — a
/// document that posts somewhere the listener never claimed would fail for a
/// reason unrelated to what it tests.
///
/// It is a function rather than the `const` it replaced because the endpoint is
/// a property of the RUN, not of the source: its port is the one the listener
/// of this run bound ([`http_test_endpoint_port`]).
pub fn http_test_server_url() -> String {
    format!(
        "http://localhost:{}{}",
        http_test_endpoint_port(),
        http_test_endpoint_path()
    )
}

/// Verify the W3C BasicHTTP test server is reachable. Panics with a
/// developer-actionable message on first call if the socket connect fails.
///
/// Process-scoped via `OnceLock` so the check fires at most once per
/// `cargo test` binary regardless of how many BasicHTTP tests run.
fn assert_http_test_server_reachable() {
    use std::sync::OnceLock;

    static CHECKED: OnceLock<()> = OnceLock::new();
    CHECKED.get_or_init(|| {
        let probe = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(1))
            .build()
            .expect("HTTP probe client build");

        // Any non-network error (e.g. 404/405) still means the socket is
        // accepting connections, which is all we need to assert here — the
        // server's handler shape is exercised by the test itself.
        let url = http_test_server_url();
        if probe.get(&url).send().is_err() {
            panic!(
                "\nW3C BasicHTTP test server is not reachable at {url}.\n\
                 \n\
                 Start it before running BasicHTTP-dependent tests, and point\n\
                 SCE_W3C_HTTP_PORT at the port its `[READY] port=<n>` line names:\n\
                 \n\
                   node tests/w3c/standalone_http_server.js 0 {path}\n\
                 \n\
                 Affected tests include: test_201, test_207, test_208,\n\
                 test_509, test_510, test_513, test_518, test_519, test_520,\n\
                 test_522, test_531, test_532, test_534, test_567 (every\n\
                 fixture whose SM emits `<send type=\"...#BasicHTTPEventProcessor\">`).\n\
                 Without the server these silently route to their\n\
                 `<transition event=\"*\"/>` fail branch.\n",
                url = url,
                path = http_test_endpoint_path(),
            );
        }
    });
}

/// Minimal URL encoding for W3C SCXML C.2 form parameters.
fn urlencoding(s: &str) -> String {
    let mut encoded = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(b as char);
            }
            b' ' => encoded.push('+'),
            _ => {
                encoded.push('%');
                encoded.push_str(&format!("{:02X}", b));
            }
        }
    }
    encoded
}

/// Run every registered AOT test sequentially. Returns `(passed, failed, timed_out)`.
///
/// Generated tests populate `AOT_TESTS`; with none registered this returns `(0, 0, 0)`.
pub fn run_all_tests() -> (usize, usize, usize) {
    let mut passed = 0;
    let mut failed = 0;
    let mut timed_out = 0;
    for test_fn in AOT_TESTS {
        match test_fn() {
            TestResult::Pass => passed += 1,
            TestResult::Fail { .. } => failed += 1,
            TestResult::Timeout => timed_out += 1,
        }
    }
    (passed, failed, timed_out)
}
