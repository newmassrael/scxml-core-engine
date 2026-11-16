// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Cross-kind import graph verification.
//
// Walks the `<sce:import>` graph below a forge document, refuses a cycle
// as `validation/cross-kind-circular-dependency`, and returns every
// document the walk read — the transitive closure a build system must
// invalidate on.
//
// ⚠ WHAT THIS MODULE NO LONGER DOES, AND WHY. It also walked the
// expressions of algorithm documents and checked each `<alias>.<field>`
// against the imported kind's fields. That check ran where it could never
// be right and nowhere it was needed: in an algorithm an import's alias is
// not a value at all, so every `frame.msg_id` it accepted was refused
// right after as an undeclared name, and every `frame.msg_idd` it refused
// sent the author to fix a field of a name that still would not resolve.
// Meanwhile a procedure, where the alias IS a value, was never walked, and
// `frame.msgIdd` there generated with exit 0 (measured 2026-09-21). The
// member check now lives in the expression layer
// (`forge::expr::reject_undeclared_member`), which reads every kind's
// expressions and knows which names are records.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::forge::error::{Located, ValidationError};
use crate::forge::import_source::ImportSource;
use crate::forge::model::{ForgeImport, ParsedForge};

/// Walk the `<sce:import>` graph rooted at `entry_label` (the document
/// being compiled) and reject any cycle with
/// `CrossKindCircularDependency`. DFS with a visited stack — first
/// back-edge wins (a single cycle surfaces once; pre-release accepts
/// the first-fire semantics rather than enumerating every cycle in
/// pathological cases).
///
/// The cycle path is reported in traversal order from the back-edge
/// target through the recursion frames back to the originating node,
/// e.g. `a.scxml → b.scxml → a.scxml`, each document named by the `src`
/// that reached it (the entry by its file name).
///
/// ⚠ Two things the walk once got wrong (measured 2026-09-25). The entry
/// document was not on the stack, so `a ↔ b` compiled from `a` re-read `a`
/// as an import and reported `b → a → b`. And the start of the cycle was
/// found by comparing the `src` SPELLINGS on the stack, falling back to the
/// whole stack when none matched, so a document imported as `x.scxml` and
/// again as `./x.scxml` put documents outside the cycle into its path. A
/// document is now one place on the stack, keyed by its canonical path,
/// and the entry is on it from the start whenever it is a file.
pub(crate) fn check_imports_acyclic(
    entry_imports: &[ForgeImport],
    entry_label: &str,
    base_dir: &Path,
) -> Result<Vec<PathBuf>, Located<crate::forge::error::ForgeError>> {
    // `visited` holds documents fully processed (no cycle through them);
    // `stack` is the current DFS frontier, each document's canonical path
    // and the name the cycle path gives it, so a back-edge into it is the
    // cycle signal and the cycle is the stack from that document on.
    let mut visited: HashSet<PathBuf> = HashSet::new();
    let mut stack: Vec<(PathBuf, String)> =
        entry_identity(entry_label, base_dir).into_iter().collect();
    // Every document this walk read, in traversal order. The walk
    // already visits the transitive closure of `<sce:import>` to decide
    // acyclicity; returning what it read costs nothing and is the only
    // place in the pipeline that knows the whole set. Deriving it a
    // second time from `parsed.imports` yields the *direct* imports
    // alone, and a build told only those ships a stale artefact after a
    // grandchild edit — `codegen_depfile_content` measures exactly that.
    let mut sources: Vec<PathBuf> = Vec::new();

    fn dfs(
        doc_label: &str,
        imports: &[ForgeImport],
        base_dir: &Path,
        visited: &mut HashSet<PathBuf>,
        stack: &mut Vec<(PathBuf, String)>,
        sources: &mut Vec<PathBuf>,
    ) -> Result<(), Located<crate::forge::error::ForgeError>> {
        for imp in imports {
            let child_path = crate::forge::stdlib::resolve(base_dir, &imp.src);
            // Keyed as `confine` keys a document, so an import held in
            // memory compares as the file on disk does.
            let canonical =
                crate::confine::canonicalize(&child_path).unwrap_or_else(|_| child_path.clone());

            if let Some(start) = stack.iter().position(|(on, _)| on == &canonical) {
                // Cycle detected: the stack from the document this import
                // reaches again, closed with the import that reaches it.
                let mut cycle: Vec<String> = stack[start..]
                    .iter()
                    .map(|(_, name)| name.clone())
                    .collect();
                cycle.push(imp.src.clone());
                return Err(Located::new(
                    ValidationError::CrossKindCircularDependency { cycle }.into(),
                    doc_label,
                    imp.line,
                    None,
                ));
            }
            if visited.contains(&canonical) {
                continue;
            }
            stack.push((canonical.clone(), imp.src.clone()));

            // Re-parse the imported document to read its own imports.
            // Read errors / parse errors flow up from the import
            // enrichment pass; here we silently treat them as "no
            // child imports" so the cycle detector does not double-
            // emit on a separately-diagnosed failure.
            let source = ImportSource::read(base_dir, imp).ok();
            if source.is_some() {
                // Recorded on the read, not on the edge: a prerequisite
                // is a file that was opened. An import naming a file
                // that does not exist is diagnosed by the enrichment
                // pass; listing it here would put a path nothing writes
                // into the depfile and leave the target permanently
                // dirty.
                sources.push(canonical.clone());
            }
            let child_imports: Vec<ForgeImport> = source
                .and_then(|s| s.parse().ok().flatten())
                .map(|p| p.imports)
                .unwrap_or_default();
            let child_base_dir = child_path
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| base_dir.to_path_buf());
            let child_doc_label = child_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(doc_label)
                .to_string();
            dfs(
                &child_doc_label,
                &child_imports,
                &child_base_dir,
                visited,
                stack,
                sources,
            )?;
            stack.pop();
            visited.insert(canonical);
        }
        Ok(())
    }

    dfs(
        entry_label,
        entry_imports,
        base_dir,
        &mut visited,
        &mut stack,
        &mut sources,
    )?;
    Ok(sources)
}

/// The entry document's place on the import graph: its canonical path and
/// its file name, when its label names a file in the directory its imports
/// resolve against — which is where an import that comes back to it
/// resolves. `None` for a document given as text with no such file (an
/// in-memory label), which no import can reach again.
fn entry_identity(entry_label: &str, base_dir: &Path) -> Option<(PathBuf, String)> {
    let name = Path::new(entry_label).file_name()?;
    let file = base_dir.join(name);
    if !file.is_file() {
        return None;
    }
    let canonical = crate::confine::canonicalize(&file).ok()?;
    Some((canonical, name.to_string_lossy().into_owned()))
}

/// Refuse a cyclic `<sce:import>` graph below `parsed`, and return every
/// forge document the walk read, sorted — the transitive closure of
/// `<sce:import>`, which callers surface as
/// [`crate::generator::GeneratedOutput::deps`] so a build system can
/// invalidate on any of them.
pub fn check(
    parsed: &ParsedForge,
    base_dir: &Path,
    label: &str,
) -> Result<Vec<PathBuf>, Located<crate::forge::error::ForgeError>> {
    let mut sources = check_imports_acyclic(&parsed.imports, label, base_dir)?;
    // Deterministic and duplicate-free: a depfile that reorders between
    // runs defeats a "regenerate and expect no diff" gate as surely as a
    // changing timestamp, and a diamond import graph reaches the same
    // document by two paths.
    sources.sort();
    sources.dedup();
    Ok(sources)
}
