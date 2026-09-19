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

/// Which sibling outputs an expression reads, in declaration order.
///
/// ⚠ Word-boundary matching over the expression TEXT, not the AST. The
/// dependency graph has to be known BEFORE `transpile_typed` runs — the
/// rename map it is handed is built from this very graph — so an AST is
/// not available yet. Erring wide is the safe direction here: a name
/// that appears inside a string literal would add an edge that is not
/// real, which can only turn a legal document into a refusal, never the
/// reverse. Transform expressions carrying string literals that happen
/// to spell an output id are not a shape this grammar produces today,
/// and if one ever appears the refusal says exactly which ids collided.
pub fn sibling_reads<'a>(expr: &str, outputs: &'a [String], exclude: &str) -> Vec<&'a str> {
    outputs
        .iter()
        .map(String::as_str)
        .filter(|id| *id != exclude && mentions(expr, id))
        .collect()
}

fn mentions(expr: &str, id: &str) -> bool {
    let is_word = |c: char| c.is_alphanumeric() || c == '_' || c == '$';
    let bytes = expr.as_bytes();
    let mut from = 0;
    while let Some(rel) = expr[from..].find(id) {
        let start = from + rel;
        let end = start + id.len();
        let before_ok = start == 0 || !is_word(expr[..start].chars().next_back().unwrap_or(' '));
        let after_ok = end == bytes.len() || !is_word(expr[end..].chars().next().unwrap_or(' '));
        if before_ok && after_ok {
            return true;
        }
        from = start + 1;
    }
    false
}

/// First cycle in the output dependency graph, as a path that starts and
/// ends with the same id. `None` when the graph is acyclic.
///
/// ⚠ A self-reference (`<data id="a" expr="a + 1"/>`) is a cycle of
/// length one and is reported as `a → a`. It is caught here rather than
/// left to the general walk because `sibling_reads` excludes the node's
/// own id — that exclusion is what lets a legal document mention nothing
/// of itself, so the self case needs its own question.
fn first_cycle(m: &TransformModel) -> Option<Vec<String>> {
    let ids: Vec<String> = m.outputs.iter().map(|o| o.id.clone()).collect();

    let mut deps: HashMap<&str, Vec<&str>> = HashMap::new();
    for out in &m.outputs {
        let expr = out.expr.as_deref().unwrap_or("");
        if mentions(expr, &out.id) {
            return Some(vec![out.id.clone(), out.id.clone()]);
        }
        deps.insert(out.id.as_str(), sibling_reads(expr, &ids, &out.id));
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

    #[test]
    fn a_word_boundary_separates_a_read_from_a_prefix() {
        let outs = ids(&["warn", "warnLatched"]);
        // `warnLatched` contains `warn`; only the whole word is a read.
        assert_eq!(
            sibling_reads("warnLatched && x", &outs, "other"),
            ["warnLatched"]
        );
        assert_eq!(sibling_reads("warn && x", &outs, "other"), ["warn"]);
    }

    #[test]
    fn an_underscore_or_digit_does_not_end_a_word() {
        let outs = ids(&["a"]);
        // `a1` and `a_b` are different identifiers, not a read of `a`.
        assert!(sibling_reads("a1 + 2", &outs, "other").is_empty());
        assert!(sibling_reads("a_b + 2", &outs, "other").is_empty());
        assert_eq!(sibling_reads("(a) + 2", &outs, "other"), ["a"]);
    }

    #[test]
    fn the_excluded_id_is_never_a_read() {
        let outs = ids(&["a", "b"]);
        assert_eq!(sibling_reads("a + b", &outs, "a"), ["b"]);
    }
}
