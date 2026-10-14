// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What a revision of a work did, requirement by requirement: the work's words beside its
//! evidence, and whether the revision stayed within the reach of what changed
//! (`docs/adr/0009-a-revision-stays-within-the-reach-of-what-changed.md`).
//!
//! The words come from two states of the work's OWN chain of requirement lists, the list the
//! owner's acceptance was taken of and the list the work has now, each with its lineage (a list
//! made before lineages is adopted from its sidecar). The evidence is the product's own comparison
//! of the design now with what the owner was shown. Both are the work's, so nothing here can be
//! handed another specification's delta, and the step between them is whatever happened between
//! those two revisions, however many there were. The judgment is `sce-revision`, the same words
//! as the authoring tools give (ADR 0011, D1 (c)); this only lays the work's parts out for it.

use serde_json::{json, Value};

use crate::requirements::Requirements;

/// A revision that cannot be judged, in the sentence a person is told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotJudged(pub String);

/// The texts of a list, as `sce_revision::of_list` reads them.
fn held(list: &Requirements) -> Value {
    let mut held = json!({ "manifest_text": list.manifest });
    if let Some(sidecar) = &list.sidecar {
        held["sidecar_text"] = json!(sidecar);
    }
    if let Some(lineage) = &list.lineage {
        held["lineage_text"] = json!(lineage);
    }
    held
}

/// The sentences of the specification behind the ids of a list, as its sidecar holds them.
fn sentences_of(list: &Requirements) -> Value {
    let words = list
        .sidecar
        .as_deref()
        .and_then(|sidecar| serde_json::from_str::<Value>(sidecar).ok())
        .and_then(|sidecar| sidecar.get("text").and_then(Value::as_object).cloned());
    Value::Object(
        words
            .unwrap_or_default()
            .into_iter()
            .filter(|(_, text)| text.is_string())
            .collect(),
    )
}

/// What the revision between `older` (the list the acceptance was taken of) and `newer` (the
/// list the work has now) did to the words and to the design.
///
/// `pin` is the manifest the acceptance pinned (its document, revision and the digests of the
/// manifest and the sidecar), `lines` the lines the product wrote comparing the design now with
/// the record, `of` the revisions the two states are about, and `title` names the page. With
/// `with_sentences` the page carries the specification's sentences and says it is a local
/// artefact.
pub fn judge(
    older: &Requirements,
    newer: &Requirements,
    pin: &Value,
    lines: &Value,
    of: Value,
    with_sentences: bool,
    title: &str,
) -> Result<Value, NotJudged> {
    let cannot_say = |why: sce_revision::LineageError| {
        NotJudged(format!(
            "cannot say what the revision did to each requirement's words: {why}. Build the list \
             again with scxml_requirement_set against the lineage works_read gives and save it \
             with works_save_requirements"
        ))
    };
    let words = sce_revision::between(
        &sce_revision::of_list(&held(older), "the owner accepted").map_err(cannot_say)?,
        &sce_revision::of_list(&held(newer), "the work has now").map_err(cannot_say)?,
    )
    .map_err(cannot_say)?;
    let record = json!({ "manifest": pin });
    let said = |why: sce_revision::LineageError| NotJudged(why.message().to_string());
    sce_revision::belongs_to(&words, &record).map_err(said)?;
    let mut result =
        sce_revision::join(&words, &sce_revision::delta_object(lines)).map_err(said)?;
    result["of"] = of;
    let sentences = with_sentences.then(|| sentences_of(newer));
    result["page"] = json!(sce_revision::render(&result, sentences.as_ref(), title));
    Ok(result)
}
