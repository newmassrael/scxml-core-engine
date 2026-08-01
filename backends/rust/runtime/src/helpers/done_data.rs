// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! §scxml-5.5 / 5.7: Donedata processing helpers.
//!
//! Ports the `<content>` literal arm of `sce/include/common/DoneDataHelper.h`.
//! Param evaluation and the JSON assembly around it are lowered by the Rust
//! AOT template directly against `helpers::event_data`, so this module carries
//! only what the emitted code calls.
//!
//! SCE Protocol-Synthesis RFC §synth-5-J-2 (lines 1989-1994): under
//! `--features=no_std`, [`emit_content_literal`] stays available — the Rust AOT
//! template `entry_exit_actions.rs.jinja2` emits a call to it unconditionally
//! for `<content>literal</content>` donedata — and returns [`crate::SceString`]
//! so it composes with the capped `EventMetadata.data` field.

use crate::{sce_string_from_str, SceString};

/// §scxml-5.5: Emit an inline `<content>` literal as `_event.data`.
///
/// 1:1 port of C++ `SCE::DoneDataHelper::emitContentLiteral`
/// (`sce/include/common/DoneDataHelper.h`). When `<content>` has no `expr`
/// attribute the spec says "the children are used as the content value" —
/// no evaluation happens and no script engine is required. The literal
/// text **is** the value.
///
/// This is the SSoT consumed by the `literal` branch of the Rust AOT
/// codegen (`tools/codegen/templates/rust/entry_exit_actions.rs.jinja2`),
/// matching the C++ `emitContentLiteral` / Go `EmitContentLiteral` /
/// Kotlin `emitContentLiteral` helpers so all four backends share one
/// semantic definition.
///
/// # Arguments
///
/// * `literal` - Inline text content from `<content>literal</content>`
///
/// # Returns
///
/// The literal as `_event.data` (raw string — no JSON quoting). Returned as
/// [`SceString`] so the no_std variant (capped `heapless::String`) composes
/// with `EventMetadata.data`.
pub fn emit_content_literal(literal: &str) -> SceString {
    sce_string_from_str(literal)
}
