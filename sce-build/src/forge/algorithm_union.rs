// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! An algorithm over a union (docs/adr/0092, Landing order step 5): the
//! unions its parameters and locals hold, the case a union local builds, and
//! the point of a body no path passes.
//!
//! A parameter or a local of `union:U` is a place of the one value type
//! ([`crate::forge::value_path`]), read through its tag anywhere and through a
//! field of its cases where a test of the tag leaves cases that each declare
//! it, by the narrowing a statechart's judge keeps ([`crate::forge::narrowing`]).
//! What is an algorithm's own is said here, once, for the judge that runs
//! before any backend and for the backends that lower one:
//!
//! - a union local is built whole: one `<sce:set>` names the tag with a
//!   variant literal of the enum the algorithm imports the tag's enum as, and
//!   the others give exactly the fields of that case's record, a record the
//!   algorithm imports, since a backend names its type where it builds the
//!   case ([`AlgorithmUnions::built`]);
//! - a point where a subject of a union has no case left is not reached
//!   (section 6.1 item 4, [`AlgorithmUnions::exhausted`]), so a body whose
//!   every path that reaches its end passes such a point needs no
//!   `<sce:return>` there ([`reaches_end`]). A union that gains a case leaves
//!   that case at the point, the path reaches the end again, and the body is
//!   refused with `algorithm/return-missing` until it handles the case.

use crate::attribute_spelling::AttributeSpelling;
use crate::forge::error::{ExprError, ForgeError, ValidationError};
use crate::forge::expression_site::{ExpressionSite, WrittenAt};
use crate::forge::generator::{import_type_scope, ImportContext};
use crate::forge::model::{AlgorithmModel, AlgorithmStmt, RecordFieldInit};
use crate::forge::narrowing::{after_algorithm_statement, Chain, Facts, Statement};
use crate::forge::value_path::{
    case_of_tag_literal, case_record_alias, cases_left, tag_enum_alias,
};
use crate::forge::value_type::{ScopedCase, ScopedUnion, TypeScope};

/// The unions an algorithm holds, and the scope its imports name them in.
pub(crate) struct AlgorithmUnions<'a> {
    /// The scope the algorithm's imports name enums, records and unions in
    /// ([`import_type_scope`]).
    pub scope: &'a TypeScope,
    /// Each parameter and local of `union:<alias>`, by name, with its alias
    /// ([`crate::forge::model::algorithm_unions`]).
    pub held: &'a [(&'a str, &'a str)],
}

/// What a union local's `<sce:set>` children build ([`AlgorithmUnions::built`]):
/// the case its tag names, the alias the algorithm imports that case's record
/// by (`None` for an empty case), and the `<sce:set>` of each field of the
/// record, in the order the record declares them. The tag's is not among them.
pub(crate) struct BuiltCase<'u, 'f> {
    pub case: &'u ScopedCase,
    pub record: Option<String>,
    pub fields: Vec<&'f RecordFieldInit>,
}

impl<'a> AlgorithmUnions<'a> {
    /// The union the algorithm imports as `alias`.
    pub fn union(&self, alias: &str) -> Option<&'a ScopedUnion> {
        self.scope.unions.get(alias)
    }

    /// Whether the name `name` is a union parameter or local.
    pub fn holds(&self, name: &str) -> bool {
        self.held.iter().any(|(held, _)| *held == name)
    }

    /// The alias of the union the parameter or local `name` holds.
    pub fn alias_of(&self, name: &str) -> Option<&'a str> {
        self.held
            .iter()
            .find(|(held, _)| *held == name)
            .map(|(_, alias)| *alias)
    }

    /// Whether a subject of a union has no case left where `facts` hold: the
    /// tests of its tag that hold there name every case (docs/adr/0092,
    /// section 6.1 item 4). Such a point is not reached.
    pub fn exhausted(&self, facts: &Facts) -> bool {
        self.held.iter().any(|&(name, alias)| {
            self.union(alias).is_some_and(|union| {
                cases_left(Some(self.scope), union, Some(name), Some(facts)).is_empty()
            })
        })
    }

    /// The case the union local `name` of `union:<alias>` builds from its
    /// `<sce:set>` children `fields` (docs/adr/0092, section 6.1 item 5): one
    /// names the tag with a variant literal of the tag's enum, as the algorithm
    /// imports it, and the others give each field of that case's record exactly
    /// once. Refused where no `<sce:set>` names the tag
    /// (`validation/attribute-rule-violated` on the local's `name`), where the
    /// tag's `expr` is no such literal (`expression/unsupported-construct`: the
    /// build must know the case to know which fields follow), where the
    /// algorithm does not import the case's record, which a backend names where
    /// it builds the case, and where the fields are not exactly the record's
    /// ([`crate::forge::generator::order_record_fields`]).
    pub fn built<'f>(
        &self,
        name: &str,
        name_spelling: Option<&AttributeSpelling>,
        alias: &str,
        fields: &'f [RecordFieldInit],
    ) -> Result<BuiltCase<'a, 'f>, ForgeError> {
        let element = format!("<sce:var name=\"{name}\">");
        let union = self.union(alias).ok_or_else(|| -> ForgeError {
            crate::forge::error::GenerateError::unsupported(format!(
                "{element} is a union:{alias}, a union whose document was not read"
            ))
            .into()
        })?;
        let enum_alias =
            tag_enum_alias(self.scope, union).unwrap_or_else(|| union.tag_enum.clone());
        let Some(tag) = fields.iter().find(|f| f.name == union.tag) else {
            return Err(WrittenAt::value(name_spelling).place_reporting(
                ValidationError::AttributeRuleViolated {
                    element,
                    attr: "name".into(),
                    value: name.to_string(),
                    rule: format!(
                        "a union built whole — one <sce:set name=\"{}\"> names its case with a \
                         variant of `{enum_alias}`, and none does",
                        union.tag
                    ),
                }
                .into(),
            ));
        };
        let tag_site = ExpressionSite::new(&tag.expr, tag.expr_spelling.as_ref());
        let case = case_of_tag_literal(self.scope, union, &tag.expr).ok_or_else(|| {
            tag_site.place(
                ExprError::UnsupportedConstruct {
                    construct: format!(
                        "`{}`, the tag of a union local, which is no variant literal of \
                         `{enum_alias}` (`{enum_alias}.<variant>`): the build must know the \
                         case to know which fields follow (docs/adr/0092, section 6.1 item 5)",
                        tag.expr.trim()
                    ),
                    observed: Some(tag.expr.trim().to_string()),
                }
                .at(None),
            )
        })?;
        let record = match &case.record {
            None => None,
            Some(_) => Some(case_record_alias(self.scope, union, case).ok_or_else(|| {
                tag_site.place(
                    ExprError::UnsupportedConstruct {
                        construct: format!(
                            "the case `{}` of union:{alias}, whose record the algorithm does \
                             not import (import its event schema to build the case)",
                            case.variant.name
                        ),
                        observed: Some(tag.expr.trim().to_string()),
                    }
                    .at(None),
                )
            })?),
        };
        let mut declared: Vec<&str> = vec![union.tag.as_str()];
        if let Some(record) = record.as_deref().and_then(|r| self.scope.records.get(r)) {
            declared.extend(record.fields.iter().map(|(id, _)| id.as_str()));
        }
        let ordered = crate::forge::generator::order_record_fields(
            fields,
            &declared,
            &format!("union:{alias} in its case `{}`", case.variant.name),
            element,
            "name",
            name,
            name_spelling,
        )?;
        Ok(BuiltCase {
            case,
            record,
            fields: ordered.into_iter().skip(1).collect(),
        })
    }
}

/// Whether a path through `block`, entered where `entry` holds, reaches the
/// block's end: one that passes no `<sce:return>`, no `<sce:if>` whose two
/// blocks each end every path through them, and no point where `exhausted`
/// says a subject of a union has no case left (docs/adr/0092, section 6.1
/// item 4). The facts grow by the one sequencing rule of an algorithm's body
/// ([`after_algorithm_statement`]), and an `<sce:if>`'s blocks are read by its
/// [`Chain`], so a chain of tests of a tag whose blocks each return leaves, after
/// it, a subject with no case left once every case was named.
pub(crate) fn reaches_end(
    block: &[AlgorithmStmt],
    entry: &Facts,
    exhausted: &dyn Fn(&Facts) -> bool,
) -> bool {
    let mut facts = entry.clone();
    for statement in block {
        if exhausted(&facts) {
            return false;
        }
        match statement {
            AlgorithmStmt::Return { .. } => return false,
            AlgorithmStmt::If {
                cond,
                then_body,
                else_body: Some(else_body),
                ..
            } => {
                let chain = Chain::of(&facts, &[cond.as_str()]);
                if !reaches_end(then_body, &chain.block(0), exhausted)
                    && !reaches_end(else_body, &chain.otherwise(), exhausted)
                {
                    return false;
                }
            }
            _ => {}
        }
        facts = after_algorithm_statement(&facts, statement);
    }
    !exhausted(&facts)
}

/// The alias `m` imports each record by that a union local of it builds a case
/// of: no expression names that record, and a backend names its type where it
/// builds the case, so the import is kept for the renderer
/// ([`crate::forge::import_use`]). Read from the union's cases in `imports`, the
/// algorithm's own, enriched; a local whose case the build cannot tell names
/// none, and is refused where the body is judged ([`AlgorithmUnions::built`]).
pub(crate) fn built_case_records(m: &AlgorithmModel, imports: &[ImportContext]) -> Vec<String> {
    fn union_locals<'s>(stmts: &'s [AlgorithmStmt], out: &mut Vec<&'s AlgorithmStmt>) {
        for statement in stmts {
            if matches!(statement, AlgorithmStmt::UnionVar { .. }) {
                out.push(statement);
            }
            for block in statement.blocks() {
                union_locals(block, out);
            }
        }
    }
    let mut locals = Vec::new();
    union_locals(&m.body, &mut locals);
    if locals.is_empty() {
        return Vec::new();
    }
    let scope = import_type_scope(imports);
    locals
        .into_iter()
        .filter_map(|statement| {
            let AlgorithmStmt::UnionVar { alias, fields, .. } = statement else {
                return None;
            };
            let union = scope.unions.get(alias)?;
            let tag = fields.iter().find(|f| f.name == union.tag)?;
            let case = case_of_tag_literal(&scope, union, &tag.expr)?;
            case_record_alias(&scope, union, case)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::model::SceType;
    use crate::forge::value_type::{ScopedEnum, ScopedRecord, ScopedVariant, ValueType};

    /// A scope that names the enum `K` of the variants `a`, `b` and `c`, the
    /// record `R` of one field `n`, and the union `U` whose tag `kind` is a
    /// value of `K`, `a` holding `R`, `b` and `c` empty.
    fn scope() -> TypeScope {
        let kind = ScopedEnum {
            name: "k".into(),
            variants: ["a", "b", "c"]
                .into_iter()
                .zip(0i128..)
                .map(|(name, value)| ScopedVariant {
                    name: name.to_string(),
                    value,
                })
                .collect(),
            underlying: SceType::Uint8,
            strict: true,
        };
        let record = ScopedRecord {
            identity: "r.scxml".into(),
            name: "r".into(),
            fields: vec![("n".into(), ValueType::Scalar(SceType::Int32))],
            scope: TypeScope::default(),
        };
        let mut own = TypeScope::default();
        own.enums.insert("K".into(), kind.clone());
        own.records.insert("R".into(), record.clone());
        let case = |name: &str, value: i128, record: Option<&str>| ScopedCase {
            variant: ScopedVariant {
                name: name.to_string(),
                value,
            },
            record: record.map(str::to_string),
        };
        let union = ScopedUnion {
            identity: "u.scxml".into(),
            name: "u".into(),
            tag: "kind".into(),
            tag_enum: "K".into(),
            cases: vec![
                case("a", 0, Some("R")),
                case("b", 1, None),
                case("c", 2, None),
            ],
            scope: own.clone(),
        };
        let mut scope = own;
        scope.unions.insert("U".into(), union);
        scope
    }

    fn if_returning(cond: &str) -> AlgorithmStmt {
        AlgorithmStmt::If {
            cond: cond.into(),
            cond_spelling: None,
            then_body: vec![AlgorithmStmt::Return {
                expr: Some("1".into()),
                expr_spelling: None,
            }],
            else_body: None,
        }
    }

    fn set(name: &str, expr: &str) -> RecordFieldInit {
        RecordFieldInit {
            name: name.into(),
            name_spelling: None,
            expr: expr.into(),
            expr_spelling: None,
        }
    }

    /// A chain of tests of a union's tag whose blocks each return leaves no
    /// path to the end of the body once every case was named, and one while a
    /// case is left (docs/adr/0092, section 6.1 item 4).
    #[test]
    fn a_body_ends_where_every_case_of_its_union_has_returned() {
        let scope = scope();
        let held = [("e", "U")];
        let unions = AlgorithmUnions {
            scope: &scope,
            held: &held,
        };
        let exhausted = |facts: &Facts| unions.exhausted(facts);
        let every = [
            if_returning("e.kind === K.a"),
            if_returning("e.kind === K.b"),
            if_returning("e.kind === K.c"),
        ];
        assert!(!reaches_end(&every, &Facts::none(), &exhausted));
        // `c` is left after the chain, so its path reaches the end.
        assert!(reaches_end(&every[..2], &Facts::none(), &exhausted));
        // Two cases named by one test, the third by another.
        let joined = [
            if_returning("e.kind === K.a || e.kind === K.b"),
            if_returning("e.kind === K.c"),
        ];
        assert!(!reaches_end(&joined, &Facts::none(), &exhausted));
        // An `<sce:if>` and its `<sce:else>` that each end every path.
        let branched = [AlgorithmStmt::If {
            cond: "e.kind === K.a".into(),
            cond_spelling: None,
            then_body: vec![AlgorithmStmt::Return {
                expr: Some("1".into()),
                expr_spelling: None,
            }],
            else_body: Some(vec![
                if_returning("e.kind === K.b"),
                if_returning("e.kind === K.c"),
            ]),
        }];
        assert!(!reaches_end(&branched, &Facts::none(), &exhausted));
        // A test of another subject names no case of this one.
        let other = [
            if_returning("e.kind === K.a"),
            if_returning("f.kind === K.b"),
            if_returning("e.kind === K.c"),
        ];
        assert!(reaches_end(&other, &Facts::none(), &exhausted));
        // A body that holds no union ends only at a `<sce:return>`.
        assert!(reaches_end(&every, &Facts::none(), &|_: &Facts| false));
    }

    /// A union local names its case by a variant literal of the tag's enum,
    /// and gives exactly the fields of that case's record.
    #[test]
    fn a_union_local_builds_the_case_its_tag_names() {
        let scope = scope();
        let held = [("w", "U")];
        let unions = AlgorithmUnions {
            scope: &scope,
            held: &held,
        };
        let fields = [set("n", "3"), set("kind", "K.a")];
        let built = unions
            .built("w", None, "U", &fields)
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(built.case.variant.name, "a");
        assert_eq!(built.record.as_deref(), Some("R"));
        assert_eq!(
            built
                .fields
                .iter()
                .map(|f| f.name.as_str())
                .collect::<Vec<_>>(),
            ["n"]
        );
        // An empty case is its tag alone.
        let empty = [set("kind", "K.b")];
        let built = unions.built("w", None, "U", &empty).unwrap();
        assert!(built.record.is_none() && built.fields.is_empty());
        // A tag that is no variant literal, a field the case does not declare,
        // and no tag at all are each refused.
        assert!(unions.built("w", None, "U", &[set("kind", "x")]).is_err());
        assert!(unions
            .built("w", None, "U", &[set("kind", "K.b"), set("n", "1")])
            .is_err());
        assert!(unions.built("w", None, "U", &[set("n", "1")]).is_err());
        // A case the algorithm imports no record of cannot be built.
        let mut bare = scope.clone();
        bare.records.clear();
        let bare_unions = AlgorithmUnions {
            scope: &bare,
            held: &held,
        };
        assert!(bare_unions.built("w", None, "U", &fields).is_err());
    }
}
