// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
            push(
                &mut out,
                m.fields.iter().map(|f| f.id.as_str()),
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
        // collection size storage, an enum lists values (their uniqueness is
        // the enum parser's), and a statechart's names are the ECMAScript
        // datamodel's, resolved by `crate::ecmascript::scope`.
        ForgeDocument::Timer(_)
        | ForgeDocument::Link(_)
        | ForgeDocument::BufferPool(_)
        | ForgeDocument::Worker(_)
        | ForgeDocument::BoundedCollection(_)
        | ForgeDocument::Enum(_)
        | ForgeDocument::Statechart(_) => {}
    }
    out
}

/// Refuse the first name the document declares twice.
pub fn check(parsed: &ParsedForge, document: &str) -> Result<(), Located<ForgeError>> {
    let kind: ForgeKind = parsed.document.kind();
    let mut first: BTreeMap<&str, Declared> = BTreeMap::new();
    for (name, what) in declarations(parsed) {
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
