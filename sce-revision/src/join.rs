// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Join what a revision did to the WORDS of each requirement with what it did to the design's
//! EVIDENCE, and say whether the revision stayed within its reach
//! (`docs/adr/0009-a-revision-stays-within-the-reach-of-what-changed.md`).
//!
//! Two facts exist about every requirement of a revised specification and neither is enough
//! alone:
//!
//! * words: carried / changed / new / retired, the delta of the revised list built against its
//!   lineage ([`crate::between`] derives it from two lineages);
//! * evidence: unchanged / changed / new / dropped, the accepted record against the design now.
//!
//! A requirement whose words did not change and whose evidence did is a design that moved with
//! nothing asking it to; one whose words changed and whose evidence did not is a design that may
//! not have heard. Joined by requirement id the two are what an owner needs to look at again, and
//! the rest, both read the same, is what they do not. The Python reference is
//! `tools/authoring/sce_author/revision.py`; the sentences and the page are its own, because
//! they are what a person is told.
//!
//! `carries-over` means the product's closure of what a requirement depends on reads the same,
//! not that the requirement is met, and an acceptance still lapses by its bytes. A requirement
//! that no node cites, in the record or now, has no closure to read: it is `uncited` and
//! `uncovered`, never `carries-over`.
//!
//! A carried requirement whose evidence moved is a `moved-without-reason` violation unless every
//! place it moved is one that a requirement whose words changed (or that is new) also stands on:
//! a node is cited by several requirements, so asking one to change moves the others' evidence,
//! and that case is `moved-with-a-changed-neighbour`, a `look` that names the neighbour.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{json, Map, Value};

use crate::{refuse, Judgment};

const EVIDENCE: [&str; 4] = ["unchanged", "changed", "new", "dropped"];

const OK: &str = "ok";
const LOOK: &str = "look";
const VIOLATION: &str = "violation";
const UNCOVERED: &str = "uncovered";

/// A carried requirement whose evidence moved only where a changed neighbour also stands.
pub const SHARED: &str = "moved-with-a-changed-neighbour";

/// (how the words delta lists an id, what the evidence line says) -> (kind, severity). `None` is
/// "no evidence line": a requirement no node cites in the record or in the design.
fn classify(words: &str, evidence: Option<&str>) -> (&'static str, &'static str) {
    match (words, evidence) {
        ("carried", Some("unchanged")) => ("carries-over", OK),
        ("carried", Some("changed" | "dropped")) => ("moved-without-reason", VIOLATION),
        ("carried", Some("new")) => ("newly-cited", LOOK),
        ("carried", None) => ("uncited", UNCOVERED),
        ("changed", Some("changed" | "new")) => ("revised", OK),
        ("changed", Some("unchanged")) => ("words-changed-design-same", LOOK),
        ("changed", Some("dropped") | None) => ("changed-but-uncited", LOOK),
        ("new", Some("new" | "changed")) => ("implemented-new", OK),
        ("new", Some("unchanged" | "dropped") | None) => ("unimplemented-new", LOOK),
        ("retired", Some("dropped") | None) => ("retired-cleanly", OK),
        ("retired", Some("unchanged" | "changed" | "new")) => ("retired-still-cited", VIOLATION),
        _ => ("unlisted", LOOK),
    }
}

/// The lines `acceptance-delta` writes (a line per requirement, the rows that claim nothing, a
/// summary) as the one object [`join`] reads, whoever asked the product: the generator run on a
/// tree, or the application's command layer asking it of a work.
///
/// Gathered, not read again: what each requirement's evidence did is the product's to say.
pub fn delta_object(lines: &Value) -> Value {
    let lines = lines.as_array().map(Vec::as_slice).unwrap_or_default();
    let first_of = |kind: &str| lines.iter().find(|line| line["kind"] == kind);
    let pick = |line: &Value, keys: &[&str]| {
        let mut picked = Map::new();
        for key in keys {
            picked.insert((*key).to_string(), line[*key].clone());
        }
        Value::Object(picked)
    };
    let requirements: Vec<Value> = lines
        .iter()
        .filter(|line| line["kind"] == "acceptance-delta")
        .filter_map(|line| line.as_object())
        .map(|line| {
            Value::Object(
                line.iter()
                    .filter(|(key, _)| key.as_str() != "v" && key.as_str() != "kind")
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect(),
            )
        })
        .collect();
    json!({
        "version": 1,
        "summary": first_of("acceptance-delta-summary").map(|summary| pick(
            summary,
            &["requirements", "unchanged", "changed", "new", "dropped"],
        )),
        "requirements": requirements,
        "unclaimed": first_of("acceptance-delta-unclaimed")
            .map(|unclaimed| pick(unclaimed, &["added", "gone"])),
    })
}

/// requirement id -> its words status, from the delta the requirement-set tool returned.
fn words_of(delta: &Value) -> Judgment<BTreeMap<String, String>> {
    let Some(found) = delta["requirements"].as_object() else {
        return refuse(
            "the words `delta` has to be the object scxml_requirement_set returned for a \
             revision: it has no `requirements`. A first list has no delta, and there is nothing \
             to revise from",
        );
    };
    let mut words: BTreeMap<String, String> = BTreeMap::new();
    let mut put = |id: &Value, status: &str| -> Judgment<()> {
        let Some(id) = id.as_str().filter(|id| !id.is_empty()) else {
            return refuse(
                "the words delta names a requirement that is not an id (an id is a text and not \
                 empty)",
            );
        };
        if let Some(earlier) = words.get(id) {
            return refuse(format!(
                "the words delta names {id} twice ({earlier} and {status})"
            ));
        }
        words.insert(id.to_string(), status.to_string());
        Ok(())
    };
    for status in ["carried", "new", "retired"] {
        let Some(listed) = found.get(status).and_then(Value::as_array) else {
            return refuse(format!("the words delta has no `{status}` list"));
        };
        for id in listed {
            put(id, status)?;
        }
    }
    let Some(listed) = found.get("changed").and_then(Value::as_array) else {
        return refuse("the words delta has no `changed` list");
    };
    for entry in listed {
        match entry {
            Value::Object(_) => put(&entry["id"], "changed")?,
            other => put(other, "changed")?,
        }
    }
    Ok(words)
}

/// A value Python would take as nothing: absent, null, false, zero, empty text, list or object.
fn is_falsy(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::Bool(flag) => !flag,
        Value::Number(number) => number.as_f64() == Some(0.0),
        Value::String(text) => text.is_empty(),
        Value::Array(items) => items.is_empty(),
        Value::Object(fields) => fields.is_empty(),
    }
}

type Lines<'a> = (BTreeMap<String, &'a Value>, Value);

/// requirement id -> its evidence line, and the unclaimed line, from the acceptance delta.
fn evidence_of(delta: &Value) -> Judgment<Lines<'_>> {
    let Some(requirements) = delta["requirements"].as_array() else {
        return refuse(
            "the evidence delta has to be the object scxml_acceptance_delta returned: it has no \
             `requirements` list",
        );
    };
    let mut lines: BTreeMap<String, &Value> = BTreeMap::new();
    for (position, line) in requirements.iter().enumerate() {
        let id = line["requirement"].as_str();
        let known = line["evidence"]
            .as_str()
            .is_some_and(|evidence| EVIDENCE.contains(&evidence));
        let Some(id) = id.filter(|_| line.is_object() && known) else {
            // The position and not the line: a line written out is Python's notation again.
            return refuse(format!(
                "evidence line {} is not {{requirement, evidence}} with evidence one of {}",
                position + 1,
                EVIDENCE.join(", ")
            ));
        };
        if lines.insert(id.to_string(), line).is_some() {
            return refuse(format!("the evidence delta names {id} twice"));
        }
    }
    let unclaimed = if is_falsy(&delta["unclaimed"]) {
        json!({ "added": [], "gone": 0 })
    } else {
        delta["unclaimed"].clone()
    };
    let added_ok = unclaimed
        .as_object()
        .is_some_and(|object| matches!(object.get("added"), None | Some(Value::Array(_))));
    if !added_ok {
        return refuse("the evidence delta's `unclaimed` is not {added, gone}");
    }
    Ok((lines, unclaimed))
}

fn places_of(row: &Map<String, Value>, keys: &[&str]) -> Vec<String> {
    for key in keys {
        if let Some(places) = row.get(*key).and_then(Value::as_array) {
            if !places.is_empty() {
                return places.iter().map(crate::show).collect();
            }
        }
    }
    Vec::new()
}

/// Re-read, in place, the carried requirements that moved only at places a requirement whose
/// words DID change (or that is new) also stands on.
///
/// A design node is cited by several requirements at once, and the product's closure of each one
/// includes the node's rows. When the specification asks for one requirement to change, the node
/// it changes moves the evidence of every requirement that cites the same node, and those
/// requirements' words did not change. Calling that `moved-without-reason` blames the design for
/// obeying the specification.
///
/// It is a `look` and not `ok`: the place is exactly where the changed neighbour's edit could
/// have broken the requirement that was not asked to change, and the row names that neighbour. It
/// is explained only when EVERY place it moved is one the neighbour moved or newly stands on, and
/// when no more recorded rows are gone than places moved (a row that disappeared cannot be
/// located, so more rows gone than places gained is a loss nothing accounts for). A carried
/// neighbour never explains: two requirements whose words did not change have no reason to move
/// together.
fn explained_by_neighbours(rows: &mut [Map<String, Value>]) {
    let mut asked: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for row in rows.iter() {
        let words = row["words"].as_str().unwrap_or_default();
        let evidence = row["evidence"].as_str().unwrap_or_default();
        if matches!(words, "changed" | "new") && matches!(evidence, "changed" | "new") {
            let places = places_of(row, &["moved", "at"]);
            asked.insert(
                row["requirement"].as_str().unwrap_or_default().to_string(),
                places.into_iter().collect(),
            );
        }
    }
    for row in rows.iter_mut() {
        if row["kind"] != "moved-without-reason" || row["evidence"] != "changed" {
            continue;
        }
        let places: BTreeSet<String> = places_of(row, &["moved"]).into_iter().collect();
        let gone = row.get("gone").and_then(Value::as_i64).unwrap_or(0);
        if places.is_empty() || gone > places.len() as i64 {
            continue;
        }
        let sharing: BTreeMap<&String, &BTreeSet<String>> = asked
            .iter()
            .filter(|(_, shared)| !places.is_disjoint(shared))
            .collect();
        let mut covered: BTreeSet<&String> = BTreeSet::new();
        for shared in sharing.values() {
            covered.extend(shared.iter());
        }
        if places.iter().all(|place| covered.contains(place)) {
            row.insert("kind".to_string(), json!(SHARED));
            row.insert("severity".to_string(), json!(LOOK));
            row.insert(
                "shared_with".to_string(),
                json!(sharing.keys().collect::<Vec<_>>()),
            );
        }
    }
}

/// The words and the evidence of a revision, joined by requirement id.
///
/// A requirement the words delta calls `carried` and no node cites, in the record or now, is
/// `uncited` and `uncovered`: nothing about it moved, but nothing was compared either, and the
/// result counts it apart (`summary.uncovered`) so that "carries over" only ever means a
/// requirement whose evidence was read on both sides. It is not a finding against the revision.
pub fn join(words_delta: &Value, evidence_delta: &Value) -> Judgment<Value> {
    let words = words_of(words_delta)?;
    let (evidence, unclaimed) = evidence_of(evidence_delta)?;
    let ids: BTreeSet<&String> = words.keys().chain(evidence.keys()).collect();
    let mut rows: Vec<Map<String, Value>> = Vec::new();
    for id in ids {
        let (w, line) = (words.get(id), evidence.get(id));
        let e = line.and_then(|line| line["evidence"].as_str());
        let (kind, severity) = match w {
            Some(w) => classify(w, e),
            None => ("unlisted", LOOK),
        };
        let mut row = Map::new();
        row.insert("requirement".to_string(), json!(id));
        row.insert(
            "words".to_string(),
            json!(w.map_or("unlisted", String::as_str)),
        );
        row.insert("evidence".to_string(), json!(e.unwrap_or("none")));
        row.insert("kind".to_string(), json!(kind));
        row.insert("severity".to_string(), json!(severity));
        if let Some(line) = line {
            for key in ["moved", "at", "gone"] {
                if let Some(value) = line.get(key) {
                    row.insert(key.to_string(), value.clone());
                }
            }
        }
        rows.push(row);
    }
    explained_by_neighbours(&mut rows);
    let added: Vec<Value> = unclaimed["added"].as_array().cloned().unwrap_or_default();
    let mut kinds: BTreeMap<&str, usize> = BTreeMap::new();
    for row in &rows {
        *kinds
            .entry(row["kind"].as_str().unwrap_or_default())
            .or_default() += 1;
    }
    let count = |severity: &str| {
        rows.iter()
            .filter(|row| row["severity"] == severity)
            .count()
    };
    let violations = count(VIOLATION);
    let gone = unclaimed["gone"]
        .as_i64()
        .filter(|_| !is_falsy(&unclaimed["gone"]))
        .unwrap_or(0);
    Ok(json!({
        "verdict": if violations > 0 { "outside-reach" } else { "within-reach" },
        "summary": {
            "requirements": rows.len(),
            "violations": violations,
            "look": count(LOOK) + usize::from(!added.is_empty()),
            "ok": count(OK),
            "uncovered": count(UNCOVERED),
            // How many requirements the check saw evidence for, on either side. 0 with
            // requirements listed means it compared nothing, whatever the verdict says.
            "seen": rows.iter().filter(|row| row["evidence"] != "none").count(),
            "kinds": kinds,
        },
        "requirements": rows,
        "unclaimed": { "added": added, "gone": gone },
    }))
}

fn why(kind: &str) -> &str {
    match kind {
        "moved-without-reason" => "its words did not change and the design moved",
        SHARED => {
            "its words did not change; the design moved only where a requirement whose words did \
             change also stands, so check that this one still holds there"
        }
        "retired-still-cited" => "the specification dropped it and the design still cites it",
        "newly-cited" => "its words did not change and the design now cites it",
        "words-changed-design-same" => "its words changed and the design reads the same",
        "changed-but-uncited" => "its words changed and no node cites it",
        "unimplemented-new" => "it is new and nothing in the design carries it",
        "unlisted" => "the design cites it and the revised list does not have it",
        "revised" => "its words and the design both moved",
        "implemented-new" => "it is new and the design now carries it",
        "retired-cleanly" => "the specification dropped it and no node cites it",
        "uncited" => {
            "its words did not change and no node cites it, before or now, so there was nothing \
             to compare"
        }
        other => other,
    }
}

/// The page an owner reads: only what to look at again, and the requirements that carry over
/// folded into one line. `sentences` (id -> the specification's sentence) is printed only when
/// given, and the page then says it carries someone else's sentences.
pub fn render(result: &Value, sentences: Option<&Value>, title: &str) -> String {
    let rows: Vec<&Value> = result["requirements"]
        .as_array()
        .map(|rows| rows.iter().collect())
        .unwrap_or_default();
    let carried: Vec<&str> = rows
        .iter()
        .filter(|row| row["kind"] == "carries-over")
        .map(|row| row["requirement"].as_str().unwrap_or_default())
        .collect();
    let others: Vec<&Value> = rows
        .iter()
        .copied()
        .filter(|row| row["kind"] != "carries-over")
        .collect();
    let verdict = result["verdict"].as_str().unwrap_or_default();
    let summary = &result["summary"];
    let heading = if title.is_empty() {
        String::new()
    } else {
        format!(": {title}")
    };
    let mut out: Vec<String> = vec![format!("# Revision report{heading}"), String::new()];
    out.push(format!(
        "Verdict: {verdict}. {}",
        if verdict == "within-reach" {
            "The design moved only where the words did.".to_string()
        } else {
            format!(
                "{} requirement(s) moved where the words did not ask them to, or are cited \
                 though the specification dropped them.",
                summary["violations"]
            )
        }
    ));
    out.push(String::new());
    out.push(
        "`carries over` means the words read the same and so does the product's closure of what \
         the requirement depends on. It does not say the requirement is met, and the acceptance \
         still lapses by its bytes: accepting again is the owner's."
            .to_string(),
    );
    out.push(String::new());
    if sentences.is_some() {
        out.push(
            "\u{26a0} This page carries sentences of the specification. It is a local artefact: \
             do not check it into a repository."
                .to_string(),
        );
        out.push(String::new());
    }
    let requirements = summary["requirements"].as_i64().unwrap_or(0);
    if requirements != 0 && summary["seen"].as_i64().unwrap_or(0) == 0 {
        out.push(
            "\u{26a0} This check saw no evidence for any requirement: no node of the design \
             cites one (or its kind has nowhere to cite one), so it compared nothing. The verdict \
             above says nothing about the design."
                .to_string(),
        );
        out.push(String::new());
    } else if summary["uncovered"].as_i64().unwrap_or(0) != 0 {
        out.push(format!(
            "{} requirement(s) are cited by no node, before or now, so this check could not \
             compare them; they are listed apart below and are not carried over.",
            summary["uncovered"]
        ));
        out.push(String::new());
    }
    for (heading, severity) in [
        ("Outside the revision's reach", VIOLATION),
        ("Look again", LOOK),
        ("Changed as the words asked", OK),
        ("Not covered by this check", UNCOVERED),
    ] {
        let group: Vec<&&Value> = others
            .iter()
            .filter(|row| row["severity"] == severity)
            .collect();
        if group.is_empty() {
            continue;
        }
        out.push(format!("## {heading} ({})", group.len()));
        out.push(String::new());
        for row in group {
            let kind = row["kind"].as_str().unwrap_or_default();
            let id = row["requirement"].as_str().unwrap_or_default();
            let mut line = format!("- {id} -- {kind}: {}", why(kind));
            line.push_str(&format!(
                " (words {}, evidence {})",
                row["words"].as_str().unwrap_or_default(),
                row["evidence"].as_str().unwrap_or_default()
            ));
            let places = {
                let moved = places_of_value(row, "moved");
                if moved.is_empty() {
                    places_of_value(row, "at")
                } else {
                    moved
                }
            };
            if !places.is_empty() {
                line.push_str(&format!("; now at {}", places.join(", ")));
            }
            let shared = places_of_value(row, "shared_with");
            if !shared.is_empty() {
                line.push_str(&format!("; shared with {}", shared.join(", ")));
            }
            if !is_falsy(&row["gone"]) {
                line.push_str(&format!("; {} recorded row(s) gone", row["gone"]));
            }
            out.push(line);
            if let Some(sentence) = sentences
                .and_then(|sentences| sentences[id].as_str())
                .filter(|sentence| !sentence.is_empty())
            {
                out.push(format!("    \"{sentence}\""));
            }
        }
        out.push(String::new());
    }
    let added: Vec<String> = result["unclaimed"]["added"]
        .as_array()
        .map(|added| added.iter().map(crate::show).collect())
        .unwrap_or_default();
    if !added.is_empty() {
        out.push(format!(
            "## Rows that claim no requirement (new: {})",
            added.len()
        ));
        out.push(String::new());
        out.push(format!(
            "Behaviour no sentence asked for: {}",
            added.join(", ")
        ));
        out.push(String::new());
    }
    out.push(format!("## Carries over ({})", carried.len()));
    out.push(String::new());
    out.push(if carried.is_empty() {
        "none".to_string()
    } else {
        carried.join(", ")
    });
    out.push(String::new());
    out.join("\n")
}

fn places_of_value(row: &Value, key: &str) -> Vec<String> {
    row[key]
        .as_array()
        .map(|places| places.iter().map(crate::show).collect())
        .unwrap_or_default()
}
