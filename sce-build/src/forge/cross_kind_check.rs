// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
/// e.g. `a.scxml → b.scxml → a.scxml`.
pub(crate) fn check_imports_acyclic(
    entry_imports: &[ForgeImport],
    entry_label: &str,
    base_dir: &Path,
) -> Result<Vec<PathBuf>, Located<crate::forge::error::ForgeError>> {
    // `visited` holds documents fully processed (no cycle through them);
    // `on_stack` holds the current DFS frontier so a back-edge into it
    // is the cycle signal. `path` mirrors `on_stack` but as a Vec so we
    // can render the cycle in traversal order on the diagnostic.
    let mut visited: HashSet<PathBuf> = HashSet::new();
    let mut on_stack: HashSet<PathBuf> = HashSet::new();
    let mut path: Vec<String> = Vec::new();
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
        on_stack: &mut HashSet<PathBuf>,
        path: &mut Vec<String>,
        sources: &mut Vec<PathBuf>,
    ) -> Result<(), Located<crate::forge::error::ForgeError>> {
        for imp in imports {
            let child_path = base_dir.join(&imp.src);
            let canonical = child_path
                .canonicalize()
                .unwrap_or_else(|_| child_path.clone());

            if on_stack.contains(&canonical) {
                // Cycle detected. Render the cycle slice of `path`
                // from the first occurrence of the child up to the
                // current frame, then close it with the child name
                // again.
                let child_label = imp.src.clone();
                let cycle_start = path.iter().position(|p| p == &child_label).unwrap_or(0);
                let mut cycle: Vec<String> = path[cycle_start..].to_vec();
                cycle.push(child_label);
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
            on_stack.insert(canonical.clone());
            path.push(imp.src.clone());

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
                on_stack,
                path,
                sources,
            )?;
            on_stack.remove(&canonical);
            path.pop();
            visited.insert(canonical);
        }
        Ok(())
    }

    dfs(
        entry_label,
        entry_imports,
        base_dir,
        &mut visited,
        &mut on_stack,
        &mut path,
        &mut sources,
    )?;
    Ok(sources)
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
