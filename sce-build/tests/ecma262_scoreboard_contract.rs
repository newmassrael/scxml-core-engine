// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! `ARCHITECTURE.md`'s ECMA-262 column is a measurement, so it is derived here
//! rather than typed there.
//!
//! Two files already answer the question the column asks.
//! `tests/ecmascript/ecma262_semantics.json` is what ECMA-262 says, written by
//! hand, one case per clause — every engine claiming `datamodel="ecmascript"`
//! is measured against it. `tests/ecmascript/lua_engine_divergences.json` is
//! the enumerated set the `lua` selection answers differently, and
//! `ecmascript_semantics_test` holds the engine to it in both directions: an
//! undeclared disagreement is red, and so is a declared one that has been
//! repaired.
//!
//! What nothing held was the SCOREBOARD — the two cells a reader consults
//! before choosing an engine. They had been typed, and by 2026-08-27 they read
//! `58/58` and `32/58` against a table holding 98 cases: both engines scored
//! out of a denominator that no longer existed, in the document that exists to
//! tell a consumer which engine to pick.
//!
//! That is not a new failure mode here. The divergence list's own header
//! records the same one a layer down — "A comment said 26 of 58 on the day
//! someone measured it, the shared table then grew to 98 cases, and nothing
//! re-answered the question: the real number on 2026-08-18 was 44." The list
//! was created so a count could stop living in prose. This gate is that
//! reasoning applied to the last place the prose survived.
//!
//! The rule it enforces generalises past today's two rows:
//!
//! * Every denominator is the shared table's length. A cell scored out of
//!   anything else is scored out of a table that does not exist.
//! * A row for an engine offered AS ECMAScript must read all-of-all. Anything
//!   less is a disagreement nobody registered, and the reader is being sold an
//!   engine on a number the suite is not defending.
//! * The `lua` row is the table minus the declared divergences. That
//!   subtraction is only arithmetic if every declared entry answers a real
//!   case, so that is checked before it is trusted.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const SCOREBOARD_DOC: &str = "ARCHITECTURE.md";
const CASES_JSON: &str = "tests/ecmascript/ecma262_semantics.json";
const DIVERGENCES_JSON: &str = "tests/ecmascript/lua_engine_divergences.json";

/// Every declared-divergence list beside the shared table.
///
/// Two engines rewrite ECMAScript into Lua — the C++ `lua` selection and the
/// Kotlin backend's `LuaScriptEngine` — with different transformers onto
/// different Luas, so each keeps its own measurement and neither may be
/// derived from the other. Their integrity is one question, asked here once:
/// a list whose entries name no case is a list its own suite compares against
/// and cannot fault. The Kotlin one is checked from HERE rather than only
/// from its suite because this lane needs no JVM, so an orphan is caught on
/// every push instead of only when the Kotlin gate is selected.
const DIVERGENCE_LISTS: &[&str] = &[
    DIVERGENCES_JSON,
    "tests/ecmascript/kotlin_lua_divergences.json",
];

/// The matrix is found by its header rather than by position: a section that
/// moves keeps its table, and a table that is renamed should fail here rather
/// than be silently skipped.
const MATRIX_HEADER: &str = "| Engine | Standard | Selection | W3C IRP | ECMA-262";

/// The same floor `ecmascript_semantics_test` carries, for the same reason: a
/// table that shrank to nothing would score every engine perfectly.
const MIN_CASES: usize = 55;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent dir")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(repo_root().join(rel))
        .unwrap_or_else(|e| panic!("{rel} is readable: {e}"))
}

fn json(rel: &str) -> serde_json::Value {
    serde_json::from_str(&read(rel)).unwrap_or_else(|e| panic!("{rel} is JSON: {e}"))
}

/// `(source, clause)` — what identifies one case. `source` alone does not: the
/// table asks `a && b` under two different clauses, and a divergence list keyed
/// on the expression alone would collapse them into one.
fn key(v: &serde_json::Value) -> (String, String) {
    (
        v.get("source")
            .and_then(|s| s.as_str())
            .unwrap_or_default()
            .to_string(),
        v.get("clause")
            .and_then(|s| s.as_str())
            .unwrap_or_default()
            .to_string(),
    )
}

fn cases() -> Vec<serde_json::Value> {
    json(CASES_JSON)
        .get("cases")
        .and_then(|c| c.as_array())
        .unwrap_or_else(|| panic!("{CASES_JSON} has a `cases` array"))
        .clone()
}

fn divergences_in(rel: &str) -> Vec<serde_json::Value> {
    json(rel)
        .get("divergences")
        .and_then(|d| d.as_array())
        .unwrap_or_else(|| panic!("{rel} has a `divergences` array"))
        .clone()
}

fn divergences() -> Vec<serde_json::Value> {
    divergences_in(DIVERGENCES_JSON)
}

/// One row of the engine matrix: the engine's name and the `N/M` it claims.
#[derive(Debug)]
struct Row {
    engine: String,
    scored: usize,
    outof: usize,
}

fn matrix_rows() -> Vec<Row> {
    let doc = read(SCOREBOARD_DOC);
    let at = doc.find(MATRIX_HEADER).unwrap_or_else(|| {
        panic!(
            "{SCOREBOARD_DOC} carries no engine matrix (looked for a row opening \
             `{MATRIX_HEADER}`). That table is where a consumer reads which engine \
             answers ECMA-262; without it this gate would pass by measuring nothing."
        )
    });

    let mut rows = Vec::new();
    for line in doc[at..].lines().skip(1) {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            break;
        }
        let cells: Vec<&str> = trimmed
            .trim_start_matches('|')
            .trim_end_matches('|')
            .split('|')
            .map(str::trim)
            .collect();
        if cells.len() < 5 {
            continue;
        }
        // The `|---|` separator.
        if cells[0].chars().all(|c| c == '-' || c == ':') {
            continue;
        }
        let cell = cells[cells.len() - 1].trim().trim_matches('*').trim();
        let (scored, outof) = cell.split_once('/').unwrap_or_else(|| {
            panic!(
                "the ECMA-262 cell for `{}` reads '{cell}', which is not `N/M`. The column \
                 is a score against a named table, and a cell that stopped being one \
                 cannot be checked against it.",
                cells[0]
            )
        });
        let parse = |s: &str| {
            s.trim()
                .parse::<usize>()
                .unwrap_or_else(|_| panic!("the ECMA-262 cell for `{}` reads '{cell}'", cells[0]))
        };
        rows.push(Row {
            engine: cells[0].to_string(),
            scored: parse(scored),
            outof: parse(outof),
        });
    }
    rows
}

/// Lower bound. A parse that lost the rows would report every remaining claim
/// as consistent by having none left to check.
fn rows_or_panic() -> Vec<Row> {
    let rows = matrix_rows();
    assert!(
        rows.len() >= 2,
        "the engine matrix parsed to {} row(s). It exists to put the selectable \
         engines side by side, and a one-row comparison is not one.\nrows: {rows:?}",
        rows.len()
    );
    rows
}

#[test]
fn every_declared_divergence_answers_a_real_case() {
    let cases = cases();
    assert!(
        cases.len() >= MIN_CASES,
        "the shared ECMA-262 table produced only {} case(s); the floor is {MIN_CASES}. \
         A table that shrank would score every engine perfectly.",
        cases.len()
    );

    let known: BTreeSet<(String, String)> = cases.iter().map(key).collect();
    assert_eq!(
        known.len(),
        cases.len(),
        "the shared table has two cases with the same (source, clause). The scoreboard \
         subtracts declared divergences from this length, and that arithmetic needs the \
         key to identify a case."
    );

    assert!(
        DIVERGENCE_LISTS.len() >= 2,
        "two engines rewrite ECMAScript into Lua and each keeps its own list; this sweep \
         is down to {} and is no longer asking about both.",
        DIVERGENCE_LISTS.len()
    );

    for list in DIVERGENCE_LISTS {
        let declared = divergences_in(list);
        assert!(
            !declared.is_empty(),
            "{list} declares nothing. Each list is what its suite compares an engine \
             against in both directions, so an empty one silently scores that engine \
             perfect — the claim these files were written to stop being made in prose."
        );

        let seen: BTreeSet<(String, String)> = declared.iter().map(key).collect();
        assert_eq!(
            seen.len(),
            declared.len(),
            "{list} lists the same (source, clause) twice. A duplicate scores the engine \
             lower than the table can justify, and hides that one of the two was never \
             looked at."
        );

        let orphans: Vec<&(String, String)> = seen.difference(&known).collect();
        assert!(
            orphans.is_empty(),
            "{} entr(ies) in {list} name no case in {CASES_JSON}. Each list's own suite \
             would report these as cases that stopped diverging — which is the wrong \
             sentence for an entry that never named one — and the C++ scoreboard \
             subtracts this list's length, which is only arithmetic while every entry \
             answers a real case.\norphans: {orphans:?}",
            orphans.len()
        );
    }
}

#[test]
fn every_scoreboard_denominator_is_the_shared_tables_length() {
    let total = cases().len();
    for row in rows_or_panic() {
        assert_eq!(
            row.outof, total,
            "`{}` is scored out of {} in {SCOREBOARD_DOC}, and the shared table holds {total} \
             cases. A denominator typed by hand outlives the table it was typed against — \
             this column read 58 after the table had grown to 98, so both engines were \
             scored out of a table that no longer existed.",
            row.engine, row.outof
        );
    }
}

#[test]
fn an_engine_offered_as_ecmascript_is_scored_all_of_all() {
    let total = cases().len();
    let rows = rows_or_panic();
    let full: Vec<&Row> = rows
        .iter()
        .filter(|r| !r.engine.to_ascii_lowercase().contains("lua"))
        .collect();
    assert!(
        !full.is_empty(),
        "no row in the engine matrix is an ECMAScript engine. The matrix exists to offer \
         one; a table of nothing but the rewriter is not a choice."
    );
    for row in full {
        assert_eq!(
            row.scored, total,
            "`{}` is offered for `datamodel=\"ecmascript\"` and scored {}/{}. Anything short \
             of the whole table is a disagreement nobody enumerated, and this row is what a \
             consumer reads before picking an engine.",
            row.engine, row.scored, row.outof
        );
    }
}

#[test]
fn the_lua_row_is_the_table_minus_the_declared_divergences() {
    let total = cases().len();
    let declared = divergences().len();
    let rows = rows_or_panic();
    let lua: Vec<&Row> = rows
        .iter()
        .filter(|r| r.engine.to_ascii_lowercase().contains("lua"))
        .collect();
    assert_eq!(
        lua.len(),
        1,
        "the engine matrix has {} row(s) naming Lua. Exactly one selection reaches the \
         rewriter, and its score is derived from a single divergence list.",
        lua.len()
    );
    assert_eq!(
        lua[0].scored,
        total - declared,
        "the Lua row claims {}/{}, and {DIVERGENCES_JSON} declares {declared} of the \
         {total} cases as answered differently — so the derived score is {}. This cell is \
         not a measurement someone takes and types; it is what the list already says.",
        lua[0].scored,
        lua[0].outof,
        total - declared
    );
}
