// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The runtime of the test profile (docs/SCE_TEST_PROFILE.md §12.2): the round
//! scheduler, `NOW`, port queues and snapshots, timer bindings, verdicts, the
//! entity table and the guard, written once for every suite.
//!
//! A suite's generated units reach it from here: a test component's machine
//! through [`ComponentContext`], which it keeps from [`ComponentMachine::start`]
//! on; a configuration as a [`Configuration`] and a [`MachineFactory`]; a test
//! case and a suite as [`TestCase`] and [`Suite`], which [`run_suite`] runs. The
//! algorithm is the one the document specifies and the C++ runtime
//! (`sce/include/test/`) implements, type for type.
//!
//! The TTCN-3 length of a text, which a template's length restriction holds a
//! `string[N]` to (§6.1, §11.4), is its count of characters, `str::chars().count()`
//! in Rust, so it needs no function here.
//!
//! A run's test log (§12.5) is its [`TestCaseResult::records`], written in its
//! one canonical text by [`TestCaseResult::canonical_log`]; a message's value
//! reaches it in the canonical form [`canonical`] spells, which the generated
//! unit of a template over a record writes (`<template>::canonical`).
//!
//! A run's inputs may be a recorded trace (docs/SCE_TTCN3_LANE.md §10), the
//! wire surface `schemas/sce-test-trace.v1.schema.json`: a [`Trace`], played
//! by a [`TraceAdapter`], each value read back from its canonical form by
//! [`from_canonical`] through the reader of the test case's configuration
//! ([`TestCase::inputs`]). [`run_suite_with_traces`] runs a suite so, and
//! [`runner`] is the program of a generated suite that does.
//!
//! ⚠ `std` only: a run holds its components' values, queues and log on the heap.

#![cfg(feature = "std")]

pub mod canonical;
pub mod from_canonical;
mod module_parameters;
mod port;
mod run;
pub mod runner;
mod suite;
mod test_log;
mod timer;
mod trace;
mod verdict;

pub use module_parameters::{ModuleParameterValue, ModuleParameters};
pub use port::{message_value, Duration, Instant, Message, MessageValue, Port, TestValue};
pub use run::{
    receives, ComponentContext, ComponentMachine, Configuration, Endpoint, LogKind, LogRecord,
    MachineFactory, SystemAdapter, TestCaseResult, TestCaseRun, MTC_NAME,
};
pub use suite::{
    enabled, run_suite, run_suite_with_traces, CaseResult, Execute, Features, OverrideRefusal,
    Selection, Suite, TestCase,
};
pub use test_log::{TestLogEvent, TestLogRecord, TEST_LOG_SCHEMA, TEST_LOG_SCHEMA_STATUS};
pub use timer::{Timer, TimerStatus};
pub use trace::{
    no_inputs, InputReader, SystemOutput, SystemOutputs, Trace, TraceAdapter, TraceInput,
    TraceRefusal, TEST_TRACE_SCHEMA, TEST_TRACE_SCHEMA_STATUS,
};
pub use verdict::{worse, Verdict};
