// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The ledger of a divided document, as the specs folder reads it (ADR 0040).
//!
//! The authoring tools make the ledger and count it (`tools/authoring/sce_author/ledger.py`); the store
//! never recomputes the count. What the store holds a ledger to is its form, and that it is of the
//! extraction it is kept for: the same rules the tools hold it to, so that a ledger the store keeps is one the
//! tools would write. The two are held to one list of cases (`tools/authoring/tests/fixtures/
//! ledger_wire_cases.json`), which a test on each side reads, so that a rule added to one and not the other
//! fails there.
//!
//! A ledger is a list of allotments, each a destination (a spec that exists, a spec to be made, or left out)
//! and the blocks of the extraction that go there. It is kept exactly as its maker wrote it, and named by the
//! hash of those bytes: the tools write the canonical text, so the hash is the ledger's revision.

use std::collections::BTreeSet;

use serde_json::{json, Value};

use crate::error::StoreError;
use crate::revision::Revision;
use crate::store::SpecId;

/// The most characters of a spec's title, and of a reason a part was left out.
pub const MAX_TITLE: usize = 200;
pub const MAX_REASON: usize = 1000;

/// Where an allotment goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Destination {
    /// A spec that exists, by its id.
    Spec(String),
    /// A spec to be made when the division is confirmed, by the title the owner gave it.
    New(String),
    /// Out of the document, on purpose.
    LeftOut,
}

impl Destination {
    /// The destination as the form says it.
    pub fn to_value(&self) -> Value {
        match self {
            Destination::Spec(id) => json!({ "kind": "spec", "spec": id }),
            Destination::New(title) => json!({ "kind": "new", "title": title }),
            Destination::LeftOut => json!({ "kind": "left-out" }),
        }
    }
}

/// The blocks that go to one destination, in the extraction's order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Allotment {
    pub to: Destination,
    /// Why a part was left out; empty for anything else, and allowed to be empty for what is left out.
    pub reason: String,
    pub blocks: Vec<String>,
}

/// A ledger that is of an extraction and has been read as the tools write one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ledger {
    pub extraction: Revision,
    pub allotments: Vec<Allotment>,
}

fn bad(message: impl Into<String>, detail: Value) -> StoreError {
    StoreError::refused("bad-ledger", message, detail)
}

fn unknown_field(
    object: &serde_json::Map<String, Value>,
    known: &[&str],
    what: &str,
) -> Result<(), StoreError> {
    match object.keys().find(|key| !known.contains(&key.as_str())) {
        Some(unknown) => Err(bad(
            format!("{what} has a field nobody defined (`{unknown}`)"),
            json!({ "field": unknown }),
        )),
        None => Ok(()),
    }
}

fn destination(value: Option<&Value>) -> Result<Destination, StoreError> {
    let object = value
        .and_then(Value::as_object)
        .ok_or_else(|| bad("an allotment has no destination", Value::Null))?;
    unknown_field(object, &["kind", "spec", "title"], "a destination")?;
    let kind = object
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(|| bad("a destination has no kind", Value::Null))?;
    let spec = object.get("spec");
    let title = object.get("title");
    match kind {
        "spec" => {
            let id = spec.and_then(Value::as_str).filter(|_| title.is_none());
            let id = id.and_then(|id| SpecId::parse(id).ok()).ok_or_else(|| {
                bad(
                    "a destination names a spec by something that is not a spec's id",
                    Value::Null,
                )
            })?;
            Ok(Destination::Spec(id.as_str().to_string()))
        }
        "new" => {
            let text = title
                .and_then(Value::as_str)
                .filter(|_| spec.is_none())
                .filter(|text| !text.trim().is_empty())
                .ok_or_else(|| bad("a spec to be made has no title", Value::Null))?;
            if text.chars().count() > MAX_TITLE {
                return Err(bad(
                    format!("the title of a spec to be made is more than {MAX_TITLE} characters"),
                    json!({ "limit": MAX_TITLE }),
                ));
            }
            Ok(Destination::New(text.to_string()))
        }
        "left-out" if spec.is_none() && title.is_none() => Ok(Destination::LeftOut),
        "left-out" => Err(bad("a part that is left out names a spec", Value::Null)),
        other => Err(bad(
            format!("a destination is of a kind nobody defined (`{other}`)"),
            json!({ "kind": other }),
        )),
    }
}

impl Ledger {
    /// Read the text of a ledger kept for `extraction`, whose blocks are `order` in the order it has them.
    ///
    /// Refused as `bad-ledger` when it is not a ledger of the form, is of another extraction, names a block the
    /// extraction does not have, holds blocks out of order or twice in one allotment, gives an allotment no
    /// block, gives two allotments one destination, or gives a reason to what is not left out. A ledger with
    /// no allotment is one: nothing is placed yet.
    pub fn read(text: &str, extraction: &Revision, order: &[String]) -> Result<Ledger, StoreError> {
        let value: Value = serde_json::from_str(text)
            .map_err(|e| bad(format!("a ledger is not JSON ({e})"), Value::Null))?;
        let object = value
            .as_object()
            .ok_or_else(|| bad("a ledger is not a JSON object", Value::Null))?;
        unknown_field(object, &["v", "extraction", "allotments"], "a ledger")?;
        if object.get("v").and_then(Value::as_u64) != Some(1) {
            return Err(bad(
                "a ledger is of a version this core does not read",
                Value::Null,
            ));
        }
        if object.get("extraction").and_then(Value::as_str) != Some(extraction.as_str()) {
            return Err(bad(
                "the ledger is of another extraction than the intake's",
                json!({ "extraction": extraction }),
            ));
        }
        let entries = object
            .get("allotments")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                bad(
                    "a ledger does not hold its allotments as a list",
                    Value::Null,
                )
            })?;
        let mut allotments = Vec::with_capacity(entries.len());
        let mut seen: BTreeSet<(u8, String)> = BTreeSet::new();
        for entry in entries {
            let entry = entry
                .as_object()
                .ok_or_else(|| bad("an allotment is not a JSON object", Value::Null))?;
            unknown_field(entry, &["to", "reason", "blocks"], "an allotment")?;
            let to = destination(entry.get("to"))?;
            let reason = match entry.get("reason") {
                None => String::new(),
                Some(Value::String(reason)) => reason.clone(),
                Some(_) => return Err(bad("a reason is not text", Value::Null)),
            };
            if !reason.is_empty() && to != Destination::LeftOut {
                return Err(bad(
                    "a reason is given for a part that is not left out",
                    Value::Null,
                ));
            }
            if reason.chars().count() > MAX_REASON {
                return Err(bad(
                    format!("a reason is more than {MAX_REASON} characters"),
                    json!({ "limit": MAX_REASON }),
                ));
            }
            let blocks = entry
                .get("blocks")
                .and_then(Value::as_array)
                .ok_or_else(|| {
                    bad(
                        "an allotment does not hold its blocks as a list of ids",
                        Value::Null,
                    )
                })?
                .iter()
                .map(|id| {
                    id.as_str()
                        .filter(|id| !id.is_empty())
                        .map(str::to_string)
                        .ok_or_else(|| bad("an allotment names a block with no id", Value::Null))
                })
                .collect::<Result<Vec<String>, StoreError>>()?;
            if blocks.is_empty() {
                return Err(bad("an allotment of no block is not one", Value::Null));
            }
            // Each block once, and in the order the extraction has them: one division has one spelling.
            let mut at = 0;
            for block in &blocks {
                match order[at..].iter().position(|held| held == block) {
                    Some(found) => at += found + 1,
                    None => {
                        return Err(bad(
                            format!(
                                "the ledger names the block `{block}`, which the extraction does not have \
                                 there (a block is named once in an allotment, in the extraction's order)"
                            ),
                            json!({ "block": block }),
                        ));
                    }
                }
            }
            let identity = match &to {
                Destination::Spec(id) => (0, id.clone()),
                Destination::New(title) => (1, title.clone()),
                Destination::LeftOut => (2, reason.clone()),
            };
            if !seen.insert(identity) {
                return Err(bad(
                    "two allotments go to the same destination",
                    Value::Null,
                ));
            }
            allotments.push(Allotment { to, reason, blocks });
        }
        Ok(Ledger {
            extraction: extraction.clone(),
            allotments,
        })
    }

    /// The allotment of a spec: the one that goes to it by its id, or to a spec to be made that was made as
    /// it (`bound` maps the title of such a spec to the id it was made under).
    pub fn allotment_of(&self, spec: &str, bound: &[(String, String)]) -> Option<&Allotment> {
        self.allotments
            .iter()
            .find(|allotment| match &allotment.to {
                Destination::Spec(id) => id == spec,
                Destination::New(title) => bound
                    .iter()
                    .any(|(bound_title, bound_spec)| bound_title == title && bound_spec == spec),
                Destination::LeftOut => false,
            })
    }
}
