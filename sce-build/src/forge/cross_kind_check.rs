// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// NL→IR Mapping Roadmap Item 2 — cross-kind typed binding verification.
//
// Walks every expression site in a parsed Forge document and validates each
// `<alias>.<field>` member-access reference against the imported kind's
// declared member surface. Closes the silent-broken pattern documented in
// `feedback_spec_mirror_parity.md`: the typed-expression pipeline already
// has the symbol table (populated by `validate_and_enrich_imports` →
// `discover_stateful_member_fields`) but `infer_types` returns
// `InferredType::Unknown` for unresolved Member access — no diagnostic.
// AI-generated SCXML hallucinates plausible-looking field names that
// historically survived to codegen.
//
// Three diagnostics — `validation/cross-kind-field-not-found` (with
// closed `Fix::ReplaceOneOf` `did_you_mean`-style candidate list),
// `validation/cross-kind-type-mismatch` (signature return type vs the
// declared type of the referenced field), and the defensive
// `validation/cross-kind-circular-dependency` over the `<sce:import>`
// graph.
//
// Scope (v1): the validator is wired only on the Forge→Forge path inside
// `compile_forge_from_parsed` after `validate_and_enrich_imports`. The
// module's public API is kind-agnostic so a future Statechart→Forge
// binding (currently zero consumers — see
// `nl_to_ir_mapping_roadmap.md` Item 2 "Forge→Forge first" decision)
// would add a second call site without changing diagnostic shape or
// payload.
//
// Coverage (v1):
//   * Algorithm body — every statement variant's expression slot walked
//     (Var.init, Assign.target/expr, If.cond, While.cond, Return.expr,
//     Call.args). Foreach.source is a bare alias / param name (not an
//     expression) — covered by parser stage's
//     `algorithm/foreach-source-not-iterable`.
//   * AlgorithmConst.init (init expression — `None` skipped, those are
//     `<sce:fold>` bodies handled by the const-fold module).
//   * `<sce:return expr="alias.field"/>` typed against
//     `AlgorithmSignature.return_type` for the type-mismatch axis when
//     the expression is a bare Member access (the only shape where the
//     resolved type is statically known).
//
// Procedure body, Codec embed/variant predicate expressions, and other
// kinds' expression sites extend the same `walk_expression` helper as
// follow-up atomics if a real silent-broken consumer surfaces. Per
// `feedback_verify_before_ship`, we do not pre-build for hypothetical
// sites.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::forge::error::{Located, ValidationError};
use crate::forge::expr::{parse_to_ast, ExprKind, TypedExpr};
use crate::forge::model::{
    AlgorithmModel, AlgorithmStmt, ForgeDocument, ForgeImport, ForgeKind, ParsedForge, SceType,
};

/// Per-imported-kind member surface used for typed-binding resolution.
///
/// Mirrors `lib::discover_stateful_member_fields` but keyed by the
/// importing document's alias (not the imported kind's bare field id)
/// so the validator can answer "given `<alias>.<field>` in an
/// expression, is `<field>` declared?" with a single map lookup.
///
/// Stateless imports (Transform / Condition / Lookup / Interpolation /
/// Algorithm) are absent from this table: those flow through expression
/// surface as `<alias>(arg, arg)` function calls, not as
/// `<alias>.<field>` member access — the typo-axis is already covered
/// by parser-stage validators (`algorithm/call-target-unknown`) and the
/// cross-kind validator stays silent.
struct ImportMemberSurface {
    /// Imported kind (`codec`, `bounded-collection`, …) — surfaced
    /// verbatim in the diagnostic so authors disambiguate when the
    /// same alias name shadows a local field.
    imported_kind: ForgeKind,
    /// Imported document's `name` attribute (the kind's name, not the
    /// file basename) so the diagnostic carries the spec-anchorable
    /// identifier.
    imported_name: String,
    /// Sorted, deduplicated member surface — drives the closed-set
    /// `Fix::ReplaceOneOf` candidate list. Type info preserved so the
    /// type-mismatch axis can read it for the same field without a
    /// second resolution pass.
    fields: Vec<(String, SceType)>,
}

/// Build the per-alias member surface table from a document's parsed
/// imports. Returns a map keyed by import alias — the same key shape
/// `forge::type_ctx::insert_stateful_imports` uses for the
/// `<alias>.<field>` qualified `TypeCtx::vars` lookup.
///
/// Re-reads each imported file at validator time (cheap: imports are
/// small, the parse is single-pass). An earlier draft tried to thread
/// the imports through `ParsedForge` to avoid the re-read; that path
/// crosses the `validate_and_enrich_imports` boundary and forces every
/// caller to pre-walk imports, so the simple re-read won.
fn build_surface_table(
    imports: &[ForgeImport],
    base_dir: &Path,
) -> Result<HashMap<String, ImportMemberSurface>, Located<crate::forge::error::ForgeError>> {
    let mut out: HashMap<String, ImportMemberSurface> = HashMap::new();
    for imp in imports {
        let src_path = base_dir.join(&imp.src);
        let content = match std::fs::read_to_string(&src_path) {
            Ok(c) => c,
            // Existence + read errors already raise their own diagnostics
            // from `validate_and_enrich_imports` upstream; we silently
            // skip here rather than double-emit.
            Err(_) => continue,
        };
        let stem = src_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        let basename = src_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(stem);
        let label = crate::DocumentLabel {
            identifier: stem,
            diagnostic_label: basename,
        };
        // Re-parsing here is intentional: the import enrichment path
        // already parsed it once but does not retain the typed model
        // for downstream callers. Caching across both call sites would
        // require restructuring `validate_and_enrich_imports` to thread
        // ParsedForge through — out of scope for this atomic.
        let parsed = match crate::forge::parser::parse_forge_with_imports(&content, label) {
            Ok(Some(p)) => p,
            Ok(None) => continue,
            Err(_) => continue, // upstream import enrichment surfaces this
        };
        let doc = parsed.document;
        let imported_kind = doc.kind();
        let imported_name = doc.name().to_string();
        let fields = match collect_member_fields(&doc) {
            Some(fs) => fs,
            None => continue, // stateless kind — not addressable as alias.field
        };
        out.insert(
            imp.alias.clone(),
            ImportMemberSurface {
                imported_kind,
                imported_name,
                fields,
            },
        );
    }
    Ok(out)
}

/// Member field surface for a stateful imported kind. Mirrors
/// `lib::discover_stateful_member_fields` 1:1 — kept duplicated here so
/// the cross-kind validator does not depend on a private lib helper.
/// Returns `None` for stateless kinds, which surface as function-call
/// aliases through a separate diagnostic family
/// (`algorithm/call-target-*`).
fn collect_member_fields(doc: &ForgeDocument) -> Option<Vec<(String, SceType)>> {
    let mut out: Vec<(String, SceType)> = Vec::new();
    match doc {
        ForgeDocument::Codec(m) => {
            for f in &m.fields {
                out.push((f.id.clone(), f.sce_type.clone()));
            }
        }
        ForgeDocument::Validator(m) => {
            for f in &m.inputs {
                out.push((f.id.clone(), f.sce_type.clone()));
            }
        }
        ForgeDocument::Filter(m) => {
            out.push((m.output.id.clone(), m.output.sce_type.clone()));
            out.push((m.input.id.clone(), m.input.sce_type.clone()));
        }
        ForgeDocument::Observer(m) => {
            for f in &m.inputs {
                out.push((f.id.clone(), f.sce_type.clone()));
            }
        }
        ForgeDocument::Procedure(m) => {
            for f in &m.inputs {
                out.push((f.id.clone(), f.sce_type.clone()));
            }
            for f in &m.internals {
                out.push((f.id.clone(), f.sce_type.clone()));
            }
        }
        // Bounded-collection's element type is a Codec / Procedure
        // declared by name elsewhere; the *element's* fields are
        // accessed via the foreach iteration variable, not the BC
        // alias itself, so the BC import's own member surface is
        // empty for the alias.field check. The foreach-iter-var
        // typed access is covered by the per-statement Foreach branch
        // in `walk_algorithm_stmt`.
        ForgeDocument::BoundedCollection(_) => return Some(Vec::new()),
        // Stateless kinds — function-call aliases, no member surface.
        ForgeDocument::Statechart(_)
        | ForgeDocument::Transform(_)
        | ForgeDocument::Condition(_)
        | ForgeDocument::Lookup(_)
        | ForgeDocument::Interpolation(_)
        | ForgeDocument::Algorithm(_)
        | ForgeDocument::Timer(_)
        | ForgeDocument::Link(_)
        | ForgeDocument::BufferPool(_)
        | ForgeDocument::Worker(_)
        // NL→IR Item C1 Path A: Enum is a typed vocabulary declaration
        // — variants are not member fields accessed via `alias.field`.
        // Authors reference variants as `<EnumName>.<variant>` which
        // resolves through the cross-kind binding pass to the imported
        // enum's variant list, not the member-field check below.
        | ForgeDocument::Enum(_)
        // NL→IR Item C1 Path A: EventSchema is imported via
        // `<sce:import>` for cross-doc binding (the receive-side
        // typecheck pass resolves `_event.data.<field>` against the
        // schema's declared fields by *event name*, not by alias
        // access). The import alias itself is never used in
        // expression positions like `alias.field` — the link is
        // implicit through SCXML event-name matching. Empty surface
        // keeps the alias-access path from suggesting EventSchema
        // fields as candidates for an `alias.field` typo on a
        // different kind. Functional resolution lives in
        // `event_schema_check.rs`.
        | ForgeDocument::EventSchema(_) => return None,
    }
    Some(out)
}

/// Render an [`SceType`] to its canonical schema attribute spelling
/// (`uint8`, `bool`, `enum:<alias>`, …). `SceType` has no `Display`
/// impl; rendering it inline keeps the diagnostic format stable across
/// future Serialize representations. Returns `String` (not `&'static
/// str`) so the parameterized `Enum(EnumRef)` arm can interpolate the
/// import alias into the canonical form.
fn sce_type_canonical(t: &SceType) -> String {
    match t {
        SceType::Uint8 => "uint8".to_string(),
        SceType::Uint16 => "uint16".to_string(),
        SceType::Uint32 => "uint32".to_string(),
        SceType::Uint64 => "uint64".to_string(),
        SceType::Int8 => "int8".to_string(),
        SceType::Int16 => "int16".to_string(),
        SceType::Int32 => "int32".to_string(),
        SceType::Int64 => "int64".to_string(),
        SceType::Float32 => "float32".to_string(),
        SceType::Float64 => "float64".to_string(),
        SceType::Bool => "bool".to_string(),
        SceType::String => "string".to_string(),
        SceType::Bytes => "bytes".to_string(),
        SceType::Enum(r) => format!("enum:{}", r.alias),
    }
}

/// Walk one expression AST and visit every `Member { object: Ident(obj),
/// property }` site, calling `on_member` with `(obj, property)`.
///
/// The walker is intentionally shape-narrow: `Member { object: Member(…),
/// property }` (chained access like `frame.header.id`) is left alone.
/// Two reasons: (a) the current symbol-table key shape is
/// `"{alias}.{field}"` single-level only, so a chained reference cannot
/// be resolved without nested kind metadata that v1 imports don't
/// expose; (b) extending the walker to handle chained access would
/// silently widen the diagnostic surface without an integration test
/// proving the right diagnostic fires — `feedback_silently_broken_hooks`
/// pattern. When real chained references surface, the walker extends
/// with a separate `Member.Member` arm.
fn walk_expression<'a, F>(expr: &'a TypedExpr, on_member: &mut F)
where
    F: FnMut(&'a str, &'a str),
{
    match &expr.kind {
        ExprKind::Member { object, property } => {
            if let ExprKind::Ident(obj) = &object.kind {
                on_member(obj.as_str(), property.as_str());
            }
            // Continue into nested object form so a deeper Member
            // node containing a `Ident.field` shape still surfaces.
            walk_expression(object, on_member);
        }
        ExprKind::Binary { left, right, .. } => {
            walk_expression(left, on_member);
            walk_expression(right, on_member);
        }
        ExprKind::Unary { operand, .. } => walk_expression(operand, on_member),
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        } => {
            walk_expression(condition, on_member);
            walk_expression(consequent, on_member);
            walk_expression(alternate, on_member);
        }
        ExprKind::Call { callee, args } => {
            walk_expression(callee, on_member);
            for a in args {
                walk_expression(a, on_member);
            }
        }
        ExprKind::Index { object, index } => {
            walk_expression(object, on_member);
            walk_expression(index, on_member);
        }
        // RFC c7-wildcard W-project: algorithm-kind-only projection node;
        // recurse into its source so a member reference inside still
        // surfaces (unreachable on the import-surface check path).
        ExprKind::BytesView { source, .. } => walk_expression(source, on_member),
        ExprKind::NumberLit(_)
        | ExprKind::StringLit { .. }
        | ExprKind::BytesLit { .. }
        | ExprKind::BoolLit(_)
        | ExprKind::NullLit
        | ExprKind::Ident(_)
        | ExprKind::Raw(_) => {}
    }
}

/// Parse a single expression string and validate every alias.field
/// member access against the surface table. Empty / unparseable
/// expressions are silently skipped — those are caught by the typed
/// expression pipeline's own diagnostics (`expression/empty`,
/// `expression/lex`, …) at codegen entry and surfacing them here would
/// double-emit.
fn check_expression(
    expr: &str,
    surface: &HashMap<String, ImportMemberSurface>,
    importing_kind: ForgeKind,
    importing_name: &str,
    location: &str,
    line: Option<u32>,
) -> Result<(), Located<crate::forge::error::ForgeError>> {
    let trimmed = expr.trim();
    if trimmed.is_empty() {
        return Ok(());
    }
    let ast = match parse_to_ast(trimmed) {
        Ok(a) => a,
        Err(_) => return Ok(()), // typed pipeline surfaces this
    };
    // First-fire semantics: a single expression with multiple
    // alias.field misses surfaces the first miss in walk order. The
    // author fixes one at a time; emitting all of them in one shot
    // would inflate the diagnostic stream and break the FNV1a id
    // uniqueness guarantee on rerun (the same expression with two
    // misses generates two distinct ids whose stability tracks order
    // of walk rather than author intent).
    let mut first_error: Option<ValidationError> = None;
    walk_expression(&ast, &mut |obj, property| {
        if first_error.is_some() {
            return;
        }
        let Some(s) = surface.get(obj) else {
            return; // obj is not a known import alias
        };
        if s.fields.iter().any(|(name, _)| name == property) {
            return; // resolved
        }
        // Build the closed candidate set. Pre-sorted is the expected
        // shape for `Fix::ReplaceOneOf`; the surface table already
        // preserves the imported kind's declared field order, so a
        // dedicated sort here keeps the wire form stable even if the
        // imported model later changes field iteration order.
        let mut candidates: Vec<String> = s.fields.iter().map(|(n, _)| n.clone()).collect();
        candidates.sort_unstable();
        candidates.dedup();
        first_error = Some(ValidationError::CrossKindFieldNotFound {
            importing_kind,
            importing_name: importing_name.to_string(),
            alias: obj.to_string(),
            field: property.to_string(),
            imported_kind: s.imported_kind,
            imported_name: s.imported_name.clone(),
            candidates,
        });
    });
    if let Some(err) = first_error {
        return Err(Located::new(err.into(), location, line, None));
    }
    Ok(())
}

/// Walk every statement in an algorithm body and check each expression
/// slot. Recurses into nested blocks (If.then/else, While.body,
/// Foreach.body).
fn walk_algorithm_stmt(
    stmt: &AlgorithmStmt,
    surface: &HashMap<String, ImportMemberSurface>,
    importing_kind: ForgeKind,
    importing_name: &str,
    location: &str,
) -> Result<(), Located<crate::forge::error::ForgeError>> {
    match stmt {
        AlgorithmStmt::Var { init, .. } => {
            check_expression(
                init,
                surface,
                importing_kind,
                importing_name,
                location,
                None,
            )?;
        }
        AlgorithmStmt::Assign { target, expr } => {
            check_expression(
                target,
                surface,
                importing_kind,
                importing_name,
                location,
                None,
            )?;
            check_expression(
                expr,
                surface,
                importing_kind,
                importing_name,
                location,
                None,
            )?;
        }
        AlgorithmStmt::If {
            cond,
            then_body,
            else_body,
        } => {
            check_expression(
                cond,
                surface,
                importing_kind,
                importing_name,
                location,
                None,
            )?;
            for st in then_body {
                walk_algorithm_stmt(st, surface, importing_kind, importing_name, location)?;
            }
            if let Some(else_) = else_body {
                for st in else_ {
                    walk_algorithm_stmt(st, surface, importing_kind, importing_name, location)?;
                }
            }
        }
        AlgorithmStmt::While { cond, body, .. } => {
            check_expression(
                cond,
                surface,
                importing_kind,
                importing_name,
                location,
                None,
            )?;
            for st in body {
                walk_algorithm_stmt(st, surface, importing_kind, importing_name, location)?;
            }
        }
        AlgorithmStmt::Foreach { body, .. } => {
            // `source` is a bare alias / param name resolved by parser
            // stage; not an expression. Recurse into body only.
            for st in body {
                walk_algorithm_stmt(st, surface, importing_kind, importing_name, location)?;
            }
        }
        AlgorithmStmt::Return { expr } => {
            if let Some(e) = expr {
                check_expression(e, surface, importing_kind, importing_name, location, None)?;
            }
        }
        AlgorithmStmt::Call { args, .. } => {
            for arg in args {
                check_expression(arg, surface, importing_kind, importing_name, location, None)?;
            }
        }
    }
    Ok(())
}

/// Check `<sce:return expr="alias.field"/>` against the algorithm's
/// declared return type for the type-mismatch axis. Only the bare
/// `Ident.Member` shape qualifies (e.g. `<sce:return expr="entry.callback_id"/>`)
/// — composite expressions (`alias.field + 1`, `alias.field * 2`) carry
/// implicit promotion semantics that v1 expression inference resolves
/// at transpile time; surfacing those here would duplicate the
/// transpile-stage coercion rules without an integration test pinning
/// the exact failure mode.
fn check_algorithm_return_type(
    algo: &AlgorithmModel,
    surface: &HashMap<String, ImportMemberSurface>,
    location: &str,
) -> Result<(), Located<crate::forge::error::ForgeError>> {
    let Some(expected_ret) = algo.signature.return_type.as_ref() else {
        return Ok(()); // void return; no type contract to violate
    };
    // Walk every Return statement, including nested ones inside
    // If/While/Foreach bodies.
    fn visit<'a>(stmts: &'a [AlgorithmStmt], out: &mut Vec<&'a str>) {
        for st in stmts {
            match st {
                AlgorithmStmt::Return { expr: Some(e) } => out.push(e.as_str()),
                AlgorithmStmt::If {
                    then_body,
                    else_body,
                    ..
                } => {
                    visit(then_body, out);
                    if let Some(eb) = else_body {
                        visit(eb, out);
                    }
                }
                AlgorithmStmt::While { body, .. } | AlgorithmStmt::Foreach { body, .. } => {
                    visit(body, out);
                }
                _ => {}
            }
        }
    }
    let mut return_exprs: Vec<&str> = Vec::new();
    visit(&algo.body, &mut return_exprs);

    for expr in return_exprs {
        let trimmed = expr.trim();
        let ast = match parse_to_ast(trimmed) {
            Ok(a) => a,
            Err(_) => continue,
        };
        // Only the bare `Ident.Member` shape qualifies — see the
        // function-level rationale on composite expressions.
        let (obj, property) = match &ast.kind {
            ExprKind::Member { object, property } => match &object.kind {
                ExprKind::Ident(o) => (o.as_str(), property.as_str()),
                _ => continue,
            },
            _ => continue,
        };
        let Some(s) = surface.get(obj) else {
            continue;
        };
        let Some((_, actual_ty)) = s.fields.iter().find(|(n, _)| n == property) else {
            // Field-not-found case is the dedicated diagnostic — skip
            // here so we don't double-emit on the same source span.
            continue;
        };
        if actual_ty != expected_ret {
            return Err(Located::new(
                ValidationError::CrossKindTypeMismatch {
                    importing_kind: ForgeKind::Algorithm,
                    importing_name: algo.name.clone(),
                    alias: obj.to_string(),
                    field: property.to_string(),
                    actual: sce_type_canonical(actual_ty),
                    expected: sce_type_canonical(expected_ret),
                }
                .into(),
                location,
                None,
                None,
            ));
        }
    }
    Ok(())
}

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
) -> Result<(), Located<crate::forge::error::ForgeError>> {
    // `visited` holds documents fully processed (no cycle through them);
    // `on_stack` holds the current DFS frontier so a back-edge into it
    // is the cycle signal. `path` mirrors `on_stack` but as a Vec so we
    // can render the cycle in traversal order on the diagnostic.
    let mut visited: HashSet<PathBuf> = HashSet::new();
    let mut on_stack: HashSet<PathBuf> = HashSet::new();
    let mut path: Vec<String> = Vec::new();

    fn dfs(
        doc_label: &str,
        imports: &[ForgeImport],
        base_dir: &Path,
        visited: &mut HashSet<PathBuf>,
        on_stack: &mut HashSet<PathBuf>,
        path: &mut Vec<String>,
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
            let content = std::fs::read_to_string(&child_path).ok();
            let child_imports: Vec<ForgeImport> = if let Some(content) = content {
                let stem = child_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown");
                let basename = child_path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or(stem);
                let label = crate::DocumentLabel {
                    identifier: stem,
                    diagnostic_label: basename,
                };
                match crate::forge::parser::parse_forge_with_imports(&content, label) {
                    Ok(Some(p)) => p.imports,
                    _ => Vec::new(),
                }
            } else {
                Vec::new()
            };
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
    )
}

/// Cross-kind binding validator entry point. Runs after
/// `validate_and_enrich_imports` populates per-import enrichment data
/// (the validator reads its own surface table off the import file
/// contents rather than depending on the enrichment slice, so it
/// stays single-responsibility for the binding axis only).
///
/// Today wired only on the Forge→Forge path. A future Statechart→Forge
/// binding extends the same function with a Statechart arm; the
/// diagnostic shape stays identical.
pub fn check(
    parsed: &ParsedForge,
    base_dir: &Path,
    label: &str,
) -> Result<(), Located<crate::forge::error::ForgeError>> {
    // Step 1 — defensive cycle detection. Runs first so a cyclic
    // import does not infinite-loop the surface-table builder below.
    check_imports_acyclic(&parsed.imports, label, base_dir)?;

    // Step 2 — build per-alias member surface table. Empty when no
    // imports (typical for standalone fixtures) — the field walker
    // below early-returns on every Member access since no alias is
    // ever a hit.
    let surface = build_surface_table(&parsed.imports, base_dir)?;
    if surface.is_empty() {
        return Ok(());
    }

    // Step 3 — per-kind expression walker. v1 covers Algorithm only;
    // other stateful kinds (Procedure body, Codec embed/variant
    // predicate expressions, Filter/Validator/Observer expressions)
    // extend this branch as silent-broken consumers surface — see
    // module-level scope comment.
    if let ForgeDocument::Algorithm(algo) = &parsed.document {
        for stmt in &algo.body {
            walk_algorithm_stmt(stmt, &surface, ForgeKind::Algorithm, &algo.name, label)?;
        }
        for c in &algo.consts {
            // `init` is `None` when the const carries a `<sce:fold>`
            // body instead — RFC §5.F build-time fold path. Fold
            // expressions have their own typed-binding surface
            // handled by the const-fold module; the alias.field
            // walker stays narrow on scalar consts.
            if let Some(init) = &c.init {
                check_expression(
                    init,
                    &surface,
                    ForgeKind::Algorithm,
                    &algo.name,
                    label,
                    None,
                )?;
            }
        }
        check_algorithm_return_type(algo, &surface, label)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `walk_expression` surfaces every Ident.Member pair, including
    /// those nested inside Binary / Call / Conditional. Verifies the
    /// walker contract before the per-stmt enumerator depends on it.
    #[test]
    fn walk_collects_member_accesses() {
        let ast = parse_to_ast("a.x + foo(b.y, c.z) ? d.w : e.v").unwrap();
        let mut hits: Vec<(String, String)> = Vec::new();
        walk_expression(&ast, &mut |o, p| hits.push((o.to_string(), p.to_string())));
        hits.sort_unstable();
        assert_eq!(
            hits,
            vec![
                ("a".to_string(), "x".to_string()),
                ("b".to_string(), "y".to_string()),
                ("c".to_string(), "z".to_string()),
                ("d".to_string(), "w".to_string()),
                ("e".to_string(), "v".to_string()),
            ]
        );
    }

    /// Empty expression strings + unparseable text return Ok — the
    /// typed-expression pipeline surfaces those failures via its own
    /// diagnostics later. Cross-kind validator stays silent so we
    /// don't double-emit on the same expression span.
    #[test]
    fn check_expression_silent_on_empty_or_unparseable() {
        let surface: HashMap<String, ImportMemberSurface> = HashMap::new();
        assert!(check_expression(
            "",
            &surface,
            ForgeKind::Algorithm,
            "test",
            "test.scxml",
            None
        )
        .is_ok());
        assert!(check_expression(
            "((",
            &surface,
            ForgeKind::Algorithm,
            "test",
            "test.scxml",
            None
        )
        .is_ok());
    }

    /// Member access on an unknown object name is silent — only
    /// references whose object IS a known import alias get the
    /// field-not-found check. Locals / signature params resolve
    /// elsewhere.
    #[test]
    fn check_expression_silent_when_object_not_an_alias() {
        let surface: HashMap<String, ImportMemberSurface> = HashMap::new();
        // `local_var.x` references a local — surface table has no
        // `local_var` alias entry, so the check is silent.
        assert!(check_expression(
            "local_var.x",
            &surface,
            ForgeKind::Algorithm,
            "test",
            "test.scxml",
            None
        )
        .is_ok());
    }

    /// Known alias + unknown field → `CrossKindFieldNotFound` with the
    /// sorted candidate set as the closed `Fix::ReplaceOneOf` carrier.
    #[test]
    fn check_expression_rejects_typo_on_known_alias() {
        let mut surface: HashMap<String, ImportMemberSurface> = HashMap::new();
        surface.insert(
            "frame".to_string(),
            ImportMemberSurface {
                imported_kind: ForgeKind::Codec,
                imported_name: "udp_frame".to_string(),
                fields: vec![
                    ("msg_id".to_string(), SceType::Uint8),
                    ("payload".to_string(), SceType::Bytes),
                ],
            },
        );
        let err = check_expression(
            "frame.msgid", // typo: should be msg_id
            &surface,
            ForgeKind::Algorithm,
            "test",
            "test.scxml",
            None,
        )
        .expect_err("typo must reject");
        match err.error {
            crate::forge::error::ForgeError::Validation(boxed) => match *boxed {
                ValidationError::CrossKindFieldNotFound {
                    alias,
                    field,
                    candidates,
                    ..
                } => {
                    assert_eq!(alias, "frame");
                    assert_eq!(field, "msgid");
                    assert_eq!(
                        candidates,
                        vec!["msg_id".to_string(), "payload".to_string()]
                    );
                }
                other => panic!("unexpected variant: {other:?}"),
            },
            other => panic!("expected Validation, got {other:?}"),
        }
    }

    /// Known alias + known field → silent. Both the type-mismatch and
    /// field-not-found axes resolve the field correctly; the binding
    /// is well-typed.
    #[test]
    fn check_expression_silent_on_resolved_field() {
        let mut surface: HashMap<String, ImportMemberSurface> = HashMap::new();
        surface.insert(
            "frame".to_string(),
            ImportMemberSurface {
                imported_kind: ForgeKind::Codec,
                imported_name: "udp_frame".to_string(),
                fields: vec![("payload".to_string(), SceType::Bytes)],
            },
        );
        assert!(check_expression(
            "frame.payload",
            &surface,
            ForgeKind::Algorithm,
            "test",
            "test.scxml",
            None,
        )
        .is_ok());
    }
}
