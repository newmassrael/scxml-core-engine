// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The names an algorithm's body binds and no expression of it reads
// (SCE_FORGE.md §4.12): a loop item counted and not looked at, a local that is
// only written, a parameter a later version of the signature still takes.
//
// Such a name is legal, since every backend binds it and nothing depends on it,
// but the target's compiler says otherwise: Go refuses a local that is declared
// and not used, C and C++ refuse one under `-Werror=unused`, Rust and Kotlin
// warn. The generator therefore binds none where it can (a loop over a list) and
// says on purpose that it is unread where it must declare one.

use std::collections::HashSet;

use crate::forge::expr::read_identifiers;
use crate::forge::model::AlgorithmStmt;

/// The names `body` reads: every identifier of an expression that is read for its
/// value, and every name a statement uses through the thing it holds (a buffer
/// appended to, a map written, a list walked). The root of an `<sce:assign>`'s
/// target and the outcome of a map's statement are written and not read.
///
/// `None` where an expression does not parse: it is refused, with its place, where
/// it is lowered, and nothing is decided by it here.
pub(crate) fn names_read(body: &[AlgorithmStmt]) -> Option<HashSet<String>> {
    let mut reads = Reads(HashSet::new());
    reads.block(body)?;
    Some(reads.0)
}

struct Reads(HashSet<String>);

impl Reads {
    fn expr(&mut self, text: &str) -> Option<()> {
        self.0.extend(read_identifiers(text).ok()?);
        Some(())
    }

    fn block(&mut self, stmts: &[AlgorithmStmt]) -> Option<()> {
        stmts.iter().try_for_each(|s| self.statement(s))
    }

    fn statement(&mut self, stmt: &AlgorithmStmt) -> Option<()> {
        match stmt {
            AlgorithmStmt::Var { init, .. } => match init {
                Some(init) => self.expr(init),
                None => Some(()),
            },
            AlgorithmStmt::RecordVar { fields, .. } | AlgorithmStmt::UnionVar { fields, .. } => {
                fields.iter().try_for_each(|f| self.expr(&f.expr))
            }
            AlgorithmStmt::RecordFromCall { init, .. }
            | AlgorithmStmt::UnionCopied { init, .. } => self.expr(init),
            AlgorithmStmt::Assign { expr, .. } => self.expr(expr),
            AlgorithmStmt::Append { target, expr, .. } => {
                self.expr(target)?;
                self.expr(expr)
            }
            AlgorithmStmt::Put {
                target, key, expr, ..
            } => {
                self.expr(target)?;
                self.expr(key)?;
                match expr {
                    Some(expr) => self.expr(expr),
                    None => Some(()),
                }
            }
            AlgorithmStmt::Remove { target, key, .. } => {
                self.expr(target)?;
                self.expr(key)
            }
            AlgorithmStmt::Clear { target, .. } => self.expr(target),
            AlgorithmStmt::If {
                cond,
                then_body,
                else_body,
                ..
            } => {
                self.expr(cond)?;
                self.block(then_body)?;
                match else_body {
                    Some(else_body) => self.block(else_body),
                    None => Some(()),
                }
            }
            AlgorithmStmt::While { cond, body, .. } => {
                self.expr(cond)?;
                self.block(body)
            }
            AlgorithmStmt::Foreach { source, body, .. } => {
                self.expr(source)?;
                self.block(body)
            }
            AlgorithmStmt::Return { expr, .. } => match expr {
                Some(expr) => self.expr(expr),
                None => Some(()),
            },
            AlgorithmStmt::Call { target, args, .. } => {
                self.expr(target)?;
                args.iter().try_for_each(|arg| self.expr(&arg.expr))
            }
            AlgorithmStmt::Require { cond, .. } => self.expr(cond),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body_of(body: &str) -> Vec<AlgorithmStmt> {
        let source = format!(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
  sce:kind="algorithm" name="probe" version="1.0">
  <sce:signature>
    <sce:param name="xs" type="list&lt;int32&gt;"/>
    <sce:param name="a" type="uint32"/>
    <sce:return type="uint32" may-fail="true"/>
  </sce:signature>
  <sce:body>{body}</sce:body>
</scxml>"#
        );
        let parsed = crate::forge::parser::parse_forge_with_imports(
            &source,
            crate::DocumentLabel::symmetric("probe.scxml"),
        )
        .expect("the probe parses")
        .expect("the probe is a forge document");
        match parsed.document {
            crate::forge::model::ForgeDocument::Algorithm(m) => m.body,
            _ => panic!("the probe is an algorithm"),
        }
    }

    fn reads(body: &str) -> HashSet<String> {
        names_read(&body_of(body)).expect("every expression parses")
    }

    #[test]
    fn a_loop_item_is_read_only_where_an_expression_names_it() {
        let counted = r#"<sce:var name="n" type="uint32" init="0"/>
<sce:foreach item="x" in="xs"><sce:assign target="n" expr="n + 1"/></sce:foreach>
<sce:return expr="n"/>"#;
        let names = reads(counted);
        assert!(names.contains("xs") && names.contains("n"), "{names:?}");
        assert!(!names.contains("x"), "{names:?}");
        assert!(reads(&counted.replace("n + 1", "n + a")).contains("a"));
        assert!(reads(&counted.replace("n + 1", "n + (x > 0 ? 1 : 2)")).contains("x"));
    }

    #[test]
    fn a_local_that_is_only_written_is_not_read() {
        let names = reads(
            r#"<sce:var name="n" type="uint32" init="a"/>
<sce:assign target="n" expr="a"/>
<sce:return expr="a"/>"#,
        );
        assert!(names.contains("a") && !names.contains("n"), "{names:?}");
    }

    #[test]
    fn a_name_read_in_a_nested_block_is_read() {
        let names = reads(
            r#"<sce:var name="n" type="uint32" init="0"/>
<sce:if cond="a > 0"><sce:return expr="n"/></sce:if>
<sce:return expr="a"/>"#,
        );
        assert!(names.contains("n"), "{names:?}");
    }
}
