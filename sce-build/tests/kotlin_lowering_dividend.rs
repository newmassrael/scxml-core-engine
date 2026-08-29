// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! How many of the Kotlin backend's 46 runtime-rewriter divergences the
//! build-time frontend already answers — held to the measurement instead of
//! to a sentence.
//!
//! `tests/ecmascript/kotlin_lua_divergences.json` says WHICH cases this
//! backend's `LuaScriptEngine` answers differently from ECMA-262, and every
//! one of them is on one path, `runtime-rewriter`, because that is the only
//! route the generated Kotlin has. What that list could not say is the thing a
//! reader deciding whether to fund the lowered seam actually asks: **of these,
//! how many would simply stop existing if translation moved to build time?**
//!
//! Until `LoweredEcma262Test` landed there was no way to answer it on this
//! backend, because nothing had ever put the frontend's Lua to this backend's
//! Lua engine. It has now, and the answer splits three ways rather than two —
//! the rewriter destroys the frontend's own Lua for some cases on the way in,
//! so those cannot be asked at all yet. Each entry therefore carries a
//! `build_time_frontend` verdict, and this file is what keeps those verdicts
//! from being an opinion:
//!
//! * `answers` — put to the engine, and ECMA-262's answer came back. The seam
//!   retires this entry.
//! * `diverges` — put to the engine unchanged, and the answer was still wrong.
//!   Declared in `kotlin_lowered_ecma262.json`'s `divergences`.
//! * `unmeasured` — could not be put to the engine as the frontend wrote it.
//!   Declared in that file's `unreachable`.
//!
//! ## Why this lives here and not beside the suite that measures it
//!
//! The same reason `ecma262_scoreboard_contract` gives for checking the Kotlin
//! divergence list from a Rust lane: this one needs no JVM, so a verdict that
//! stops describing the tree is caught on every push rather than only when the
//! Kotlin gate is selected. The measurement stays in Kotlin, where the engine
//! is; what is re-derived here is whether the list agrees with it.
//!
//! ## What makes the verdicts a measurement rather than three words
//!
//! Nothing here trusts the label. Each one is RE-DERIVED from
//! `kotlin_lowered_ecma262.json` — the file `LoweredEcma262Test` holds in both
//! directions against the running engine — and compared. So the chain from a
//! verdict back to an execution has no prose in it: the engine answers, the
//! Kotlin suite writes and defends the two arrays, and this lane derives the
//! verdict from those arrays. A verdict typed by hand and a verdict earned are
//! the same value here only when they agree.
//!
//! ## The terminal state is that the field is GONE
//!
//! `build_time_frontend` exists because `build-time-lowering` is not yet a
//! path this backend HAS, so the answer cannot be written where it belongs —
//! in `diverges_on`, beside the path it is about. The day
//! `Language::Kotlin.supports_script_engine_target(Lua)` is true, that key
//! carries the same fact and this one becomes a second vocabulary for it.
//! `the_field_retires_when_the_seam_opens` is the assertion that says so on
//! exactly that day, which is what keeps this from becoming a permanent
//! parallel spelling nobody removes.

use sce_build::generator::{Language, ScriptEngineTarget};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const DIVERGENCES_JSON: &str = "tests/ecmascript/kotlin_lua_divergences.json";
const LOWERED_JSON: &str = "tests/ecmascript/kotlin_lowered_ecma262.json";
const SEAM_DOC: &str = "docs/SCE_LUA_TRANSLATION_SEAM.md";

/// The key `build_time_frontend` lives under on each entry, and the top-level
/// block that declares its vocabulary and tally.
const FIELD: &str = "build_time_frontend";

/// The verdict a case gets when the frontend's Lua reached the engine and the
/// engine answered what ECMA-262 says.
const VERDICT_ANSWERS: &str = "answers";

/// The verdict for a case that reached the engine unchanged and was still
/// answered wrong — `kotlin_lowered_ecma262.json`'s `divergences`.
const VERDICT_DIVERGES: &str = "diverges";

/// The verdict for a case the lowered route cannot put to the engine at all —
/// `kotlin_lowered_ecma262.json`'s `unaskable`.
///
/// That array is EMPTY since the lowered entry point landed, so no entry
/// carries this today. It stays in the vocabulary rather than being deleted:
/// the value is what a future route without an entry point would need, and a
/// verdict this lane can derive but the file does not admit is rejected as a
/// misspelling by `every_entry_says_what_the_build_time_frontend_answers`.
const VERDICT_UNMEASURED: &str = "unmeasured";

/// The anchor the seam document carries above the table this lane re-derives.
///
/// A comment rather than a heading, for the reason the other anchored table in
/// that file gives: a heading is prose someone may reword, and a table found
/// by its heading is a table that silently stops being checked when the
/// wording moves.
const SEAM_ANCHOR: &str = "<!-- sce:kotlin-lowering-dividend";

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

/// `(source, clause)` — what identifies one case, the same key every reader of
/// these lists uses. `source` alone does not: the shared table asks `a && b`
/// under two clauses.
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

fn divergences() -> Vec<serde_json::Value> {
    json(DIVERGENCES_JSON)
        .get("divergences")
        .and_then(|d| d.as_array())
        .unwrap_or_else(|| panic!("{DIVERGENCES_JSON} has a `divergences` array"))
        .clone()
}

/// One declared array of the lowered file, as a key set.
///
/// Missing is a panic rather than an empty set. An empty set and an absent one
/// mean opposite things — "the suite asked and found none" against "nothing
/// here was measured" — and the second silently turns every entry into
/// `answers`, which is the direction that overstates what the seam buys.
fn lowered_keys(array: &str) -> BTreeSet<(String, String)> {
    json(LOWERED_JSON)
        .get(array)
        .and_then(|d| d.as_array())
        .unwrap_or_else(|| {
            panic!(
                "{LOWERED_JSON} has no `{array}` array. Every `{FIELD}` verdict is \
                 derived from it, so without it this lane would report agreement \
                 with a measurement that is not there."
            )
        })
        .iter()
        .map(key)
        .collect()
}

/// The verdict the lowered measurement implies for one case.
///
/// The order is not arbitrary. `unreachable` is checked FIRST because it is a
/// statement about whether the case could be asked at all, and a case that was
/// never asked cannot have produced a divergence — reading them the other way
/// round would let a stale `divergences` entry mask an exemption.
fn derived_verdict(
    k: &(String, String),
    unreachable: &BTreeSet<(String, String)>,
    diverging: &BTreeSet<(String, String)>,
) -> &'static str {
    if unreachable.contains(k) {
        VERDICT_UNMEASURED
    } else if diverging.contains(k) {
        VERDICT_DIVERGES
    } else {
        VERDICT_ANSWERS
    }
}

/// The vocabulary the file declares, which is what an entry's verdict may be.
///
/// Declared rather than hard-coded here for the same reason `paths` is: a
/// value this lane knows and the file does not (or the reverse) is a spelling
/// one side accepts and the other cannot fault.
fn declared_values() -> BTreeSet<String> {
    json(DIVERGENCES_JSON)
        .get(FIELD)
        .and_then(|b| b.get("values"))
        .and_then(|v| v.as_object())
        .unwrap_or_else(|| {
            panic!(
                "{DIVERGENCES_JSON} has no `{FIELD}.values` object. It is the set an \
                 entry's verdict may name, and without it any spelling would be \
                 accepted — including one no lane measures."
            )
        })
        .keys()
        .cloned()
        .collect()
}

/// The tally the file states, which this lane holds to the list under it.
fn declared_tally() -> BTreeMap<String, i64> {
    json(DIVERGENCES_JSON)
        .get(FIELD)
        .and_then(|b| b.get("measured"))
        .and_then(|v| v.as_object())
        .unwrap_or_else(|| {
            panic!(
                "{DIVERGENCES_JSON} has no `{FIELD}.measured` object. The count is the \
                 thing a reader takes away, and one that is not re-derived is exactly \
                 the prose count this whole file exists to have stopped writing."
            )
        })
        .iter()
        .map(|(k, v)| {
            (
                k.clone(),
                v.as_i64().unwrap_or_else(|| {
                    panic!("{DIVERGENCES_JSON} `{FIELD}.measured.{k}` is a number")
                }),
            )
        })
        .collect()
}

/// What the list itself counts, per verdict, including the verdicts that are
/// declared but unused — a vocabulary entry at zero is a real answer and has
/// to appear in the tally rather than be omitted from it.
fn actual_tally() -> BTreeMap<String, i64> {
    let mut counts: BTreeMap<String, i64> = declared_values().into_iter().map(|v| (v, 0)).collect();
    for entry in divergences() {
        if let Some(v) = entry.get(FIELD).and_then(|v| v.as_str()) {
            *counts.entry(v.to_string()).or_insert(0) += 1;
        }
    }
    counts
}

/// Whether the backend can emit a lowered artifact today — the condition that
/// retires this whole field.
fn seam_is_open() -> bool {
    Language::Kotlin.supports_script_engine_target(ScriptEngineTarget::Lua)
}

/// Every entry names a verdict, and names one this file declares.
///
/// Unclassified is RED, not a default, for the reason `diverges_on` states one
/// key above it: the value a missing field would silently mean is
/// `unmeasured`, which is the ONE value that exempts an entry from the
/// measurement. An escape hatch reached by omission is not one anybody has to
/// justify.
#[test]
fn every_entry_says_what_the_build_time_frontend_answers() {
    let values = declared_values();
    assert!(
        values.contains(VERDICT_ANSWERS)
            && values.contains(VERDICT_DIVERGES)
            && values.contains(VERDICT_UNMEASURED),
        "{DIVERGENCES_JSON} declares `{FIELD}.values` as {values:?}, and this lane \
         derives {VERDICT_ANSWERS:?}, {VERDICT_DIVERGES:?} and {VERDICT_UNMEASURED:?} \
         from the lowered measurement. A verdict this lane can produce and the file \
         does not admit would be rejected as a misspelling; one the file admits and \
         this lane cannot produce is a value nothing can ever assign."
    );

    let missing: Vec<String> = divergences()
        .iter()
        .filter(|e| e.get(FIELD).and_then(|v| v.as_str()).is_none())
        .map(|e| {
            let (s, c) = key(e);
            format!("  {s}  ({c})")
        })
        .collect();
    assert!(
        missing.is_empty(),
        "{} entr(ies) in {DIVERGENCES_JSON} carry no `{FIELD}`. The verdict says what \
         the OTHER route into the Lua engine already answers, which is what a reader \
         deciding whether to fund the lowered seam is actually asking; an entry \
         without one is counted in the list's length and in none of its three \
         answers.\n{}",
        missing.len(),
        missing.join("\n"),
    );

    let unknown: Vec<String> = divergences()
        .iter()
        .filter_map(|e| {
            let v = e.get(FIELD)?.as_str()?;
            if values.contains(v) {
                return None;
            }
            let (s, c) = key(e);
            Some(format!("  {s}  ({c}) → {v:?}"))
        })
        .collect();
    assert!(
        unknown.is_empty(),
        "{} entr(ies) name a `{FIELD}` that {DIVERGENCES_JSON} does not declare in \
         `{FIELD}.values`. A spelling no lane derives is a claim nothing can \
         fault.\n{}",
        unknown.len(),
        unknown.join("\n"),
    );
}

/// Each verdict is the lowered measurement, re-derived — in both directions.
///
/// This is the assertion that keeps the field from being an opinion. The two
/// arrays it derives from are themselves held against the running engine by
/// `LoweredEcma262Test`, in both directions, so the chain from one of these
/// three words back to an execution contains no prose.
///
/// Both directions here too, and the second one matters more: a verdict that
/// says `unmeasured` for a case the rewriter has stopped mangling is an entry
/// claiming to be unknowable while the suite beside it can now ask it, which
/// is how a measurement quietly shrinks.
#[test]
fn every_verdict_is_the_lowered_measurement_re_derived() {
    let unreachable = lowered_keys("unaskable");
    let diverging = lowered_keys("divergences");

    let wrong: Vec<String> = divergences()
        .iter()
        .filter_map(|entry| {
            let declared = entry.get(FIELD)?.as_str()?;
            let k = key(entry);
            let derived = derived_verdict(&k, &unreachable, &diverging);
            if declared == derived {
                return None;
            }
            Some(format!(
                "  {}  ({})\n    says {declared:?}, {LOWERED_JSON} makes it {derived:?}",
                k.0, k.1
            ))
        })
        .collect();

    assert!(
        wrong.is_empty(),
        "{} `{FIELD}` verdict(s) disagree with the lowered measurement they are \
         derived from.\n\
         The verdict is not a judgement anyone makes here: `answers` is a case \
         absent from BOTH arrays of {LOWERED_JSON}, `diverges` is one in its \
         `divergences`, `unmeasured` is one in its `unreachable` — and \
         `LoweredEcma262Test` holds both arrays against this backend's running Lua \
         engine in both directions. So a disagreement is either a stale label or a \
         suite that has re-measured, and both are read the same way: re-derive the \
         label.\n{}",
        wrong.len(),
        wrong.join("\n"),
    );
}

/// The stated tally is the list it summarises.
///
/// The number is the thing a reader takes away, and every count in this
/// repository that was typed rather than derived has been wrong within two
/// growths of the table under it — "27 of its 58" over a table of 98, `58/58`
/// on a scoreboard, "58-case" in a gate's own comment. This is that lesson
/// applied to the count this round produced, on the day it produced it.
#[test]
fn the_declared_tally_is_the_list_it_summarises() {
    let declared = declared_tally();
    let actual = actual_tally();
    assert_eq!(
        declared, actual,
        "{DIVERGENCES_JSON}'s `{FIELD}.measured` states {declared:?} and the entries \
         under it count {actual:?}. The tally is a summary of the list, so the list \
         is what to re-count — not the summary that disagrees with it."
    );

    let total: i64 = actual.values().sum();
    assert_eq!(
        total,
        divergences().len() as i64,
        "the three verdicts account for {total} entr(ies) and {DIVERGENCES_JSON} \
         holds {}. Every entry is in exactly one verdict, so a total that is short \
         means an entry is in none of them.",
        divergences().len(),
    );
}

/// The seam document states the split, and states the one this list holds.
///
/// The document is where a person meets this number — the lists are read by
/// suites, the document by whoever is deciding what to build next — so it is
/// the copy most worth holding to the tree, and the least likely to be
/// re-derived by hand. Found by an anchor comment rather than by a heading:
/// the heading is prose, and a table located by prose stops being checked the
/// first time somebody rewords it.
#[test]
fn the_seam_document_carries_the_split_this_list_measures() {
    let doc = read(SEAM_DOC);
    let at = doc.find(SEAM_ANCHOR).unwrap_or_else(|| {
        panic!(
            "{SEAM_DOC} carries no `{SEAM_ANCHOR}` anchor. The split is a measurement \
             a reader consults before deciding whether to move Kotlin's translation \
             to build time, and a document that states it without being held to it is \
             the prose count this lane exists to have replaced."
        )
    });

    // The table is the rows immediately under the anchor, and it ends at the
    // first line that is not one. Reading to the end of the file instead would
    // let a later table's numbers answer for this one.
    let mut stated: BTreeMap<String, i64> = BTreeMap::new();
    for line in doc[at..].lines().skip(1) {
        let trimmed = line.trim();
        if trimmed.is_empty() || !trimmed.starts_with('|') {
            if stated.is_empty() {
                continue;
            }
            break;
        }
        let cells: Vec<&str> = trimmed
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect();
        if cells.len() < 2 {
            continue;
        }
        let verdict = cells[0].trim_matches('`').trim_matches('*').trim();
        let Ok(count) = cells[1].trim_matches('*').trim().parse::<i64>() else {
            continue;
        };
        stated.insert(verdict.to_string(), count);
    }

    let mut expected = actual_tally();
    expected.insert("total".to_string(), divergences().len() as i64);

    assert_eq!(
        stated, expected,
        "the table under `{SEAM_ANCHOR}` in {SEAM_DOC} states {stated:?} and \
         {DIVERGENCES_JSON} measures {expected:?}. The document is the copy a person \
         reads; re-derive it from the list rather than the other way round — \
         `python3 -c \"import json,collections; d=json.load(open('{DIVERGENCES_JSON}')); \
         print(collections.Counter(e['{FIELD}'] for e in d['divergences']))\"`."
    );
}

/// The day the seam opens, this field is a second spelling of `diverges_on` —
/// so it fails on that day rather than surviving as one.
///
/// This is the honest answer to "is there a path by which these counts reach
/// zero". There is, and it is not each verdict being repaired one at a time:
/// it is `build-time-lowering` becoming a path this backend HAS, at which
/// point every one of these three answers belongs in `diverges_on` beside the
/// path it is about — `answers` as an entry that names only `runtime-rewriter`,
/// `diverges` as one that names both. A field that survived that migration
/// would be a parallel vocabulary for the same fact, and the two would drift.
///
/// `ecma262_scoreboard_contract` already goes red on the same day, asking
/// which path each entry is about. This one names what to do with the answer
/// this round measured, so the two reds together are the migration.
#[test]
fn the_field_retires_when_the_seam_opens() {
    if !seam_is_open() {
        return;
    }
    let carried: Vec<String> = divergences()
        .iter()
        .filter(|e| e.get(FIELD).is_some())
        .map(|e| {
            let (s, c) = key(e);
            format!("  {s}  ({c})")
        })
        .collect();
    assert!(
        carried.is_empty(),
        "`Language::Kotlin.supports_script_engine_target(Lua)` is now true, so \
         `build-time-lowering` is a path this backend HAS and `diverges_on` can say \
         what {} entr(ies) still spell in `{FIELD}`. Move each verdict there — an \
         `{VERDICT_ANSWERS}` entry names only `runtime-rewriter`, a \
         `{VERDICT_DIVERGES}` entry names both paths, an `{VERDICT_UNMEASURED}` one \
         is now measurable and has a real answer — then delete `{FIELD}` and the \
         `{FIELD}` block above the list. Keeping both is two vocabularies for one \
         fact, which is the drift this file was written to end.\n{}",
        carried.len(),
        carried.join("\n"),
    );
}
