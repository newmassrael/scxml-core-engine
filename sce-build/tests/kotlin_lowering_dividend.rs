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

/// The key `build_time_frontend` USED to live under on each entry.
///
/// Retired 2026-08-29 when the seam opened. It survives here as the name of
/// the thing that must not come back — see the second half of
/// `every_entry_says_what_the_build_time_frontend_answers`.
const FIELD: &str = "build_time_frontend";

/// The two routes into this backend's Lua engine, which is what an entry's
/// `diverges_on` names — and, since the seam opened, what carries the verdict
/// this lane re-derives.
const PATH_RUNTIME_REWRITER: &str = "runtime-rewriter";
const PATH_BUILD_TIME_LOWERING: &str = "build-time-lowering";

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

/// The vocabulary this lane derives, which is what an entry's verdict may be.
///
/// Hard-coded here since the seam opened, and that is a change of OWNER rather
/// than a loss of one. While the verdict lived in its own field the file
/// declared the spellings, so the file and this lane could not accept
/// different ones; now the verdict IS the entry's `diverges_on`, whose legal
/// values are the `paths` array — already derived from the backend by
/// `ecma262_scoreboard_contract::every_list_declares_the_paths_its_backend_actually_has`.
/// Declaring the three words twice would be the second vocabulary this
/// migration removed.
fn declared_values() -> BTreeSet<String> {
    [VERDICT_ANSWERS, VERDICT_DIVERGES, VERDICT_UNMEASURED]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

/// The verdict an entry SAYS, read off the paths it names.
///
/// The mapping is the migration `the_field_retires_when_the_seam_opens`
/// prescribed: an entry the build-time frontend answers is wrong on the
/// runtime rewriter alone; an entry the frontend gets wrong too is wrong on
/// both. `unmeasured` has no shape here — a case the lowered route cannot ask
/// is not a set of paths — so an entry can never spell it, which is exactly
/// right now that the lowered entry point exists and every case is askable.
fn declared_verdict(entry: &serde_json::Value) -> Option<&'static str> {
    let paths: BTreeSet<String> = entry
        .get("diverges_on")?
        .as_array()?
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    let runtime_only: BTreeSet<String> = [PATH_RUNTIME_REWRITER.to_string()].into_iter().collect();
    let both: BTreeSet<String> = [
        PATH_BUILD_TIME_LOWERING.to_string(),
        PATH_RUNTIME_REWRITER.to_string(),
    ]
    .into_iter()
    .collect();
    if paths == runtime_only {
        Some(VERDICT_ANSWERS)
    } else if paths == both {
        Some(VERDICT_DIVERGES)
    } else {
        None
    }
}

/// What the list itself counts, per verdict, including the verdicts that are
/// derivable but unused — a vocabulary entry at zero is a real answer and has
/// to appear in the tally rather than be omitted from it.
fn actual_tally() -> BTreeMap<String, i64> {
    let mut counts: BTreeMap<String, i64> = declared_values().into_iter().map(|v| (v, 0)).collect();
    for entry in divergences() {
        if let Some(v) = declared_verdict(&entry) {
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
    let unreadable: Vec<String> = divergences()
        .iter()
        .filter(|e| declared_verdict(e).is_none())
        .map(|e| {
            let (s, c) = key(e);
            let paths = e
                .get("diverges_on")
                .map(|p| p.to_string())
                .unwrap_or_else(|| "absent".to_string());
            format!("  {s}  ({c}) → {paths}")
        })
        .collect();
    assert!(
        unreadable.is_empty(),
        "{} entr(ies) in {DIVERGENCES_JSON} name a `diverges_on` this lane cannot read \
         as a verdict. Since the seam opened there are exactly two shapes an entry can \
         have: [{PATH_RUNTIME_REWRITER:?}] means the build-time frontend answers this \
         case and the seam retires it, and both paths mean the frontend gets it wrong \
         too. A third shape is a claim about the lowered route that \
         {LOWERED_JSON} never measured.\n{}",
        unreadable.len(),
        unreadable.join("\n"),
    );

    // Neither vocabulary survives the other. The retired field would be a
    // second spelling of the verdict, free to disagree with the paths beside
    // it — which is the drift `the_field_retires_when_the_seam_opens` fired to
    // end, and which nothing else would notice if it crept back one entry at a
    // time.
    let revived: Vec<String> = divergences()
        .iter()
        .filter(|e| e.get(FIELD).is_some())
        .map(|e| {
            let (s, c) = key(e);
            format!("  {s}  ({c})")
        })
        .collect();
    assert!(
        revived.is_empty(),
        "{} entr(ies) carry `{FIELD}` again. It retired when \
         `supports_script_engine_target(Lua)` became true and `diverges_on` started \
         carrying the same fact; two vocabularies for one fact is what this file's \
         own header calls the drift it was written to end.\n{}",
        revived.len(),
        revived.join("\n"),
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
            let declared = declared_verdict(entry)?;
            let k = key(entry);
            let derived = derived_verdict(&k, &unreachable, &diverging);
            if declared == derived {
                return None;
            }
            Some(format!(
                "  {}  ({})\n    its `diverges_on` says {declared:?}, {LOWERED_JSON} \
                 makes it {derived:?}",
                k.0, k.1
            ))
        })
        .collect();

    assert!(
        wrong.is_empty(),
        "{} `diverges_on` verdict(s) disagree with the lowered measurement they are \
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
    let actual = actual_tally();
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
