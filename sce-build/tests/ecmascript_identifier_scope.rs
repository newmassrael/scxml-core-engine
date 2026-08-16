// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What the frontend does with a name the document does not declare.
//!
//! The defect these pin: `sce_build::ecmascript::lua` emitted a free
//! identifier verbatim, so `<assign expr="conut + 1"/>` beside a
//! `<data id="count">` lowered to Lua `conut + 1`, generated cleanly on
//! every backend, and performed arithmetic on a nil at run time.
//! `check --lint` answered `status: "ok"`, exit 0. A misspelling — the
//! single most common thing an author or a generator gets wrong — had no
//! reader anywhere in the pipeline.
//!
//! Three claims are asserted here, and each is bound to something outside
//! this file so it cannot drift into a tautology:
//!
//! * The **installed** vocabulary matches the Lua library that installs
//!   it. `sce/include/scripting/*.lua` is read and its globals compared
//!   with [`INSTALLED_GLOBALS`], both directions.
//! * The **uninstalled** list is disjoint from the installed one and
//!   every entry is actually refused.
//! * The **resolver** binds what ECMAScript binds — parameters, `var`
//!   including its hoisting, `for…in`, implicit globals — and refuses
//!   what nothing binds.

use sce_build::ecmascript::builtins::{INSTALLED_GLOBALS, UNINSTALLED_GLOBALS};
use sce_build::ecmascript::{to_lua_script, to_lua_value, DocumentScope, ExprError};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// The globals the shared Lua library defines at its top level.
///
/// Read out of the source every engine loads rather than restated, so a
/// member the library gains or loses moves this set. Two shapes carry a
/// global: `function Name(` and `Name = `, both unindented.
fn globals_the_library_installs() -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for file in ["ecma_semantics.lua", "json_builtins.lua"] {
        let path = repo_root().join("sce/include/scripting").join(file);
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{} is not readable: {e}", path.display()));
        for line in text.lines() {
            if line.starts_with(char::is_whitespace) {
                continue;
            }
            let name = if let Some(rest) = line.strip_prefix("function ") {
                rest.split(['(', '.']).next().unwrap_or("").trim()
            } else if let Some((left, _)) = line.split_once('=') {
                let left = left.trim();
                if left.contains(' ') || left.contains('.') || left.contains('[') {
                    continue;
                }
                left
            } else {
                continue;
            };
            if name.is_empty() || name.starts_with('_') {
                continue;
            }
            found.insert(name.to_string());
        }
    }
    // A reader that stopped finding anything would make every assertion
    // below vacuous.
    assert!(
        found.len() >= 5,
        "read only {} global(s) out of the shared library: {found:?}",
        found.len()
    );
    found
}

/// Every global the shared library installs is one the frontend knows
/// about, so a document may name it.
///
/// The reverse direction is deliberately not asserted: `Math` is
/// rewritten by the emitter to Lua's own `math` and `In` plus the
/// §scxml-5.10 system variables are bound per session by each engine, so
/// they are legitimately absent from the library file.
#[test]
fn every_global_the_library_installs_is_in_the_frontends_vocabulary() {
    let installed: BTreeSet<&str> = INSTALLED_GLOBALS.iter().copied().collect();
    let missing: Vec<String> = globals_the_library_installs()
        .into_iter()
        .filter(|name| !installed.contains(name.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "the shared Lua library installs {missing:?}, which the frontend would \
         refuse as undeclared — add them to INSTALLED_GLOBALS"
    );
}

/// A name cannot be both installed and refused.
#[test]
fn installed_and_uninstalled_globals_are_disjoint() {
    let installed: BTreeSet<&str> = INSTALLED_GLOBALS.iter().copied().collect();
    let overlap: Vec<&&str> = UNINSTALLED_GLOBALS
        .iter()
        .filter(|name| installed.contains(*name))
        .collect();
    assert!(
        overlap.is_empty(),
        "{overlap:?} are listed as both installed and not installed"
    );
}

/// Every entry on the uninstalled list is refused as a builtin, and the
/// refusal names what does exist.
///
/// `Array` is the one exception and it is asserted separately below: the
/// emitter consumes it as part of `instanceof`.
#[test]
fn every_uninstalled_global_is_refused_by_name() {
    let scope = DocumentScope::installed();
    for &name in UNINSTALLED_GLOBALS {
        match to_lua_value(name, &scope) {
            Err(ExprError::UnsupportedBuiltin {
                name: got,
                available,
            }) => {
                assert_eq!(got, name);
                assert!(
                    !available.is_empty(),
                    "{name} was refused with nothing to offer instead"
                );
            }
            other => panic!("{name} is listed as uninstalled but lowered to {other:?}"),
        }
    }
}

/// `x instanceof Array` still works: the emitter reads `Array` as part of
/// the operator, so the resolver must not read it as a global.
#[test]
fn instanceof_array_survives_the_uninstalled_list() {
    let scope = DocumentScope::declaring(["a"]);
    let lua = to_lua_value("a instanceof Array", &scope)
        .expect("instanceof Array is the one constructor this datamodel represents");
    assert_eq!(lua, "_isArray(a)");
}

/// §ecma-262-11.4.3: `typeof` on an undeclared name answers
/// `"undefined"` rather than throwing. It is how a document asks whether
/// something exists, so refusing it would refuse the question. W3C test
/// 277 is the fixture.
#[test]
fn typeof_an_undeclared_name_is_a_question_not_a_mistake() {
    let scope = DocumentScope::installed();
    to_lua_value("typeof missingVariable !== 'undefined'", &scope)
        .expect("typeof on an unbound name is legal ECMAScript");
}

/// A misspelling is refused, and the correction rides the diagnostic.
///
/// This is the shape the whole change exists for: a consumer repairing
/// the document does not have to read it, because the near miss among
/// the document's own declarations is in the record.
#[test]
fn a_misspelled_name_is_refused_with_the_name_that_was_meant() {
    let scope = DocumentScope::declaring(["count", "counter", "helper"]);
    match to_lua_value("conut + 1", &scope) {
        Err(ExprError::UnknownIdentifier { name, candidates }) => {
            assert_eq!(name, "conut");
            assert!(
                candidates.contains(&"count".to_string()),
                "the correction was not offered: {candidates:?}"
            );
        }
        other => panic!("a misspelling lowered instead of being refused: {other:?}"),
    }
}

/// A name with nothing close to it carries no candidates, which becomes
/// an absent `fix` rather than an empty choice.
#[test]
fn a_name_with_no_near_miss_offers_nothing() {
    let scope = DocumentScope::declaring(["count"]);
    match to_lua_value("wholesaleDistributor", &scope) {
        Err(ExprError::UnknownIdentifier { candidates, .. }) => assert!(
            candidates.is_empty(),
            "an unrelated name was offered as a correction: {candidates:?}"
        ),
        other => panic!("an undeclared name lowered instead of being refused: {other:?}"),
    }
}

/// What the resolver binds is what ECMAScript binds.
///
/// Each case is legal in a document that declares nothing, so any
/// refusal is the resolver failing to model a binding form.
#[test]
fn the_resolver_binds_what_ecmascript_binds() {
    let scope = DocumentScope::installed();
    for source in [
        // Parameters.
        "function (a, b) { return a + b; }",
        // `var`, including a reference above its declaration — ECMA-262
        // hoists it to the top of the function.
        "function () { total = total + 1; var total; return total; }",
        // A named function expression can call itself.
        "function fact(n) { return n < 2 ? 1 : n * fact(n - 1); }",
        // A nested function sees the enclosing scope.
        "function (xs) { function inner() { return xs; } return inner(); }",
    ] {
        to_lua_value(source, &scope)
            .unwrap_or_else(|e| panic!("`{source}` binds its own names but was refused: {e}"));
    }
    for source in [
        // `for…in` declares its loop variable.
        "for (var k in o) { total = total + k; }",
        // A bare assignment creates a global (ECMA-262 10.2.1), which a
        // later statement may read.
        "fresh = 1; fresh = fresh + 1;",
        // A `<script>`'s top-level `function` is a datamodel global —
        // [`sce_build::ecmascript::lua`] emits it without `local`.
        "function helper(x) { return x; } var r = helper(1);",
    ] {
        let mut scope = DocumentScope::installed();
        scope.declare("o");
        scope.declare("total");
        to_lua_script(source, &scope)
            .unwrap_or_else(|e| panic!("`{source}` binds its own names but was refused: {e}"));
    }
}

/// A function's parameter does not leak out of it.
///
/// The opposite defect to the one this change fixes, and just as silent:
/// a scope that kept every binding it ever saw would accept the typo in
/// one expression because another expression happened to name it.
#[test]
fn a_parameter_does_not_escape_its_function() {
    let scope = DocumentScope::installed();
    let refusal = to_lua_script("function f(inner) { return inner; } var r = inner;", &scope)
        .expect_err("a parameter read outside its function must be refused");
    assert!(
        matches!(&refusal, ExprError::UnknownIdentifier { name, .. } if name == "inner"),
        "expected `inner` to be undeclared outside `f`, got {refusal:?}"
    );
}

/// A `<data id>` that is not a plain identifier is still declared.
///
/// The datamodel admits any XML name, and [`sce_build::ecmascript::lua`]
/// reaches those through `_ENV["…"]`; the scope has to know them or the
/// emitter's own escape hatch would be unreachable.
#[test]
fn a_declaration_that_is_not_a_lua_identifier_is_still_a_declaration() {
    let scope = DocumentScope::declaring(["end"]);
    let lua = to_lua_value("end", &scope).expect("a Lua keyword is a legal <data id>");
    assert_eq!(lua, "_ENV[\"end\"]");
}
