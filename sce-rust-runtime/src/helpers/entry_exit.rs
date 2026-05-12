// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! W3C SCXML 3.8/3.9: Entry/exit action block execution with error isolation.
//!
//! 1:1 port of `sce/include/core/EntryExitHelper.h`. Provides block-based
//! execution of `<onentry>` and `<onexit>` handlers with W3C-mandated error
//! isolation: if one block fails, remaining blocks still execute.
//!
//! In the C++ code, blocks are `std::vector<std::function<void()>>` and
//! error isolation is provided by lambda scope (return stops one block only).
//! In Rust, we use `&mut [Box<dyn FnMut()>]` or closures for the same effect.
//!
//! Watching-zenoh RFC §5.J.2 (lines 1989-1994): the `Box<dyn FnMut()>` block
//! variants ([`execute_entry_blocks`] / [`execute_exit_blocks`]) require
//! `alloc` and are gated to `!no_std`. The pure-closure variants
//! ([`execute_entry_closures`] / [`execute_exit_closures`]) take
//! `&mut [&mut dyn FnMut()]` slices (no allocation) and stay available under
//! `--features=no_std`. The generated AOT code under no_std prefers the
//! closure form per the same lock-in.

use crate::sce_log_debug;

/// W3C SCXML 3.8: Execute onentry action blocks with error isolation.
///
/// Each block corresponds to one `<onentry>` element. If an error occurs in
/// one block (it raises error.execution and returns), subsequent blocks still
/// execute per W3C SCXML 3.8.
///
/// Ports C++ `EntryExitHelper<StatePolicy, Engine>::executeEntryBlocks`.
///
/// Watching-zenoh RFC §5.J.2: gated to `!no_std` because `Box<dyn FnMut()>`
/// requires `alloc`. The closure-slice variant
/// [`execute_entry_closures`] is the no_std-compatible alternative.
#[cfg(not(feature = "no_std"))]
pub fn execute_entry_blocks(blocks: &mut [Box<dyn FnMut()>], state_id: &str) {
    let count = blocks.len();

    if !state_id.is_empty() {
        sce_log_debug!(
            "W3C SCXML 3.8: Executing {} onentry blocks for state: {}",
            count,
            state_id
        );
    } else {
        sce_log_debug!("W3C SCXML 3.8: Executing {} onentry blocks", count);
    }

    for (i, block) in blocks.iter_mut().enumerate() {
        sce_log_debug!(
            "W3C SCXML 3.8: Executing onentry block {}/{}",
            i + 1,
            count
        );
        // W3C SCXML 3.8: Each block runs independently; error in one
        // does not prevent execution of subsequent blocks.
        block();
    }

    if !state_id.is_empty() {
        sce_log_debug!(
            "W3C SCXML 3.8: Completed onentry blocks for state: {}",
            state_id
        );
    }
}

/// W3C SCXML 3.9: Execute onexit action blocks with error isolation.
///
/// Same semantics as [`execute_entry_blocks`] but for `<onexit>` handlers.
///
/// Ports C++ `EntryExitHelper<StatePolicy, Engine>::executeExitBlocks`.
///
/// Watching-zenoh RFC §5.J.2: gated to `!no_std` because `Box<dyn FnMut()>`
/// requires `alloc`. The closure-slice variant
/// [`execute_exit_closures`] is the no_std-compatible alternative.
#[cfg(not(feature = "no_std"))]
pub fn execute_exit_blocks(blocks: &mut [Box<dyn FnMut()>], state_id: &str) {
    let count = blocks.len();

    if !state_id.is_empty() {
        sce_log_debug!(
            "W3C SCXML 3.9: Executing {} onexit blocks for state: {}",
            count,
            state_id
        );
    } else {
        sce_log_debug!("W3C SCXML 3.9: Executing {} onexit blocks", count);
    }

    for (i, block) in blocks.iter_mut().enumerate() {
        sce_log_debug!(
            "W3C SCXML 3.9: Executing onexit block {}/{}",
            i + 1,
            count
        );
        block();
    }

    if !state_id.is_empty() {
        sce_log_debug!(
            "W3C SCXML 3.9: Completed onexit blocks for state: {}",
            state_id
        );
    }
}

/// Convenience variant: execute entry blocks from closures (no allocation needed).
///
/// Accepts an iterator of closures instead of boxed trait objects.
pub fn execute_entry_closures(closures: &mut [&mut dyn FnMut()], state_id: &str) {
    let count = closures.len();

    if !state_id.is_empty() {
        sce_log_debug!(
            "W3C SCXML 3.8: Executing {} onentry closures for state: {}",
            count,
            state_id
        );
    }

    for (i, closure) in closures.iter_mut().enumerate() {
        sce_log_debug!(
            "W3C SCXML 3.8: Executing onentry closure {}/{}",
            i + 1,
            count
        );
        closure();
    }
}

/// Convenience variant: execute exit blocks from closures.
pub fn execute_exit_closures(closures: &mut [&mut dyn FnMut()], state_id: &str) {
    let count = closures.len();

    if !state_id.is_empty() {
        sce_log_debug!(
            "W3C SCXML 3.9: Executing {} onexit closures for state: {}",
            count,
            state_id
        );
    }

    for (i, closure) in closures.iter_mut().enumerate() {
        sce_log_debug!(
            "W3C SCXML 3.9: Executing onexit closure {}/{}",
            i + 1,
            count
        );
        closure();
    }
}
