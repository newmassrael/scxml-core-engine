// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A text template (docs/adr/0087, section 3): the rules of its holes and its
//! fence, the longest text it renders, and the templates a `sce-static`
//! statechart imports.
//!
//! The rendering itself is not here. It is one pure function written once per
//! runtime (the removal of the fence markers, `remove_markers`, in each
//! language's forge runtime) and spelled by each backend's generated unit; the
//! build only decides what is accepted and how long a rendering can be, which
//! every backend reads from here so that one document has one `MAX_LEN`
//! everywhere.

use std::collections::BTreeMap;
use std::path::Path;

use crate::forge::error::{ForgeError, Located, ValidationError};
use crate::forge::model::{
    ForgeDocument, ForgeKind, HoleKind, HoleTrust, SceType, TemplateFence, TemplatePart,
    TextTemplateModel,
};
use crate::forge::value_type::ValueType;

/// The most bytes a fence marker holds (docs/adr/0087, section 3.1, item 3).
pub const MARKER_MAX: usize = 32;

/// What a hole of type `ty`, with `sce:trust` as written, holds — or why it is
/// no hole (`validation/text-template-hole`): a type whose text is not the same
/// on every backend, `sce:trust` missing on a text hole, or written on a hole
/// that is not a text.
pub fn hole_kind(ty: &ValueType, trust: Option<HoleTrust>) -> Result<HoleKind, String> {
    let no_trust = |what: &str| -> Result<(), String> {
        match trust {
            None => Ok(()),
            Some(written) => Err(format!(
                "is {what}, which carries no instruction, and writes sce:trust=\"{}\": only a \
                 text hole says whether it is trusted",
                written.as_attr()
            )),
        }
    };
    match ty {
        ValueType::Text { max } => match trust {
            Some(trust) => Ok(HoleKind::Text {
                max: max.clone(),
                trust,
            }),
            None => Err(format!(
                "is a text ({ty}), and a text hole says whether its value is the program's own: \
                 write sce:trust=\"trusted\" or sce:trust=\"untrusted\""
            )),
        },
        ValueType::Scalar(
            scalar @ (SceType::Uint8
            | SceType::Uint16
            | SceType::Uint32
            | SceType::Uint64
            | SceType::Int8
            | SceType::Int16
            | SceType::Int32
            | SceType::Int64),
        ) => {
            no_trust("a number")?;
            Ok(HoleKind::Integer(scalar.clone()))
        }
        ValueType::Scalar(SceType::Enum(reference)) => {
            no_trust("a value of an enum")?;
            Ok(HoleKind::Enum(reference.clone()))
        }
        other => Err(format!(
            "is of {other}, whose text is not the same on every backend: a hole is a string[B], \
             a fixed-width integer or a value of a closed enum"
        )),
    }
}

/// Why `fence` is refused against the template's literal parts
/// (`validation/text-template-fence`), as the marker the rule is broken at and
/// what is wrong: a marker that is empty, longer than [`MARKER_MAX`] bytes or
/// not printable ASCII; a marker inside the other; a marker in the template's
/// own text. `None` for a fence that holds.
pub fn fence_refusal(fence: &TemplateFence, literals: &[&str]) -> Option<(String, String)> {
    for (attr, marker) in [("open", &fence.open), ("close", &fence.close)] {
        if marker.is_empty() {
            return Some((
                String::new(),
                format!(
                    "the fence's {attr} marker is empty: a marker holds 1 to {MARKER_MAX} bytes"
                ),
            ));
        }
        if marker.len() > MARKER_MAX {
            return Some((
                marker.clone(),
                format!(
                    "the fence's {attr} marker is {} bytes long: a marker holds 1 to {MARKER_MAX} \
                     bytes",
                    marker.len()
                ),
            ));
        }
        if !marker.bytes().all(|b| (0x20..=0x7e).contains(&b)) {
            return Some((
                marker.clone(),
                format!(
                    "the fence's {attr} marker holds a byte that is not printable ASCII: a \
                     marker is printable ASCII (0x20 to 0x7E), so removing it from a UTF-8 text \
                     leaves UTF-8"
                ),
            ));
        }
    }
    if fence.open.contains(&fence.close) || fence.close.contains(&fence.open) {
        return Some((
            fence.close.clone(),
            "one marker of the fence is inside the other: a value could then not be told from \
             the fence around it"
                .to_string(),
        ));
    }
    for marker in [&fence.open, &fence.close] {
        if literals.iter().any(|text| text.contains(marker.as_str())) {
            return Some((
                marker.clone(),
                "the marker occurs in the template's own text: a marker begins or ends an \
                 untrusted value, and nowhere else"
                    .to_string(),
            ));
        }
    }
    None
}

/// The most bytes the text of `model` renders (docs/adr/0087, section 3.1,
/// item 5): the bytes of its literal parts, and for each put `B` for a trusted
/// `string[B]`, `|open| + B + |close|` for an untrusted one, the most digits of
/// an integer with its sign, and the longest declared name of an enum.
/// `longest_name` answers the last for the alias the template imports the enum
/// under; `None` when an enum hole's enum is not known to it.
pub fn max_len(
    model: &TextTemplateModel,
    longest_name: &dyn Fn(&str) -> Option<u64>,
) -> Option<u64> {
    let fence = model
        .fence
        .as_ref()
        .map_or(0, |f| (f.open.len() + f.close.len()) as u64);
    let mut total: u64 = 0;
    for part in &model.parts {
        let len = match part {
            TemplatePart::Literal(text) => text.len() as u64,
            TemplatePart::Put(id) => match &model.hole(id)?.kind {
                HoleKind::Text {
                    max,
                    trust: HoleTrust::Trusted,
                } => u64::from(max.max),
                HoleKind::Text {
                    max,
                    trust: HoleTrust::Untrusted,
                } => fence + u64::from(max.max),
                HoleKind::Integer(ty) => crate::forge::text_bound::integer_text_bound(
                    crate::forge::types::InferredType::from_sce_type(ty),
                ),
                HoleKind::Enum(reference) => longest_name(&reference.alias)?,
            },
        };
        total = total.checked_add(len)?;
    }
    Some(total)
}

/// The longest declared name among `names`, in bytes.
pub fn longest<'a>(names: impl IntoIterator<Item = &'a str>) -> u64 {
    names.into_iter().map(|n| n.len() as u64).max().unwrap_or(0)
}

/// A text template a `sce-static` statechart imports (docs/adr/0087, section
/// 3.1, item 6), as its `<sce:render>` is judged and lowered by it.
#[derive(Debug, Clone)]
pub struct ImportedTextTemplate {
    /// The template document.
    pub model: TextTemplateModel,
    /// The enum each enum hole holds a value of, by the hole's id, read from
    /// the template's own imports and keyed as the statechart keys the enum
    /// documents it holds ([`crate::forge::schema_revision::document_key`]), so
    /// a value of the statechart's is of the hole's enum when the two keys are
    /// one.
    pub enums: BTreeMap<String, crate::forge::schema_revision::EnumDocument>,
    /// The most bytes the template renders, as its unit declares it
    /// (`MAX_LEN`).
    pub max_len: u64,
}

/// Every `<sce:import kind="text-template">` of `model`, read from `base_dir`
/// (the statechart's directory), by alias. A template whose document cannot be
/// read, or is not a text template, is refused where it is imported, as is one
/// whose enum hole names an enum that is not a closed set (a variable holds a
/// closed enum only, docs/SCE_ACCEPTED_SUBSET.md section 2.15) or whose
/// document cannot be read.
pub fn resolve(
    model: &crate::model::SCXMLModel,
    base_dir: &Path,
    diag_label: &str,
) -> Result<BTreeMap<String, ImportedTextTemplate>, Located<ForgeError>> {
    use crate::forge::import_source::ImportSource;
    let mut templates = BTreeMap::new();
    for import in model
        .forge_imports
        .iter()
        .filter(|i| i.kind == ForgeKind::TextTemplate)
    {
        let refused = |error: ForgeError| Located::new(error, diag_label, import.line, None);
        let source = ImportSource::read(base_dir, import).map_err(|e| refused(e.into()))?;
        let parsed = source.parse()?.ok_or_else(|| {
            refused(
                crate::forge::error::ImportError::NotForge {
                    src: import.src.clone(),
                }
                .into(),
            )
        })?;
        let actual = parsed.document.kind();
        let ForgeDocument::TextTemplate(template) = parsed.document else {
            return Err(refused(
                crate::forge::error::ImportError::KindMismatch {
                    src: import.src.clone(),
                    declared: ForgeKind::TextTemplate.to_string(),
                    actual: actual.to_string(),
                }
                .into(),
            ));
        };
        let template_dir = source.path.parent().unwrap_or(base_dir).to_path_buf();
        let mut enums = BTreeMap::new();
        for hole in &template.holes {
            let HoleKind::Enum(reference) = &hole.kind else {
                continue;
            };
            let document = parsed
                .imports
                .iter()
                .find(|i| i.alias == reference.alias && i.kind == ForgeKind::Enum)
                .and_then(|i| ImportSource::read(&template_dir, i).ok())
                .and_then(|enum_source| {
                    let parsed = enum_source.parse().ok()??;
                    match parsed.document {
                        ForgeDocument::Enum(enum_model) => {
                            Some(crate::forge::schema_revision::EnumDocument {
                                key: crate::forge::schema_revision::document_key(
                                    base_dir,
                                    &enum_source.path,
                                ),
                                model: enum_model,
                            })
                        }
                        _ => None,
                    }
                });
            let detail = match &document {
                None => Some(format!(
                    "holds a value of enum:{}, and the template imports no enum document it can \
                     read under that alias",
                    reference.alias
                )),
                Some(document) if !document.model.strict_variants => Some(format!(
                    "holds a value of enum:{}, which declares sce:strict-variants=\"false\": a \
                     hole holds a closed enum, whose every value has a declared name to put",
                    reference.alias
                )),
                Some(_) => None,
            };
            if let Some(detail) = detail {
                return Err(refused(
                    ValidationError::TextTemplateHole {
                        template: template.name.clone(),
                        hole: hole.id.clone(),
                        detail,
                    }
                    .into(),
                ));
            }
            if let Some(document) = document {
                enums.insert(hole.id.clone(), document);
            }
        }
        let longest_name = |alias: &str| {
            template
                .holes
                .iter()
                .find(|h| matches!(&h.kind, HoleKind::Enum(r) if r.alias == alias))
                .and_then(|h| enums.get(&h.id))
                .map(|d| longest(d.model.variants.iter().map(|v| v.name.as_str())))
        };
        let max_len = max_len(&template, &longest_name).ok_or_else(|| {
            refused(
                crate::forge::error::GenerateError::unsupported(format!(
                    "text-template '{}': the longest text it renders has no size the build can \
                     compute",
                    template.name
                ))
                .into(),
            )
        })?;
        templates.insert(
            import.alias.clone(),
            ImportedTextTemplate {
                model: template,
                enums,
                max_len,
            },
        );
    }
    Ok(templates)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::model::{EnumRef, TemplateHole};
    use crate::forge::value_type::Bound;

    fn fence(open: &str, close: &str) -> TemplateFence {
        TemplateFence {
            open: open.to_string(),
            close: close.to_string(),
        }
    }

    /// Each hole kind is admitted with the trust it needs and refused without
    /// it, and every other type is refused by its spelling.
    #[test]
    fn a_hole_is_a_text_an_integer_or_an_enum() {
        let text = ValueType::Text { max: Bound::of(8) };
        assert!(matches!(
            hole_kind(&text, Some(HoleTrust::Untrusted)),
            Ok(HoleKind::Text {
                trust: HoleTrust::Untrusted,
                ..
            })
        ));
        assert!(hole_kind(&text, None)
            .unwrap_err()
            .contains("sce:trust=\"trusted\""));
        let number = ValueType::Scalar(SceType::Int16);
        assert_eq!(
            hole_kind(&number, None),
            Ok(HoleKind::Integer(SceType::Int16))
        );
        assert!(hole_kind(&number, Some(HoleTrust::Trusted))
            .unwrap_err()
            .contains("only a text hole"));
        let named = ValueType::Scalar(SceType::Enum(EnumRef {
            alias: "Day".to_string(),
        }));
        assert!(matches!(hole_kind(&named, None), Ok(HoleKind::Enum(_))));
        assert!(hole_kind(&named, Some(HoleTrust::Untrusted)).is_err());
        for refused in [
            ValueType::Scalar(SceType::Bool),
            ValueType::Scalar(SceType::Float64),
            ValueType::Bytes { max: Bound::of(4) },
            ValueType::Optional {
                elem: Box::new(ValueType::Text { max: Bound::of(4) }),
            },
            ValueType::Record {
                alias: "R".to_string(),
            },
        ] {
            let why = hole_kind(&refused, Some(HoleTrust::Trusted)).unwrap_err();
            assert!(
                why.contains("not the same on every backend"),
                "{refused}: {why}"
            );
        }
    }

    /// Every fence rule refuses its own case, and a fence that keeps them all
    /// is admitted.
    #[test]
    fn each_fence_rule_refuses_its_case() {
        assert_eq!(fence_refusal(&fence("<<<", ">>>"), &["Hello "]), None);
        let empty = fence_refusal(&fence("", ">>>"), &[]).unwrap();
        assert!(empty.1.contains("empty"));
        let long = "x".repeat(MARKER_MAX + 1);
        assert_eq!(fence_refusal(&fence(&long, ">"), &[]).unwrap().0, long);
        let at_bound = "y".repeat(MARKER_MAX);
        assert_eq!(fence_refusal(&fence(&at_bound, ">"), &[]), None);
        assert!(fence_refusal(&fence("<\u{e9}", ">"), &[])
            .unwrap()
            .1
            .contains("printable ASCII"));
        assert!(fence_refusal(&fence("<\t", ">"), &[]).is_some());
        assert!(fence_refusal(&fence("TEXT", "TEXT>>>"), &[])
            .unwrap()
            .1
            .contains("inside the other"));
        assert!(fence_refusal(&fence("AB", "AB"), &[]).is_some());
        let in_text = fence_refusal(&fence("<<<", ">>>"), &["a <<< b"]).unwrap();
        assert_eq!(in_text.0, "<<<");
    }

    /// `MAX_LEN` sums the literal bytes and each put's longest text: a trusted
    /// text's bound, an untrusted one's with both markers, an integer's digits
    /// and sign, and an enum's longest name, once for every put.
    #[test]
    fn max_len_sums_every_part() {
        let model = TextTemplateModel {
            name: "t".to_string(),
            revision: None,
            holes: vec![
                TemplateHole {
                    id: "a".to_string(),
                    kind: HoleKind::Text {
                        max: Bound::of(5),
                        trust: HoleTrust::Trusted,
                    },
                },
                TemplateHole {
                    id: "b".to_string(),
                    kind: HoleKind::Text {
                        max: Bound::of(7),
                        trust: HoleTrust::Untrusted,
                    },
                },
                TemplateHole {
                    id: "n".to_string(),
                    kind: HoleKind::Integer(SceType::Int64),
                },
                TemplateHole {
                    id: "e".to_string(),
                    kind: HoleKind::Enum(EnumRef {
                        alias: "Day".to_string(),
                    }),
                },
            ],
            fence: Some(fence("<<", ">>>")),
            parts: vec![
                TemplatePart::Literal("é:".to_string()),
                TemplatePart::Put("a".to_string()),
                TemplatePart::Put("b".to_string()),
                TemplatePart::Put("b".to_string()),
                TemplatePart::Put("n".to_string()),
                TemplatePart::Put("e".to_string()),
            ],
            source_location: None,
        };
        let names = |alias: &str| (alias == "Day").then_some(9);
        // 3 + 5 + 2 * (2 + 7 + 3) + 20 + 9
        assert_eq!(max_len(&model, &names), Some(61));
        assert_eq!(max_len(&model, &|_| None), None);
        assert_eq!(longest(["mon", "wednesday", "fri"]), 9);
    }
}
