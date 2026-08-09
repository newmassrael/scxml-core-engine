// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Protocol-Synthesis RFC §synth-5-O — sourcemap JSON emit.
//
// Each emitted backend artifact writes a companion `sce_sourcemap.json`
// in its output directory. The schema (spec lines 3219-3243):
//
//   {
//     "version": 1,
//     "source_hash":   "<hex sha256 — byte-equal to §synth-6.2.6 header>",
//     "template_hash": "<hex sha256 — byte-equal to §synth-6.2.6 header>",
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
// The file is BYTE-IDENTICAL across all 6 backends —
// the symbol table is BTreeMap-sorted so iteration order is
// deterministic, and `source_hash` delegates to
// `forge::drift::compute_source_hash` so the value is provably equal
// to the §synth-6.2.6 header. The runtime-level reverse-lookup
// (`sce-codegen addr2sce`) keys off this JSON.

#[cfg(test)]
use crate::forge::error::SourceLocation;
use crate::forge::symbol_mangling::SymbolEntry;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Current sourcemap schema version. Bumped only on breaking shape
/// changes per the same policy that governs the diagnostic schema.
/// Additive field growth (new optional fields under `symbols.*`) does
/// NOT bump this constant.
pub const SOURCEMAP_VERSION: u32 = 1;

/// Schema lifecycle marker — *not* a wire field. Mirrors the precedent
/// set by the diagnostic schema (`SCHEMA_STATUS`) and the forge-AST
/// schema (`FORGE_AST_SCHEMA_STATUS`): the schema-about-the-schema
/// lives in the schema file header
/// (`schemas/sce-sourcemap.v1.schema.json`, `x-sce-schema-status`),
/// never in the emitted `sce_sourcemap.json` payload — so the 404
/// committed sidecars stay byte-stable and the status signal is read
/// from the checked-in schema, not by linking this crate. Flipping
/// `pre-release` → `stable` requires updating this constant AND the
/// schema file header in one commit; the `schema_file_declares_status`
/// test below guards the two-way sync. See `SCE_WIRE_CONTRACTS.md` for
/// the shared stability + deprecation policy across SCE wire surfaces.
pub const SOURCEMAP_SCHEMA_STATUS: &str = "pre-release";

/// Top-level sourcemap document. Serialised verbatim into
/// `out/{language}/sce_sourcemap.json`.
///
/// `source_hash` + `template_hash` are hex-encoded sha256 strings
/// matching the §synth-6.2.6 header values for the same artifact. Reused
/// from `forge::drift::DriftHashes::source_hex()` /
/// `template_hex()` so a hash drift surfaces immediately.
/// `Deserialize` alongside `Serialize` because the sourcemap is read
/// back, not only written: both lookup directions (`addr2sce`,
/// `sce2sym`) load this file. Reading it as an untyped
/// `serde_json::Value` — which is what the CLI did before these
/// derives existed — means every consumer re-states the field names by
/// hand and a shape change breaks them silently at runtime instead of
/// at compile time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sourcemap {
    pub version: u32,
    pub source_hash: String,
    pub template_hash: String,
    /// BTreeMap → deterministic JSON key order across runs +
    /// platforms. Per the §synth-5-O byte-identity requirement: any
    /// HashMap-style insertion order would surface as a backend-
    /// dependent diff.
    pub symbols: BTreeMap<String, SourceSymbol>,
}

/// One row of the symbol table. Field order matches the spec example
/// (lines 3219-3243) so a `serde_json::to_string_pretty` matches the
/// hand-authored shape. Optional fields are skipped on `None` so the
/// JSON stays tight where the runtime metadata is absent.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// sourcemap emit never populates it (no profiler consumer yet;
    /// the field still ships because addr2sce wants the read-path
    /// live).
    /// Set to `None` at emit time; downstream tooling can rewrite the
    /// JSON to inject WCET data without a schema bump.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wcet_us: Option<u32>,
}

/// Build a [`Sourcemap`] from a symbol-table + hash pair. The hash
/// values come from the caller's `DriftContext` so the sourcemap and
/// the §synth-6.2.6 header share a single source of truth — see
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
        let event = entry.event.clone();
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
                // WCET field reserved for a future profiler
                // consumer; omitted from the wire output until a
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

/// Load a sourcemap from the JSON text of an `sce_sourcemap.json`.
///
/// The single read path for both lookup directions. Typed rather than
/// `serde_json::Value` so a shape change is a compile error at every
/// consumer instead of a `None` at runtime.
pub fn from_json(text: &str) -> Result<Sourcemap, serde_json::Error> {
    serde_json::from_str(text)
}

/// Which symbols a reverse lookup is asking for.
///
/// Every field is an independent narrowing predicate and `None` means
/// "do not constrain this axis". An all-`None` query therefore matches
/// the whole table, which is the useful default for "what did this
/// document lower to" — a reverse lookup with no way to ask a broad
/// question would force the caller to already know the answer.
///
/// The forward direction (`addr2sce`) needs no such struct: a mangled
/// symbol is a map key, so it is one exact-match lookup. The asymmetry
/// is real — one SCXML coordinate legitimately lowers to several
/// symbols (a state's body, its entry block, each of its transitions),
/// and across backends to several files.
#[derive(Debug, Default, Clone)]
pub struct SymbolQuery<'a> {
    /// Canonical state-hierarchy path, matched exactly (e.g. `s1/s1p1`).
    pub state_path: Option<&'a str>,
    /// 1-based source line that must fall inside the symbol's
    /// `line_range`, inclusive at both ends.
    pub line: Option<u32>,
    /// IR node kind, matched exactly (`state`, `transition`, …).
    pub kind: Option<&'a str>,
    /// Event name, matched exactly. Symbols carrying no event never
    /// match a constrained query.
    pub event: Option<&'a str>,
    /// Author-facing SCXML path, matched exactly. Useful when one
    /// sourcemap covers a machine assembled from several documents.
    pub file: Option<&'a str>,
}

impl SymbolQuery<'_> {
    /// Whether this query constrains nothing — every symbol matches.
    pub fn is_unconstrained(&self) -> bool {
        self.state_path.is_none()
            && self.line.is_none()
            && self.kind.is_none()
            && self.event.is_none()
            && self.file.is_none()
    }

    /// Whether `symbol` satisfies every constrained axis.
    fn matches(&self, symbol: &SourceSymbol) -> bool {
        if let Some(want) = self.state_path {
            if symbol.scxml_state_path != want {
                return false;
            }
        }
        if let Some(want) = self.kind {
            if symbol.kind != want {
                return false;
            }
        }
        if let Some(want) = self.file {
            if symbol.scxml_file != want {
                return false;
            }
        }
        if let Some(want) = self.event {
            // A symbol with no event cannot satisfy an event
            // constraint — matching it would report a state body as an
            // answer to "which symbol handles event X".
            match symbol.event.as_deref() {
                Some(have) if have == want => {}
                _ => return false,
            }
        }
        if let Some(line) = self.line {
            let [start, end] = symbol.line_range;
            if line < start || line > end {
                return false;
            }
        }
        true
    }
}

/// Every symbol in `map` matching `query`, in the table's own order.
///
/// `Sourcemap::symbols` is a `BTreeMap`, so the result is sorted by
/// mangled symbol name and stable across runs and platforms — a
/// reverse lookup that reordered its hits between invocations could not
/// be diffed in a build log.
pub fn find_symbols<'a>(
    map: &'a Sourcemap,
    query: &SymbolQuery<'_>,
) -> Vec<(&'a str, &'a SourceSymbol)> {
    map.symbols
        .iter()
        .filter(|(_, symbol)| query.matches(symbol))
        .map(|(name, symbol)| (name.as_str(), symbol))
        .collect()
}

/// Schema version of a symbol-lookup record.
pub const SYMBOL_LOOKUP_SCHEMA_VERSION: u32 = 1;

/// Stability status of the symbol-lookup wire surface. Pinned to the
/// `x-sce-schema-status` header of
/// `schemas/sce-symbol-lookup.v1.schema.json`.
pub const SYMBOL_LOOKUP_SCHEMA_STATUS: &str = "pre-release";

/// Which direction produced a lookup record.
///
/// The two directions answer opposite questions over the same table
/// and share one record shape, so a consumer parses one schema and
/// branches on this field rather than maintaining two readers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LookupKind {
    /// Mangled symbol (or PC) resolved to SCXML coordinates.
    Addr2Sce,
    /// SCXML coordinates resolved to the symbols they lowered to.
    Sce2Sym,
}

impl LookupKind {
    pub fn as_str(self) -> &'static str {
        match self {
            LookupKind::Addr2Sce => "addr2sce",
            LookupKind::Sce2Sym => "sce2sym",
        }
    }
}

/// Every lookup kind, checked against the schema's `kind.enum`.
pub const ALL_LOOKUP_KINDS: &[LookupKind] = &[LookupKind::Addr2Sce, LookupKind::Sce2Sym];

/// One resolved symbol on the wire.
///
/// Field order is the wire order and puts `v` first, matching the
/// diagnostic and manifest surfaces. The pre-existing `addr2sce`
/// emitter built this object through `serde_json::json!`, whose
/// alphabetical key order put `v` last — the same record, spelled in a
/// way no other SCE surface spells it.
#[derive(Debug, Serialize)]
pub struct SymbolLookupRecord<'a> {
    pub v: u32,
    pub kind: &'static str,
    /// Path of the `sce_sourcemap.json` this hit came from. A reverse
    /// lookup may span several backends' sidecars in one invocation,
    /// so the record names its own source rather than leaving the
    /// caller to correlate by position.
    pub sourcemap: String,
    /// Mangled symbol name.
    pub symbol: &'a str,
    pub entry: &'a SourceSymbol,
}

impl SymbolLookupRecord<'_> {
    /// Serialise to the single line that goes on stdout.
    pub fn to_line(&self) -> String {
        serde_json::to_string(self).expect("SymbolLookupRecord serialises")
    }
}

/// Render the sourcemap to a JSON string. Pretty-printed for human
/// inspection; `serde_json::to_string` (compact) would still work but
/// debugging an addr2sce miss is faster against the indented form.
/// Per the §synth-5-O byte-identity requirement the indent must be deterministic across platforms —
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

/// Walks `out_dir` recursively and verifies that every SCE-emitted
/// file (identified by a parseable §synth-6.2.6 drift header — see
/// `ARCHITECTURE.md` "Traceability Ownership Boundary") contains at
/// least one `SCE-MAP:` marker line. Returns on the first violation
/// so the diagnostic surfaces a single concrete file rather than a
/// batch report.
///
/// Files without a drift header are silently skipped per the
/// boundary contract: external meta-generator output (protoc,
/// bindgen, cbindgen) and hand-authored sources sit outside SCE's
/// traceability ownership.
///
/// The walker fires
/// [`ValidationError::TraceabilityMetaGeneratedSourceLineMarkerMissing`]
/// when a drift-headered file lacks the marker — that combination
/// indicates a codegen-internal invariant violation (a template
/// regressed, lost its `SCE-MAP:` macro call) rather than an author
/// repair surface, so the diagnostic is informational-only with no
/// `Fix`.
pub fn validate_emitted_files_have_markers(
    out_dir: &std::path::Path,
) -> Result<(), crate::forge::error::Located<crate::forge::error::ForgeError>> {
    use crate::forge::drift::parse_embedded_hashes;
    use std::collections::BTreeSet;

    fn walk(dir: &std::path::Path, out: &mut BTreeSet<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if matches!(ext, "rs" | "cpp" | "h" | "kt" | "go" | "py" | "c") {
                    out.insert(path);
                }
            }
        }
    }

    let mut files = BTreeSet::new();
    walk(out_dir, &mut files);

    for file in &files {
        let Ok(content) = std::fs::read_to_string(file) else {
            continue;
        };
        if parse_embedded_hashes(&content).is_none() {
            // ARCHITECTURE.md "Traceability Ownership Boundary":
            // external meta-generator output is out-of-scope by
            // design — skip silently.
            continue;
        }
        if !content.contains("SCE-MAP:") {
            let file_str = file.display().to_string();
            return Err(crate::forge::error::Located::new(
                crate::forge::error::ValidationError::TraceabilityMetaGeneratedSourceLineMarkerMissing {
                    file: file_str.clone(),
                }
                .into(),
                &file_str,
                None,
                None,
            ));
        }
    }
    Ok(())
}

/// Construct a [`SourceSymbol`] from a raw [`SourceLocation`] +
/// metadata. Convenience for tests and ad-hoc callers; production
/// `build` walks the symbol table directly.
#[cfg(test)]
pub(crate) fn synth_symbol(loc: &SourceLocation, state_path: &str, kind: &str) -> SourceSymbol {
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
        // byte-equal. This is the foundation for the §synth-5-O cross-
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

    /// Drift guard between [`SOURCEMAP_SCHEMA_STATUS`] (the Rust source
    /// of truth) and the `x-sce-schema-status` field in
    /// `schemas/sce-sourcemap.v1.schema.json` (the downstream-visible
    /// declaration). Both must agree and stay in the closed value set;
    /// otherwise a consumer reading the schema file would see a
    /// stability claim that diverges from the crate. Mirrors the
    /// diagnostic schema's `schema_file_declares_status` test. See
    /// `SCE_WIRE_CONTRACTS.md` for the transition criterion.
    #[test]
    fn schema_file_declares_status() {
        let schema_bytes = include_str!("../../../schemas/sce-sourcemap.v1.schema.json");
        let parsed: serde_json::Value =
            serde_json::from_str(schema_bytes).expect("schema file must be valid JSON");
        let declared = parsed
            .get("x-sce-schema-status")
            .and_then(|v| v.as_str())
            .expect(
                "schema must declare x-sce-schema-status at top level — \
                 see SCE_WIRE_CONTRACTS.md",
            );
        assert!(
            matches!(declared, "pre-release" | "stable"),
            "x-sce-schema-status must be 'pre-release' or 'stable'; got {declared:?}",
        );
        assert_eq!(
            declared, SOURCEMAP_SCHEMA_STATUS,
            "schema file's x-sce-schema-status drifted from \
             SOURCEMAP_SCHEMA_STATUS const — update one to match the \
             other in the same commit (see SCE_WIRE_CONTRACTS.md)",
        );
    }

    const LOOKUP_SCHEMA_BYTES: &str =
        include_str!("../../../schemas/sce-symbol-lookup.v1.schema.json");

    fn lookup_schema() -> serde_json::Value {
        serde_json::from_str(LOOKUP_SCHEMA_BYTES).expect("lookup schema is valid JSON")
    }

    /// Same producer-const ↔ schema-header lockstep the sourcemap
    /// surface has, for the lookup-record surface it feeds.
    #[test]
    fn symbol_lookup_schema_file_declares_status() {
        let declared = lookup_schema()["x-sce-schema-status"]
            .as_str()
            .expect("lookup schema declares x-sce-schema-status")
            .to_string();
        assert!(
            matches!(declared.as_str(), "pre-release" | "stable"),
            "x-sce-schema-status must be 'pre-release' or 'stable'; got {declared:?}",
        );
        assert_eq!(
            declared, SYMBOL_LOOKUP_SCHEMA_STATUS,
            "sce-symbol-lookup.v1.schema.json x-sce-schema-status drifted from \
             SYMBOL_LOOKUP_SCHEMA_STATUS",
        );
    }

    #[test]
    fn symbol_lookup_schema_declares_matching_version() {
        let declared = lookup_schema()["properties"]["v"]["const"]
            .as_u64()
            .expect("lookup schema pins properties.v.const");
        assert_eq!(
            declared as u32, SYMBOL_LOOKUP_SCHEMA_VERSION,
            "lookup schema v.const drifted from SYMBOL_LOOKUP_SCHEMA_VERSION",
        );
    }

    /// A third lookup direction must reach the schema in the same
    /// commit that adds it, or every record it emits is invalid on a
    /// consumer's validator while the producer tests stay green.
    #[test]
    fn symbol_lookup_schema_kind_enum_matches_rust_source_of_truth() {
        let mut declared: Vec<String> = lookup_schema()["properties"]["kind"]["enum"]
            .as_array()
            .expect("kind.enum is an array")
            .iter()
            .map(|v| v.as_str().expect("kind is a string").to_string())
            .collect();
        declared.sort();
        let mut actual: Vec<String> = ALL_LOOKUP_KINDS
            .iter()
            .map(|k| k.as_str().to_string())
            .collect();
        actual.sort();
        assert_eq!(
            declared, actual,
            "lookup schema kind.enum drifted from ALL_LOOKUP_KINDS",
        );
    }

    /// The `entry` sub-object must accept the very shape the sourcemap
    /// emits — the two schemas describe the same row and would
    /// otherwise be free to disagree about which fields are required.
    #[test]
    fn symbol_lookup_entry_required_fields_match_the_sourcemap_schema() {
        let sourcemap_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../schemas/sce-sourcemap.v1.schema.json"
        ))
        .expect("sourcemap schema is valid JSON");
        let mut from_sourcemap: Vec<String> = sourcemap_schema["definitions"]["sourceSymbol"]
            ["required"]
            .as_array()
            .expect("sourceSymbol.required is an array")
            .iter()
            .map(|v| v.as_str().expect("field name").to_string())
            .collect();
        from_sourcemap.sort();
        let mut from_lookup: Vec<String> = lookup_schema()["properties"]["entry"]["required"]
            .as_array()
            .expect("entry.required is an array")
            .iter()
            .map(|v| v.as_str().expect("field name").to_string())
            .collect();
        from_lookup.sort();
        assert_eq!(
            from_lookup, from_sourcemap,
            "the lookup record's `entry` and the sourcemap's `sourceSymbol` \
             disagree about required fields; they describe the same row",
        );
    }

    /// The schema file's declared `version` const must equal the
    /// producer-side [`SOURCEMAP_VERSION`]. A drift here means an
    /// external validator would reject payloads the producer emits.
    #[test]
    fn schema_file_declares_matching_version() {
        let schema_bytes = include_str!("../../../schemas/sce-sourcemap.v1.schema.json");
        let parsed: serde_json::Value =
            serde_json::from_str(schema_bytes).expect("schema file must be valid JSON");
        let declared = parsed
            .pointer("/properties/version/const")
            .and_then(|v| v.as_u64())
            .expect("schema must pin properties.version.const");
        assert_eq!(
            declared as u32, SOURCEMAP_VERSION,
            "schema file's version const drifted from SOURCEMAP_VERSION",
        );
    }

    #[test]
    fn synth_symbol_helper_smoke() {
        let loc = SourceLocation {
            file: "f.scxml".into(),
            line: Some(7),
            col: None,
        };
        let s = synth_symbol(&loc, "s1", "state");
        assert_eq!(s.scxml_file, "f.scxml");
        assert_eq!(s.line_range, [7, 7]);
        assert_eq!(s.kind, "state");
    }
}
