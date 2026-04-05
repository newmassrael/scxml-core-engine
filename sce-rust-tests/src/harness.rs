// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
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
/// The slice is walked by an end-to-end "run all" helper (Phase 5 CI integration).
#[linkme::distributed_slice]
pub static AOT_TESTS: [fn() -> TestResult] = [..];

/// Count of registered tests. Useful for diagnostics.
pub fn registered_test_count() -> usize {
    AOT_TESTS.len()
}

/// Run every registered AOT test sequentially. Returns `(passed, failed, timed_out)`.
///
/// Phase 1: `AOT_TESTS` is empty, so this returns `(0, 0, 0)`. Phase 2+ populates it.
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
