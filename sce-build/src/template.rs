// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// `sce:template` / `sce:use` / `sce:param` preprocessing for SCXML
// source documents. AOT-only expansion per RFC §6.5 Phase A; the C++
// runtime does not implement template expansion, so documents
// containing `<sce:use>` are accepted only through the `sce-build`
// pipeline.
//
// Pairs logically with `sce-build/src/xinclude.rs` and runs
// immediately after it in `parser.rs` so templates see a
// post-XInclude document and can reference fragments composed
// through either mechanism. See `ARCHITECTURE.md` → "Scope &
// Composition" for the composition charter; spec frozen in
// `claudedocs/rfc-sce-template-sce-param.md`.
//
// # Syntax (RFC §3)
//
// Template declaration — standalone file, root `<sce:template>`:
//
//     <sce:template xmlns:sce="http://sce.dev/ext" name="port_guard">
//       <sce:param name="port" required="true"/>
//       <sce:param name="proto" default="TCP"/>
//       <transition cond="_event.data.port == {$port}" target="accept"/>
//     </sce:template>
//
// Template invocation — anywhere in an SCXML document:
//
//     <sce:use template="guards.sce-template.xml" port="80"/>
//
// `{$name}` tokens inside attribute values and text nodes are
// replaced by the parameter's bound string in a single lexical pass.
// Expansion happens on the raw template text (after a well-formed
// parse): parameter values are opaque strings and the caller is
// responsible for escaping per RFC §6.2.
//
// # Rejected features (out of scope per RFC §2)
//
// - Turing-complete templating (no `<if>`, no loops, no computed
//   expressions inside template bodies).
// - Back-references in `default="..."` — literal only (RFC §6.1).
// - Per-language value escaping — templates operate on the XML byte
//   stream (RFC §6.2).

use crate::position_map::{Origin, PositionMap};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Maximum nesting depth for recursive `<sce:use>` expansion.
///
/// Mirrors the value and rationale of [`crate::xinclude::MAX_XINCLUDE_DEPTH`]:
/// an independent constant because template expansion is an AOT-only
/// feature and does not cross the language boundary, but the same
/// acyclic-chain protection applies. `template_depth_matches_xinclude`
/// in the test module pins both values so a future change to one
/// surfaces as a red test rather than silent divergence in
/// diagnostic behaviour.
pub const MAX_TEMPLATE_DEPTH: u32 = 10;

/// SCE extension namespace. The discriminator for `<sce:template>`,
/// `<sce:use>`, and `<sce:param>` elements; templates are not
/// lenient about the namespace declaration (unlike XInclude, which
/// matches either the official namespace or a bare local name) —
/// `sce:` is strict because the extension namespace is the SCE
/// wire boundary.
pub const SCE_EXT_NS: &str = "http://sce.dev/ext";

/// Errors raised by the `<sce:use>` / `<sce:template>` preprocessor.
///
/// Each variant maps to exactly one `xml/template-*` diagnostic code
/// in `sce-build/src/forge/diagnostic.rs`. The offending `<sce:use>`
/// node's row and column are attached by [`expand`] via
/// [`TemplateLocation`], so the variants themselves stay orthogonal
/// to position.
#[derive(Debug, thiserror::Error)]
pub enum TemplateError {
    /// `<sce:use template="...">` resolved neither against the
    /// including file's directory nor the current working directory.
    /// `searched` lists the paths tried so the operator can pick the
    /// right one without guessing. Also fired when the `template`
    /// attribute is empty, since an empty path never resolves.
    #[error("<sce:use template=\"{template}\">: file not found (searched: {searched})")]
    NotFound { template: String, searched: String },

    /// Resolved file exists but could not be read — permission
    /// denied, I/O failure, etc. Classified Diagnostic-only in the
    /// acceptance doc: infrastructure failure the SCXML author
    /// cannot prevent by editing the document.
    #[error("<sce:use template=\"{template}\">: cannot read: {source}")]
    ReadError {
        template: String,
        #[source]
        source: std::io::Error,
    },

    /// Template file was read but either (a) is not well-formed XML,
    /// (b) its root is not `<sce:template>`, or (c) a `<sce:param>`
    /// declaration is ill-formed (missing `name`, invalid name
    /// pattern, duplicate name, or both `required="true"` and
    /// `default=...` declared). These three repair surfaces share
    /// the Malformed code because each points at the template file
    /// itself; errors at the call site (e.g. `<sce:use>` missing
    /// the required `template` attribute) ride
    /// [`TemplateError::MissingTemplateAttribute`] instead so
    /// repair agents can dispatch call-site vs file-side fixes
    /// without text parsing.
    #[error("<sce:use template=\"{template}\">: template is malformed: {detail}")]
    Malformed { template: String, detail: String },

    /// `<sce:use>` element is missing the required `template`
    /// attribute (or the attribute is present but empty). Carries
    /// a deterministic `add_attribute` fix so repair agents can
    /// insert the attribute without reading the call-site context.
    /// Mirrors the [`crate::xinclude::XIncludeError::MissingHref`]
    /// pattern: the required-attribute case gets its own code so
    /// `xml/template-malformed` stays focused on file-side issues.
    #[error("<sce:use> missing required `template` attribute")]
    MissingTemplateAttribute,

    /// `<sce:use>` omitted a `<sce:param required="true">`. Carries
    /// a deterministic `add_attribute` fix so repair agents can
    /// insert the missing attribute without re-parsing the
    /// template file.
    #[error(
        "<sce:use template=\"{template}\">: missing required parameter '{param}'"
    )]
    MissingParam { template: String, param: String },

    /// `<sce:use>` attribute does not match any `<sce:param name>`
    /// declared on the target template. `declared` lists the known
    /// parameter names so the operator can pick the right one
    /// without re-reading the template file.
    #[error(
        "<sce:use template=\"{template}\">: unknown parameter '{param}' (declared: {declared})"
    )]
    UnknownParam {
        template: String,
        param: String,
        declared: String,
    },

    /// A cycle has been detected in the template inclusion graph:
    /// including the referenced file would revisit a file already
    /// on the current expansion stack. `chain` is the rendered
    /// stack (root → leaf, separated by " → ") for operator
    /// diagnosis.
    #[error("<sce:use template=\"{template}\">: cycle detected ({chain})")]
    Cycle { template: String, chain: String },

    /// Recursion exceeded [`MAX_TEMPLATE_DEPTH`]. This catches
    /// pathological (but acyclic) template chains where each file
    /// pulls in another without looping back.
    #[error("<sce:use> template nesting exceeds depth limit of {limit}")]
    TooDeep { limit: u32 },
}

/// Location of an `<sce:use>` inside the source string — 1-based
/// row, 1-based column, matching the [`roxmltree::TextPos`]
/// convention used across the forge error pipeline.
#[derive(Debug, Clone, Copy)]
pub struct TemplateLocation {
    pub row: u32,
    pub col: u32,
}

/// Declaration of a single `<sce:param>` inside a template file.
struct ParamDecl {
    name: String,
    required: bool,
    /// Literal default value. `None` when the param is either
    /// `required="true"` (and therefore the call site must bind it)
    /// or has neither `required` nor `default` (empty-default
    /// semantics per RFC §3.1).
    default: Option<String>,
}

/// Expand every `<sce:use>` element in `content`.
///
/// `self_path` is the filesystem path of the document supplying
/// `content`. It is added to the cycle-detection stack, used for
/// diagnostic rendering, and used as the `Origin::CallSite` path
/// when a `<sce:use>` in this document supplies parameter values.
/// Callers that have no filesystem identity (in-memory documents)
/// should pass a stable label. `base_dir` is the directory
/// `<sce:use template="relative/...">` is resolved against —
/// typically `Path::new(self_path).parent()`.
///
/// `input_map` is the [`PositionMap`] keyed by `content`'s bytes,
/// produced by the preprocessor stage immediately upstream — at
/// the parser-boundary call site that is the [`crate::xinclude::expand`]
/// output, so every byte of `content` already traces back to an
/// outer-file or included-fragment origin. The returned
/// `PositionMap` composes `input_map` (for bytes copied from
/// `content`) with this expander's own [`Origin::File`] entries
/// (for template-body bytes) and [`Origin::CallSite`] entries
/// (for `{$param}` substitutions) so every post-expansion
/// diagnostic can be remapped to a file/row/col the author can
/// open. See `docs/SCE_ACCEPTED_SUBSET.md` §2.9 for the attribution
/// contract.
///
/// Returns the expanded document as an owned `String` suitable for
/// handing to `roxmltree::Document::parse`. Short-circuits with
/// `Ok((content.to_string(), input_map.clone()))` when the input
/// contains no `sce:use` substring, so documents that do not use
/// templates pay only a single substring search plus a map clone
/// on the critical path.
pub fn expand(
    content: &str,
    self_path: &str,
    base_dir: Option<&Path>,
    input_map: &PositionMap,
) -> Result<(String, PositionMap, Vec<PathBuf>), (TemplateError, TemplateLocation)> {
    if !content.contains("sce:use") {
        return Ok((content.to_string(), input_map.clone(), Vec::new()));
    }
    let self_file = PathBuf::from(self_path);
    let mut stack: Vec<PathBuf> = Vec::new();
    if let Ok(abs) = std::fs::canonicalize(self_path) {
        stack.push(abs);
    } else {
        stack.push(self_file.clone());
    }
    // Every `<sce:use template="...">` fragment we successfully open
    // feeds this collector; the parse-boundary call site
    // (`expand_preprocessors`) surfaces it to the depfile sink so
    // CMake/Ninja invalidate generated `_sm.{h,inl}` artifacts when a
    // fragment changes. Without this list the depfile only carries
    // the SCE jinja2 templates and the host SCXML, leaving fragment
    // edits as silent no-ops — see tc8-harness feedback report.
    let mut deps: Vec<PathBuf> = Vec::new();
    let (out, map) = expand_impl(content, &self_file, base_dir, 0, &mut stack, input_map, &mut deps)?;
    Ok((out, map, deps))
}

fn expand_impl(
    content: &str,
    content_file: &Path,
    base_dir: Option<&Path>,
    depth: u32,
    stack: &mut Vec<PathBuf>,
    input_map: &PositionMap,
    deps: &mut Vec<PathBuf>,
) -> Result<(String, PositionMap), (TemplateError, TemplateLocation)> {
    if depth >= MAX_TEMPLATE_DEPTH {
        return Err((
            TemplateError::TooDeep {
                limit: MAX_TEMPLATE_DEPTH,
            },
            TemplateLocation { row: 1, col: 1 },
        ));
    }

    let doc = roxmltree::Document::parse(content).map_err(|e| {
        let pos = e.pos();
        (
            TemplateError::Malformed {
                template: String::new(),
                detail: e.to_string(),
            },
            TemplateLocation {
                row: pos.row,
                col: pos.col,
            },
        )
    })?;

    let root = doc.root_element();
    let uses: Vec<roxmltree::Node> = collect_uses(&root);
    if uses.is_empty() {
        // No `<sce:use>` in this content — the output is a 1:1
        // copy of `content`, so the upstream map already describes
        // every emitted byte.
        return Ok((content.to_string(), input_map.clone()));
    }

    // Walk original byte stream, replacing each `<sce:use>` range
    // with the rendered template body. Document-order processing
    // lets a single cursor serve the splice. The outer position
    // map is built in lock-step with the output string: prefix /
    // tail regions compose from `input_map`, and each spliced
    // body composes from the nested expansion's own map (which in
    // turn carries template-file `Origin::File` entries and
    // caller-file `Origin::CallSite` entries).
    let mut out = String::with_capacity(content.len());
    let mut cursor = 0usize;
    let mut out_map = PositionMap::default();

    for node in uses {
        let use_range = node.range();

        // Prefix [cursor, use_range.start) — bytes unchanged from
        // `content`, so compose from `input_map`.
        if cursor < use_range.start {
            let out_start = out.len();
            out.push_str(&content[cursor..use_range.start]);
            out_map.append_mapped_substring(input_map, cursor, use_range.start, out_start);
        }

        let loc = doc_loc(&doc, use_range.start);

        let template_attr = node
            .attribute("template")
            .filter(|v| !v.is_empty())
            .ok_or((TemplateError::MissingTemplateAttribute, loc))?;

        let resolved =
            resolve_template_path(template_attr, base_dir).map_err(|e| (e, loc))?;

        // Cycle detection via canonicalised path. Also deduplicates
        // aliased forms (`./foo.xml`, `foo.xml`, `../dir/foo.xml`).
        let canon = std::fs::canonicalize(&resolved).unwrap_or_else(|_| resolved.clone());
        if stack.contains(&canon) {
            let chain = render_chain(stack, &canon);
            return Err((
                TemplateError::Cycle {
                    template: template_attr.to_string(),
                    chain,
                },
                loc,
            ));
        }

        let template_raw = std::fs::read_to_string(&resolved).map_err(|e| {
            (
                TemplateError::ReadError {
                    template: template_attr.to_string(),
                    source: e,
                },
                loc,
            )
        })?;

        // Record the successful open *after* read_to_string returned
        // bytes. Cycle/depth pre-checks above can short-circuit before
        // the file is actually consumed, so pushing earlier would put
        // unread paths into the depfile.
        deps.push(canon.clone());

        let params_bound = collect_use_bindings(&node);
        // Substitute `{$name}` tokens inside the template body only —
        // the `<sce:template>` wrapper and `<sce:param>` declarations
        // are preserved verbatim so (a) nested `<sce:use>` inside the
        // body can be recursively expanded with the wrapper's
        // namespace context intact, and (b) `<sce:param default="...">`
        // literals are never themselves substituted (RFC §6.1).
        // `intermediate_map` is keyed by `substituted` bytes and
        // records File origins for template-file regions + CallSite
        // origins for each `{$param}` splice so the recursive expander
        // below can compose them further without redoing the work.
        let (substituted, intermediate_map) = substitute_into_template_with_map(
            &template_raw,
            &resolved,
            template_attr,
            &params_bound,
            content_file,
            loc.row,
            loc.col,
        )
        .map_err(|e| (e, loc))?;

        // Recurse on the substituted template. Cycle detection and
        // depth bound guard against pathological chains. The nested
        // call inherits `intermediate_map` as its `input_map`, so
        // its output map threads caller→template→nested-template
        // origins all the way down.
        stack.push(canon);
        let nested_base = resolved.parent().map(|p| p.to_path_buf());
        let (expanded_template, expanded_map) = expand_impl(
            &substituted,
            &resolved,
            nested_base.as_deref(),
            depth + 1,
            stack,
            &intermediate_map,
            deps,
        )
        .map_err(|(err, _)| (remap_nested(err, template_attr), loc))?;
        stack.pop();

        // Extract body (children of `<sce:template>` minus
        // `<sce:param>`) from the fully-expanded template and splice
        // it into the outer document in place of the `<sce:use>`
        // node. Ranges are byte-range pairs into `expanded_template`;
        // each segment composes the matching slice of
        // `expanded_map` into the outer map.
        let body_ranges =
            extract_template_body_ranges(&expanded_template, template_attr)
                .map_err(|e| (e, loc))?;
        for range in &body_ranges {
            let seg_splice_start = out.len();
            out.push_str(&expanded_template[range.start..range.end]);
            out_map.append_mapped_substring(
                &expanded_map,
                range.start,
                range.end,
                seg_splice_start,
            );
        }

        cursor = use_range.end;
    }

    // Tail [cursor, content.len()) — unchanged from `content`.
    if cursor < content.len() {
        let out_start = out.len();
        out.push_str(&content[cursor..]);
        out_map.append_mapped_substring(input_map, cursor, content.len(), out_start);
    }

    Ok((out, out_map))
}

/// Collect every top-level `<sce:use>` element in the document in
/// document order. Nested `<sce:use>` elements inside an expanded
/// template body are handled by the recursive [`expand_impl`] call
/// on the rendered output — this walker only needs the top-level
/// shape of the current document.
fn collect_uses<'a, 'input>(
    root: &roxmltree::Node<'a, 'input>,
) -> Vec<roxmltree::Node<'a, 'input>> {
    let mut out = Vec::new();
    collect_uses_into(root, &mut out);
    out
}

fn collect_uses_into<'a, 'input>(
    node: &roxmltree::Node<'a, 'input>,
    out: &mut Vec<roxmltree::Node<'a, 'input>>,
) {
    for child in node.children() {
        if !child.is_element() {
            continue;
        }
        if is_sce(&child, "use") {
            out.push(child);
        } else {
            collect_uses_into(&child, out);
        }
    }
}

/// Resolve `template` to an absolute path using the same precedence
/// as [`crate::xinclude::resolve_href`]: absolute → base directory →
/// current working directory. Returns `NotFound` with the search
/// trail on failure so the operator can see which paths were tried.
fn resolve_template_path(
    template: &str,
    base_dir: Option<&Path>,
) -> Result<PathBuf, TemplateError> {
    let path = Path::new(template);
    let mut tried: Vec<String> = Vec::new();

    if path.is_absolute() {
        if path.exists() {
            return Ok(path.to_path_buf());
        }
        tried.push(path.display().to_string());
    } else {
        if let Some(base) = base_dir {
            let candidate = base.join(path);
            if candidate.exists() {
                return Ok(candidate);
            }
            tried.push(candidate.display().to_string());
        }
        if path.exists() {
            return Ok(path.to_path_buf());
        }
        tried.push(path.display().to_string());
    }

    Err(TemplateError::NotFound {
        template: template.to_string(),
        searched: tried.join(", "),
    })
}

/// Parse a template file, validate its structure, bind call-site
/// parameters, and produce the *full* template text with body-scoped
/// `{$name}` substitution applied — along with a [`PositionMap`]
/// keyed by the returned text's bytes. The `<sce:template>` wrapper
/// and the `<sce:param>` declarations are emitted verbatim so (a)
/// the return value is a self-contained XML document that can be
/// fed back through [`expand_impl`] for nested `<sce:use>` expansion
/// without losing the `sce:` namespace binding declared on the
/// template root, and (b) `<sce:param default="...">` literals are
/// not themselves substituted (Q1 literal-only defaults).
///
/// The emitted map carries one [`Origin::File`] entry per contiguous
/// run of template-file bytes (prefix before body, non-substituted
/// body regions, suffix after body) and one [`Origin::CallSite`]
/// entry per non-empty `{$param}` substitution, where the call-site
/// (row, col) is the `<sce:use>` element's position in the caller —
/// depth-1 per RFC §6.3 Q3 and `SCE_ACCEPTED_SUBSET.md` §2.9.
fn substitute_into_template_with_map(
    template_raw: &str,
    template_path: &Path,
    template_href: &str,
    bound: &HashMap<String, String>,
    caller_file: &Path,
    caller_row: u32,
    caller_col: u32,
) -> Result<(String, PositionMap), TemplateError> {
    let doc = roxmltree::Document::parse(template_raw).map_err(|e| {
        TemplateError::Malformed {
            template: template_href.to_string(),
            detail: e.to_string(),
        }
    })?;
    let root = doc.root_element();

    if !is_sce(&root, "template") {
        return Err(TemplateError::Malformed {
            template: template_href.to_string(),
            detail: format!(
                "root element must be <sce:template>, got <{}>",
                root.tag_name().name()
            ),
        });
    }

    // Collect `<sce:param>` declarations and compute the body byte
    // span (first non-param child start → last non-param child end).
    // Substitution applies only within that span.
    let mut decls: Vec<ParamDecl> = Vec::new();
    let mut body_start: Option<usize> = None;
    let mut body_end: Option<usize> = None;

    for child in root.children() {
        if child.is_element() && is_sce(&child, "param") {
            decls.push(parse_param_decl(&child, template_href)?);
            continue;
        }
        if !child.is_element() && !child.is_text() && !child.is_comment() {
            continue;
        }
        let range = child.range();
        if body_start.is_none() {
            body_start = Some(range.start);
        }
        body_end = Some(range.end);
    }

    // Reject duplicate declarations — ambiguity of "last one wins"
    // is worse than a hard error at authoring time.
    let mut seen: Vec<&str> = Vec::with_capacity(decls.len());
    for d in &decls {
        if seen.contains(&d.name.as_str()) {
            return Err(TemplateError::Malformed {
                template: template_href.to_string(),
                detail: format!("duplicate <sce:param name=\"{}\"> declaration", d.name),
            });
        }
        seen.push(&d.name);
    }

    // Validate bindings: every bound name must be declared, every
    // required param must be bound, defaults fill the rest.
    let declared_names = decls
        .iter()
        .map(|d| d.name.clone())
        .collect::<Vec<_>>()
        .join(", ");
    for name in bound.keys() {
        if !decls.iter().any(|d| &d.name == name) {
            return Err(TemplateError::UnknownParam {
                template: template_href.to_string(),
                param: name.clone(),
                declared: if declared_names.is_empty() {
                    "<none>".to_string()
                } else {
                    declared_names.clone()
                },
            });
        }
    }

    let mut params: HashMap<String, String> = HashMap::with_capacity(decls.len());
    for d in &decls {
        if let Some(v) = bound.get(&d.name) {
            params.insert(d.name.clone(), v.clone());
        } else if d.required {
            return Err(TemplateError::MissingParam {
                template: template_href.to_string(),
                param: d.name.clone(),
            });
        } else if let Some(default) = &d.default {
            params.insert(d.name.clone(), default.clone());
        } else {
            // Neither bound, required, nor defaulted — empty per
            // RFC §3.1 (the three-state fallback).
            params.insert(d.name.clone(), String::new());
        }
    }

    let mut out_map = PositionMap::default();
    out_map.register_file(template_path.to_path_buf(), template_raw);

    // Reassemble: original prefix up to body start, substituted
    // body, original suffix after body end. If the template has no
    // body (only params or only whitespace), pass the raw through
    // as a single identity entry over the template file.
    match (body_start, body_end) {
        (Some(bs), Some(be)) => {
            let mut out = String::with_capacity(template_raw.len() + 32);
            // Prefix (template-file bytes [0, bs)).
            if bs > 0 {
                out.push_str(&template_raw[..bs]);
                out_map.push_entry(
                    0,
                    bs,
                    Origin::File {
                        path: template_path.to_path_buf(),
                        source_offset: 0,
                    },
                );
            }
            // Body with substitutions. `apply_substitution_with_tracking`
            // returns entries keyed by substituted-body offsets; we
            // shift them by `body_base = out.len()` so they line up
            // inside `out_map`.
            let body_base = out.len();
            let (substituted_body, body_entries) = apply_substitution_with_tracking(
                &template_raw[bs..be],
                bs,
                template_path,
                &params,
                caller_file,
                caller_row,
                caller_col,
            );
            out.push_str(&substituted_body);
            for (seg_start, seg_end, origin) in body_entries {
                out_map.push_entry(body_base + seg_start, body_base + seg_end, origin);
            }
            // Suffix (template-file bytes [be, template_raw.len())).
            if be < template_raw.len() {
                let suffix_start = out.len();
                out.push_str(&template_raw[be..]);
                out_map.push_entry(
                    suffix_start,
                    out.len(),
                    Origin::File {
                        path: template_path.to_path_buf(),
                        source_offset: be,
                    },
                );
            }
            Ok((out, out_map))
        }
        _ => {
            // No body — return template_raw as an identity map over
            // the template file.
            out_map.push_entry(
                0,
                template_raw.len(),
                Origin::File {
                    path: template_path.to_path_buf(),
                    source_offset: 0,
                },
            );
            Ok((template_raw.to_string(), out_map))
        }
    }
}

/// Extract the byte ranges of the body children of a post-substitution,
/// post-recursion template: every non-`<sce:param>` child of the
/// `<sce:template>` root, in document order. Returned ranges are
/// byte offsets into `expanded_template`. The caller splices the
/// matching slices into the outer document in place of the
/// `<sce:use>` node and composes each range's slice of the
/// expanded map into the outer map — keeping every emitted byte
/// traceable to a File or CallSite origin.
fn extract_template_body_ranges(
    expanded_template: &str,
    template_href: &str,
) -> Result<Vec<std::ops::Range<usize>>, TemplateError> {
    let doc = roxmltree::Document::parse(expanded_template).map_err(|e| {
        TemplateError::Malformed {
            template: template_href.to_string(),
            detail: format!("expanded template is malformed: {}", e),
        }
    })?;
    let root = doc.root_element();
    if !is_sce(&root, "template") {
        return Err(TemplateError::Malformed {
            template: template_href.to_string(),
            detail: "expanded template root is not <sce:template>".to_string(),
        });
    }
    let mut ranges = Vec::new();
    for child in root.children() {
        if child.is_element() && is_sce(&child, "param") {
            continue;
        }
        let range = child.range();
        ranges.push(range.start..range.end);
    }
    Ok(ranges)
}

/// Parse a single `<sce:param>` declaration. Validates the `name`
/// pattern and the mutual exclusion of `required` and `default`.
fn parse_param_decl(
    node: &roxmltree::Node,
    template_href: &str,
) -> Result<ParamDecl, TemplateError> {
    let name = node
        .attribute("name")
        .ok_or_else(|| TemplateError::Malformed {
            template: template_href.to_string(),
            detail: "<sce:param> missing `name` attribute".to_string(),
        })?
        .to_string();

    if !is_valid_param_name(&name) {
        return Err(TemplateError::Malformed {
            template: template_href.to_string(),
            detail: format!(
                "<sce:param name=\"{}\">: name must match [A-Za-z_][A-Za-z0-9_-]*",
                name
            ),
        });
    }

    let required = match node.attribute("required") {
        Some("true") => true,
        Some("false") | None => false,
        Some(other) => {
            return Err(TemplateError::Malformed {
                template: template_href.to_string(),
                detail: format!(
                    "<sce:param name=\"{}\"> `required` must be \"true\" or \"false\", got \"{}\"",
                    name, other
                ),
            });
        }
    };

    let default = node.attribute("default").map(|s| s.to_string());

    if required && default.is_some() {
        return Err(TemplateError::Malformed {
            template: template_href.to_string(),
            detail: format!(
                "<sce:param name=\"{}\"> declares both `required=\"true\"` and `default=\"...\"` — they are mutually exclusive",
                name
            ),
        });
    }

    Ok(ParamDecl {
        name,
        required,
        default,
    })
}

/// Collect parameter bindings from a `<sce:use>` element. Every
/// attribute other than the reserved `template` attribute contributes
/// one binding. Namespace-prefixed attributes (e.g. `sce:foo`) are
/// currently not expected on `<sce:use>` and are folded into the
/// binding set by local name — malformed usage surfaces as
/// `UnknownParam` against the template's declarations.
fn collect_use_bindings(node: &roxmltree::Node) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for attr in node.attributes() {
        let name = attr.name();
        if name == "template" && attr.namespace().is_none() {
            continue;
        }
        map.insert(name.to_string(), attr.value().to_string());
    }
    map
}

/// Single-pass lexical substitution of `{$name}` tokens, annotated
/// with per-region [`Origin`] entries for the emitted bytes.
/// Atomic: replacements do not cascade (a parameter value that
/// itself contains `{$other}` is emitted verbatim, matching
/// RFC §6.1 literal-only default semantics).
///
/// Non-substituted runs of `body` get [`Origin::File`] entries
/// pointing at `template_path` (with `source_offset` computed from
/// `body_source_offset + body_cursor` so later lookups resolve into
/// the template file's own text). Each non-empty `{$name}`
/// substitution gets an [`Origin::CallSite`] entry naming
/// `caller_file` at `(caller_row, caller_col)` — the depth-1
/// collapse per RFC §6.3 Q3.
///
/// Undeclared `{$name}` tokens and malformed `{$` prefixes that
/// do not match any declared parameter are emitted verbatim as
/// template-file bytes; diagnostic for undeclared refs is the
/// author's responsibility.
///
/// Returned entries start at byte 0 of the returned string and are
/// contiguous — the caller shifts them by the body's splice offset
/// inside the containing [`PositionMap`].
fn apply_substitution_with_tracking(
    body: &str,
    body_source_offset: usize,
    template_path: &Path,
    params: &HashMap<String, String>,
    caller_file: &Path,
    caller_row: u32,
    caller_col: u32,
) -> (String, Vec<(usize, usize, Origin)>) {
    let mut out = String::with_capacity(body.len());
    let mut entries: Vec<(usize, usize, Origin)> = Vec::new();
    let mut pos = 0usize;
    while let Some(start_rel) = body[pos..].find("{$") {
        let start = pos + start_rel;
        // Flush non-substituted prefix [pos, start) as template-file bytes.
        if start > pos {
            let out_start = out.len();
            out.push_str(&body[pos..start]);
            entries.push((
                out_start,
                out.len(),
                Origin::File {
                    path: template_path.to_path_buf(),
                    source_offset: body_source_offset + pos,
                },
            ));
        }
        let after = start + 2;
        if let Some(end_rel) = body[after..].find('}') {
            let name = &body[after..after + end_rel];
            if is_valid_param_name(name) {
                if let Some(value) = params.get(name) {
                    if !value.is_empty() {
                        let out_start = out.len();
                        out.push_str(value);
                        entries.push((
                            out_start,
                            out.len(),
                            Origin::CallSite {
                                path: caller_file.to_path_buf(),
                                row: caller_row,
                                col: caller_col,
                            },
                        ));
                    }
                    pos = after + end_rel + 1;
                    continue;
                }
            }
        }
        // Not a valid `{$name}` token — emit `{$` literally as
        // template-file bytes. The next loop iteration picks up
        // from `after` without re-matching the `{$` we just emitted.
        let out_start = out.len();
        out.push_str("{$");
        entries.push((
            out_start,
            out.len(),
            Origin::File {
                path: template_path.to_path_buf(),
                source_offset: body_source_offset + start,
            },
        ));
        pos = after;
    }
    // Tail (any bytes after the last `{$`).
    if pos < body.len() {
        let out_start = out.len();
        out.push_str(&body[pos..]);
        entries.push((
            out_start,
            out.len(),
            Origin::File {
                path: template_path.to_path_buf(),
                source_offset: body_source_offset + pos,
            },
        ));
    }
    (out, entries)
}

/// Validate a `<sce:param name>` value. Matches
/// `[A-Za-z_][A-Za-z0-9_-]*` per RFC §3.1.
fn is_valid_param_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Check that `node` is an element in the SCE extension namespace
/// with the given local name. Strict namespace matching — unlike
/// XInclude, `sce:*` is the SCE wire boundary and must be
/// explicitly prefixed.
fn is_sce(node: &roxmltree::Node, local_name: &str) -> bool {
    node.is_element()
        && node.tag_name().namespace() == Some(SCE_EXT_NS)
        && node.tag_name().name() == local_name
}

/// Convert a byte offset into a 1-based (row, col) pair via
/// [`roxmltree::Document::text_pos_at`]. Used to tag the offending
/// `<sce:use>` node with a position the diagnostic pipeline can
/// surface.
fn doc_loc(doc: &roxmltree::Document, offset: usize) -> TemplateLocation {
    let pos = doc.text_pos_at(offset);
    TemplateLocation {
        row: pos.row,
        col: pos.col,
    }
}

/// Render a cycle chain as `outer → inner → …` for diagnostics.
fn render_chain(stack: &[PathBuf], next: &Path) -> String {
    let mut parts: Vec<String> = stack.iter().map(|p| p.display().to_string()).collect();
    parts.push(next.display().to_string());
    parts.join(" → ")
}

/// Rewrite a nested error's `template` field so the outer
/// diagnostic names the `<sce:use>` the operator sees, not the
/// transitive reference inside the included chain.
fn remap_nested(err: TemplateError, outer_href: &str) -> TemplateError {
    match err {
        TemplateError::TooDeep { .. }
        | TemplateError::Cycle { .. }
        | TemplateError::MissingTemplateAttribute => err,
        TemplateError::NotFound { searched, .. } => TemplateError::NotFound {
            template: outer_href.to_string(),
            searched,
        },
        TemplateError::ReadError { source, .. } => TemplateError::ReadError {
            template: outer_href.to_string(),
            source,
        },
        TemplateError::Malformed { detail, .. } => TemplateError::Malformed {
            template: outer_href.to_string(),
            detail,
        },
        TemplateError::MissingParam { param, .. } => TemplateError::MissingParam {
            template: outer_href.to_string(),
            param,
        },
        TemplateError::UnknownParam {
            param, declared, ..
        } => TemplateError::UnknownParam {
            template: outer_href.to_string(),
            param,
            declared,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, content).expect("tempfile write");
        path
    }

    #[test]
    fn template_depth_matches_xinclude() {
        // The two preprocessors bound nesting by the same value so
        // operators do not need to remember two different limits.
        // If either changes, update the other in the same commit.
        assert_eq!(MAX_TEMPLATE_DEPTH, crate::xinclude::MAX_XINCLUDE_DEPTH);
    }

    #[test]
    fn passthrough_when_no_sce_use_substring() {
        let src = "<root><state id=\"s1\"/></root>";
        let input_map = PositionMap::identity("inline", src);
        let (out, map, _deps) = expand(src, "inline", None, &input_map).expect("no sce:use");
        assert_eq!(out, src);
        // Short-circuit path must hand the upstream map through
        // untouched — no splices happened.
        assert!(map.is_identity());
    }

    #[test]
    fn passthrough_when_substring_but_no_element() {
        // "sce:use" appears as a word in text/attribute but no
        // element exists — must parse to tell, and pass unchanged.
        let src = "<root description=\"how to sce:use this\"/>";
        let input_map = PositionMap::identity("inline", src);
        let (out, map, _deps) = expand(src, "inline", None, &input_map)
            .expect("no sce:use elements");
        assert_eq!(out, src);
        assert!(map.is_identity());
    }

    #[test]
    fn expands_single_template() {
        let tmp = TempDir::new().unwrap();
        let tpl = write(
            tmp.path(),
            "guard.sce-template.xml",
            r#"<sce:template xmlns:sce="http://sce.dev/ext" name="guard">
  <sce:param name="port" required="true"/>
  <transition cond="_event.port == {$port}" target="accept"/>
</sce:template>"#,
        );
        let main_src = format!(
            r#"<scxml xmlns:sce="http://sce.dev/ext"><sce:use template="{}" port="80"/></scxml>"#,
            tpl.file_name().unwrap().to_str().unwrap()
        );
        let main_path = write(tmp.path(), "main.scxml", &main_src);

        let input_map = PositionMap::identity(main_path.to_str().unwrap(), &main_src);
        let (out, map, _deps) = expand(
            &main_src,
            main_path.to_str().unwrap(),
            Some(tmp.path()),
            &input_map,
        )
        .expect("expansion succeeds");
        assert!(out.contains("_event.port == 80"));
        assert!(!out.contains("<sce:use"));
        assert!(!out.contains("{$port}"));
        // A real splice happened — the output is no longer a 1:1
        // copy of any single file, so the returned map must carry
        // entries beyond identity.
        assert!(!map.is_identity());
    }

    #[test]
    fn default_value_substitutes_when_call_site_omits() {
        let tmp = TempDir::new().unwrap();
        let tpl = write(
            tmp.path(),
            "g.sce-template.xml",
            r#"<sce:template xmlns:sce="http://sce.dev/ext" name="g">
  <sce:param name="proto" default="TCP"/>
  <marker value="{$proto}"/>
</sce:template>"#,
        );
        let main_src = format!(
            r#"<root xmlns:sce="http://sce.dev/ext"><sce:use template="{}"/></root>"#,
            tpl.file_name().unwrap().to_str().unwrap()
        );
        let main_path = write(tmp.path(), "main.xml", &main_src);
        let input_map = PositionMap::identity(main_path.to_str().unwrap(), &main_src);
        let (out, _map, _deps) = expand(
            &main_src,
            main_path.to_str().unwrap(),
            Some(tmp.path()),
            &input_map,
        )
        .unwrap();
        assert!(out.contains(r#"value="TCP""#));
    }

    #[test]
    fn sibling_uses_are_independent() {
        let tmp = TempDir::new().unwrap();
        let tpl = write(
            tmp.path(),
            "g.sce-template.xml",
            r#"<sce:template xmlns:sce="http://sce.dev/ext" name="g">
  <sce:param name="n" required="true"/>
  <marker n="{$n}"/>
</sce:template>"#,
        );
        let main_src = format!(
            r#"<root xmlns:sce="http://sce.dev/ext">
<sce:use template="{name}" n="1"/>
<sce:use template="{name}" n="2"/>
</root>"#,
            name = tpl.file_name().unwrap().to_str().unwrap()
        );
        let main_path = write(tmp.path(), "main.xml", &main_src);
        let input_map = PositionMap::identity(main_path.to_str().unwrap(), &main_src);
        let (out, _map, _deps) = expand(
            &main_src,
            main_path.to_str().unwrap(),
            Some(tmp.path()),
            &input_map,
        )
        .unwrap();
        assert!(out.contains(r#"n="1""#));
        assert!(out.contains(r#"n="2""#));
        assert_eq!(out.matches("<marker").count(), 2);
    }

    #[test]
    fn missing_template_attribute_is_own_variant() {
        let tmp = TempDir::new().unwrap();
        let main_src = r#"<root xmlns:sce="http://sce.dev/ext"><sce:use/></root>"#;
        let main_path = write(tmp.path(), "main.xml", main_src);
        let input_map = PositionMap::identity(main_path.to_str().unwrap(), main_src);
        let err = expand(
            main_src,
            main_path.to_str().unwrap(),
            Some(tmp.path()),
            &input_map,
        )
        .unwrap_err();
        assert!(matches!(err.0, TemplateError::MissingTemplateAttribute));
    }

    #[test]
    fn empty_template_attribute_is_missing_template_attribute() {
        let tmp = TempDir::new().unwrap();
        let main_src = r#"<root xmlns:sce="http://sce.dev/ext"><sce:use template=""/></root>"#;
        let main_path = write(tmp.path(), "main.xml", main_src);
        let input_map = PositionMap::identity(main_path.to_str().unwrap(), main_src);
        let err = expand(
            main_src,
            main_path.to_str().unwrap(),
            Some(tmp.path()),
            &input_map,
        )
        .unwrap_err();
        assert!(matches!(err.0, TemplateError::MissingTemplateAttribute));
    }

    #[test]
    fn missing_required_param_is_error() {
        let tmp = TempDir::new().unwrap();
        let tpl = write(
            tmp.path(),
            "g.sce-template.xml",
            r#"<sce:template xmlns:sce="http://sce.dev/ext" name="g">
  <sce:param name="port" required="true"/>
  <marker p="{$port}"/>
</sce:template>"#,
        );
        let main_src = format!(
            r#"<root xmlns:sce="http://sce.dev/ext"><sce:use template="{}"/></root>"#,
            tpl.file_name().unwrap().to_str().unwrap()
        );
        let main_path = write(tmp.path(), "main.xml", &main_src);
        let input_map = PositionMap::identity(main_path.to_str().unwrap(), &main_src);
        let err = expand(
            &main_src,
            main_path.to_str().unwrap(),
            Some(tmp.path()),
            &input_map,
        )
        .unwrap_err();
        match err.0 {
            TemplateError::MissingParam { param, .. } => assert_eq!(param, "port"),
            other => panic!("expected MissingParam, got {:?}", other),
        }
    }

    #[test]
    fn unknown_param_attribute_is_error() {
        let tmp = TempDir::new().unwrap();
        let tpl = write(
            tmp.path(),
            "g.sce-template.xml",
            r#"<sce:template xmlns:sce="http://sce.dev/ext" name="g">
  <sce:param name="port" required="true"/>
  <marker p="{$port}"/>
</sce:template>"#,
        );
        let main_src = format!(
            r#"<root xmlns:sce="http://sce.dev/ext"><sce:use template="{}" port="80" typo="x"/></root>"#,
            tpl.file_name().unwrap().to_str().unwrap()
        );
        let main_path = write(tmp.path(), "main.xml", &main_src);
        let input_map = PositionMap::identity(main_path.to_str().unwrap(), &main_src);
        let err = expand(
            &main_src,
            main_path.to_str().unwrap(),
            Some(tmp.path()),
            &input_map,
        )
        .unwrap_err();
        match err.0 {
            TemplateError::UnknownParam { param, declared, .. } => {
                assert_eq!(param, "typo");
                assert_eq!(declared, "port");
            }
            other => panic!("expected UnknownParam, got {:?}", other),
        }
    }

    #[test]
    fn nonexistent_template_file_is_not_found() {
        let tmp = TempDir::new().unwrap();
        let main_src = r#"<root xmlns:sce="http://sce.dev/ext"><sce:use template="missing.sce-template.xml"/></root>"#;
        let main_path = write(tmp.path(), "main.xml", main_src);
        let input_map = PositionMap::identity(main_path.to_str().unwrap(), main_src);
        let err = expand(
            main_src,
            main_path.to_str().unwrap(),
            Some(tmp.path()),
            &input_map,
        )
        .unwrap_err();
        assert!(matches!(err.0, TemplateError::NotFound { .. }));
    }

    #[test]
    fn malformed_template_root_is_error() {
        let tmp = TempDir::new().unwrap();
        let _tpl = write(
            tmp.path(),
            "bad.xml",
            r#"<not-a-template><x/></not-a-template>"#,
        );
        let main_src = r#"<root xmlns:sce="http://sce.dev/ext"><sce:use template="bad.xml"/></root>"#;
        let main_path = write(tmp.path(), "main.xml", main_src);
        let input_map = PositionMap::identity(main_path.to_str().unwrap(), main_src);
        let err = expand(
            main_src,
            main_path.to_str().unwrap(),
            Some(tmp.path()),
            &input_map,
        )
        .unwrap_err();
        assert!(matches!(err.0, TemplateError::Malformed { .. }));
    }

    #[test]
    fn param_with_both_required_and_default_is_malformed() {
        let tmp = TempDir::new().unwrap();
        let _tpl = write(
            tmp.path(),
            "bad.sce-template.xml",
            r#"<sce:template xmlns:sce="http://sce.dev/ext" name="bad">
  <sce:param name="port" required="true" default="80"/>
  <marker/>
</sce:template>"#,
        );
        let main_src = r#"<root xmlns:sce="http://sce.dev/ext"><sce:use template="bad.sce-template.xml" port="80"/></root>"#;
        let main_path = write(tmp.path(), "main.xml", main_src);
        let input_map = PositionMap::identity(main_path.to_str().unwrap(), main_src);
        let err = expand(
            main_src,
            main_path.to_str().unwrap(),
            Some(tmp.path()),
            &input_map,
        )
        .unwrap_err();
        assert!(matches!(err.0, TemplateError::Malformed { .. }));
    }

    #[test]
    fn cycle_is_detected() {
        let tmp = TempDir::new().unwrap();
        let a_path = tmp.path().join("a.sce-template.xml");
        let b_path = tmp.path().join("b.sce-template.xml");
        // a uses b, b uses a — cycle through template bodies.
        fs::write(
            &a_path,
            r#"<sce:template xmlns:sce="http://sce.dev/ext" name="a">
  <sce:use template="b.sce-template.xml"/>
</sce:template>"#,
        )
        .unwrap();
        fs::write(
            &b_path,
            r#"<sce:template xmlns:sce="http://sce.dev/ext" name="b">
  <sce:use template="a.sce-template.xml"/>
</sce:template>"#,
        )
        .unwrap();
        let main_src = r#"<root xmlns:sce="http://sce.dev/ext"><sce:use template="a.sce-template.xml"/></root>"#;
        let main_path = write(tmp.path(), "main.xml", main_src);
        let input_map = PositionMap::identity(main_path.to_str().unwrap(), main_src);
        let err = expand(
            main_src,
            main_path.to_str().unwrap(),
            Some(tmp.path()),
            &input_map,
        )
        .unwrap_err();
        assert!(matches!(err.0, TemplateError::Cycle { .. }));
    }

    #[test]
    fn nested_templates_expand_transitively() {
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "inner.sce-template.xml",
            r#"<sce:template xmlns:sce="http://sce.dev/ext" name="inner">
  <leaf/>
</sce:template>"#,
        );
        write(
            tmp.path(),
            "outer.sce-template.xml",
            r#"<sce:template xmlns:sce="http://sce.dev/ext" name="outer">
  <wrap><sce:use template="inner.sce-template.xml"/></wrap>
</sce:template>"#,
        );
        let main_src = r#"<root xmlns:sce="http://sce.dev/ext"><sce:use template="outer.sce-template.xml"/></root>"#;
        let main_path = write(tmp.path(), "main.xml", main_src);
        let input_map = PositionMap::identity(main_path.to_str().unwrap(), main_src);
        let (out, _map, _deps) = expand(
            main_src,
            main_path.to_str().unwrap(),
            Some(tmp.path()),
            &input_map,
        )
        .unwrap();
        assert!(out.contains("<leaf/>"));
        assert!(!out.contains("<sce:use"));
        assert!(!out.contains("<sce:template"));
    }

    #[test]
    fn substitution_is_single_pass() {
        // A param value that itself contains {$name} is emitted
        // verbatim — no cascading (RFC §6.1 literal-only).
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "g.sce-template.xml",
            r#"<sce:template xmlns:sce="http://sce.dev/ext" name="g">
  <sce:param name="a" required="true"/>
  <sce:param name="b" default="{$a}"/>
  <marker b="{$b}"/>
</sce:template>"#,
        );
        let main_src = r#"<root xmlns:sce="http://sce.dev/ext"><sce:use template="g.sce-template.xml" a="HIT"/></root>"#;
        let main_path = write(tmp.path(), "main.xml", main_src);
        let input_map = PositionMap::identity(main_path.to_str().unwrap(), main_src);
        let (out, _map, _deps) = expand(
            main_src,
            main_path.to_str().unwrap(),
            Some(tmp.path()),
            &input_map,
        )
        .unwrap();
        // `b` was not bound at the call site and took its default
        // `{$a}` literally — the literal appears in the output.
        assert!(out.contains(r#"b="{$a}""#));
        assert!(!out.contains(r#"b="HIT""#));
    }

    #[test]
    fn undeclared_braces_pass_through() {
        // `{$` sequence that is not a valid param reference (e.g.
        // malformed or referencing an undeclared name) is emitted
        // verbatim. This preserves authored literal content.
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "g.sce-template.xml",
            r#"<sce:template xmlns:sce="http://sce.dev/ext" name="g">
  <sce:param name="a" required="true"/>
  <marker a="{$a}" note="literal{$undeclared}text"/>
</sce:template>"#,
        );
        let main_src = r#"<root xmlns:sce="http://sce.dev/ext"><sce:use template="g.sce-template.xml" a="X"/></root>"#;
        let main_path = write(tmp.path(), "main.xml", main_src);
        let input_map = PositionMap::identity(main_path.to_str().unwrap(), main_src);
        let (out, _map, _deps) = expand(
            main_src,
            main_path.to_str().unwrap(),
            Some(tmp.path()),
            &input_map,
        )
        .unwrap();
        assert!(out.contains(r#"a="X""#));
        assert!(out.contains("{$undeclared}"));
    }

    #[test]
    fn location_points_at_use_node() {
        // Whitespace before `<sce:use>` places it on row 2 — the
        // reported location must name that row, not row 1.
        let tmp = TempDir::new().unwrap();
        let main_src = "<root xmlns:sce=\"http://sce.dev/ext\">\n    <sce:use/>\n</root>";
        let main_path = write(tmp.path(), "main.xml", main_src);
        let input_map = PositionMap::identity(main_path.to_str().unwrap(), main_src);
        let err = expand(
            main_src,
            main_path.to_str().unwrap(),
            Some(tmp.path()),
            &input_map,
        )
        .unwrap_err();
        assert_eq!(err.1.row, 2);
    }

    #[test]
    fn is_valid_param_name_rejects_invalid_forms() {
        assert!(is_valid_param_name("port"));
        assert!(is_valid_param_name("_port"));
        assert!(is_valid_param_name("port-1"));
        assert!(is_valid_param_name("port_1"));
        assert!(!is_valid_param_name(""));
        assert!(!is_valid_param_name("1port"));
        assert!(!is_valid_param_name("port space"));
        assert!(!is_valid_param_name("port.dot"));
    }

    #[test]
    fn template_expand_empty_result_does_not_panic() {
        // Drift guard for the contract "`expand` always returns a
        // PositionMap on which `lookup` can safely be called".
        // `PositionMap::lookup` panics on a zero-entry map (see the
        // `!self.entries.is_empty()` assertion) and `is_identity()`
        // does *not* short-circuit the empty case because it
        // requires `entries.len() == 1`. So the contract reduces to
        // "expand never returns an empty map".
        //
        // The combinatorial worry is the splice path inside
        // `expand_impl`: it constructs `out_map` from contiguous
        // entries — prefix region, then per-`<sce:use>` body splice,
        // then tail. If the inputs were arranged so that no prefix
        // (first `<sce:use>` at byte 0), no body entries (template
        // has only `<sce:param>` declarations →
        // `extract_template_body_ranges` returns `[]`), and no tail
        // (last `<sce:use>` ends at `content.len()`) all held at
        // once, `out_map` would land at zero entries.
        //
        // This test exercises that combination by making `<sce:use>`
        // both the document root and the entire content, with a
        // body-less template. As of 2026-04-22 the path is
        // *unreachable* from any well-formed XML: `collect_uses`
        // walks `root.children()`, so when `<sce:use>` is itself the
        // root it is never collected and the splice loop never runs;
        // `expand_impl` short-circuits at the `uses.is_empty()`
        // branch and hands back `input_map.clone()`. Any legal
        // multi-`sce:use` arrangement still has root-element open /
        // close tags as prefix and tail bytes, so prefix + tail
        // entries always exist. The empty-`out_map` panic path can
        // therefore only be reached by a future change that either
        // promotes a root-level `<sce:use>` into the splice loop or
        // changes how surrounding bytes are accounted for.
        //
        // Per `feedback_green_tests_not_correct.md` a panic on user
        // input is a real defect even when masked by upstream
        // parser behaviour, so this test stays in place to catch
        // such a regression at the producer rather than waiting for
        // a downstream `lookup` call to surface it.
        let tmp = TempDir::new().unwrap();
        let tpl = write(
            tmp.path(),
            "empty.sce-template.xml",
            r#"<sce:template xmlns:sce="http://sce.dev/ext" name="empty">
  <sce:param name="x" default=""/>
</sce:template>"#,
        );
        let main_src = format!(
            r#"<sce:use xmlns:sce="http://sce.dev/ext" template="{}"/>"#,
            tpl.file_name().unwrap().to_str().unwrap()
        );
        let main_path = write(tmp.path(), "main.xml", &main_src);
        let input_map = PositionMap::identity(main_path.to_str().unwrap(), &main_src);

        let (out, map, _deps) = expand(
            &main_src,
            main_path.to_str().unwrap(),
            Some(tmp.path()),
            &input_map,
        )
        .expect("expansion succeeds even when the result is empty");

        // Sweep [0, max(out.len(), 1)) so the offset=0 case (which
        // historically triggered the panic) is always exercised
        // even when out.len() == 0. The contract this test pins is
        // non-panic + a resolvable SourcePos with sensible 1-based
        // (row, col); the exact file attribution is implementation
        // detail (the natural choice is the caller's file, since
        // the bytes were "consumed from" main.xml).
        let probe_end = out.len().max(1);
        for offset in 0..probe_end {
            let pos = map.lookup(offset);
            assert!(pos.row >= 1, "row must be 1-based, got {}", pos.row);
            assert!(pos.col >= 1, "col must be 1-based, got {}", pos.col);
        }
    }

    /// Drift guard between the XSD `paramNameType` pattern and the
    /// Rust `is_valid_param_name` implementation.
    ///
    /// Both enforce RFC §3.1 `[A-Za-z_][A-Za-z0-9_\-]*`. XSD's check
    /// fires during schema validation on `<sce:param name="...">`
    /// declarations; Rust's check fires inside the expander on the
    /// same input. A silent divergence would let a name accepted by
    /// one tool fail the other — the same `sce:template` file would
    /// schema-validate but trip expansion (or vice versa). The test
    /// extracts the XSD pattern at build time via
    /// [`include_str!`], compiles it as a Rust regex (anchored so it
    /// matches the XSD's whole-string semantics), and asserts both
    /// impls agree on a curated corpus of positive + negative cases
    /// spanning every edge the pattern's character classes care about.
    ///
    /// Follows the `xinclude::xinclude_depth_matches_runtime` pattern
    /// for cross-language mirrored constants.
    ///
    /// Phase B M2 extended this test to also verify behavioural
    /// agreement of the C++ `PARAM_NAME_PATTERN` constant (now
    /// consumed by `SCE::parsing::is_valid_param_name`). The C++
    /// header is read via `include_str!`, its raw-string literal is
    /// extracted, compiled as a Rust regex, and applied to the same
    /// corpus. Byte-equality between the C++ and XSD patterns is
    /// already pinned by `cpp_param_name_pattern_matches_rust`; this
    /// corpus agreement adds behavioural parity so a future edit
    /// that keeps bytes equal but breaks the regex (e.g. an invalid
    /// Unicode escape) surfaces as a test failure rather than a
    /// silent runtime-only divergence.
    #[test]
    fn xsd_param_name_pattern_agrees_with_rust_impl() {
        let xsd = include_str!("../../schemas/sce-forge-ext.xsd");
        // Extract the pattern value from the <xs:pattern value="..."/>
        // inside <xs:simpleType name="paramNameType">. Anchored on
        // `simpleType name="paramNameType"` so we don't accidentally
        // pick up another simpleType's pattern during a future edit.
        let anchor = r#"<xs:simpleType name="paramNameType">"#;
        let start = xsd.find(anchor).expect(
            "XSD must declare <xs:simpleType name=\"paramNameType\"> — \
             check schemas/sce-forge-ext.xsd",
        );
        let after = &xsd[start..];
        let pat_start = after
            .find(r#"<xs:pattern value=""#)
            .expect("paramNameType must declare an <xs:pattern value=\"...\"/> child");
        let pat_begin = start + pat_start + r#"<xs:pattern value=""#.len();
        let pat_end_rel = xsd[pat_begin..]
            .find('"')
            .expect("pattern value must be terminated by a double-quote");
        let xsd_pattern = &xsd[pat_begin..pat_begin + pat_end_rel];

        // Anchor the XSD pattern for Rust regex semantics: XSD
        // patterns match the *whole* lexical value, Rust `regex::is_match`
        // matches anywhere. Without the anchors the Rust regex would
        // accept trailing garbage the XSD rejects.
        let anchored = format!("^(?:{})$", xsd_pattern);
        let xsd_re = regex::Regex::new(&anchored).unwrap_or_else(|e| {
            panic!("XSD paramNameType pattern is not a valid Rust regex: {e} — \
                    pattern text: {xsd_pattern:?}")
        });

        // Extract the C++ raw-string pattern (same raw-string-delimiter
        // shape `cpp_param_name_pattern_matches_rust` pins) and compile
        // it as a Rust regex. Corpus agreement closes the triangle:
        // XSD regex <-> Rust validator <-> C++ regex all verdict the
        // same cases. Option 2 from the Phase B M2 task spec (pure
        // Rust, no cross-build dependency).
        let cpp_hdr =
            include_str!("../../sce/include/parsing/TemplateConstants.h");
        let cpp_begin_anchor = r#"R"pat("#;
        let cpp_end_anchor = r#")pat""#;
        let cpp_begin = cpp_hdr.find(cpp_begin_anchor).unwrap_or_else(|| {
            panic!(
                "TemplateConstants.h must declare PARAM_NAME_PATTERN with \
                 the R\"pat(...)pat\" raw-string form; if the delimiter \
                 changed, update this drift test in the same commit"
            )
        });
        let cpp_content_start = cpp_begin + cpp_begin_anchor.len();
        let cpp_content_end_rel = cpp_hdr[cpp_content_start..]
            .find(cpp_end_anchor)
            .unwrap_or_else(|| {
                panic!(
                    "Unterminated R\"pat(...)pat\" literal in \
                     TemplateConstants.h — closing `)pat\"` not found"
                )
            });
        let cpp_pattern =
            &cpp_hdr[cpp_content_start..cpp_content_start + cpp_content_end_rel];
        let cpp_anchored = format!("^(?:{})$", cpp_pattern);
        let cpp_re = regex::Regex::new(&cpp_anchored).unwrap_or_else(|e| {
            panic!(
                "C++ PARAM_NAME_PATTERN is not a valid Rust regex: {e} — \
                 pattern text: {cpp_pattern:?}"
            )
        });

        // Corpus: positive + negative cases exercising every edge of
        // `[A-Za-z_][A-Za-z0-9_\-]*`. Expand this list whenever the
        // grammar grows a new character class.
        let corpus: &[(&str, bool)] = &[
            ("port", true),
            ("Port", true),
            ("PORT", true),
            ("_port", true),
            ("port_1", true),
            ("port-1", true),
            ("p", true),
            ("a-b-c", true),
            ("", false),
            ("1port", false),
            ("-port", false),
            ("port.dot", false),
            ("port space", false),
            ("port!", false),
            ("ポート", false),
        ];

        for (name, expected) in corpus {
            let rust_verdict = is_valid_param_name(name);
            let xsd_verdict = xsd_re.is_match(name);
            let cpp_verdict = cpp_re.is_match(name);
            assert_eq!(
                rust_verdict, *expected,
                "Rust is_valid_param_name disagrees on {name:?} — \
                 expected {expected}, got {rust_verdict}"
            );
            assert_eq!(
                xsd_verdict, *expected,
                "XSD paramNameType pattern disagrees on {name:?} — \
                 expected {expected}, got {xsd_verdict} (pattern: {xsd_pattern:?})"
            );
            assert_eq!(
                cpp_verdict, *expected,
                "C++ PARAM_NAME_PATTERN disagrees on {name:?} — \
                 expected {expected}, got {cpp_verdict} (pattern: {cpp_pattern:?})"
            );
        }
    }

    /// Drift guard between the Rust AOT expander's
    /// `MAX_TEMPLATE_DEPTH` and the C++ Interpreter runtime's
    /// `sce/include/parsing/TemplateConstants.h::MAX_TEMPLATE_DEPTH`.
    ///
    /// Phase B M1 declares the C++ constant ahead of its M3
    /// consumer so the drift test has a pinned target the moment
    /// the constant exists. Without this test a future change on
    /// either side would silently drift — e.g. a Rust bump to 16
    /// could leave C++ still enforcing 10, so a document accepted
    /// by sce-codegen would be rejected by the Interpreter
    /// (or vice versa) once M3 lands the recursion loop.
    ///
    /// Follows `xsd_param_name_pattern_agrees_with_rust_impl` and
    /// `xinclude::xinclude_depth_matches_runtime` for the
    /// `include_str!` + extract + compare pattern. The regex pins
    /// the declaration shape so a future rewrite (e.g. moving from
    /// `inline constexpr` to a `#define`) surfaces as a clear test
    /// failure with a next-step pointer rather than a confused
    /// no-match.
    #[test]
    fn cpp_template_depth_matches_rust() {
        let hdr = include_str!("../../sce/include/parsing/TemplateConstants.h");
        let re = regex::Regex::new(
            r"inline\s+constexpr\s+int\s+MAX_TEMPLATE_DEPTH\s*=\s*(\d+)\s*;",
        )
        .unwrap();
        let captures = re.captures(hdr).unwrap_or_else(|| {
            panic!(
                "sce/include/parsing/TemplateConstants.h must declare \
                 `inline constexpr int MAX_TEMPLATE_DEPTH = <n>;` — if the \
                 declaration shape changed, update this drift test in \
                 the same commit"
            )
        });
        let cpp_value: u32 = captures[1].parse().unwrap();
        assert_eq!(
            cpp_value, MAX_TEMPLATE_DEPTH,
            "MAX_TEMPLATE_DEPTH drift: Rust = {}, C++ header = {}. \
             Change both sides in the same commit.",
            MAX_TEMPLATE_DEPTH, cpp_value
        );
    }

    /// Drift guard between the C++ Interpreter runtime's
    /// `PARAM_NAME_PATTERN` string_view and the XSD
    /// `paramNameType` pattern text.
    ///
    /// `xsd_param_name_pattern_agrees_with_rust_impl` already pins
    /// the XSD pattern against the Rust regex; this test extends
    /// the agreement to the C++ side by asserting the C++ raw
    /// string literal is byte-identical to the XSD pattern. With
    /// all three on record (XSD, Rust, C++), a change on any one
    /// side surfaces as a red test on the other two before the
    /// drift can reach a released artifact.
    ///
    /// The C++ validator that consumes `PARAM_NAME_PATTERN` lands
    /// in M2. M1 declares the constant so the drift test has a
    /// pinned target the moment the header exists — RFC §3 M1 /
    /// §4 names M2 as the consumer per
    /// `feedback_built_but_unconsumed.md`.
    #[test]
    fn cpp_param_name_pattern_matches_rust() {
        let hdr = include_str!("../../sce/include/parsing/TemplateConstants.h");
        // Extract the raw string literal between R"pat( and )pat".
        // The delimiter `pat` is part of the header's source shape
        // so the test pins both the pattern text AND the raw-string
        // delimiter convention — a future edit that changes the
        // delimiter name surfaces as a clear no-match rather than a
        // byte-equal pass on a matching-but-differently-framed
        // literal.
        let begin_anchor = r#"R"pat("#;
        let end_anchor = r#")pat""#;
        let begin = hdr.find(begin_anchor).unwrap_or_else(|| {
            panic!(
                "TemplateConstants.h must declare PARAM_NAME_PATTERN \
                 with the R\"pat(...)pat\" raw-string form; if the \
                 delimiter changed, update this drift test in the \
                 same commit"
            )
        });
        let content_start = begin + begin_anchor.len();
        let content_end = hdr[content_start..].find(end_anchor).unwrap_or_else(|| {
            panic!(
                "Unterminated R\"pat(...)pat\" literal in \
                 TemplateConstants.h — closing `)pat\"` not found"
            )
        });
        let cpp_pattern = &hdr[content_start..content_start + content_end];

        // Extract the XSD pattern (same logic as
        // xsd_param_name_pattern_agrees_with_rust_impl).
        let xsd = include_str!("../../schemas/sce-forge-ext.xsd");
        let anchor = r#"<xs:simpleType name="paramNameType">"#;
        let start = xsd.find(anchor).expect(
            "XSD must declare <xs:simpleType name=\"paramNameType\">",
        );
        let after = &xsd[start..];
        let pat_start = after
            .find(r#"<xs:pattern value=""#)
            .expect("paramNameType must declare an <xs:pattern value=\"...\"/> child");
        let pat_begin = start + pat_start + r#"<xs:pattern value=""#.len();
        let pat_end_rel = xsd[pat_begin..]
            .find('"')
            .expect("XSD pattern value must be terminated by a double-quote");
        let xsd_pattern = &xsd[pat_begin..pat_begin + pat_end_rel];

        assert_eq!(
            cpp_pattern, xsd_pattern,
            "PARAM_NAME_PATTERN drift: C++ header = {:?}, XSD = {:?}. \
             Change both sides in the same commit.",
            cpp_pattern, xsd_pattern
        );
    }

    /// Pin the 1:1 mapping between Rust `TemplateError` variants,
    /// the `xml/template-*` DiagnosticCodes they emit, and the C++
    /// `SCE::parsing::Template<Variant>` subtypes declared in
    /// `sce/include/parsing/TemplateError.h`. When M4 lands every
    /// named subtype, the three sets must agree — a commit on any
    /// one side that fails to update the other two is the drift
    /// this test is designed to catch.
    ///
    /// Follows the `cpp_template_depth_matches_rust` +
    /// `cpp_param_name_pattern_matches_rust` precedent: read the
    /// authoritative C++ header via `include_str!` at test compile
    /// time, regex-scan the class declarations, assert set equality
    /// against the Rust-side ground truth. Every `class Template* :
    /// public TemplateError` declaration in the header must appear
    /// in the ground-truth table; a new class without a Rust
    /// counterpart (or vice versa) surfaces as a pointed BTreeSet
    /// diff rather than silent drift.
    #[test]
    fn cpp_template_subtypes_match_rust_diagnostic_codes() {
        use std::collections::BTreeSet;

        // Authoritative Rust ground truth — the 8 xml/template-*
        // DiagnosticCodes, paired with the C++ class name that
        // must exist in the header. Table-form so an audit reading
        // this test sees exactly which Rust variant maps to which
        // C++ class without having to walk two files in parallel.
        let rust_to_cpp: &[(&str, &str)] = &[
            ("xml/template-not-found", "TemplateNotFound"),
            ("xml/template-read-error", "TemplateReadError"),
            ("xml/template-malformed", "TemplateMalformed"),
            ("xml/template-missing-attribute", "TemplateMissingAttribute"),
            ("xml/template-missing-param", "TemplateMissingParam"),
            ("xml/template-unknown-param", "TemplateUnknownParam"),
            ("xml/template-cycle", "TemplateCycle"),
            ("xml/template-too-deep", "TemplateTooDeep"),
        ];
        assert_eq!(
            rust_to_cpp.len(),
            8,
            "Expected 8-way mapping; update rust_to_cpp if the \
             DiagnosticCode set grew or shrank"
        );
        let expected_cpp: BTreeSet<&str> =
            rust_to_cpp.iter().map(|(_, cpp)| *cpp).collect();

        // Scan the C++ header for every `class TemplateXxx : public
        // TemplateError` declaration. Matches the precedent in
        // `cpp_template_depth_matches_rust` of pinning the
        // declaration *shape* — a future rewrite that switches to
        // `struct` or inserts attributes surfaces as a clear
        // no-match rather than a false pass.
        let hdr = include_str!("../../sce/include/parsing/TemplateError.h");
        let re = regex::Regex::new(
            r"class\s+(Template\w+)\s*:\s*public\s+TemplateError\b",
        )
        .unwrap();
        let mut found: BTreeSet<String> = BTreeSet::new();
        for captures in re.captures_iter(hdr) {
            found.insert(captures[1].to_string());
        }

        // Sanity: the scan found *something*. If the header shape
        // changed so the regex no longer matches any class, fail
        // with a pointed message instead of silently reporting an
        // empty set equals an empty set.
        assert!(
            !found.is_empty(),
            "sce/include/parsing/TemplateError.h must declare at \
             least one `class Template<Variant> : public \
             TemplateError` — if the declaration shape changed, \
             update this drift test in the same commit"
        );

        let found_refs: BTreeSet<&str> =
            found.iter().map(|s| s.as_str()).collect();
        assert_eq!(
            found_refs, expected_cpp,
            "TemplateError subtype drift: C++ header = {:?}, \
             expected (from DiagnosticCode mapping) = {:?}. Change \
             both sides in the same commit — see RFC §1 Q4 \
             (claudedocs/rfc-sce-template-phase-b.md) for the \
             invariant.",
            found_refs, expected_cpp
        );

        // Cross-check: every Rust DiagnosticCode name MUST be
        // spelled as the `serde(rename = "...")` literal in
        // `sce-build/src/forge/diagnostic.rs`. If a future edit
        // renames a code (e.g. `xml/template-not-found` →
        // `xml/template-missing-file`), this assertion fails with
        // a pointed diff rather than the drift travelling silently
        // through JSON wire contracts.
        let diag = include_str!("forge/diagnostic.rs");
        for (rust_code, cpp_name) in rust_to_cpp {
            let needle = format!("\"{}\"", rust_code);
            assert!(
                diag.contains(&needle),
                "DiagnosticCode `{}` (paired with C++ `{}`) is not \
                 declared as a `serde(rename)` literal in \
                 sce-build/src/forge/diagnostic.rs. Keep the wire \
                 name, the Rust variant, and the C++ subtype in \
                 sync — see RFC §1 Q4.",
                rust_code, cpp_name
            );
        }
    }

    /// W1 (`claudedocs/rfc-sce-diagnostic-wire-unification.md`) makes
    /// each C++ `Template<Variant>` subtype override
    /// `Diagnostic::code()` to return its `xml/template-*` wire string.
    /// The sister test above pins **subtype names** between Rust and
    /// C++; this one pins the **wire-string return literal** inside
    /// each subtype's `code()` body so a future rename on either side
    /// (`xml/template-cycle` → `xml/template-loop`) cannot drift the
    /// JSON wire contract silently.
    ///
    /// The bite: a hand-edit that changes `return "xml/template-cycle"`
    /// to `return "xml/template-cycleXXX"` in `TemplateError.h` reds
    /// here with a pointed `does not contain` diff naming the exact
    /// class and exact missing literal.
    #[test]
    fn cpp_template_subtype_code_returns_rust_wire_string() {
        let rust_to_cpp: &[(&str, &str)] = &[
            ("xml/template-not-found", "TemplateNotFound"),
            ("xml/template-read-error", "TemplateReadError"),
            ("xml/template-malformed", "TemplateMalformed"),
            ("xml/template-missing-attribute", "TemplateMissingAttribute"),
            ("xml/template-missing-param", "TemplateMissingParam"),
            ("xml/template-unknown-param", "TemplateUnknownParam"),
            ("xml/template-cycle", "TemplateCycle"),
            ("xml/template-too-deep", "TemplateTooDeep"),
        ];
        assert_eq!(rust_to_cpp.len(), 8);

        let hdr = include_str!("../../sce/include/parsing/TemplateError.h");

        for (rust_code, cpp_class) in rust_to_cpp {
            // Locate the class block. The header's shape (one class per
            // subtype, each terminated with `};`) keeps a forward
            // `find("};")` accurate enough for a drift guard; if a
            // future rewrite nests braces inside a subtype we update
            // this scanner in the same commit.
            let class_marker =
                format!("class {} : public TemplateError", cpp_class);
            let class_start = hdr.find(&class_marker).unwrap_or_else(|| {
                panic!(
                    "class `{}` not found in sce/include/parsing/\
                     TemplateError.h — drift in subtype naming, see \
                     `cpp_template_subtypes_match_rust_diagnostic_codes`",
                    cpp_class
                )
            });
            let body_start =
                hdr[class_start..].find('{').unwrap() + class_start + 1;
            let body_end_rel = hdr[body_start..].find("};").unwrap();
            let body = &hdr[body_start..body_start + body_end_rel];

            let needle = format!("return \"{}\";", rust_code);
            assert!(
                body.contains(&needle),
                "Class `{}` body does not contain `{}` — the C++ \
                 subtype's `code()` override must return the Rust \
                 DiagnosticCode wire literal exactly so the JSON wire \
                 emitted by `to_json()` agrees with `--error-format=\
                 json`. RFC §W1 / SCE_ERROR_CONTRACT.md §3.",
                cpp_class, needle
            );
        }

        // Sanity-count: the header should declare exactly 8 `code()`
        // overrides, one per subtype. A 9th appearing without an
        // expansion of `rust_to_cpp` reds with a count diff rather
        // than silently passing.
        let override_count = hdr
            .matches("std::string_view code() const noexcept override")
            .count();
        // 8 subtype overrides + 1 base-class declaration = 9 occurrences.
        assert_eq!(
            override_count, 9,
            "expected 9 `code() const noexcept override` lines in \
             TemplateError.h (1 pure-virtual on TemplateError + 8 \
             subtype overrides); found {}",
            override_count
        );
    }
}
