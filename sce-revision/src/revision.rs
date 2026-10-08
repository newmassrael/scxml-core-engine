// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A revision of a design joined to what became of the words, as the Python reference
//! (`tools/authoring/sce_author/revision.py`) judges it, in its sentences.

use serde_json::Value;

use crate::{refuse, Judgment};

/// The first twelve characters of a digest, which is how a sentence names one.
fn head(digest: &str) -> String {
    digest.chars().take(12).collect()
}

fn text(value: &Value) -> &str {
    value.as_str().unwrap_or_default()
}

/// Refuse a words delta that is not the one of the revision this acceptance was taken against.
///
/// The join is by requirement id, and an id means nothing outside the specification that issued
/// it: another specification's delta that happens to name `R1` to `R5` joins the design's
/// evidence without a word of complaint and gives a verdict about nothing. A delta names the
/// specification it describes (`doc_id`), the revision it starts from (`from_rev`) and the
/// digests of the manifest and the sidecar of that revision's list (`from_manifest_sha256`,
/// `from_sidecar_sha256`); the record pins the manifest it was taken against (`manifest.doc_id`,
/// `manifest.rev`, `manifest.sha256`) and the sidecar beside it (`manifest.sidecar_sha256`). All
/// have to agree: a name and a number are carried by every copy of a specification, and only the
/// digest says it is THIS list. The manifest's digest is not enough: a manifest is coordinates
/// only, so a list of another specification with the same shape has it too; the sidecar's, over
/// the words behind the ids, is what tells the two apart. A delta from a LATER revision than the
/// record's is refused too: it says what changed since that revision, not since the one the
/// design was accepted for, and a requirement reworded in between would read as carried.
pub fn belongs_to(delta: &Value, record: &Value) -> Judgment<()> {
    if !(delta.is_object() && delta["doc_id"].is_string() && delta["from_rev"].is_string()) {
        return refuse(
            "the words delta names no specification (`doc_id`) or starting revision \
             (`from_rev`): it was not built by this version of scxml_requirement_set, so nothing \
             says it describes this record's specification. Build the revised list again against \
             its lineage and give the `delta` it returns",
        );
    }
    if !delta["from_manifest_sha256"].is_string() {
        return refuse(
            "the words delta does not say which list it starts from (`from_manifest_sha256`): \
             the lineage it was built against predates recording the manifest's digest, or the \
             revision it starts from was adopted without the manifest. A name and a revision \
             number do not tell two lists of one specification apart; build the revised list \
             again from the manifest and sidecar the design was accepted against",
        );
    }
    let manifest = &record["manifest"];
    let pinned = manifest.is_object()
        && manifest["doc_id"].is_string()
        && manifest["rev"].is_string()
        && manifest["sha256"].is_string();
    if !pinned {
        return refuse(
            "the acceptance record pins no manifest `doc_id`, `rev` and `sha256`, so there is \
             nothing to check the words delta against",
        );
    }
    let (doc_id, from_rev) = (text(&delta["doc_id"]), text(&delta["from_rev"]));
    let (pinned_doc, pinned_rev) = (text(&manifest["doc_id"]), text(&manifest["rev"]));
    if doc_id != pinned_doc {
        return refuse(format!(
            "the words delta is of specification '{doc_id}' and the acceptance record was taken \
             against '{pinned_doc}'"
        ));
    }
    if from_rev != pinned_rev {
        return refuse(format!(
            "the words delta starts from revision {from_rev} of '{doc_id}' and the acceptance \
             record was taken against revision {pinned_rev}: it describes a different step. \
             Build the list again against the lineage as it was at the revision the design was \
             accepted for"
        ));
    }
    let (from_manifest, pinned_manifest) = (
        text(&delta["from_manifest_sha256"]),
        text(&manifest["sha256"]),
    );
    if from_manifest != pinned_manifest {
        return refuse(format!(
            "the words delta starts from a list whose manifest digest is {} and the acceptance \
             record was taken against a manifest whose digest is {}: they are not the same list, \
             though both are revision {pinned_rev} of '{pinned_doc}' (another copy of the \
             specification, or a list written again for the same text)",
            head(from_manifest),
            head(pinned_manifest)
        ));
    }
    if !delta["from_sidecar_sha256"].is_string() {
        return refuse(
            "the words delta does not say which words it starts from (`from_sidecar_sha256`): \
             the lineage it was built against predates recording the sidecar's digest, or the \
             revision it starts from was adopted without the sidecar. A manifest is coordinates \
             only, so it does not tell two lists of one shape apart; build the revised list \
             again from the manifest and sidecar the design was accepted against",
        );
    }
    if !manifest["sidecar_sha256"].is_string() {
        return refuse(
            "the acceptance record pins no sidecar (`manifest.sidecar_sha256`): it was taken \
             without the words behind the ids, and a manifest is coordinates only, so nothing \
             says the words delta starts from the list that was accepted. Accept the design \
             again, giving the sidecar",
        );
    }
    let (from_sidecar, pinned_sidecar) = (
        text(&delta["from_sidecar_sha256"]),
        text(&manifest["sidecar_sha256"]),
    );
    if from_sidecar != pinned_sidecar {
        return refuse(format!(
            "the words delta starts from words whose sidecar digest is {} and the acceptance \
             record was taken against a sidecar whose digest is {}: they are not the same list, \
             though the manifest is the same (another specification of the same shape, or the \
             same ids over other sentences)",
            head(from_sidecar),
            head(pinned_sidecar)
        ));
    }
    Ok(())
}
