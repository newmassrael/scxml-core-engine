// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The judgment of a revision of a requirement list.
//!
//! A requirement keeps its id when its specification is revised because a lineage remembers which
//! id was issued for which requirement (`docs/adr/0006-a-requirement-keeps-its-id-across-a-
//! revision.md`), and a work keeps that lineage beside its list (`docs/adr/0011-a-work-keeps-its-
//! requirement-lineage-with-its-requirement-list.md`). What the lineage means is judged here:
//!
//! * [`parse`] and [`check`]: is this a lineage?
//! * [`extends`]: does it continue the one a work holds? An id is issued once and a retired one
//!   never lives again, so a lineage that took anything back is refused;
//! * [`belongs_to_list`]: is it the lineage of THIS list, its manifest and its sidecar?
//! * [`between`]: what became of each requirement's words between two states of one lineage;
//! * [`belongs_to`]: is a words delta the one of the revision an acceptance was taken against?
//! * [`join`] and [`render`]: the words of each requirement joined with what the design's
//!   evidence did, and the page an owner reads of it.
//!
//! The Python in `tools/authoring/sce_author/requirement_lineage.py` is the reference, and this is
//! held to it by the cases it writes (`sce-build/tests/fixtures/revision_judgment/cases.json`,
//! read by `tests/the_cases_the_python_gives.rs`). A refusal's sentence is part of the judgment,
//! because it is the word a person is told: the same refusal in other words has changed what they
//! are told. So the sentences here are the Python's, and a lineage is a [`serde_json::Value`] and
//! not a struct: what `extends` is asked about may be a lineage that is not well formed (a case
//! that takes a revision away leaves requirements naming a revision that is gone), and the
//! refusal it gives is its own, not the one [`check`] would give.

use std::collections::{BTreeMap, HashSet};
use std::fmt;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

mod join;
mod revision;

pub use join::{delta_object, join, render, SHARED};
pub use revision::belongs_to;

/// What a lineage says it is.
pub const LINEAGE_KIND: &str = "sce-requirement-lineage";
/// The version of the format this reads.
pub const LINEAGE_VERSION: i64 = 1;

const REVISION_KEYS: [&str; 5] = [
    "rev",
    "spec_sha256",
    "sentence_sha256",
    "manifest_sha256",
    "sidecar_sha256",
];
/// The two digests that name the list a revision was written as. The manifest is coordinates only
/// (an id, a section, a modality), so two lists of one shape and different sentences share its
/// digest; the sidecar's, the words behind the ids, is what tells them apart.
const LIST_DIGESTS: [&str; 2] = ["manifest_sha256", "sidecar_sha256"];

static NULL: Value = Value::Null;

/// A lineage that cannot be used, or a judgment that refuses one, in the sentence a person is told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineageError {
    message: String,
}

impl LineageError {
    fn new(message: impl Into<String>) -> Self {
        LineageError {
            message: message.into(),
        }
    }

    /// The sentence.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for LineageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for LineageError {}

pub(crate) type Judgment<T> = Result<T, LineageError>;

pub(crate) fn refuse<T>(message: impl Into<String>) -> Judgment<T> {
    Err(LineageError::new(message))
}

/// The digest of a text, as the lineage and an acceptance record write it.
pub fn sha256_text(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

// -- how a value is written into a sentence ---------------------------------------------------

/// A value as the sentences write it with `{}`: a text as itself, nothing as `None`, a truth
/// value as `True` or `False`.
pub(crate) fn show(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Null => "None".to_string(),
        Value::Bool(true) => "True".to_string(),
        Value::Bool(false) => "False".to_string(),
        other => other.to_string(),
    }
}

/// A value as the sentences write it with `{!r}`: a text in quotes.
fn repr(value: &Value) -> String {
    match value {
        Value::String(text) if text.contains('\'') && !text.contains('"') => format!("\"{text}\""),
        Value::String(text) => format!("'{}'", text.replace('\\', "\\\\").replace('\'', "\\'")),
        other => show(other),
    }
}

fn malformed(what: &str) -> LineageError {
    LineageError::new(format!("the lineage is malformed: {what}"))
}

fn array<'a>(value: &'a Value, what: &str) -> Judgment<&'a [Value]> {
    value
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| malformed(what))
}

fn whole_number(value: &Value) -> Option<i128> {
    value
        .as_i64()
        .map(i128::from)
        .or_else(|| value.as_u64().map(i128::from))
}

/// `Some(n)` for an id spelt `R` and digits alone.
fn id_number(id: &str) -> Option<u128> {
    let digits = id.strip_prefix('R')?;
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    Some(digits.parse().unwrap_or(u128::MAX))
}

/// Ids in the order a reader expects: `R2` before `R10`, and what is not spelt that way last.
fn id_order(id: &str) -> (u8, u128, String) {
    match id_number(id) {
        Some(number) => (0, number, id.to_string()),
        None => (1, 0, id.to_string()),
    }
}

fn last_quote(row: &Value) -> &Value {
    row["quotes"]
        .as_array()
        .and_then(|quotes| quotes.last())
        .unwrap_or(&NULL)
}

// -- reading ----------------------------------------------------------------------------------

/// A lineage read back from its text, or the reason it cannot be used.
pub fn parse(text: &str) -> Judgment<Value> {
    // No parser detail in the sentence: it is one library's wording, and the sentence is a case
    // another implementation has to say in the same words.
    let data: Value =
        serde_json::from_str(text).map_err(|_| LineageError::new("the lineage is not JSON"))?;
    check(&data)?;
    Ok(data)
}

/// Whether `lineage` is one this format writes, else why not.
pub fn check(lineage: &Value) -> Judgment<()> {
    const KEYS: [&str; 6] = [
        "doc_id",
        "lineage",
        "next",
        "requirements",
        "revisions",
        "v",
    ];
    let well_formed = lineage.as_object().is_some_and(|object| {
        object.len() == KEYS.len() && KEYS.iter().all(|key| object.contains_key(*key))
    });
    if !well_formed {
        return refuse(format!(
            "not a requirement lineage: it has to be an object with exactly {}",
            KEYS.join(", ")
        ));
    }
    if lineage["lineage"] != json!(LINEAGE_KIND) || lineage["v"] != json!(LINEAGE_VERSION) {
        return refuse(format!("not a v{LINEAGE_VERSION} {LINEAGE_KIND}"));
    }
    if !lineage["doc_id"]
        .as_str()
        .is_some_and(|name| !name.is_empty())
    {
        return refuse("the lineage has no doc_id");
    }
    let revisions = match lineage["revisions"].as_array() {
        Some(revisions) if !revisions.is_empty() => revisions,
        _ => return refuse("the lineage holds no revision"),
    };
    let mut seen_revs: Vec<&str> = Vec::new();
    for row in revisions {
        // The digests of the list are written always and may be absent from a lineage made before
        // they were recorded, which says the list is unknown, as null does.
        let valid = row.as_object().is_some_and(|row| {
            ["rev", "spec_sha256", "sentence_sha256"]
                .iter()
                .all(|key| row.contains_key(*key))
                && row.keys().all(|key| REVISION_KEYS.contains(&key.as_str()))
                && row["rev"].is_string()
                && row["sentence_sha256"].is_array()
                && matches!(row["spec_sha256"], Value::Null | Value::String(_))
                && LIST_DIGESTS.iter().all(|key| {
                    matches!(
                        row.get(*key),
                        None | Some(Value::Null) | Some(Value::String(_))
                    )
                })
        });
        if !valid {
            return refuse(
                "a revision row of the lineage is not {rev, spec_sha256, sentence_sha256, \
                 manifest_sha256, sidecar_sha256}",
            );
        }
        seen_revs.push(row["rev"].as_str().unwrap_or_default());
    }
    if seen_revs.iter().collect::<HashSet<_>>().len() != seen_revs.len() {
        return refuse("the lineage names a rev twice");
    }
    let Some(rows) = lineage["requirements"].as_array() else {
        return refuse("the lineage's requirements are not a list");
    };
    let mut ids: HashSet<&str> = HashSet::new();
    let mut top: u128 = 0;
    for row in rows {
        let valid = row.as_object().is_some_and(|object| {
            object.len() == 4
                && ["id", "first_rev", "retired_rev", "quotes"]
                    .iter()
                    .all(|key| object.contains_key(*key))
        }) && row["id"].is_string()
            && row["quotes"]
                .as_array()
                .is_some_and(|quotes| !quotes.is_empty());
        if !valid {
            return refuse(
                "a requirement row of the lineage is not {id, first_rev, retired_rev, quotes}",
            );
        }
        let id = row["id"].as_str().unwrap_or_default();
        if !ids.insert(id) {
            return refuse(format!("the lineage issues {id} twice"));
        }
        let quotes = array(&row["quotes"], "quotes")?;
        let refs = [&row["first_rev"], &row["retired_rev"]]
            .into_iter()
            .chain(quotes.iter().map(|quote| match quote {
                Value::Object(_) => &quote["rev"],
                _ => &NULL,
            }));
        for reference in refs {
            let known = reference
                .as_str()
                .is_some_and(|rev| seen_revs.contains(&rev));
            if !reference.is_null() && !known {
                return refuse(format!(
                    "{id} names rev {}, which the lineage has no row for",
                    repr(reference)
                ));
            }
        }
        for quote in quotes {
            let valid = quote
                .as_object()
                .is_some_and(|object| object.len() == 2 && object.contains_key("rev"))
                && quote["sha256"].is_string();
            if !valid {
                return refuse(format!("a quote row of {id} is not {{rev, sha256}}"));
            }
        }
        if let Some(number) = id_number(id) {
            top = top.max(number);
        }
    }
    let next = &lineage["next"];
    let past = !next.is_boolean()
        && whole_number(next).is_some_and(|next| next >= 0 && next as u128 > top);
    if !past {
        return refuse(format!(
            "the lineage's next ({}) is not past every id it issued (R{top}): an id would be \
             issued twice",
            repr(next)
        ));
    }
    Ok(())
}

// -- the lineage of a list that predates lineages --------------------------------------------

/// Whitespace collapsed to single spaces: a specification wrapped at another width, or a quote
/// copied across a line break, is the same words. What counts as the same words for every digest
/// the lineage keeps. (Python's `\s` also takes the four ASCII separators U+001C to U+001F, which
/// Unicode does not call whitespace and Rust does not collapse; no specification has one.)
pub fn normalise(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A lineage for a list that was built before lineages existed: the ids are the manifest's, the
/// words behind them are `texts` (the sidecar's, already normalised), the revision is the
/// manifest's. Whether the specification changed since cannot be told (its digest was never
/// kept), so the next revision is a new one. The digests are those of the manifest and sidecar
/// texts it was adopted from, which the lineage keeps as that revision's list.
pub fn adopt(
    doc_id: &Value,
    manifest: &Value,
    texts: &BTreeMap<String, String>,
    manifest_sha256: Option<&str>,
    sidecar_sha256: Option<&str>,
) -> Judgment<Value> {
    if manifest["doc_id"] != *doc_id {
        return refuse(format!(
            "the previous manifest is for '{}', not '{}'",
            show(&manifest["doc_id"]),
            show(doc_id)
        ));
    }
    if manifest["extraction"]["ids"] != "synthesized" || !manifest["extraction"].is_object() {
        return refuse(
            "the previous manifest carries the source's own ids; nothing here renumbers those, \
             and nothing here would carry them either",
        );
    }
    let rev = &manifest["rev"];
    let whole = rev
        .as_str()
        .is_some_and(|rev| !rev.is_empty() && rev.bytes().all(|byte| byte.is_ascii_digit()));
    if !whole {
        return refuse(format!(
            "the previous manifest's rev {} is not a whole number, so the next one cannot be told",
            repr(rev)
        ));
    }
    let mut rows: Vec<Value> = Vec::new();
    let mut top: u128 = 0;
    let entries = manifest["requirements"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    for entry in entries {
        let id = &entry["id"];
        let quote = id
            .as_str()
            .and_then(|id| texts.get(id))
            .filter(|quote| !quote.is_empty());
        let (Some(id_text), Some(quote)) = (id.as_str(), quote) else {
            return refuse(format!(
                "the previous sidecar has no words for the previous manifest's '{}'",
                show(id)
            ));
        };
        if let Some(number) = id_number(id_text) {
            top = top.max(number);
        }
        rows.push(json!({
            "id": id_text, "first_rev": rev, "retired_rev": Value::Null,
            "quotes": [{ "rev": rev, "sha256": sha256_text(quote) }],
        }));
    }
    if rows.is_empty() {
        return refuse("the previous manifest lists no requirements");
    }
    let lineage = json!({
        "lineage": LINEAGE_KIND, "v": LINEAGE_VERSION, "doc_id": doc_id,
        "next": u64::try_from(top + 1).unwrap_or(u64::MAX),
        "revisions": [{
            "rev": rev, "spec_sha256": Value::Null, "sentence_sha256": [],
            "manifest_sha256": manifest_sha256, "sidecar_sha256": sidecar_sha256,
        }],
        "requirements": rows,
    });
    check(&lineage)?;
    Ok(lineage)
}

/// The lineage of a list a work kept: the one saved with it, or, for a list made before
/// lineages, the one adopting it makes (the ids it has, the words behind them from its sidecar),
/// which is what a later list built against it was given in the first place. Refuses a list that
/// gives neither, saying what to do. `held` carries the texts of the list (`manifest_text`,
/// `sidecar_text` and `lineage_text`, the last two absent when it keeps none) and `what` names
/// the list in the sentence.
pub fn of_list(held: &Value, what: &str) -> Judgment<Value> {
    if let Some(lineage) = held.get("lineage_text") {
        return parse(lineage.as_str().unwrap_or_default());
    }
    let Some(sidecar_text) = held.get("sidecar_text") else {
        return refuse(format!(
            "the list {what} keeps no lineage and no sidecar, so the words behind its ids are not \
             known"
        ));
    };
    let (manifest_text, sidecar_text) = (
        held["manifest_text"].as_str().unwrap_or_default(),
        sidecar_text.as_str().unwrap_or_default(),
    );
    // No parser detail in the sentence: it is one library's wording, and the sentence is a case
    // another implementation has to say in the same words.
    let (Ok(manifest), Ok(sidecar)) = (
        serde_json::from_str::<Value>(manifest_text),
        serde_json::from_str::<Value>(sidecar_text),
    ) else {
        return refuse(format!("the list {what} is not JSON"));
    };
    if !manifest.is_object() || !sidecar.is_object() {
        return refuse(format!("the list {what} is not a pair of JSON objects"));
    }
    let texts: BTreeMap<String, String> = sidecar["text"]
        .as_object()
        .map(|words| {
            words
                .iter()
                .filter_map(|(id, text)| text.as_str().map(|text| (id.clone(), normalise(text))))
                .collect()
        })
        .unwrap_or_default();
    adopt(
        &manifest["doc_id"],
        &manifest,
        &texts,
        Some(&sha256_text(manifest_text)),
        Some(&sha256_text(sidecar_text)),
    )
}

// -- does it continue the work's -------------------------------------------------------------

/// Refuse a lineage that is not the continuation of `previous`.
///
/// A lineage is only ever appended to: nothing it said is taken back. An id once issued is still
/// there, with the words it had, and a retired id stays retired and never lives again; the
/// revisions are the same revisions with more after them; and the ids issued since are numbered
/// from where `previous` left off, so that none is issued twice. What `previous` was is the work's;
/// nothing here reads it from a file.
///
/// Two things may change in the last revision's row and the quotes it made, and only there: the
/// same text re-read into a different list is the same revision with another manifest and
/// sidecar (so the digests differ), and a requirement re-quoted within it has its last quote
/// replaced.
pub fn extends(previous: &Value, following: &Value) -> Judgment<()> {
    if previous["doc_id"] != following["doc_id"] {
        return refuse(format!(
            "the lineage is of '{}' and the work's is of '{}': one lineage follows one \
             specification",
            show(&following["doc_id"]),
            show(&previous["doc_id"])
        ));
    }
    let (previous_next, following_next) = (
        whole_number(&previous["next"]).ok_or_else(|| malformed("next"))?,
        whole_number(&following["next"]).ok_or_else(|| malformed("next"))?,
    );
    if following_next < previous_next {
        return refuse(format!(
            "the lineage's next ({following_next}) is behind the work's ({previous_next}): ids \
             already issued would be issued again"
        ));
    }
    let before = array(&previous["revisions"], "revisions")?;
    let after = array(&following["revisions"], "revisions")?;
    if after.len() < before.len() {
        return refuse(format!(
            "the lineage has {} revision(s) and the work's has {}: revisions are never taken back",
            after.len(),
            before.len()
        ));
    }
    for (position, row) in before.iter().enumerate() {
        let now = &after[position];
        let same_text_again = position == before.len() - 1 && after.len() == before.len();
        let kept = row.as_object().ok_or_else(|| malformed("a revision row"))?;
        let kept_as_it_was = kept
            .iter()
            .filter(|(key, _)| !(same_text_again && LIST_DIGESTS.contains(&key.as_str())))
            .all(|(key, value)| now.get(key) == Some(value));
        if !kept_as_it_was {
            return refuse(format!(
                "revision {} of the lineage is not the revision the work holds: a revision is \
                 not rewritten",
                show(&row["rev"])
            ));
        }
    }
    let last_rev = &before.last().ok_or_else(|| malformed("no revision"))?["rev"];
    let held: BTreeMap<&str, &Value> = array(&following["requirements"], "requirements")?
        .iter()
        .filter_map(|row| row["id"].as_str().map(|id| (id, row)))
        .collect();
    let previous_rows = array(&previous["requirements"], "requirements")?;
    for row in previous_rows {
        let id = show(&row["id"]);
        let Some(now) = row["id"].as_str().and_then(|id| held.get(id)) else {
            return refuse(format!(
                "{id} is in the work's lineage and not in this one: an id is never forgotten"
            ));
        };
        if now["first_rev"] != row["first_rev"] {
            return refuse(format!(
                "{id} was first issued in revision {} and this lineage says {}",
                show(&row["first_rev"]),
                show(&now["first_rev"])
            ));
        }
        if !row["retired_rev"].is_null() && now["retired_rev"] != row["retired_rev"] {
            return refuse(format!(
                "{id} was retired in revision {} and this lineage makes it live again: a retired \
                 id is never issued again",
                show(&row["retired_rev"])
            ));
        }
        let (old, new) = (
            array(&row["quotes"], "quotes")?,
            array(&now["quotes"], "quotes")?,
        );
        if old.is_empty() {
            return Err(malformed("a requirement with no quote"));
        }
        if new.len() < old.len() || old[..old.len() - 1] != new[..old.len() - 1] {
            return refuse(format!(
                "the words {id} has had are not the ones the work holds: a requirement's history \
                 is only added to"
            ));
        }
        let (edge, edge_now) = (&old[old.len() - 1], &new[old.len() - 1]);
        if edge != edge_now && !(edge["rev"] == edge_now["rev"] && edge["rev"] == *last_rev) {
            return refuse(format!(
                "the last words {id} had in revision {} are not the ones the work holds",
                show(&edge["rev"])
            ));
        }
    }
    let issued: HashSet<&str> = previous_rows
        .iter()
        .filter_map(|row| row["id"].as_str())
        .collect();
    let mut added: Vec<&str> = held
        .keys()
        .copied()
        .filter(|id| !issued.contains(id))
        .collect();
    added.sort_by_key(|id| id_order(id));
    for id in added {
        let from_the_next = match id_number(id) {
            Some(number) => i128::try_from(number).map_or(true, |number| number >= previous_next),
            None => false,
        };
        if !from_the_next {
            return refuse(format!(
                "{id} is new in this lineage and is not an id issued from the work's next \
                 (R{previous_next}): an id would be issued twice"
            ));
        }
    }
    Ok(())
}

// -- is it this list's -----------------------------------------------------------------------

/// Refuse a lineage that is not the lineage of this list, its manifest and its sidecar.
///
/// A list and its lineage are one fact, "which ids this list uses and which were ever issued", and
/// a save of one beside the other's of another revision would store a lineage whose last revision
/// names a list that is not the one beside it. The lineage names its last list by the digests of
/// the exact manifest text (`manifest_sha256`) and sidecar text (`sidecar_sha256`), the document
/// and the revision, and all are compared with what was handed over. The manifest alone is not
/// enough: it is coordinates only, so a list of another specification with the same shape has its
/// digest.
pub fn belongs_to_list(
    lineage: &Value,
    manifest_text: &str,
    sidecar_text: Option<&str>,
) -> Judgment<()> {
    let manifest: Value = serde_json::from_str(manifest_text)
        .map_err(|_| LineageError::new("the manifest is not JSON"))?;
    if !manifest.is_object() {
        return refuse("the manifest is not a JSON object");
    }
    let last = array(&lineage["revisions"], "revisions")?
        .last()
        .ok_or_else(|| malformed("no revision"))?;
    if lineage["doc_id"] != manifest["doc_id"] {
        return refuse(format!(
            "the lineage is of '{}' and the manifest of '{}'",
            show(&lineage["doc_id"]),
            show(&manifest["doc_id"])
        ));
    }
    if last["rev"] != manifest["rev"] {
        return refuse(format!(
            "the lineage's last revision is {} and the manifest's is {}",
            show(&last["rev"]),
            show(&manifest["rev"])
        ));
    }
    let Some(manifest_digest) = last["manifest_sha256"].as_str() else {
        return refuse(
            "the lineage does not say which manifest its last revision was written as (it \
             predates recording the digest, or was adopted without the manifest): build the list \
             again with scxml_requirement_set",
        );
    };
    if manifest_digest != sha256_text(manifest_text) {
        return refuse(
            "the lineage was made with another manifest than this one (the digests differ): give \
             the manifest_text and the lineage_text of one call of scxml_requirement_set, \
             unchanged",
        );
    }
    let Some(sidecar_digest) = last["sidecar_sha256"].as_str() else {
        return refuse(
            "the lineage does not say which sidecar its last revision was written as (it \
             predates recording the digest, or was adopted without the sidecar): build the list \
             again with scxml_requirement_set",
        );
    };
    let Some(sidecar_text) = sidecar_text else {
        return refuse(
            "the lineage names the sidecar its last revision was written as and no sidecar was \
             given: give the sidecar_text of the same call of scxml_requirement_set, unchanged",
        );
    };
    if sidecar_digest != sha256_text(sidecar_text) {
        return refuse(
            "the lineage was made with another sidecar than this one (the digests differ): give \
             the sidecar_text and the lineage_text of one call of scxml_requirement_set, \
             unchanged",
        );
    }
    Ok(())
}

// -- what became of the words ----------------------------------------------------------------

/// What happened to each requirement's WORDS between two states of one work's lineage: the
/// `delta` that `scxml_requirement_set` would have returned had the revisions been built one after
/// the other, derived from the two lineages alone.
///
/// `older` is the lineage of the list an acceptance was taken of and `newer` the lineage of the
/// list the work has now. `newer` has to continue `older` ([`extends`]): a lineage of another
/// history says nothing about what became of the requirements of this one. An id that is live in
/// both is `carried` when the words it last had are the same and `changed` when they are not; one
/// live in `older` and retired in `newer` is `retired`; one live in `newer` and not in `older` is
/// `new`. One issued AND retired between the two is not listed: no design accepted at `older` can
/// cite it, and none now should.
///
/// Any two states can be compared, not only neighbours. A requirement reworded and reworded back
/// is `carried`, as it is the same words as it was accepted with.
pub fn between(older: &Value, newer: &Value) -> Judgment<Value> {
    extends(older, newer)?;
    let old_rows: BTreeMap<&str, &Value> = array(&older["requirements"], "requirements")?
        .iter()
        .filter_map(|row| row["id"].as_str().map(|id| (id, row)))
        .collect();
    let mut rows: Vec<&Value> = array(&newer["requirements"], "requirements")?
        .iter()
        .collect();
    rows.sort_by_key(|row| id_order(row["id"].as_str().unwrap_or_default()));
    let (mut carried, mut changed, mut new, mut retired) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for row in rows {
        let id = row["id"].clone();
        let was = row["id"].as_str().and_then(|id| old_rows.get(id));
        let live_now = row["retired_rev"].is_null();
        let live_then = was.is_some_and(|was| was["retired_rev"].is_null());
        match was {
            Some(was) if live_then && live_now => {
                if last_quote(was)["sha256"] == last_quote(row)["sha256"] {
                    carried.push(id);
                } else {
                    changed.push(json!({ "id": id, "how": "lineage" }));
                }
            }
            _ if live_then => retired.push(id),
            // Not live then and live now is an id issued since: `extends` has refused a retired
            // id that lives again, so there is no other way to be here.
            _ if live_now => new.push(id),
            _ => {}
        }
    }
    let first = array(&older["revisions"], "revisions")?
        .last()
        .ok_or_else(|| malformed("no revision"))?;
    let last = array(&newer["revisions"], "revisions")?
        .last()
        .ok_or_else(|| malformed("no revision"))?;
    Ok(json!({
        "doc_id": newer["doc_id"],
        "from_rev": first["rev"],
        "from_manifest_sha256": first["manifest_sha256"],
        "from_sidecar_sha256": first["sidecar_sha256"],
        "rev": last["rev"],
        "specification_changed": first["spec_sha256"] != last["spec_sha256"],
        "requirements": {
            "carried": carried,
            "changed": changed,
            "new": new,
            "retired": retired,
        },
    }))
}
