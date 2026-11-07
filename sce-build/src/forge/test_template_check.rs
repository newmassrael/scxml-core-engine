// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A test template (docs/SCE_TEST_PROFILE.md §11.4): the rules of its
//! matchers that need the documents it imports, and whether it is specific
//! through the templates it references.
//!
//! The parser holds a template to every rule its own spelling reaches: the
//! root, the elements of a list it spells, the items of a value list or a
//! complement ([`crate::forge::parser`]). A field of an imported record and the
//! case of an imported union are typed by their documents, and so is every
//! position below them; a referenced template's type and formal parameters
//! are its own document's. Those are asked here, where the imports are read,
//! by the same reading of a matcher at a position
//! ([`crate::forge::test_profile::check_position`]):
//!
//! - a `record-match` holds one matcher for each field of its record and no
//!   other (B.1.1), each admitted by its field's type, an optional field also
//!   taking `omit`, `any-value-or-none` and `ifpresent` (B.1.2.4, B.1.2.8,
//!   B.1.4.2);
//! - a `union-match` names a case of its union, and holds the matcher of the
//!   case's record, or none for a case that holds no record;
//! - a literal is of its position's type, an enum's the name of one of its
//!   variants; a `param` and a range's bound are of its position's type;
//! - a `template-ref` names a template whose type is its position's, and
//!   binds each of that template's formal parameters, in order, to one of this
//!   template's own of the same type (-1 §15.1, §5.4.1.1);
//! - references form no cycle across documents.
//!
//! Two types read in two documents are one type when each fits the other,
//! every record, union and enum told by the document its alias names in its
//! own document (docs/adr/0086, section 5.1). An import that cannot be read is
//! left out, as [`crate::forge::import_source::parse_quietly`] leaves it: the
//! import pass reports it in its own words.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::forge::error::{ForgeError, Located, TestProfileRule, ValidationError};
use crate::forge::import_source::{type_scope, ImportSource};
use crate::forge::model::{
    ForgeDocument, ForgeImport, ForgeKind, ParsedForge, SceType, TestMatcher, TestMatcherForm,
    TestTemplateModel, TestValue,
};
use crate::forge::static_datamodel::ScopedNames;
use crate::forge::test_profile::{self, MatcherRefusal};
use crate::forge::value_type::{fits, TypeScope, ValueType};

/// Refuse a test template whose references form a cycle
/// (`validation/cross-kind-circular-dependency`, on the reference that leads
/// into it) or one of whose matchers breaks a rule of its position that the
/// imported documents decide (the codes of
/// [`crate::forge::test_profile::check_position`];
/// `test-profile/record-fields`; `validation/invalid-attribute` for a variant
/// the union does not declare; `validation/attribute-rule-violated` for a
/// case's matcher that is missing or has no record to match). A document of
/// any other kind has none of these rules.
///
/// `base_dir` is the template's directory, which its imports are written
/// relative to; `document` labels the refusal.
pub fn check(
    parsed: &ParsedForge,
    base_dir: &Path,
    document: &str,
) -> Result<(), Located<ForgeError>> {
    let ForgeDocument::TestTemplate(template) = &parsed.document else {
        return Ok(());
    };
    refuse_reference_cycle(template, &parsed.imports, base_dir, document)?;

    let own = type_scope(&parsed.imports, base_dir);
    let references: Vec<Reference> = parsed
        .imports
        .iter()
        .filter(|import| import.kind == ForgeKind::TestTemplate)
        .filter_map(|import| {
            let template = referenced(&parsed.imports, base_dir, &import.alias)?;
            let scope = type_scope(&template.imports, &template.dir);
            Some(Reference {
                alias: import.alias.clone(),
                template,
                scope,
            })
        })
        .collect();
    let checker = Checker {
        template,
        document,
        own: &own,
        references: &references,
    };
    checker.matcher(&template.matcher, &template.template_type, &own)
}

/// Whether `template` is specific (docs/SCE_TEST_PROFILE.md §6.2, §11.4):
/// its matchers are only `specific`, `record-match`, `list-match`,
/// `union-match`, `omit` and `template-ref`, none with a `length-min`, a
/// `length-max` or an `ifpresent`, and every template it references is
/// specific in turn. A send names a specific template (-1 §22.2.1); a receive
/// names any.
///
/// `imports` are the template's own, written relative to `base_dir`, its
/// directory. A reference that cannot be read, or that leads back to a
/// template on the way, makes no specific template: the answer is `false`,
/// and the import pass refuses the first, [`check`] the second.
pub fn is_specific(template: &TestTemplateModel, imports: &[ForgeImport], base_dir: &Path) -> bool {
    specific_below(template, imports, base_dir, &mut Vec::new())
}

/// [`is_specific`], with the identities of the templates on the way down.
fn specific_below(
    template: &TestTemplateModel,
    imports: &[ForgeImport],
    base_dir: &Path,
    on_path: &mut Vec<String>,
) -> bool {
    if !template.matcher.is_specific() {
        return false;
    }
    references_of(&template.matcher)
        .into_iter()
        .all(|(alias, _)| {
            let Some(next) = referenced(imports, base_dir, alias) else {
                return false;
            };
            if on_path.contains(&next.identity) {
                return false;
            }
            on_path.push(next.identity.clone());
            let specific = specific_below(&next.model, &next.imports, &next.dir, on_path);
            on_path.pop();
            specific
        })
}

// ── The templates a template references ────────────────────────

/// A template another references, as its import reads it: the document it
/// is, the `src` its importer wrote, its model, and its own imports with the
/// directory they are written relative to.
struct Referenced {
    /// What tells two references to one document apart
    /// ([`crate::forge::stdlib::identity`]).
    identity: String,
    src: String,
    model: TestTemplateModel,
    imports: Vec<ForgeImport>,
    dir: PathBuf,
}

/// The template `imports` (written relative to `base_dir`) import as
/// `alias`, or `None` when no import of a template goes by it or it cannot be
/// read.
fn referenced(imports: &[ForgeImport], base_dir: &Path, alias: &str) -> Option<Referenced> {
    let import = imports
        .iter()
        .find(|import| import.alias == alias && import.kind == ForgeKind::TestTemplate)?;
    let source = ImportSource::read(base_dir, import).ok()?;
    let parsed = source.parse().ok()??;
    let ForgeDocument::TestTemplate(model) = parsed.document else {
        return None;
    };
    Some(Referenced {
        identity: crate::forge::stdlib::identity(&source.path),
        src: import.src.clone(),
        model,
        imports: parsed.imports,
        dir: source.path.parent().unwrap_or(base_dir).to_path_buf(),
    })
}

/// The alias of every template `matcher` and the matchers inside it
/// reference, in document order, with the line of each reference.
fn references_of(matcher: &TestMatcher) -> Vec<(&str, Option<u32>)> {
    let mut found = Vec::new();
    let mut pending = vec![matcher];
    while let Some(next) = pending.pop() {
        if let TestMatcherForm::TemplateRef { template, .. } = &next.form {
            found.push((template.as_str(), next.source_line));
        }
        pending.extend(next.children().iter().rev());
    }
    found
}

/// Refuse a template whose references, followed through the documents they
/// name, come back to a template already on the way: such a template has no
/// value to match by (docs/SCE_TEST_PROFILE.md §11.4). The refusal stands on
/// the reference of `template` that leads into the cycle, and names the
/// cycle by the `src` of each import on it, as an import cycle is named.
fn refuse_reference_cycle(
    template: &TestTemplateModel,
    imports: &[ForgeImport],
    base_dir: &Path,
    document: &str,
) -> Result<(), Located<ForgeError>> {
    let mut acyclic: HashSet<String> = HashSet::new();
    for (alias, line) in references_of(&template.matcher) {
        let mut on_path: Vec<(String, String)> = Vec::new();
        if let Some(cycle) = cycle_below(imports, base_dir, alias, &mut on_path, &mut acyclic) {
            return Err(Located::new(
                ValidationError::CrossKindCircularDependency { cycle }.into(),
                document,
                line,
                None,
            ));
        }
    }
    Ok(())
}

/// The cycle the references from the template imported as `alias` close,
/// `on_path` holding the identity and `src` of each template on the way down;
/// `acyclic` holds the templates already followed to their end without one.
fn cycle_below(
    imports: &[ForgeImport],
    base_dir: &Path,
    alias: &str,
    on_path: &mut Vec<(String, String)>,
    acyclic: &mut HashSet<String>,
) -> Option<Vec<String>> {
    let next = referenced(imports, base_dir, alias)?;
    if let Some(start) = on_path
        .iter()
        .position(|(identity, _)| *identity == next.identity)
    {
        let mut cycle: Vec<String> = on_path[start..]
            .iter()
            .map(|(_, src)| src.clone())
            .collect();
        cycle.push(next.src);
        return Some(cycle);
    }
    if acyclic.contains(&next.identity) {
        return None;
    }
    on_path.push((next.identity.clone(), next.src.clone()));
    for (further, _) in references_of(&next.model.matcher) {
        if let Some(cycle) = cycle_below(&next.imports, &next.dir, further, on_path, acyclic) {
            return Some(cycle);
        }
    }
    on_path.pop();
    acyclic.insert(next.identity);
    None
}

// ── Types read in two documents ────────────────────────────────

/// Whether every record, union and enum `ty` names is read in `scope`.
fn resolved(ty: &ValueType, scope: &TypeScope) -> bool {
    ty.record_aliases()
        .iter()
        .all(|alias| scope.records.contains_key(*alias))
        && ty
            .union_aliases()
            .iter()
            .all(|alias| scope.unions.contains_key(*alias))
        && ty
            .enum_aliases()
            .iter()
            .all(|alias| scope.enums.contains_key(*alias))
}

/// Whether `a`, read in `a_scope`, and `b`, read in `b_scope`, are shown to
/// be two types: both are read, and one of them does not fit the other
/// (docs/adr/0086, section 5.1). A type that names a document no scope reads
/// is shown nothing: its import is refused where it is read.
fn differ(a: &ValueType, a_scope: &TypeScope, b: &ValueType, b_scope: &TypeScope) -> bool {
    if !resolved(a, a_scope) || !resolved(b, b_scope) {
        return false;
    }
    let there = ScopedNames {
        from: Some(a_scope),
        to: Some(b_scope),
    };
    let back = ScopedNames {
        from: Some(b_scope),
        to: Some(a_scope),
    };
    fits(a, b, &there).is_err() || fits(b, a, &back).is_err()
}

// ── The matchers of one template ───────────────────────────────

/// A template the checked one imports, by the alias it imports it under, with
/// the scope its own types are read in.
struct Reference {
    alias: String,
    template: Referenced,
    scope: TypeScope,
}

/// The matchers of one template, held to the types of their positions.
struct Checker<'c> {
    template: &'c TestTemplateModel,
    /// The label a refusal is placed under.
    document: &'c str,
    /// The scope the template's own types are read in: its type's and its
    /// formal parameters'.
    own: &'c TypeScope,
    references: &'c [Reference],
}

impl Checker<'_> {
    /// A rule of the profile a matcher breaks, on its line.
    fn refused(&self, line: Option<u32>, refusal: MatcherRefusal) -> Located<ForgeError> {
        Located::new(
            ValidationError::TestProfile {
                rule: refusal.rule,
                kind: ForgeKind::TestTemplate,
                document: self.template.name.clone(),
                actual: refusal.actual,
                detail: refusal.detail,
            }
            .into(),
            self.document,
            line,
            None,
        )
    }

    /// A shape rule every kind shares, on `line`.
    fn shape(&self, line: Option<u32>, error: ValidationError) -> Located<ForgeError> {
        Located::new(error.into(), self.document, line, None)
    }

    /// `matcher`, standing where a value of `ty`, read in `scope`, is
    /// matched, and every matcher inside it at its own position.
    fn matcher(
        &self,
        matcher: &TestMatcher,
        ty: &ValueType,
        scope: &TypeScope,
    ) -> Result<(), Located<ForgeError>> {
        let line = matcher.source_line;
        let parameters = self.template.parameters.as_slice();
        let same =
            |position: &ValueType, value: &ValueType| !differ(position, scope, value, self.own);
        test_profile::check_position(matcher, ty, parameters, &same)
            .map_err(|refusal| self.refused(line, refusal))?;
        match &matcher.form {
            TestMatcherForm::Specific {
                value: TestValue::Value(text),
            } => self.enum_literal(line, ty, scope, text),
            TestMatcherForm::RecordMatch { fields } => {
                self.record_match(matcher, fields, ty, scope)
            }
            TestMatcherForm::ListMatch { elements } => match test_profile::list_element(ty) {
                Some(element_ty) => elements
                    .iter()
                    .try_for_each(|element| self.matcher(element, element_ty, scope)),
                None => Ok(()),
            },
            TestMatcherForm::UnionMatch { variant, case } => {
                self.union_match(matcher, variant, case.as_deref(), ty, scope)
            }
            TestMatcherForm::ValueList { items } | TestMatcherForm::Complement { items } => items
                .iter()
                .try_for_each(|item| self.matcher(item, ty, scope)),
            TestMatcherForm::ValueRange { lower, upper } => {
                for (bound, element) in [(lower, "<sce:lower>"), (upper, "<sce:upper>")] {
                    if let Some(value) = bound {
                        test_profile::check_range_bound(
                            element,
                            value,
                            Some(ty),
                            parameters,
                            &same,
                        )
                        .map_err(|refusal| self.refused(line, refusal))?;
                    }
                }
                Ok(())
            }
            TestMatcherForm::TemplateRef { template, args } => {
                self.template_ref(line, template, args, ty, scope)
            }
            TestMatcherForm::Specific {
                value: TestValue::Param(_),
            }
            | TestMatcherForm::AnyValue
            | TestMatcherForm::AnyValueOrNone
            | TestMatcherForm::Omit => Ok(()),
        }
    }

    /// A literal of an enum names one of its variants: the rule
    /// [`test_profile::literal_of`] leaves to the enum's document.
    fn enum_literal(
        &self,
        line: Option<u32>,
        ty: &ValueType,
        scope: &TypeScope,
        text: &str,
    ) -> Result<(), Located<ForgeError>> {
        let ValueType::Scalar(SceType::Enum(reference)) = test_profile::present(ty) else {
            return Ok(());
        };
        let Some(declared) = scope.enums.get(&reference.alias) else {
            return Ok(());
        };
        if declared.variants.iter().any(|v| v.name == text.trim()) {
            return Ok(());
        }
        let names: Vec<&str> = declared.variants.iter().map(|v| v.name.as_str()).collect();
        Err(self.refused(
            line,
            MatcherRefusal {
                rule: TestProfileRule::Literal,
                actual: text.to_string(),
                detail: format!(
                    "<sce:specific> value '{text}' is not a value of {ty}: expected the name of \
                     a variant of the enum {} ({})",
                    reference.alias,
                    names.join(", ")
                ),
            },
        ))
    }

    /// A `record-match` (B.1.1) where a value of `ty` is matched: one matcher
    /// for each field of the record and for no other, each at its field's
    /// type, read in the record's own document.
    fn record_match(
        &self,
        matcher: &TestMatcher,
        fields: &[TestMatcher],
        ty: &ValueType,
        scope: &TypeScope,
    ) -> Result<(), Located<ForgeError>> {
        let ValueType::Record { alias } = test_profile::present(ty) else {
            return Ok(());
        };
        let Some(record) = scope.records.get(alias) else {
            return Ok(());
        };
        let declared: Vec<&str> = record.fields.iter().map(|(id, _)| id.as_str()).collect();
        let rule = "a <sce:record-match> holds one matcher for each field of its record, and \
                    for no other";
        // A field the record does not declare, on the matcher that names it.
        for field in fields {
            let name = field.field.as_deref().unwrap_or_default();
            if !declared.contains(&name) {
                return Err(self.refused(
                    field.source_line,
                    MatcherRefusal {
                        rule: TestProfileRule::RecordFields,
                        actual: name.to_string(),
                        detail: format!(
                            "<sce:{}> names the field '{name}', which the record {alias} does \
                             not declare (declared: {}): {rule}",
                            field.form.element(),
                            declared.join(", ")
                        ),
                    },
                ));
            }
        }
        // A field no matcher names is written in the record alone, so the
        // refusal stands on the record-match, with no `actual`.
        let missing: Vec<String> = declared
            .iter()
            .filter(|id| !fields.iter().any(|f| f.field.as_deref() == Some(**id)))
            .map(|id| format!("'{id}'"))
            .collect();
        if !missing.is_empty() {
            return Err(self.refused(
                matcher.source_line,
                MatcherRefusal {
                    rule: TestProfileRule::RecordFields,
                    actual: String::new(),
                    detail: format!(
                        "<sce:record-match> of the record {alias} holds no matcher for the \
                         field(s) {}: {rule}",
                        missing.join(", ")
                    ),
                },
            ));
        }
        for field in fields {
            let name = field.field.as_deref().unwrap_or_default();
            if let Some((_, field_ty)) = record.fields.iter().find(|(id, _)| id == name) {
                self.matcher(field, field_ty, &record.scope)?;
            }
        }
        Ok(())
    }

    /// A `union-match` where a value of `ty` is matched: its variant is a case
    /// of the union, and it holds the matcher of the case's record, read in
    /// the union's own document, or none for a case that holds no record.
    fn union_match(
        &self,
        matcher: &TestMatcher,
        variant: &str,
        case: Option<&TestMatcher>,
        ty: &ValueType,
        scope: &TypeScope,
    ) -> Result<(), Located<ForgeError>> {
        let ValueType::Union { alias } = test_profile::present(ty) else {
            return Ok(());
        };
        let Some(union) = scope.unions.get(alias) else {
            return Ok(());
        };
        let line = matcher.source_line;
        let Some(declared) = union.cases.iter().find(|c| c.variant.name == variant) else {
            return Err(self.shape(
                line,
                ValidationError::InvalidAttribute {
                    element: "<sce:union-match>".into(),
                    attr: "variant".into(),
                    value: variant.to_string(),
                    allowed: union.cases.iter().map(|c| c.variant.name.clone()).collect(),
                },
            ));
        };
        let element = format!("<sce:union-match variant=\"{variant}\">");
        match (&declared.record, case) {
            (Some(record), Some(case)) => self.matcher(
                case,
                &ValueType::Record {
                    alias: record.clone(),
                },
                &union.scope,
            ),
            (None, None) => Ok(()),
            (Some(record), None) => Err(self.shape(
                line,
                ValidationError::AttributeRuleViolated {
                    element,
                    attr: "matcher".into(),
                    value: "0".into(),
                    rule: format!(
                        "one matcher, of the record {record} the case '{variant}' of the union \
                         {alias} holds"
                    ),
                },
            )),
            (None, Some(_)) => Err(self.shape(
                line,
                ValidationError::AttributeRuleViolated {
                    element,
                    attr: "matcher".into(),
                    value: "1".into(),
                    rule: format!(
                        "no matcher: the case '{variant}' of the union {alias} holds no record, \
                         its tag alone"
                    ),
                },
            )),
        }
    }

    /// A `template-ref` (-1 §15.1) where a value of `ty`, read in `scope`, is
    /// matched: the referenced template is of that type, and `args` binds
    /// each of its formal parameters, in order, to one of this template's own
    /// of the same type (-1 §5.4.1.1).
    fn template_ref(
        &self,
        line: Option<u32>,
        alias: &str,
        args: &[String],
        ty: &ValueType,
        scope: &TypeScope,
    ) -> Result<(), Located<ForgeError>> {
        let Some(reference) = self.references.iter().find(|r| r.alias == alias) else {
            return Ok(());
        };
        let referenced = &reference.template.model;
        let of_position = |position: &ValueType, value: &ValueType| {
            !differ(position, scope, value, &reference.scope)
        };
        if !test_profile::stands_at(ty, &referenced.template_type, &of_position) {
            return Err(self.refused(
                line,
                MatcherRefusal {
                    rule: TestProfileRule::MatcherType,
                    actual: alias.to_string(),
                    detail: format!(
                        "<sce:template-ref> names {alias}, a template of {}, where a value of \
                         {ty} is matched: a reference stands where a value of its template's \
                         type is matched",
                        referenced.template_type
                    ),
                },
            ));
        }
        if args.len() != referenced.parameters.len() {
            let formal: Vec<&str> = referenced
                .parameters
                .iter()
                .map(|p| p.id.as_str())
                .collect();
            return Err(self.refused(
                line,
                MatcherRefusal {
                    rule: TestProfileRule::TemplateParameter,
                    actual: args.len().to_string(),
                    detail: format!(
                        "<sce:template-ref> binds {} argument(s) to {alias}, which takes {} \
                         formal parameter(s) ({}): args binds each formal parameter, in order, \
                         to one of this template's own",
                        args.len(),
                        formal.len(),
                        formal.join(", ")
                    ),
                },
            ));
        }
        for (arg, formal) in args.iter().zip(&referenced.parameters) {
            let own = test_profile::formal_parameter(
                &self.template.parameters,
                "<sce:template-ref>",
                arg,
            )
            .map_err(|refusal| self.refused(line, refusal))?;
            if differ(
                &own.parameter_type,
                self.own,
                &formal.parameter_type,
                &reference.scope,
            ) {
                return Err(self.refused(
                    line,
                    MatcherRefusal {
                        rule: TestProfileRule::TemplateParameter,
                        actual: arg.clone(),
                        detail: format!(
                            "<sce:template-ref> binds '{arg}', of type {}, to the formal \
                             parameter '{}' of {alias}, of type {}: an argument is of its \
                             formal parameter's type",
                            own.parameter_type, formal.id, formal.parameter_type
                        ),
                    },
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::model::EnumRef;
    use crate::forge::value_type::{Bound, ScopedEnum, ScopedVariant, TypeOf};

    fn color(variants: &[&str]) -> ScopedEnum {
        ScopedEnum {
            name: "paint_color".to_string(),
            variants: variants
                .iter()
                .zip(0..)
                .map(|(name, value)| ScopedVariant {
                    name: (*name).to_string(),
                    value,
                })
                .collect(),
            underlying: SceType::Uint8,
            strict: true,
        }
    }

    fn scope_with(alias: &str, declared: ScopedEnum) -> TypeScope {
        let mut scope = TypeScope::default();
        scope.enums.insert(alias.to_string(), declared);
        scope
    }

    fn enum_of(alias: &str) -> ValueType {
        TypeOf::Scalar(SceType::Enum(EnumRef {
            alias: alias.to_string(),
        }))
    }

    /// One enum read under two aliases in two documents is one type; two
    /// enums of different variants are two, and so are two bounds.
    #[test]
    fn two_types_read_in_two_documents_are_told_by_what_their_aliases_name() {
        let here = scope_with("Color", color(&["red", "green"]));
        let there = scope_with("Hue", color(&["red", "green"]));
        let other = scope_with("Hue", color(&["red", "blue"]));
        assert!(!differ(&enum_of("Color"), &here, &enum_of("Hue"), &there));
        assert!(differ(&enum_of("Color"), &here, &enum_of("Hue"), &other));
        let short: ValueType = TypeOf::Text { max: Bound::of(8) };
        let long: ValueType = TypeOf::Text { max: Bound::of(16) };
        assert!(differ(&short, &here, &long, &there));
        assert!(!differ(&short, &here, &short, &there));
        // An optional and its present value are two types.
        let optional: ValueType = TypeOf::Optional {
            elem: Box::new(short.clone()),
        };
        assert!(differ(&optional, &here, &short, &here));
    }

    /// A type that names a document no scope reads is shown nothing.
    #[test]
    fn a_type_no_scope_reads_differs_from_none() {
        let empty = TypeScope::default();
        assert!(!differ(
            &enum_of("Missing"),
            &empty,
            &TypeOf::Scalar(SceType::Uint8),
            &empty
        ));
    }
}
