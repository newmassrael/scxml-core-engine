// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Build-time diagnostics emitted by the parser and downstream passes.
//
// Kept out of `model.rs` on purpose: `SCXMLModel` is a rendering/codegen
// domain model that survives into every template context, while
// diagnostics are parse-pass artifacts with a different lifecycle and
// audience. Co-locating them here gives the CLI and build.rs a single
// place to look for notice types.

use std::fmt;

// ── SCXML parse-time deprecations ─────────────────────────────

/// A build-time deprecation notice emitted by the SCXML parser when it
/// encounters a syntactic construct that has been removed by the spec but
/// is still tolerated for a migration window. Kept structured — never
/// `eprintln!` — so consumers can filter, group, or promote uniformly.
///
/// Scope: SCXML document attributes (e.g. `sce:qos` on `<send>`).
/// deploy.yaml configuration errors are typed directly on
/// `mesh::error::ExternalConfigError` and do not pass through here.
#[derive(Debug, Clone)]
pub struct DeprecationWarning {
    /// The attribute name, qualified with its namespace prefix
    /// (for example `sce:qos`). Never empty.
    pub attribute: String,
    /// The SCXML event carrying the deprecated attribute, when known.
    /// `None` if the attribute was not attached to an event-bearing element.
    pub event: Option<String>,
    /// Human-readable guidance pointing at the spec section that removed
    /// the construct, so readers can reach the replacement quickly.
    pub reason: String,
}

impl fmt::Display for DeprecationWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.event {
            Some(ev) => write!(
                f,
                "deprecated attribute {} on <send event=\"{}\">: {}",
                self.attribute, ev, self.reason
            ),
            None => write!(
                f,
                "deprecated attribute {}: {}",
                self.attribute, self.reason
            ),
        }
    }
}
