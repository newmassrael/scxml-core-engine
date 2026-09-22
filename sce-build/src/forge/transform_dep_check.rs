//! A transform's outputs may read each other; a cycle among them may not.
//!
//! ⚠ WHY THIS MODULE EXISTS. Measured 2026-09-18: a `sce:kind="transform"`
//! document whose output referenced a sibling output generated with
//! **exit 0** and emitted, in every language, a function body naming an
//! identifier the signature never bound — Python raised `NameError` at
//! the first call, C++ and Rust would not compile. The generator did not
//! know its own emit was unbound.
//!
//! The reference itself is legitimate and the domain wants it. A prose
//! specification routinely names an intermediate value and has several
//! outputs consume it; the conversion that surfaced this had one named
//! coolant-warning state feeding a telltale and two events. Making the
//! author paste that expression into each consumer would lose the spec's
//! own name for the thing and let the copies drift apart — so the
//! generator lowers a sibling read to a call of that sibling's own
//! `compute_*` function. This is sound because every one of them is a
//! pure function of the document's inputs, so substituting a call for a
//! name changes nothing about the value.
//!
//! A cycle is the one shape that lowering cannot serve: the emitted
//! functions would call each other until the stack ends. This module is
//! what makes the permission safe, and it runs before any language is
//! rendered so the refusal cannot depend on which backend was asked for.

use std::collections::{HashMap, HashSet};

use crate::forge::error::{ForgeError, Located, ValidationError};
use crate::forge::model::{ForgeDocument, ParsedForge, TransformModel};

/// Refuse a transform whose outputs depend on each other in a cycle.
pub fn check(parsed: &ParsedForge, source_name: &str) -> Result<(), Located<ForgeError>> {
    let ForgeDocument::Transform(m) = &parsed.document else {
        return Ok(());
    };
    if let Some(cycle) = first_cycle(m) {
        return Err(Located::new(
            ValidationError::TransformOutputCycle {
                name: m.name.clone(),
                cycle,
            }
            .into(),
            source_name.to_string(),
            None,
            None,
        ));
    }
    Ok(())
}

/// Which outputs an expression reads THIS activation, in declaration order
/// (so a reported cycle starts where it always has), or `None` when it does
/// not parse.
///
/// ⚠ Read from the parsed tree, not the TEXT. This was a whole-word search
/// over the expression string, on the reading that no tree exists before
/// `transpile_typed`; but the untyped parse that step starts from
/// (`parse_to_ast`) needs no types, and the text search had two ways to
/// be wrong that the tree does not. A string literal spelling an output's
/// name was an edge that is not there. And a read through `previous(x)` —
/// which reads the activation BEFORE this one and so cannot be part of a
/// cycle — was an edge that must not be there: `x = previous(x) + 1`
/// would have been refused as `x → x`.
///
/// `None` is left to the expression stage, which refuses an expression
/// that does not parse with its own diagnostic; reporting a cycle in text
/// nobody can parse would name the wrong fault.
fn reads_now<'a>(expr: &str, outputs: &'a [String]) -> Option<Vec<&'a str>> {
    let read = crate::forge::previous_value::reads(expr)?.ok()?;
    Some(
        outputs
            .iter()
            .filter(|id| read.now.contains(id))
            .map(String::as_str)
            .collect(),
    )
}

/// First cycle in the output dependency graph, as a path that starts and
/// ends with the same id. `None` when the graph is acyclic.
///
/// ⚠ A self-reference (`<data id="a" expr="a + 1"/>`) is a cycle of
/// length one and is reported as `a → a`. It is asked about on its own
/// rather than left to the general walk, which is handed every OTHER output
/// an expression reads — the per-output rename map excludes the output's
/// own id, so a self-read has no lowering at all.
fn first_cycle(m: &TransformModel) -> Option<Vec<String>> {
    let ids: Vec<String> = m.outputs.iter().map(|o| o.id.clone()).collect();

    let mut deps: HashMap<&str, Vec<&str>> = HashMap::new();
    for out in &m.outputs {
        let reads = reads_now(out.expr.as_deref().unwrap_or(""), &ids).unwrap_or_default();
        if reads.contains(&out.id.as_str()) {
            return Some(vec![out.id.clone(), out.id.clone()]);
        }
        deps.insert(
            out.id.as_str(),
            reads.into_iter().filter(|id| *id != out.id).collect(),
        );
    }

    let mut done: HashSet<&str> = HashSet::new();
    let mut path: Vec<&str> = Vec::new();
    for out in &m.outputs {
        if let Some(cycle) = visit(out.id.as_str(), &deps, &mut done, &mut path) {
            return Some(cycle);
        }
    }
    None
}

/// Depth-first search carrying its own stack so the cycle it finds can
/// be reported as the author's own path rather than as a bare "there is
/// a cycle somewhere".
fn visit<'a>(
    node: &'a str,
    deps: &HashMap<&'a str, Vec<&'a str>>,
    done: &mut HashSet<&'a str>,
    path: &mut Vec<&'a str>,
) -> Option<Vec<String>> {
    if done.contains(node) {
        return None;
    }
    if let Some(at) = path.iter().position(|n| *n == node) {
        let mut cycle: Vec<String> = path[at..].iter().map(|s| s.to_string()).collect();
        cycle.push(node.to_string());
        return Some(cycle);
    }
    path.push(node);
    for next in deps.get(node).into_iter().flatten() {
        if let Some(cycle) = visit(next, deps, done, path) {
            return Some(cycle);
        }
    }
    path.pop();
    done.insert(node);
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    fn now(expr: &str, outs: &[String]) -> Vec<String> {
        reads_now(expr, outs)
            .expect("the expression parses")
            .into_iter()
            .map(str::to_string)
            .collect()
    }

    #[test]
    fn a_prefix_is_a_different_name() {
        let outs = ids(&["warn", "warnLatched"]);
        assert_eq!(now("warnLatched && x", &outs), ["warnLatched"]);
        assert_eq!(now("warn && x", &outs), ["warn"]);
    }

    #[test]
    fn an_underscore_or_digit_makes_a_different_name() {
        let outs = ids(&["a"]);
        assert!(now("a1 + 2", &outs).is_empty());
        assert!(now("a_b + 2", &outs).is_empty());
        assert_eq!(now("(a) + 2", &outs), ["a"]);
    }

    #[test]
    fn reads_come_back_in_declaration_order() {
        let outs = ids(&["a", "b"]);
        assert_eq!(now("b + a", &outs), ["a", "b"]);
    }

    #[test]
    fn a_string_that_spells_an_output_is_not_a_read() {
        // The text search this replaced took `'b'` for a read of `b`, which
        // is an edge that is not there.
        let outs = ids(&["b"]);
        assert!(now("x === 'b' ? 1 : 0", &outs).is_empty());
    }

    #[test]
    fn a_read_through_previous_is_not_an_edge() {
        let outs = ids(&["shown"]);
        assert!(now("latched ? latched : previous(shown)", &outs).is_empty());
    }

    /// An output id whose first letter is three UTF-8 bytes wide, and an input
    /// id that extends it — the pair whose near miss once sliced the text
    /// inside a letter and stopped `sce-codegen check` with exit 101.
    const WIDE: &str = "\u{c628}\u{b3c4}";
    const WIDER: &str = "\u{c628}\u{b3c4}2";

    /// Whatever the tokenizer makes of such names, the question is answered
    /// without panicking: there is no text left to slice.
    #[test]
    fn a_multibyte_id_is_answered_without_panicking() {
        assert_eq!(WIDE.chars().next().map(char::len_utf8), Some(3));
        let outs = ids(&[WIDE, WIDER]);
        let _ = reads_now(&format!("{WIDER} + 1"), &outs);
        let _ = reads_now(&format!("{WIDER} + {WIDE}"), &outs);
    }
}
