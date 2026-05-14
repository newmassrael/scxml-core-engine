// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Watching-zenoh RFC §5.O Atomic 1 — sourcemap JSON foundation.
//
// Each emitted backend artifact writes a companion `sce_sourcemap.json`
// in its output directory. The schema (spec lines 3219-3243):
//
//   {
//     "version": 1,
//     "source_hash":   "<hex sha256 — byte-equal to §6.2.6 header>",
//     "template_hash": "<hex sha256 — byte-equal to §6.2.6 header>",
//     "symbols": {
//       "<mangled-symbol>": {
//         "scxml_file":       "<author-path>",
//         "scxml_state_path": "<canonical hierarchy path>",
//         "scxml_xpath":      "<XPath into the source SCXML>",
//         "line_range":       [<start>, <end>],
//         "kind":             "state|transition|on_entry|on_exit|...",
//         "event":            "<event name>",         // optional
//         "wcet_us":          <integer>               // optional
//       }
//     }
//   }
//
// Per Q-§5.O-8 lock the file is BYTE-IDENTICAL across all 6 backends —
// the symbol table is BTreeMap-sorted so iteration order is
// deterministic, and `source_hash` delegates to
// `forge::drift::compute_source_hash` so the value is provably equal
// to the §6.2.6 header. The runtime-level reverse-lookup
// (`sce-codegen addr2sce`) keys off this JSON.

#[cfg(test)]
use crate::forge::error::SourceLocation;
use crate::forge::symbol_mangling::SymbolEntry;
use serde::Serialize;
use std::collections::BTreeMap;

/// Current sourcemap schema version. Bumped only on breaking shape
/// changes per the same policy that governs the diagnostic schema.
/// Additive field growth (new optional fields under `symbols.*`) does
/// NOT bump this constant.
pub const SOURCEMAP_VERSION: u32 = 1;

/// Top-level sourcemap document. Serialised verbatim into
/// `out/{language}/sce_sourcemap.json`.
///
/// `source_hash` + `template_hash` are hex-encoded sha256 strings
/// matching the §6.2.6 header values for the same artifact. Reused
/// from `forge::drift::DriftHashes::source_hex()` /
/// `template_hex()` so a hash drift surfaces immediately.
#[derive(Debug, Clone, Serialize)]
pub struct Sourcemap {
    pub version: u32,
    pub source_hash: String,
    pub template_hash: String,
    /// BTreeMap → deterministic JSON key order across runs +
    /// platforms. Per Q-§5.O-8 byte-identity requirement: any
    /// HashMap-style insertion order would surface as a backend-
    /// dependent diff.
    pub symbols: BTreeMap<String, SourceSymbol>,
}

/// One row of the symbol table. Field order matches the spec example
/// (lines 3219-3243) so a `serde_json::to_string_pretty` matches the
/// hand-authored shape. Optional fields are skipped on `None` so the
/// JSON stays tight where the runtime metadata is absent.
#[derive(Debug, Clone, Serialize)]
pub struct SourceSymbol {
    /// Author-facing path to the SCXML file the symbol traces back to.
    /// Always populated — the symbol was constructed from a
    /// `SourceLocation::file` value at parser time.
    pub scxml_file: String,
    /// Canonical state-hierarchy path (e.g. `s1/s1p1`). Empty for the
    /// per-machine `_machine` symbol and for forge-kind body symbols
    /// (the body sits at the document root, not a state).
    pub scxml_state_path: String,
    /// Best-effort XPath that round-trips to the originating XML node.
    /// Spec lines 3219-3243 require a string; this implementation
    /// synthesises an `//state[@id='<id>']`-style approximation from
    /// the state path so consumers always get a non-empty value.
    pub scxml_xpath: String,
    /// `[start, end]` 1-based line range. `end` is the same as `start`
    /// when the source spans a single line (the parser-side
    /// `SourceLocation` records only one line per node).
    pub line_range: [u32; 2],
    /// IR node kind: `"state"`, `"transition"`, `"on_entry"`,
    /// `"on_exit"`, `"forge_body"`, or `"machine"`.
    pub kind: String,
    /// Event name for transition / `<raise>` / `<send>` symbols.
    /// Absent for state / on_entry / on_exit / machine / forge_body.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
    /// Worst-case execution-time hint in microseconds (per spec line
    /// 3232 `wcet_us`). Reserved for future profiler-fed values; the
    /// foundation Atomic 1 emit never populates it (no profiler
    /// consumer yet — per [[feedback-silently-broken-hooks]] we add
    /// the field only when a producer exists, but the foundation IS
    /// the producer here because addr2sce wants the read-path live).
    /// Set to `None` at emit time; downstream tooling can rewrite the
    /// JSON to inject WCET data without a schema bump.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wcet_us: Option<u32>,
}

/// Build a [`Sourcemap`] from a symbol-table + hash pair. The hash
/// values come from the caller's `DriftContext` so the sourcemap and
/// the §6.2.6 header share a single source of truth — see
/// `traceability/sourcemap-source-hash-mismatch` for the drift-check
/// pre-emit guard that consumes both.
pub fn build(
    symbols: &BTreeMap<String, SymbolEntry>,
    source_hash_hex: String,
    template_hash_hex: String,
) -> Sourcemap {
    let mut out: BTreeMap<String, SourceSymbol> = BTreeMap::new();
    for (mangled, entry) in symbols {
        let kind = classify_kind(&entry.artifact);
        let event = extract_event(&entry.artifact);
        let line = entry.location.line.unwrap_or(0);
        let xpath = synth_xpath(&entry.state_path, &entry.artifact);
        out.insert(
            mangled.clone(),
            SourceSymbol {
                scxml_file: entry.location.file.clone(),
                scxml_state_path: entry.state_path.clone(),
                scxml_xpath: xpath,
                line_range: [line, line],
                kind: kind.to_string(),
                event,
                // Foundation emit: WCET field reserved for future
                // profiler consumer; per [[feedback-silently-broken-
                // hooks]] we omit it from the wire output until a
                // producer materialises. The field is present on the
                // struct so consumers can read it without a schema
                // bump.
                wcet_us: None,
            },
        );
    }
    Sourcemap {
        version: SOURCEMAP_VERSION,
        source_hash: source_hash_hex,
        template_hash: template_hash_hex,
        symbols: out,
    }
}

/// Map an artifact suffix produced by `symbol_mangling::build_symbol_
/// table` to a spec-named kind string. The artifact convention is
/// internal to SCE; the kind string is part of the wire contract.
fn classify_kind(artifact: &str) -> &'static str {
    if artifact == "_machine" {
        "machine"
    } else if artifact == "_state_body" {
        "state"
    } else if artifact == "_forge_body" {
        "forge_body"
    } else if artifact.starts_with("_transition_") {
        "transition"
    } else if artifact.starts_with("_on_entry") {
        "on_entry"
    } else if artifact.starts_with("_on_exit") {
        "on_exit"
    } else {
        // Defensive fallback for future artifact families. Surfacing
        // "unknown" here rather than panicking keeps the JSON
        // emission resilient to additive artifact additions; a
        // sweep over `symbol_mangling.rs` updates classify_kind
        // alongside the new artifact pattern.
        "unknown"
    }
}

/// Extract the `event=` payload from a transition artifact. Foundation
/// emit returns `None` because the artifact convention does not
/// currently fold the event name in. Future enrichment can extend the
/// artifact format with `_transition_<idx>_event_<name>` (escape-rule
/// driven) without breaking the rest of the pipeline.
fn extract_event(_artifact: &str) -> Option<String> {
    None
}

/// Synthesise an XPath approximation from the state-path + artifact.
/// `//state[@id='s1']` for a state body; `//state[@id='s1']/transition[1]`
/// for a transition; `//scxml` for the machine-level symbol.
fn synth_xpath(state_path: &str, artifact: &str) -> String {
    if artifact == "_machine" {
        return "//scxml".to_string();
    }
    if state_path.is_empty() {
        // Forge-doc body — no state hierarchy. The author file is the
        // entire scxml root.
        return "//scxml".to_string();
    }
    let last_segment = state_path.rsplit('/').next().unwrap_or(state_path);
    let base = format!("//state[@id='{}']", last_segment);
    if artifact.starts_with("_transition_") {
        // Extract the index.
        if let Some(idx_str) = artifact.strip_prefix("_transition_") {
            if let Ok(idx) = idx_str.parse::<usize>() {
                return format!("{}/transition[{}]", base, idx + 1);
            }
        }
        format!("{}/transition", base)
    } else if artifact.starts_with("_on_entry") {
        format!("{}/onentry", base)
    } else if artifact.starts_with("_on_exit") {
        format!("{}/onexit", base)
    } else {
        base
    }
}

/// Render the sourcemap to a JSON string. Pretty-printed for human
/// inspection; `serde_json::to_string` (compact) would still work but
/// debugging an addr2sce miss is faster against the indented form.
/// Per Q-§5.O-8 the indent must be deterministic across platforms —
/// `serde_json::to_string_pretty` uses 2-space indent which is
/// platform-independent.
pub fn to_json(map: &Sourcemap) -> Result<String, serde_json::Error> {
    let mut s = serde_json::to_string_pretty(map)?;
    // serde_json::to_string_pretty does not emit a trailing newline;
    // append one so the file ends with `\n` (matches existing
    // generated-file convention).
    s.push('\n');
    Ok(s)
}

/// Build the source-hash mismatch diagnostic when the sourcemap's
/// `source_hash` field differs from a freshly-computed header hash.
/// Public helper so the codegen write-site can perform the byte-
/// equality check before serialising.
pub fn check_source_hash_matches(
    sourcemap: &Sourcemap,
    expected_header_hash_hex: &str,
    file: &str,
) -> Result<(), crate::forge::error::Located<crate::forge::error::ForgeError>> {
    if sourcemap.source_hash != expected_header_hash_hex {
        return Err(crate::forge::error::Located::new(
            crate::forge::error::ValidationError::TraceabilitySourcemapSourceHashMismatch {
                file: file.to_string(),
                sourcemap_hash: sourcemap.source_hash.clone(),
                header_hash: expected_header_hash_hex.to_string(),
            }
            .into(),
            file,
            None,
            None,
        ));
    }
    Ok(())
}

/// Construct a [`SourceSymbol`] from a raw [`SourceLocation`] +
/// metadata. Convenience for tests and ad-hoc callers; production
/// `build` walks the symbol table directly.
#[cfg(test)]
pub(crate) fn synth_symbol(
    loc: &SourceLocation,
    state_path: &str,
    kind: &str,
) -> SourceSymbol {
    SourceSymbol {
        scxml_file: loc.file.clone(),
        scxml_state_path: state_path.to_string(),
        scxml_xpath: synth_xpath(state_path, "_state_body"),
        line_range: [loc.line.unwrap_or(0), loc.line.unwrap_or(0)],
        kind: kind.to_string(),
        event: None,
        wcet_us: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::symbol_mangling::{build_symbol_table, mangle};

    #[test]
    fn empty_sourcemap_serialises() {
        let map = Sourcemap {
            version: SOURCEMAP_VERSION,
            source_hash: "abc".into(),
            template_hash: "def".into(),
            symbols: BTreeMap::new(),
        };
        let s = to_json(&map).unwrap();
        assert!(s.contains("\"version\": 1"));
        assert!(s.contains("\"source_hash\": \"abc\""));
        assert!(s.contains("\"template_hash\": \"def\""));
        assert!(s.ends_with('\n'));
    }

    #[test]
    fn classify_kind_covers_artifact_families() {
        assert_eq!(classify_kind("_machine"), "machine");
        assert_eq!(classify_kind("_state_body"), "state");
        assert_eq!(classify_kind("_forge_body"), "forge_body");
        assert_eq!(classify_kind("_transition_0"), "transition");
        assert_eq!(classify_kind("_on_entry_0_0"), "on_entry");
        assert_eq!(classify_kind("_on_exit_1_2"), "on_exit");
        assert_eq!(classify_kind("_bogus_future"), "unknown");
    }

    #[test]
    fn synth_xpath_state_and_transition() {
        assert_eq!(synth_xpath("", "_machine"), "//scxml");
        assert_eq!(synth_xpath("s1", "_state_body"), "//state[@id='s1']");
        assert_eq!(
            synth_xpath("s1", "_transition_0"),
            "//state[@id='s1']/transition[1]"
        );
        assert_eq!(
            synth_xpath("s1", "_on_entry_0_0"),
            "//state[@id='s1']/onentry"
        );
    }

    #[test]
    fn build_round_trip_produces_byte_stable_json() {
        let scxml = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" name="m" version="1.0"
       initial="s1" datamodel="ecmascript">
  <state id="s1">
    <transition event="go" target="s2"/>
  </state>
  <final id="s2"/>
</scxml>"#;
        let mut parser = crate::parser::SCXMLParser::new();
        let model = parser.parse_string(scxml, "fixture.scxml").expect("parses");
        let symbols = build_symbol_table(&model, &[]).expect("no collisions");
        let map = build(&symbols, "deadbeef".into(), "feedface".into());
        let json1 = to_json(&map).expect("serialise");
        let json2 = to_json(&map).expect("serialise");
        // Determinism: two emissions of the same sourcemap must be
        // byte-equal. This is the foundation for Q-§5.O-8 cross-
        // backend identity (which the integration test pins).
        assert_eq!(json1, json2);
        // Schema sanity: contains the keys spec lines 3219-3243 name.
        assert!(json1.contains("\"version\": 1"));
        assert!(json1.contains("\"source_hash\": \"deadbeef\""));
        // Mangled symbol from the symbol-mangling helper appears in
        // the rendered map so the wire shape is sane.
        let m1_machine = mangle(&model.name, "", "_machine");
        assert!(json1.contains(&m1_machine));
    }

    #[test]
    fn check_source_hash_matches_rejects_drift() {
        let map = Sourcemap {
            version: SOURCEMAP_VERSION,
            source_hash: "0000".into(),
            template_hash: "1111".into(),
            symbols: BTreeMap::new(),
        };
        let res = check_source_hash_matches(&map, "ffff", "out/rust/sce_sourcemap.json");
        assert!(res.is_err());
        // Same hash → no error.
        let ok = check_source_hash_matches(&map, "0000", "out/rust/sce_sourcemap.json");
        assert!(ok.is_ok());
    }

    #[test]
    fn synth_symbol_helper_smoke() {
        let loc = SourceLocation { file: "f.scxml".into(), line: Some(7), col: None };
        let s = synth_symbol(&loc, "s1", "state");
        assert_eq!(s.scxml_file, "f.scxml");
        assert_eq!(s.line_range, [7, 7]);
        assert_eq!(s.kind, "state");
    }
}
