// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Which backends lower which position of a composed value (docs/adr/0086,
//! section 7.3), asked of the one table that decides it
//! (`ComposedLowering::of`, `sce-build/src/forge/event_schema_check.rs`)
//! rather than listed by a suite.
//!
//! A suite that holds every backend to what it lowers of a composed value runs
//! `check -l <backend>` over a document and expects each backend to take it
//! where its row lowers the position and to refuse it by its own name where
//! its row does not. Naming those backends in the suite made every backend's
//! step edit the same lines of every such suite; asking the table makes a
//! step flip one row, and every suite follows. A suite's assertion is still
//! the backend's actual answer, so a row flipped without the backend's
//! lowering turns the suite red.
//!
//! The backends asked are those that lower `datamodel="sce-static"` whole
//! (`lowers_static_datamodel`): a backend that lowers it in part may refuse a
//! document for another construct before it reaches the composed value, so
//! its own suite asks it about each construct it lowers.

#![allow(dead_code)]

use sce_build::forge::event_schema_check::ComposedLowering;
use sce_build::generator::{lowers_static_datamodel, Language};

/// A position of a composed value the table answers for a backend: its
/// `reads`, `copies`, `unions`, `records` or `sends`.
pub type Position = fn(ComposedLowering) -> bool;

/// Each backend a `check -l <name>` judges a whole `sce-static` document for,
/// by the name the command line takes, with its row of the table.
pub fn checked_backends() -> Vec<(&'static str, ComposedLowering)> {
    Language::ALL
        .iter()
        .copied()
        .filter(|language| lowers_static_datamodel(*language))
        .map(|language| (language.canonical_name(), ComposedLowering::of(language)))
        .collect()
}

/// The names of the backends of [`checked_backends`] whose row lowers
/// `position`.
pub fn lowering(position: Position) -> Vec<&'static str> {
    checked_backends()
        .into_iter()
        .filter(|(_, row)| position(*row))
        .map(|(name, _)| name)
        .collect()
}

/// The names of the backends of [`checked_backends`] whose row does not lower
/// `position`, each of which refuses it by its own name.
pub fn refusing(position: Position) -> Vec<&'static str> {
    checked_backends()
        .into_iter()
        .filter(|(_, row)| !position(*row))
        .map(|(name, _)| name)
        .collect()
}

/// [`refusing`], for a suite that shows each backend that does not lower
/// `position` refusing it by its own name: once every backend lowers it, none
/// is left, and the suite `suite` says on the test's output that it is
/// skipped and why, rather than passing over nothing in silence.
pub fn refusing_or_skip(position: Position, suite: &str) -> Vec<&'static str> {
    let left = refusing(position);
    if left.is_empty() {
        eprintln!(
            "skipped: {suite}: every backend that lowers sce-static whole lowers this position \
             of a composed value, so no backend is left to refuse it (ComposedLowering::of)"
        );
    }
    left
}
