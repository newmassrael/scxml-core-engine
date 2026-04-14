// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Build-time diagnostics emitted by the parser and downstream passes.
//
// Kept out of `model.rs` on purpose: `SCXMLModel` is a rendering/codegen
// domain model that survives into every template context, while
// diagnostics are parse-pass artifacts with a different lifecycle and
// audience. Co-locating them here also gives CLI, build.rs, and future
// Stage 2 enforcement a single place to look for notice types.

use std::fmt;

// ── SCXML parse-time deprecations ─────────────────────────────

/// A build-time deprecation notice emitted by the SCXML parser when it
/// encounters a syntactic construct that has been removed by the spec but
/// is still tolerated for a migration window (Stage 1: warn + ignore;
/// Stage 2: promote to hard error). Kept structured — never `eprintln!` —
/// so consumers can filter, group, or promote uniformly.
///
/// Scope: SCXML document attributes (e.g. `sce:qos` on `<send>`). For
/// deploy.yaml deprecations see `DeployDeprecationWarning`.
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

// ── deploy.yaml parse-time deprecations ──────────────────────

/// A build-time deprecation notice emitted by the deploy.yaml pipeline
/// when it encounters a field that the spec has superseded but that
/// remains tolerated during Stage 1 migration.
///
/// Kept separate from `DeprecationWarning` because the payload differs:
/// SCXML deprecations carry (attribute, event name), whereas deploy.yaml
/// deprecations carry (field name, YAML path, transport context). Fusing
/// them forced field reuse that did not match semantics (the `event`
/// field would hold a YAML location, etc.).
#[derive(Debug, Clone)]
pub struct DeployDeprecationWarning {
    /// The deprecated field as it appears in deploy.yaml (e.g.
    /// `"service_id"`). Never empty; never namespaced.
    pub field: String,
    /// Hierarchical location inside deploy.yaml — formatted like
    /// `topology.<device>.machines.<machine>.bindings[<target>]` so the
    /// reader can find the occurrence without additional context.
    pub location: String,
    /// Human-readable migration guidance pointing at the replacement
    /// (spec section + replacement field name).
    pub reason: String,
}

impl fmt::Display for DeployDeprecationWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "deploy.yaml: deprecated field `{field}` at {loc}: {reason}",
            field = self.field,
            loc = self.location,
            reason = self.reason,
        )
    }
}
