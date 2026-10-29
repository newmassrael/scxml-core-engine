// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! One document, one namespace.
//!
//! Every name a forge document declares for its expressions to read — a
//! field, a parameter, a local, a const, a helper, an import's alias —
//! lives in one flat scope, because that is how every backend lowers it:
//! a transform's inputs become one parameter list, an algorithm's params
//! and locals become one function body, an import alias becomes one member
//! or one call name beside them. Two declarations of one name are therefore
//! not two things; they are one name the author means twice.
//!
//! # Why a pass of its own
//!
//! The rule was written down — the accepted subset says an `id` is unique
//! within its kind and a duplicate is `validation/duplicate-id` — and only
//! some kinds enforced it, each in its own parser. Measured 2026-09-21:
//!
//! * a transform with two inputs named `a` (one `int32`, one `bool`)
//!   generated with exit 0 on all six backends, emitting
//!   `fn …(a: i32, a: bool)`;
//! * an input and an output both named `a` were refused, but as
//!   `validation/transform-output-cycle` — true of the document as the
//!   generator read it, and not what the author did;
//! * two algorithm parameters named `x` were refused by the RENDERER, as
//!   `generate/invalid-config`, which `check` reads as one backend's gap;
//! * a parameter named like an import alias shadowed the import silently.
//!
//! This pass runs before every other document check, so a duplicate is
//! named as a duplicate and not as whatever it later breaks.
//!
//! # What it keeps
//!
//! An algorithm local or loop variable that reuses an earlier name keeps
//! the code it always had, `algorithm/local-shadows-param`: that refusal
//! was correct, only its place was wrong. Every other collision is
//! `validation/duplicate-id`.

use std::collections::BTreeMap;

use crate::forge::error::{ForgeError, Located, ValidationError};
use crate::forge::model::{AlgorithmBinding, ForgeDocument, ForgeKind, ParsedForge};

/// What declared a name — used to say which two declarations collided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Declared {
    Field,
    Parameter,
    Const,
    Local,
    LoopItem,
    Helper,
    Monitor,
    FlagInput,
    ImportAlias,
}

impl Declared {
    fn as_str(self) -> &'static str {
        match self {
            Declared::Field => "field",
            Declared::Parameter => "parameter",
            Declared::Const => "const",
            Declared::Local => "local",
            Declared::LoopItem => "foreach item",
            Declared::Helper => "helper",
            Declared::Monitor => "monitor",
            Declared::FlagInput => "flag input",
            Declared::ImportAlias => "import alias",
        }
    }

    /// A binding the algorithm BODY introduces — the side that keeps the
    /// `algorithm/local-shadows-param` code when it collides.
    fn is_algorithm_body_binding(self) -> bool {
        matches!(self, Declared::Local | Declared::LoopItem)
    }
}

/// Every expression-visible name the document declares, in declaration
/// order, imports first — an alias is in scope before any body reads it.
///
/// ⚠ Exhaustive over `ForgeDocument`, with no `_` arm. A kind added later
/// must say what it declares; an empty list is a decision, and each empty
/// arm below states why.
fn declarations(parsed: &ParsedForge) -> Vec<(&str, Declared)> {
    fn push<'a>(
        out: &mut Vec<(&'a str, Declared)>,
        names: impl IntoIterator<Item = &'a str>,
        what: Declared,
    ) {
        out.extend(names.into_iter().map(|n| (n, what)));
    }
    let mut out: Vec<(&str, Declared)> = Vec::new();
    push(
        &mut out,
        parsed.imports.iter().map(|imp| imp.alias.as_str()),
        Declared::ImportAlias,
    );
    match &parsed.document {
        ForgeDocument::Transform(m) => push(
            &mut out,
            m.inputs.iter().chain(&m.outputs).map(|f| f.id.as_str()),
            Declared::Field,
        ),
        ForgeDocument::Lookup(m) => push(
            &mut out,
            [m.input.id.as_str(), m.output.id.as_str()],
            Declared::Field,
        ),
        ForgeDocument::Condition(m) => push(
            &mut out,
            m.inputs.iter().map(|f| f.id.as_str()),
            Declared::Field,
        ),
        ForgeDocument::Validator(m) => push(
            &mut out,
            m.inputs.iter().map(|f| f.id.as_str()),
            Declared::Field,
        ),
        ForgeDocument::Filter(m) => push(
            &mut out,
            [m.input.id.as_str(), m.output.id.as_str()],
            Declared::Field,
        ),
        ForgeDocument::Interpolation(m) => push(
            &mut out,
            m.inputs
                .iter()
                .chain(std::iter::once(&m.output))
                .map(|f| f.id.as_str()),
            Declared::Field,
        ),
        ForgeDocument::Observer(m) => {
            push(
                &mut out,
                m.inputs.iter().map(|f| f.id.as_str()),
                Declared::Field,
            );
            push(
                &mut out,
                m.monitors.iter().map(|x| x.id.as_str()),
                Declared::Monitor,
            );
        }
        ForgeDocument::Procedure(m) => {
            push(
                &mut out,
                m.inputs.iter().chain(&m.internals).map(|f| f.id.as_str()),
                Declared::Field,
            );
            push(
                &mut out,
                m.helpers.iter().map(|h| h.name.as_str()),
                Declared::Helper,
            );
        }
        ForgeDocument::Codec(m) => {
            // A CBOR or content-line codec's entries are its fields
            // (SCE_FORGE.md §4.6.1, §4.6.4).
            push(
                &mut out,
                m.fields
                    .iter()
                    .map(|f| f.id.as_str())
                    .chain(m.cbor_entries.iter().map(|e| e.id.as_str()))
                    .chain(
                        m.content_line
                            .iter()
                            .flat_map(|c| c.entries.iter().map(|e| e.id.as_str())),
                    ),
                Declared::Field,
            );
            push(
                &mut out,
                m.flag_inputs.iter().map(|f| f.name.as_str()),
                Declared::FlagInput,
            );
        }
        ForgeDocument::EventSchema(m) => push(
            &mut out,
            m.fields.iter().map(|f| f.id.as_str()),
            Declared::Field,
        ),
        ForgeDocument::Algorithm(m) => {
            push(
                &mut out,
                m.signature.params.iter().map(|p| p.name.as_str()),
                Declared::Parameter,
            );
            push(
                &mut out,
                m.consts.iter().map(|c| c.name.as_str()),
                Declared::Const,
            );
            for binding in m.body_bindings() {
                let what = match binding {
                    AlgorithmBinding::Local { .. } | AlgorithmBinding::RecordLocal { .. } => {
                        Declared::Local
                    }
                    AlgorithmBinding::ForeachItem { .. } => Declared::LoopItem,
                };
                out.push((binding.name(), what));
            }
        }
        // No expression reads a name these declare: a timer fires an event,
        // a link and a worker wire channels, a buffer pool and a bounded
        // collection size storage, a queue names a storage mode and the
        // contexts around it, an enum lists values (their uniqueness is
        // the enum parser's), and a statechart's names are the ECMAScript
        // datamodel's, resolved by `crate::ecmascript::scope`. A feature
        // model's names are read by its importers' conditions, never by an
        // expression, and their uniqueness is the feature-model parser's.
        ForgeDocument::Timer(_)
        | ForgeDocument::Link(_)
        | ForgeDocument::BufferPool(_)
        | ForgeDocument::Worker(_)
        | ForgeDocument::BoundedCollection(_)
        | ForgeDocument::Queue(_)
        | ForgeDocument::BroadcastLog(_)
        | ForgeDocument::Enum(_)
        | ForgeDocument::FeatureModel(_)
        | ForgeDocument::Statechart(_) => {}
    }
    out
}

/// Refuse the first name the document declares twice, and a callable it
/// declares under a name of the expression vocabulary.
///
/// The vocabulary (`len`, `eq`, `round`, `floor` and the eight wrapping and
/// saturating operations, docs/adr/0063) is declared before any document: a
/// call of one of its names is the vocabulary's whatever the document says,
/// so an import alias or a helper of that name would be shadowed without a
/// word, and is one name declared twice. A value of that name is not: no
/// expression calls a value, so a parameter `len` stays a parameter.
pub fn check(parsed: &ParsedForge, document: &str) -> Result<(), Located<ForgeError>> {
    let kind: ForgeKind = parsed.document.kind();
    let mut first: BTreeMap<&str, Declared> = BTreeMap::new();
    for (name, what) in declarations(parsed) {
        if matches!(what, Declared::ImportAlias | Declared::Helper)
            && crate::forge::expr::vocabulary_names().any(|v| v == name)
        {
            let error = ValidationError::DuplicateId {
                kind,
                what: format!("name (expression vocabulary and {})", what.as_str()),
                id: name.to_string(),
            };
            return Err(Located::new(error.into(), document, None, None));
        }
        let Some(&earlier) = first.get(name) else {
            first.insert(name, what);
            continue;
        };
        // RFC §synth-5-A: an algorithm body may not rebind a parameter or
        // an earlier local, and says so with its own code.
        let error = if what.is_algorithm_body_binding() {
            ValidationError::AlgorithmLocalShadowsParam {
                name: name.to_string(),
                what: "another binding (param or earlier local)".into(),
            }
        } else {
            // Reads as `algorithm: duplicate name (import alias and
            // parameter): 'frame'` — the two declarations, first first.
            ValidationError::DuplicateId {
                kind,
                what: format!("name ({} and {})", earlier.as_str(), what.as_str()),
                id: name.to_string(),
            }
        };
        return Err(Located::new(error.into(), document, None, None));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> ParsedForge {
        crate::forge::parser::parse_forge_with_imports(
            src,
            crate::DocumentLabel::symmetric("t.scxml"),
        )
        .expect("parses")
        .expect("is a forge document")
    }

    fn refusal(src: &str) -> ValidationError {
        match check(&parse(src), "t.scxml")
            .expect_err("must be refused")
            .error
        {
            ForgeError::Validation(v) => *v,
            other => panic!("expected a validation refusal, got {other:?}"),
        }
    }

    const HEAD: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0""#;

    #[test]
    fn two_inputs_of_one_name_are_one_name_declared_twice() {
        let e = refusal(&format!(
            r#"{HEAD} sce:kind="transform" name="t"><datamodel>
                 <data id="a" sce:type="int32" sce:direction="in"/>
                 <data id="a" sce:type="bool" sce:direction="in"/>
                 <data id="o" sce:type="int32" sce:direction="out" expr="1"/>
               </datamodel></scxml>"#
        ));
        assert!(
            matches!(&e, ValidationError::DuplicateId { id, .. } if id == "a"),
            "{e:?}"
        );
    }

    #[test]
    fn an_input_and_an_output_of_one_name_is_a_duplicate_not_a_cycle() {
        // Before this pass the same document was refused as
        // `transform-output-cycle` — what the generator saw, not what the
        // author did.
        let e = refusal(&format!(
            r#"{HEAD} sce:kind="transform" name="t"><datamodel>
                 <data id="a" sce:type="int32" sce:direction="in"/>
                 <data id="a" sce:type="int32" sce:direction="out" expr="a + 1"/>
               </datamodel></scxml>"#
        ));
        assert!(
            matches!(&e, ValidationError::DuplicateId { id, .. } if id == "a"),
            "{e:?}"
        );
    }

    #[test]
    fn a_parameter_named_like_an_import_alias_is_refused() {
        let e = refusal(&format!(
            r#"{HEAD} sce:kind="algorithm" name="a">
                 <sce:import src="c.scxml" kind="codec" as="frame"/>
                 <sce:signature><sce:param name="frame" type="uint8"/><sce:return type="bool"/></sce:signature>
                 <sce:body><sce:return expr="true"/></sce:body>
               </scxml>"#
        ));
        assert!(
            matches!(&e, ValidationError::DuplicateId { id, what, .. }
                if id == "frame" && what.contains("import alias")),
            "{e:?}"
        );
    }

    /// A call of a vocabulary name is the vocabulary's (docs/adr/0063), so an
    /// import declared under one could never be called: refused where it is
    /// declared, for the old names as for the new.
    #[test]
    fn an_import_named_like_a_vocabulary_name_is_refused() {
        for alias in ["wrappingSub", "saturatingAdd", "round", "len"] {
            let e = refusal(&format!(
                r#"{HEAD} sce:kind="algorithm" name="a">
                     <sce:import src="other.scxml" kind="algorithm" as="{alias}"/>
                     <sce:signature><sce:param name="x" type="uint8"/><sce:return type="uint8"/></sce:signature>
                     <sce:body><sce:return expr="x"/></sce:body>
                   </scxml>"#
            ));
            assert!(
                matches!(&e, ValidationError::DuplicateId { id, what, .. }
                    if id == alias && what.contains("expression vocabulary") && what.contains("import alias")),
                "{alias}: {e:?}"
            );
        }
    }

    /// A value is never called, so a parameter spelled like a vocabulary name
    /// stays a parameter: only a callable collides.
    #[test]
    fn a_value_named_like_a_vocabulary_name_is_a_value() {
        check(
            &parse(&format!(
                r#"{HEAD} sce:kind="algorithm" name="a">
                     <sce:signature><sce:param name="len" type="uint8"/><sce:param name="wrappingAdd" type="uint8"/><sce:return type="uint8"/></sce:signature>
                     <sce:body><sce:return expr="len"/></sce:body>
                   </scxml>"#
            )),
            "t.scxml",
        )
        .expect("a value of a vocabulary name passes");
    }

    #[test]
    fn a_duplicate_parameter_is_the_documents_fault_not_a_backends() {
        let e = refusal(&format!(
            r#"{HEAD} sce:kind="algorithm" name="a">
                 <sce:signature><sce:param name="x" type="uint8"/><sce:param name="x" type="uint8"/><sce:return type="uint8"/></sce:signature>
                 <sce:body><sce:return expr="x"/></sce:body>
               </scxml>"#
        ));
        assert!(
            matches!(&e, ValidationError::DuplicateId { id, .. } if id == "x"),
            "{e:?}"
        );
    }

    #[test]
    fn a_local_that_reuses_a_name_keeps_its_own_code() {
        let e = refusal(&format!(
            r#"{HEAD} sce:kind="algorithm" name="a">
                 <sce:signature><sce:param name="x" type="uint8"/><sce:return type="uint8"/></sce:signature>
                 <sce:body><sce:var name="x" type="uint8" init="1"/><sce:return expr="x"/></sce:body>
               </scxml>"#
        ));
        assert!(
            matches!(&e, ValidationError::AlgorithmLocalShadowsParam { name, .. } if name == "x"),
            "{e:?}"
        );
    }

    #[test]
    fn a_document_whose_names_are_distinct_passes() {
        check(
            &parse(&format!(
                r#"{HEAD} sce:kind="transform" name="t"><datamodel>
                     <data id="a" sce:type="int32" sce:direction="in"/>
                     <data id="b" sce:type="int32" sce:direction="out" expr="a + 1"/>
                   </datamodel></scxml>"#
            )),
            "t.scxml",
        )
        .expect("distinct names pass");
    }
}
