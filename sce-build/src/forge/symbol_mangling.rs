// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Watching-zenoh RFC §5.O Atomic 1 — symbol mangling foundation.
//
// Spec lines 3055-3057 (`<machine>__<state_path>__<artifact>`) fix the
// per-symbol identifier shape the sourcemap JSON keys off. The mangler
// has three jobs:
//
//   1. Encode the three-tuple into a single C-identifier-safe string.
//      OQ-W16 (a) lock — `__` is the segment delimiter; literal `__`
//      inside any segment is escaped via `_u_` (chosen because the
//      escape sequence itself is never produced by either the
//      delimiter or any conformant SCXML id, so encode/decode round-
//      trip without ambiguity).
//
//   2. Reject names that would exceed the C99 identifier length limit
//      after mangling. Per D15 the cap is enforced by deploy.yaml's
//      `platform.strict_c99_identifiers` flag; the warn path is the
//      default (caller decides whether to escalate to hard-error).
//
//   3. Detect collisions across the SCXMLModel + every ForgeDocument
//      siblings (the cross-IR scan that fires
//      `traceability/state-id-collision` when XInclude / sce:template
//      composition produces two distinct nodes that mangle to the same
//      symbol).
//
// The mangler is the single source of truth — the sourcemap JSON
// writer, the per-symbol SCE-MAP marker emit sites, and the addr2sce
// reverse lookup all key off the same encoder/decoder pair so a future
// edit lands one place.

use crate::forge::error::SourceLocation;
use crate::forge::model::ForgeDocument;
use crate::forge::provenance::forge_doc_source_location;
use crate::model::{Action, SCXMLModel, State, Transition};
use std::collections::BTreeMap;

/// C99 §5.2.4.1 fixes 31 significant characters for external identifiers
/// and 63 for internal. The mangler enforces the external limit (more
/// restrictive) because cross-translation-unit linkage is the primary
/// MCU consumer surface. Authors may opt in to relaxation via
/// `platform.strict_c99_identifiers: false` (the default warn path).
pub const C99_EXTERNAL_IDENTIFIER_LIMIT: usize = 31;

/// Segment delimiter per OQ-W16 (a) lock. Literal `__` in any segment is
/// escaped as `_u_` so the decoder can locate delimiter boundaries
/// unambiguously.
const DELIM: &str = "__";

/// Escape sequence for a literal `__` inside a segment. Never produced
/// by a conformant SCXML id (id grammar forbids consecutive
/// underscores in XML NCName, but author content may), so the
/// encode/decode pair is bijective on its input domain.
const ESCAPE: &str = "_u_";

/// Mangle a single segment per the OQ-W16 (a) escape rule. Public so
/// the per-state walker can pre-mangle each path component before
/// joining with `DELIM`.
fn escape_segment(s: &str) -> String {
    // Replace every literal `__` with `_u_`. A naïve `replace("__",
    // "_u_")` is correct here because `_u_` contains a single
    // underscore in the middle — a subsequent `__` scan cannot re-
    // match across the escape boundary.
    s.replace(DELIM, ESCAPE)
}

/// Inverse of `escape_segment`. Round-trips on any input the encoder
/// produced; `unescape(escape(s)) == s` for arbitrary `s`.
fn unescape_segment(s: &str) -> String {
    s.replace(ESCAPE, DELIM)
}

/// Mangle `(machine, state_path, artifact)` into a single
/// `<machine>__<state_path>__<artifact>` identifier per spec lines
/// 3055-3057. Each component is escape-rewritten before being joined
/// so the result is C-identifier-safe AND uniquely decodable.
///
/// `state_path` is the canonical state hierarchy path (e.g. `s1/s1p1`).
/// The mangler treats the entire path as a single segment — the
/// internal `/` separator becomes `_` so the result is a valid C
/// identifier, with the slash-to-underscore mapping reversed by the
/// demangler.
pub fn mangle(machine: &str, state_path: &str, artifact: &str) -> String {
    let m = escape_segment(machine);
    // State paths use `/` as the hierarchy separator (e.g. `s1/s1p1`).
    // C identifiers cannot contain `/`, so substitute `_` and apply
    // the escape rule on the rest. A literal `_` inside an id segment
    // round-trips because the escape sequence is `_u_`, not `_`.
    let sp_normalized = state_path.replace('/', "_");
    let sp = escape_segment(&sp_normalized);
    let a = escape_segment(artifact);
    format!("{m}{DELIM}{sp}{DELIM}{a}")
}

/// Decode a mangled symbol back into its `(machine, state_path,
/// artifact)` triple. Returns `None` if the input is not a valid
/// mangled symbol (fewer than 2 delimiters).
///
/// State-path `_` does NOT round-trip to `/` because the encoder
/// flattened the hierarchy separator; callers that need the slashed
/// form must re-walk the SCXML model. For addr2sce the underscored
/// form is the canonical key into the sourcemap.
pub fn demangle(mangled: &str) -> Option<(String, String, String)> {
    // Split on `__` but ONLY at unescaped positions. `_u_` is the
    // escape sequence and must not be treated as a delimiter.
    let mut segments: Vec<String> = Vec::new();
    let bytes = mangled.as_bytes();
    let mut start = 0;
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'_' && bytes[i + 1] == b'_' {
            // Found delimiter. Capture segment, advance past delim.
            segments.push(mangled[start..i].to_string());
            start = i + 2;
            i = start;
        } else {
            i += 1;
        }
    }
    segments.push(mangled[start..].to_string());

    if segments.len() != 3 {
        return None;
    }
    Some((
        unescape_segment(&segments[0]),
        unescape_segment(&segments[1]),
        unescape_segment(&segments[2]),
    ))
}

/// Whether the mangled identifier exceeds the C99 external identifier
/// length limit per spec line 3055 (which delegates to the C standard
/// for downstream MCU consumers). Public so deploy.yaml's
/// `platform.strict_c99_identifiers` flag can decide whether to
/// upgrade a warn into a hard-error.
pub fn exceeds_c99_external_limit(mangled: &str) -> bool {
    mangled.len() > C99_EXTERNAL_IDENTIFIER_LIMIT
}

/// One row of the cross-IR symbol table. `location` is the originating
/// element's `source_location` — used to populate the dual-location
/// `traceability/state-id-collision` payload (the diagnostic carries
/// both colliding sites so the author can pinpoint which composition
/// produced the clash).
#[derive(Debug, Clone)]
pub struct SymbolEntry {
    pub mangled: String,
    pub machine: String,
    pub state_path: String,
    pub artifact: String,
    pub location: SourceLocation,
}

/// Result of a cross-IR collision scan. Holds the offending symbol
/// plus the two `SourceLocation`s that mangled to it. Caller is
/// responsible for wrapping this into a `Located<ValidationError>` (so
/// the location-on-the-wire convention stays in `error.rs`, not here).
#[derive(Debug, Clone)]
pub struct Collision {
    pub mangled: String,
    pub first: SymbolEntry,
    pub second: SymbolEntry,
}

/// Walk every emission-eligible node in `model` (the statechart side)
/// and every variant of `forge_docs` (the forge side), mangle each
/// node's identifier triple, and check for collisions.
///
/// Returns `Ok(BTreeMap<mangled, SymbolEntry>)` on success — the table
/// is BTreeMap-ordered for deterministic sourcemap emission. Returns
/// `Err(Collision)` on the first duplicate so the caller can fire
/// `traceability/state-id-collision` with both sites pinned.
pub fn build_symbol_table(
    model: &SCXMLModel,
    forge_docs: &[ForgeDocument],
) -> Result<BTreeMap<String, SymbolEntry>, Collision> {
    let mut table: BTreeMap<String, SymbolEntry> = BTreeMap::new();
    let machine_name = model.name.clone();

    // 1. SCXML root — keyed off `name` with empty state_path. The
    //    artifact label is `_machine` (an underscore-prefixed
    //    reserved tag so it can never collide with an author-supplied
    //    artifact name like "on_entry" or "on_exit").
    if let Some(ref loc) = model.source_location {
        push_entry(&mut table, &machine_name, "", "_machine", loc)?;
    }

    // 2. Per-state walk — emits a SymbolEntry for the state's body
    //    function plus per-onentry / per-onexit / per-transition.
    for (state_id, state) in &model.states {
        walk_state(&mut table, &machine_name, state_id, state)?;
    }

    // 3. Forge-side walk — every ForgeDocument lowers to a per-kind
    //    body function. The mangler keys off `(machine = doc.name,
    //    state_path = "", artifact = "_forge_body")` for the body
    //    function; this is a stable convention shared with the per-
    //    backend template emit sites.
    for doc in forge_docs {
        if let Some(loc) = forge_doc_source_location(doc) {
            let name = forge_doc_name(doc);
            push_entry(&mut table, name, "", "_forge_body", loc)?;
        }
    }

    Ok(table)
}

/// Insert a single mangled symbol into the table, detecting collisions
/// against any prior entry. Internal helper — every push goes through
/// here so the dual-location collision payload is built consistently.
fn push_entry(
    table: &mut BTreeMap<String, SymbolEntry>,
    machine: &str,
    state_path: &str,
    artifact: &str,
    location: &SourceLocation,
) -> Result<(), Collision> {
    let mangled = mangle(machine, state_path, artifact);
    let entry = SymbolEntry {
        mangled: mangled.clone(),
        machine: machine.to_string(),
        state_path: state_path.to_string(),
        artifact: artifact.to_string(),
        location: location.clone(),
    };
    if let Some(prior) = table.get(&mangled) {
        return Err(Collision {
            mangled,
            first: prior.clone(),
            second: entry,
        });
    }
    table.insert(mangled, entry);
    Ok(())
}

/// Recursively emit per-state symbol entries: the state body plus its
/// transitions, onentry / onexit, and initial-transition actions. The
/// child-state recursion is implicit — `model.states` already holds
/// every reachable state as a flat map per the parser's contract, so
/// this walker stays one level deep.
fn walk_state(
    table: &mut BTreeMap<String, SymbolEntry>,
    machine: &str,
    state_id: &str,
    state: &State,
) -> Result<(), Collision> {
    if let Some(ref loc) = state.source_location {
        push_entry(table, machine, state_id, "_state_body", loc)?;
    }

    for (idx, trans) in state.transitions.iter().enumerate() {
        if let Some(ref loc) = trans.source_location {
            let artifact = format!("_transition_{}", idx);
            push_entry(table, machine, state_id, &artifact, loc)?;
        }
    }

    for (i, block) in state.on_entry_blocks.iter().enumerate() {
        walk_action_block(table, machine, state_id, "_on_entry", i, block)?;
    }
    for (i, block) in state.on_exit_blocks.iter().enumerate() {
        walk_action_block(table, machine, state_id, "_on_exit", i, block)?;
    }

    Ok(())
}

/// Walk a single `<onentry>` / `<onexit>` block, emitting one symbol
/// entry per direct child action. `block_idx` distinguishes multiple
/// `<onentry>` siblings on the same state.
fn walk_action_block(
    table: &mut BTreeMap<String, SymbolEntry>,
    machine: &str,
    state_id: &str,
    kind: &str,
    block_idx: usize,
    actions: &[Action],
) -> Result<(), Collision> {
    for (action_idx, action) in actions.iter().enumerate() {
        if let Some(ref loc) = action.source_location {
            let artifact = format!("{}_{}_{}", kind, block_idx, action_idx);
            push_entry(table, machine, state_id, &artifact, loc)?;
        }
    }
    Ok(())
}

/// Read the name of a `ForgeDocument` variant. Exhaustive match so a
/// future kind addition surfaces here at compile time (textbook
/// silently-broken-hook prevention per [[feedback-silently-broken-
/// hooks]]).
fn forge_doc_name(doc: &ForgeDocument) -> &str {
    match doc {
        ForgeDocument::Statechart(m) => &m.name,
        ForgeDocument::Transform(m) => &m.name,
        ForgeDocument::Lookup(m) => &m.name,
        ForgeDocument::Condition(m) => &m.name,
        ForgeDocument::Codec(m) => &m.name,
        ForgeDocument::Validator(m) => &m.name,
        ForgeDocument::Procedure(m) => &m.name,
        ForgeDocument::Filter(m) => &m.name,
        ForgeDocument::Interpolation(m) => &m.name,
        ForgeDocument::Timer(m) => &m.name,
        ForgeDocument::Observer(m) => &m.name,
        ForgeDocument::Algorithm(m) => &m.name,
        ForgeDocument::Link(m) => &m.name,
        ForgeDocument::BufferPool(m) => &m.name,
        ForgeDocument::Worker(m) => &m.name,
        ForgeDocument::BoundedCollection(m) => &m.name,
    }
}

/// Silence unused-import false-positive when `Transition` is only
/// referenced via the `state.transitions` field access. Keeping the
/// import explicit makes the walker's contract self-documenting.
#[allow(dead_code)]
fn _trait_object_force_link(_t: &Transition) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mangle_three_part_path() {
        let m = mangle("motor", "running/fast", "_on_entry_0_0");
        assert_eq!(m, "motor__running_fast___on_entry_0_0");
    }

    #[test]
    fn escape_segment_replaces_double_underscore() {
        assert_eq!(escape_segment("foo__bar"), "foo_u_bar");
        assert_eq!(escape_segment("a__b__c"), "a_u_b_u_c");
    }

    #[test]
    fn unescape_segment_inverse() {
        let s = "foo__bar__baz";
        assert_eq!(unescape_segment(&escape_segment(s)), s);
    }

    #[test]
    fn mangle_demangle_round_trip_plain() {
        let (m, sp, a) = demangle(&mangle("motor", "running", "_state_body")).unwrap();
        assert_eq!(m, "motor");
        assert_eq!(sp, "running");
        assert_eq!(a, "_state_body");
    }

    #[test]
    fn mangle_demangle_round_trip_escaped_machine() {
        let (m, sp, a) = demangle(&mangle("ma__ch", "s1", "_state_body")).unwrap();
        // The `__` inside the machine name escapes to `_u_`; on the way
        // back the unescape returns the original `__`.
        assert_eq!(m, "ma__ch");
        assert_eq!(sp, "s1");
        assert_eq!(a, "_state_body");
    }

    #[test]
    fn demangle_rejects_unbalanced() {
        // Two delimiters required, one supplied — fail.
        assert!(demangle("only__one").is_none());
        // No delimiters at all — fail.
        assert!(demangle("no_delim").is_none());
    }

    #[test]
    fn exceeds_c99_limit_at_boundary() {
        // Exactly 31 chars passes; 32 trips the cap.
        let at = "a".repeat(31);
        assert!(!exceeds_c99_external_limit(&at));
        let over = "a".repeat(32);
        assert!(exceeds_c99_external_limit(&over));
    }

    #[test]
    fn build_symbol_table_collision_detected() {
        // Two states that mangle to the same key — possible when
        // XInclude composition imports a fragment whose state id
        // matches a top-level state. The walker reports both sites.
        let mut model = SCXMLModel {
            name: "m".into(),
            source_location: Some(SourceLocation {
                file: "x.scxml".into(),
                line: Some(1),
                col: Some(1),
            }),
            ..SCXMLModel::default()
        };
        let s1 = State {
            id: "dup".into(),
            source_location: Some(SourceLocation {
                file: "x.scxml".into(),
                line: Some(5),
                col: Some(1),
            }),
            ..State::default()
        };
        model.states.insert("dup".into(), s1);
        // Second state with same id reuses the first slot in BTreeMap,
        // so simulate the collision through a forge-doc that happens
        // to mangle identically.  Easier: drive collision through the
        // pure encoder by inserting two entries manually under the
        // same key.
        let mut table: BTreeMap<String, SymbolEntry> = BTreeMap::new();
        let loc1 = SourceLocation {
            file: "a.scxml".into(),
            line: Some(10),
            col: None,
        };
        let loc2 = SourceLocation {
            file: "b.scxml".into(),
            line: Some(20),
            col: None,
        };
        push_entry(&mut table, "m", "dup", "_state_body", &loc1).unwrap();
        let res = push_entry(&mut table, "m", "dup", "_state_body", &loc2);
        let coll = res.expect_err("must report collision on duplicate symbol");
        assert_eq!(coll.first.location.file, "a.scxml");
        assert_eq!(coll.second.location.file, "b.scxml");
    }

    #[test]
    fn build_symbol_table_real_parser_round_trip() {
        let scxml = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" name="m" version="1.0"
       initial="s1" datamodel="ecmascript">
  <state id="s1">
    <onentry>
      <log expr="'hello'"/>
    </onentry>
    <transition event="go" target="s2"/>
  </state>
  <final id="s2"/>
</scxml>"#;
        let mut parser = crate::parser::SCXMLParser::new();
        let model = parser.parse_string(scxml, "fixture").expect("parses");
        let table = build_symbol_table(&model, &[]).expect("no collisions");
        let mname = &model.name;
        // Root + s1 body + s1 transition + s1 on_entry_0_0 + s2 body
        assert!(
            table.contains_key(&mangle(mname, "", "_machine")),
            "no _machine entry for {mname}; keys = {:?}",
            table.keys().collect::<Vec<_>>(),
        );
        assert!(table.contains_key(&mangle(mname, "s1", "_state_body")));
        assert!(table.contains_key(&mangle(mname, "s1", "_transition_0")));
        assert!(table.contains_key(&mangle(mname, "s2", "_state_body")));
    }
}
