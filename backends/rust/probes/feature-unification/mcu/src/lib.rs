// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The bare-metal half of the feature-unification probe (docs/adr/0053).
//!
//! Compiled twice by the gate: in the host `cargo check --workspace`, where
//! the application crate beside it turns the runtime's `std` on, and alone
//! for `thumbv7em-none-eabihf`, where the runtime is its bare-metal profile.
//! Everything here is written against names the bare-metal profile declares,
//! so both compile; a name that only one profile declared would fail one of
//! the two.

#![no_std]
#![deny(warnings)]

use sce_rust_runtime::{sce_string_from_str, BoundedPush, SceString, SceTransitionBuf};

/// The generated `--no-std` machine, so both builds compile it.
pub use sce_portable_emit_probe as machine;

/// An event string built through the runtime's profile alias: `String` in the
/// unified host build, `heapless::String` on the MCU target.
pub fn event_text(text: &str) -> SceString {
    sce_string_from_str(text)
}

/// The runtime's microstep buffer, filled through the push both profiles
/// implement.
pub fn first_two(items: &[u8]) -> SceTransitionBuf<u8> {
    let mut buffer: SceTransitionBuf<u8> = SceTransitionBuf::new();
    for item in items.iter().take(2) {
        buffer.push_bounded(*item);
    }
    buffer
}

/// The crate's own fixed-capacity list, filled through the runtime's
/// `BoundedPush`. Its `heapless::Vec` implementation is declared in both
/// profiles, so this resolves in the unified host build as well.
pub fn first_of(items: &[u8]) -> heapless::Vec<u8, 1> {
    let mut list: heapless::Vec<u8, 1> = heapless::Vec::new();
    if let Some(item) = items.first() {
        list.push_bounded(*item);
    }
    list
}
