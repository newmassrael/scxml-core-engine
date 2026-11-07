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
//! ⚠ `std` only: a run holds its components' values, queues and log on the heap.

#![cfg(feature = "std")]

mod module_parameters;
mod port;
mod run;
mod suite;
mod timer;
mod verdict;

pub use module_parameters::{ModuleParameterValue, ModuleParameters};
pub use port::{Duration, Instant, Message, MessageValue, Port};
pub use run::{
    receives, ComponentContext, ComponentMachine, Configuration, Endpoint, LogKind, LogRecord,
    MachineFactory, SystemAdapter, TestCaseResult, TestCaseRun, MTC_NAME,
};
pub use suite::{
    enabled, run_suite, CaseResult, Execute, Features, OverrideRefusal, Selection, Suite, TestCase,
};
pub use timer::{Timer, TimerStatus};
pub use verdict::{worse, Verdict};
