// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The MCU footprint instrument: one `Engine<P>`, actually driven.
//!
//! Nothing here is a test of behaviour. Its whole job is to make the engine's
//! generic code EXIST in an object file for an MCU target, so `nm` can weigh
//! it and a gate can hold it to a budget.
//!
//! A generic function costs nothing until something instantiates it. The
//! sibling `nostd-build` probe defines a `StatePolicy` and stops there, so
//! `Engine<ThatPolicy>` is never monomorphised and its rlib contains zero
//! engine symbols — measured 2026-08-21, which is how a consumer's firmware
//! came to be the only witness in existence for a per-instantiation
//! regression this repository shipped twice.
//!
//! The entry points below are `#[no_mangle] extern "C"` for two reasons: they
//! cannot be dead-code-eliminated out of the rlib, and their names survive
//! into `nm` output unmangled so a gate can find them without matching a
//! Rust symbol hash that changes with the compiler.
//!
//! ⚠ The engine methods named here are the ones a host actually calls, and the
//! set is load-bearing: a method left out of this file is a method whose
//! footprint no gate can see. When `Engine` grows a driving entry point, it
//! belongs here too.

#![no_std]
#![deny(warnings)]

use sce_nostd_build_probe::{ParallelHistoryProbeEvent, ParallelHistoryProbePolicy};
use sce_rust_runtime::Engine;

/// A staticlib for a bare target has to name what happens on panic. Halting is
/// the honest answer for something that is never run — this crate is compiled
/// and weighed, never executed — and `panic = "abort"` in the profile keeps
/// the unwinder out of the bytes being measured.
#[panic_handler]
fn on_panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

/// `Engine::step` — the synchronous driving call, and the one whose inlined
/// body carries `run_main_event_loop`.
///
/// Returns the engine's running flag so the call cannot be folded away as
/// having no observable result.
#[no_mangle]
pub extern "C" fn sce_footprint_step() -> bool {
    let mut engine = Engine::new(ParallelHistoryProbePolicy::new());
    engine.initialize();
    engine.step();
    engine.is_running()
}

/// `Engine::process_event` — the external-event path, which reaches the main
/// loop by a different route than `step` and is the one a consumer driving
/// from an interrupt handler uses.
#[no_mangle]
pub extern "C" fn sce_footprint_process_event() -> bool {
    let mut engine = Engine::new(ParallelHistoryProbePolicy::new());
    engine.initialize();
    engine.process_event(ParallelHistoryProbeEvent::E1);
    engine.is_running()
}

/// The diagnostic counters an MCU host reads, kept in the instrument because
/// reading them is what keeps the fields — and the code that maintains them —
/// alive in the image. A consumer that never reads them pays for them anyway;
/// that is the question the budget exists to make answerable.
///
/// Absent under `no-macrostep-diagnostics`, because the accessor itself is:
/// compiling this out is how the two configurations are weighed against each
/// other, and an entry point that still called a removed accessor would just
/// be a build error dressed as a measurement.
#[cfg(not(feature = "no-macrostep-diagnostics"))]
#[no_mangle]
pub extern "C" fn sce_footprint_diagnostics() -> u32 {
    let mut engine = Engine::new(ParallelHistoryProbePolicy::new());
    engine.initialize();
    engine.step();
    engine.truncated_macrosteps()
}
