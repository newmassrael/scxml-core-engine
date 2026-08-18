// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What binds `ecmascript::builtins` to the two things it describes.
//
// The module answers one question — which standard-library names this
// datamodel provides — and two independent parties have to agree with
// its answer:
//
//   * **The emitter.** A name listed as lowered must actually be
//     lowered. Before this pass, `words.map(...)` was emitted as the
//     Lua field call `words.map(...)`: legal Lua, generated on every
//     backend, `check` answered `status: "ok"`, and it died at runtime
//     with a message about indexing a nil value. That fallthrough is
//     the defect these tests exist to keep closed, and the way to keep
//     it closed is to assert that a lowered name does *not* come back
//     as a field call.
//
//   * **The shared Lua library.** Every helper the emitter names has
//     to exist in `sce/include/scripting/ecma_semantics.lua`, which
//     each engine loads, and every member of an installed namespace has
//     to be one the library actually defines. Both directions matter: a
//     helper the library lost would fail at runtime exactly like the
//     defect above, and a member the library gained would be *refused*
//     by a frontend whose list had not moved.
//
// Nothing here reads the emitter's source or greps for a pattern; each
// claim is made by lowering an expression and reading the answer.

use sce_build::ecmascript::builtins::{
    DOM_METHODS, DOM_UNIMPLEMENTED_METHODS, JSON_MEMBERS, LOWERED_METHODS, MATH_FUNCTIONS,
    OBJECT_MEMBERS, UNIMPLEMENTED_METHODS,
};
use sce_build::ecmascript::{to_lua_value, DocumentScope, ExprError};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The receivers this file's probes hang a call on.
///
/// Every source here is a claim about a *method* or a *namespace*, so
/// `x`, `a` and `handlers` are scaffolding — they exist to give the call
/// something to be called on. Declaring them keeps these probes about
/// the vocabulary; whether an undeclared name is refused is
/// `ecmascript_identifier_scope`'s question, and answering it here too
/// would make every probe fail for the wrong reason.
fn probes() -> DocumentScope {
    DocumentScope::declaring(["a", "b", "c", "handlers", "i", "o", "x", "xs"])
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

fn shared_library(name: &str) -> String {
    let path = repo_root().join("sce/include/scripting").join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Every `function <name>(` the shared library defines, including the
/// `Table.member` forms.
fn functions_defined_in(source: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for line in source.lines() {
        let line = line.trim_start();
        let rest = match line.strip_prefix("function ") {
            Some(rest) => rest,
            None => match line.strip_prefix("local function ") {
                Some(rest) => rest,
                None => continue,
            },
        };
        if let Some(open) = rest.find('(') {
            found.insert(rest[..open].trim().to_string());
        }
    }
    found
}

/// Lower `x.<method>()` and hand back the Lua, or the refusal.
fn lower_method(method: &str) -> Result<String, ExprError> {
    to_lua_value(&format!("x.{method}()"), &probes())
}

// ── The two lists are one decision ────────────────────────────────

/// A name cannot be both lowered and unimplemented. Adding `trim` to
/// the emitter without striking it from the unimplemented list would
/// make the frontend refuse a construct it can lower — the opposite
/// defect, and just as silent from the author's side.
#[test]
fn lowered_and_unimplemented_names_are_disjoint() {
    let lowered: BTreeSet<&str> = LOWERED_METHODS.iter().copied().collect();
    let overlap: Vec<&str> = UNIMPLEMENTED_METHODS
        .iter()
        .copied()
        .filter(|name| lowered.contains(name))
        .collect();
    assert!(
        overlap.is_empty(),
        "these names are listed as both lowered and unimplemented: {overlap:?}"
    );
}

/// Both lists are sorted and free of duplicates, so a reader can find a
/// name and a reviewer can see an addition.
#[test]
fn both_vocabularies_are_sorted_and_unique() {
    for (label, list) in [
        ("LOWERED_METHODS", LOWERED_METHODS),
        ("JSON_MEMBERS", JSON_MEMBERS),
        ("OBJECT_MEMBERS", OBJECT_MEMBERS),
        ("MATH_FUNCTIONS", MATH_FUNCTIONS),
        ("DOM_METHODS", DOM_METHODS),
    ] {
        let mut sorted = list.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(
            sorted,
            list.to_vec(),
            "{label} is not sorted or has a duplicate"
        );
    }
    // `UNIMPLEMENTED_METHODS` is grouped by owning prototype and
    // `DOM_UNIMPLEMENTED_METHODS` by owning interface, rather than
    // sorted, so only uniqueness is asserted for those two.
    for (label, list) in [
        ("UNIMPLEMENTED_METHODS", UNIMPLEMENTED_METHODS),
        ("DOM_UNIMPLEMENTED_METHODS", DOM_UNIMPLEMENTED_METHODS),
    ] {
        let mut unique = list.to_vec();
        unique.sort_unstable();
        let before = unique.len();
        unique.dedup();
        assert_eq!(before, unique.len(), "{label} has a duplicate");
    }
}

// ── The emitter agrees ────────────────────────────────────────────

/// Every lowered name reaches a real lowering — not the field call the
/// unknown-name arm produces.
///
/// The field-call shape is what the whole defect looked like, so it is
/// what this asserts against: `x.map()` used to come back as `x.map()`,
/// indistinguishable from success.
#[test]
fn every_lowered_method_is_actually_lowered() {
    for &method in LOWERED_METHODS {
        let lua = lower_method(method)
            .unwrap_or_else(|e| panic!("`x.{method}()` is listed as lowered but was refused: {e}"));
        assert!(
            !lua.contains(&format!("x.{method}(")),
            "`x.{method}()` came back as a field call ({lua}) — the list \
             promises a lowering the emitter does not have"
        );
    }
}

/// Every unimplemented name is refused, and the refusal carries the
/// vocabulary that does exist.
#[test]
fn every_unimplemented_method_is_refused_with_its_alternatives() {
    for &method in UNIMPLEMENTED_METHODS {
        match lower_method(method) {
            Err(ExprError::UnsupportedBuiltin { name, available }) => {
                assert_eq!(name, format!(".{method}()"));
                assert_eq!(
                    available.len(),
                    LOWERED_METHODS.len(),
                    "`.{method}()` offered {} candidates; the lowered set has {}",
                    available.len(),
                    LOWERED_METHODS.len()
                );
            }
            Err(other) => panic!("`x.{method}()` was refused as {other} rather than as a builtin"),
            Ok(lua) => panic!("`x.{method}()` lowered to {lua} instead of being refused"),
        }
    }
}

// ── The DOM vocabulary is one decision too ────────────────────────

/// The four method vocabularies are pairwise disjoint.
///
/// A name lowered *and* refused is the silent-in-the-other-direction
/// defect the sibling test above guards; a name in both refusal lists
/// would carry whichever candidate set the reader of
/// `unsupported_method` happened to check first, which is a coin toss
/// dressed up as a fix.
#[test]
fn the_four_method_vocabularies_are_disjoint() {
    let lists: [(&str, &[&str]); 4] = [
        ("LOWERED_METHODS", LOWERED_METHODS),
        ("UNIMPLEMENTED_METHODS", UNIMPLEMENTED_METHODS),
        ("DOM_METHODS", DOM_METHODS),
        ("DOM_UNIMPLEMENTED_METHODS", DOM_UNIMPLEMENTED_METHODS),
    ];
    for (i, (left_name, left)) in lists.iter().enumerate() {
        for (right_name, right) in lists.iter().skip(i + 1) {
            let right_set: BTreeSet<&str> = right.iter().copied().collect();
            let overlap: Vec<&str> = left
                .iter()
                .copied()
                .filter(|name| right_set.contains(name))
                .collect();
            assert!(
                overlap.is_empty(),
                "{left_name} and {right_name} both list {overlap:?}"
            );
        }
    }
}

/// Every DOM method reaches the receiver-binding call the bindings
/// expose, rather than the field call an unknown name produces.
///
/// The `:` is the whole assertion. A DOM handle is userdata with a
/// metatable in five backends and a bound-method object in the other
/// two, so `d.getAttribute(name)` — the field-call shape — passes the
/// handle nowhere and dies at runtime, which is exactly how
/// `words.map(...)` failed.
#[test]
fn every_dom_method_binds_its_receiver() {
    for &method in DOM_METHODS {
        let lua = to_lua_value(&format!("x.{method}()"), &probes())
            .unwrap_or_else(|e| panic!("`x.{method}()` is a DOM method but was refused: {e}"));
        assert_eq!(
            lua,
            format!("x:{method}()"),
            "`x.{method}()` did not lower to a receiver-binding call"
        );
    }
}

/// Every DOM method SCE does not carry is refused, and the refusal
/// offers the DOM vocabulary rather than the string vocabulary.
#[test]
fn every_unimplemented_dom_method_is_refused_against_the_dom_surface() {
    let dom_candidates: BTreeSet<String> = DOM_METHODS.iter().map(|m| format!(".{m}()")).collect();
    for &method in DOM_UNIMPLEMENTED_METHODS {
        match lower_method(method) {
            Err(ExprError::UnsupportedBuiltin { name, available }) => {
                assert_eq!(name, format!(".{method}()"));
                let offered: BTreeSet<String> = available.into_iter().collect();
                assert_eq!(
                    offered, dom_candidates,
                    "`.{method}()` was refused against the wrong vocabulary"
                );
            }
            Err(other) => panic!("`x.{method}()` was refused as {other} rather than as a builtin"),
            Ok(lua) => panic!("`x.{method}()` lowered to {lua} instead of being refused"),
        }
    }
}

/// The DOM read surface is reachable as a property read on any handle.
///
/// This is the half that needed no emitter arm and had no backend:
/// measured 2026-08-18, `var1.tagName` lowered to `var1.tagName` and
/// every engine answered nil. The lowering is what this asserts; that
/// each of the seven bindings answers it is asserted where the binding
/// lives, and end to end by `integration_resources/dom_read_surface`.
#[test]
fn the_dom_read_surface_lowers_as_a_property_read() {
    for property in [
        "nodeName",
        "nodeType",
        "nodeValue",
        "parentNode",
        "childNodes",
        "firstChild",
        "lastChild",
        "nextSibling",
        "previousSibling",
        "textContent",
        "tagName",
        "data",
        "documentElement",
    ] {
        let lua = to_lua_value(&format!("x.{property}"), &probes())
            .unwrap_or_else(|e| panic!("`x.{property}` is a DOM property but was refused: {e}"));
        assert_eq!(lua, format!("x.{property}"));
    }
    // `length` is the one member of the surface this datamodel already
    // owned: ECMA-262 gives it to strings and arrays, so it is lowered
    // to Lua's `#` for every receiver and a NodeList is an array in
    // each of the seven bindings for exactly that reason.
    let lua = to_lua_value("x.childNodes.length", &probes()).expect("NodeList length");
    assert_eq!(lua, "#x.childNodes");
}

/// A method the author's own object carries is still an ordinary field
/// call. The datamodel admits a function in a `<data>` value, and
/// refusing every unknown name would take that away.
#[test]
fn an_authors_own_method_is_still_a_field_call() {
    let lua =
        to_lua_value("handlers.retry(3)", &probes()).expect("an author's own method is accepted");
    assert_eq!(lua, "handlers.retry(3)");
}

// ── The namespaces agree with the library that installs them ──────

/// `JSON`'s members are exactly what `json_builtins.lua` defines.
///
/// Both directions: a member the file lost would be emitted as a call
/// into nil, and one it gained would be refused by a stale list.
///
/// `JSON._*` is excluded, and the underscore is why: it is this repository's
/// marker for a global that exists to implement something rather than to be
/// called from a document (`_scxml_tostring`, `_scxml_params`). `JSON.parse`
/// is spread across `JSON._parse_value` and friends because the C11 backend
/// loads this file in chunks split on blank lines and cannot hold a long
/// function or a shared local — a packaging constraint, not vocabulary. An
/// author writing `JSON._parse_value(...)` is still refused, which is what
/// keeping them out of `JSON_MEMBERS` means.
#[test]
fn json_members_match_the_shared_library() {
    let defined = functions_defined_in(&shared_library("json_builtins.lua"));
    let declared: BTreeSet<String> = JSON_MEMBERS.iter().map(|m| format!("JSON.{m}")).collect();
    let from_file: BTreeSet<String> = defined
        .into_iter()
        .filter(|f| f.starts_with("JSON.") && !f.starts_with("JSON._"))
        .collect();
    assert_eq!(
        from_file, declared,
        "JSON_MEMBERS and json_builtins.lua disagree"
    );
}

/// `Object`'s members are exactly what `ecma_semantics.lua` defines.
#[test]
fn object_members_match_the_shared_library() {
    let defined = functions_defined_in(&shared_library("ecma_semantics.lua"));
    let declared: BTreeSet<String> = OBJECT_MEMBERS
        .iter()
        .map(|m| format!("Object.{m}"))
        .collect();
    let from_file: BTreeSet<String> = defined
        .into_iter()
        .filter(|f| f.starts_with("Object."))
        .collect();
    assert_eq!(
        from_file, declared,
        "OBJECT_MEMBERS and ecma_semantics.lua disagree"
    );
}

/// A member outside an installed namespace is refused against that
/// namespace's set, rather than emitted as a call into nil.
#[test]
fn a_member_outside_an_installed_namespace_is_refused() {
    for (source, expected_name) in [
        ("JSON.serialize(x)", "JSON.serialize"),
        ("Object.freeze(x)", "Object.freeze"),
        ("Math.tanh(x)", "Math.tanh"),
    ] {
        match to_lua_value(source, &probes()) {
            Err(ExprError::UnsupportedBuiltin { name, available }) => {
                assert_eq!(name, expected_name);
                assert!(
                    available.iter().all(|c| c.starts_with(&format!(
                        "{}.",
                        expected_name.split('.').next().expect("qualified name")
                    ))),
                    "{expected_name} was offered candidates from another namespace: {available:?}"
                );
            }
            other => panic!("{source} was not refused as a builtin: {other:?}"),
        }
    }
}

/// Every `Math` function listed lowers, and the two with a lowering of
/// their own are the only two that are not `math.<same name>`.
#[test]
fn every_math_member_lowers() {
    for &member in MATH_FUNCTIONS {
        // ECMA-262 15.8.2.13 takes two arguments and the emitter says
        // so; every other member here is happy with one.
        let args = if member == "pow" { "1, 2" } else { "1" };
        let lua = to_lua_value(&format!("Math.{member}({args})"), &probes())
            .unwrap_or_else(|e| panic!("Math.{member} is listed but refused: {e}"));
        match member {
            // ECMA-262 15.8.2.13 is Lua's `^`, and 15.8.2.15 sends a
            // half toward +Infinity where Lua has no rounding at all.
            "pow" | "round" => assert!(
                !lua.contains("math."),
                "Math.{member} is expected to have a lowering of its own, got {lua}"
            ),
            _ => assert!(
                lua.starts_with(&format!("math.{member}(")),
                "Math.{member} lowered to {lua}"
            ),
        }
    }
}

// ── Every helper the emitter names exists ─────────────────────────

/// The helper each lowered method reaches for is defined in the shared
/// library every engine loads.
///
/// This is the direction that used to break silently in the other
/// order: an emitter arm naming `_scxml_replace` while the library
/// defined no such function produces Lua that parses, generates and
/// dies on evaluation — the same failure the unknown-name fallthrough
/// produced, arrived at from the emitter's side.
#[test]
fn every_helper_the_emitter_names_is_defined_in_the_shared_library() {
    let defined = functions_defined_in(&shared_library("ecma_semantics.lua"));
    let mut missing = Vec::new();
    // One expression per lowered method, plus the operators whose
    // meaning differs between the two languages, so the sweep covers
    // what the emitter can emit rather than what a list says it can.
    let mut sources: Vec<String> = LOWERED_METHODS
        .iter()
        .map(|m| format!("x.{m}(1, 2)"))
        .collect();
    for extra in [
        "a + b",
        "a == b",
        "a % b",
        "a & b",
        "a | b",
        "a ^ b",
        "a << b",
        "a >> b",
        "a >>> b",
        "~a",
        "typeof a",
        "+a",
        "a instanceof Array",
        "a[i]",
        "Math.round(a)",
        "a && b",
        "a || b",
        "a ? b : c",
        "x++",
        "String(a)",
        "Number(a)",
        "Boolean(a)",
        "parseInt(a)",
        "parseFloat(a)",
    ] {
        sources.push(extra.to_string());
    }
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for source in &sources {
        let lua = match to_lua_value(source, &probes()) {
            Ok(lua) => lua,
            Err(e) => panic!("`{source}` is inside the accepted subset but was refused: {e}"),
        };
        for helper in helpers_named_in(&lua) {
            seen.insert(helper.clone());
            if !defined.contains(&helper) {
                missing.push((source.clone(), helper));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "the emitter names helpers `ecma_semantics.lua` does not define \
         (expression, helper):\n{missing:#?}"
    );
    // A sweep that reaches nothing agrees with everything. The lower
    // bound is what separates "every helper exists" from "the extractor
    // stopped finding helpers" — one helper per lowered method is
    // already more than this.
    assert!(
        seen.len() >= LOWERED_METHODS.len(),
        "the sweep only found {} helper(s) across {} expressions, which is \
         fewer than the lowered methods alone should name: {seen:?}",
        seen.len(),
        sources.len()
    );
}

/// Identifiers in `lua` that look like SCE's own helpers: the shared
/// library's names all begin with `_`, which separates them from Lua's
/// own `math.*` / `table.*` and from an author's identifiers.
fn helpers_named_in(lua: &str) -> BTreeSet<String> {
    let bytes: Vec<char> = lua.chars().collect();
    let mut found = BTreeSet::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == '_' {
            let start = i;
            while i < bytes.len() && (bytes[i].is_alphanumeric() || bytes[i] == '_') {
                i += 1;
            }
            // Only a call site names a function; `_NULL` and the
            // emitter's own `__t` / `__l` temporaries are values.
            let is_call = bytes.get(i) == Some(&'(');
            let name: String = bytes[start..i].iter().collect();
            if is_call && !name.starts_with("__") {
                found.insert(name);
            }
        } else {
            i += 1;
        }
    }
    found
}
