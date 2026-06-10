// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
//! And an integration test in `sce-rust-tests/tests/test_332.rs`:
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
    engine.initialize();
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
/// W3C test server (`standalone_http_server.js` on localhost:8080/test).
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

        // W3C SCXML C.2: Send real HTTP POST to shared test server
        let response = client
            .post(&request.target)
            .header("Content-Type", content_type)
            .body(body)
            .send()
            .ok()?;
        let text = response.text().ok()?;

        // W3C SCXML C.2: Parse JSON response for event name and data
        let json: serde_json::Value = serde_json::from_str(&text).ok()?;
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
    });
}

/// W3C BasicHTTP test server endpoint baked into codegen for
/// `conf:basicHTTPAccessURITarget=""` (see `tools/codegen/templates/rust/...` +
/// every `tests/generated/test*/test*_sm.rs` BasicHTTP fixture). Kept here
/// so the reachability assertion below uses the same string the SM emits.
const HTTP_TEST_SERVER_URL: &str = "http://localhost:8080/test";

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
        if probe.get(HTTP_TEST_SERVER_URL).send().is_err() {
            panic!(
                "\nW3C BasicHTTP test server is not reachable at {url}.\n\
                 \n\
                 Start it before running BasicHTTP-dependent tests:\n\
                 \n\
                   node tests/w3c/standalone_http_server.js 8080 /test\n\
                 \n\
                 Affected tests include: test_201, test_207, test_208,\n\
                 test_509, test_510, test_513, test_518, test_519, test_520,\n\
                 test_522, test_531, test_532, test_534, test_567 (every\n\
                 fixture whose SM emits `<send type=\"...#BasicHTTPEventProcessor\">`).\n\
                 Without the server these silently route to their\n\
                 `<transition event=\"*\"/>` fail branch.\n",
                url = HTTP_TEST_SERVER_URL,
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
