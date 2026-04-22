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
/// `content`. It is added to the cycle-detection stack and used
/// for diagnostic rendering; callers that have no filesystem
/// identity (in-memory documents) should pass a stable label.
/// `base_dir` is the directory `<sce:use template="relative/...">`
/// is resolved against — typically `Path::new(self_path).parent()`.
///
/// Returns the expanded document as an owned `String` suitable for
/// handing to `roxmltree::Document::parse`. Short-circuits with
/// `Ok(content.to_string())` when the input contains no `sce:use`
/// substring, so documents that do not use templates pay only a
/// single substring search on the critical path.
pub fn expand(
    content: &str,
    self_path: &str,
    base_dir: Option<&Path>,
) -> Result<String, (TemplateError, TemplateLocation)> {
    if !content.contains("sce:use") {
        return Ok(content.to_string());
    }
    let mut stack: Vec<PathBuf> = Vec::new();
    if let Ok(abs) = std::fs::canonicalize(self_path) {
        stack.push(abs);
    } else {
        stack.push(PathBuf::from(self_path));
    }
    expand_impl(content, base_dir, 0, &mut stack)
}

fn expand_impl(
    content: &str,
    base_dir: Option<&Path>,
    depth: u32,
    stack: &mut Vec<PathBuf>,
) -> Result<String, (TemplateError, TemplateLocation)> {
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
        return Ok(content.to_string());
    }

    // Walk original byte stream, replacing each `<sce:use>` range
    // with the rendered template body. Document-order processing
    // lets a single cursor serve the splice.
    let mut out = String::with_capacity(content.len());
    let mut cursor = 0usize;

    for node in uses {
        let range = node.range();
        out.push_str(&content[cursor..range.start]);

        let loc = doc_loc(&doc, range.start);

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

        let params_bound = collect_use_bindings(&node);
        // Substitute `{$name}` tokens inside the template body only —
        // the `<sce:template>` wrapper and `<sce:param>` declarations
        // are preserved verbatim so (a) nested `<sce:use>` inside the
        // body can be recursively expanded with the wrapper's
        // namespace context intact, and (b) `<sce:param default="...">`
        // literals are never themselves substituted (RFC §6.1).
        let substituted = substitute_into_template(&template_raw, template_attr, &params_bound)
            .map_err(|e| (e, loc))?;

        // Recurse on the substituted template. Cycle detection and
        // depth bound guard against pathological chains.
        stack.push(canon);
        let nested_base = resolved.parent().map(|p| p.to_path_buf());
        let expanded_template =
            expand_impl(&substituted, nested_base.as_deref(), depth + 1, stack)
                .map_err(|(err, _)| (remap_nested(err, template_attr), loc))?;
        stack.pop();

        // Extract body (children of `<sce:template>` minus
        // `<sce:param>`) from the fully-expanded template and splice
        // it into the outer document in place of the `<sce:use>` node.
        let body = extract_template_body(&expanded_template, template_attr)
            .map_err(|e| (e, loc))?;
        out.push_str(&body);

        cursor = range.end;
    }

    out.push_str(&content[cursor..]);
    Ok(out)
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
/// `{$name}` substitution applied. The `<sce:template>` wrapper and
/// the `<sce:param>` declarations are emitted verbatim so (a) the
/// return value is a self-contained XML document that can be fed
/// back through [`expand_impl`] for nested `<sce:use>` expansion
/// without losing the `sce:` namespace binding declared on the
/// template root, and (b) `<sce:param default="...">` literals are
/// not themselves substituted (Q1 literal-only defaults).
fn substitute_into_template(
    template_raw: &str,
    template_href: &str,
    bound: &HashMap<String, String>,
) -> Result<String, TemplateError> {
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

    // Reassemble: original prefix up to body start, substituted
    // body, original suffix after body end. If the template has no
    // body (only params or only whitespace), pass the raw through.
    match (body_start, body_end) {
        (Some(bs), Some(be)) => {
            let before = &template_raw[..bs];
            let body = &template_raw[bs..be];
            let after = &template_raw[be..];
            let mut out = String::with_capacity(template_raw.len() + 32);
            out.push_str(before);
            out.push_str(&apply_substitution(body, &params));
            out.push_str(after);
            Ok(out)
        }
        _ => Ok(template_raw.to_string()),
    }
}

/// Extract the body of a post-substitution, post-recursion template:
/// the concatenated byte ranges of every non-`<sce:param>` child of
/// the `<sce:template>` root. Fed into the outer document in place
/// of the `<sce:use>` node.
fn extract_template_body(
    expanded_template: &str,
    template_href: &str,
) -> Result<String, TemplateError> {
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
    let mut body = String::new();
    for child in root.children() {
        if child.is_element() && is_sce(&child, "param") {
            continue;
        }
        let range = child.range();
        body.push_str(&expanded_template[range.start..range.end]);
    }
    Ok(body)
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

/// Single-pass lexical substitution of `{$name}` tokens. Atomic:
/// replacements do not cascade (a parameter value that itself
/// contains `{$other}` is emitted verbatim, matching RFC §6.1
/// literal-only default semantics).
///
/// Undeclared `{$name}` tokens that do not match any declared
/// parameter are emitted verbatim; diagnostic for undeclared refs
/// is the author's responsibility (the expander cannot distinguish
/// authorial `{$literal}` text from an undeclared param reference
/// without a wire-level convention).
fn apply_substitution(body: &str, params: &HashMap<String, String>) -> String {
    let mut out = String::with_capacity(body.len());
    let mut pos = 0;
    while let Some(start_rel) = body[pos..].find("{$") {
        let start = pos + start_rel;
        out.push_str(&body[pos..start]);
        let after = start + 2;
        if let Some(end_rel) = body[after..].find('}') {
            let name = &body[after..after + end_rel];
            if is_valid_param_name(name) {
                if let Some(value) = params.get(name) {
                    out.push_str(value);
                    pos = after + end_rel + 1;
                    continue;
                }
            }
        }
        // Not a valid `{$name}` token — emit `{$` literally and
        // advance past it. The next loop iteration picks up from
        // there without re-matching the `{$` we just emitted.
        out.push_str("{$");
        pos = after;
    }
    out.push_str(&body[pos..]);
    out
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
        let out = expand(src, "inline", None).expect("no sce:use");
        assert_eq!(out, src);
    }

    #[test]
    fn passthrough_when_substring_but_no_element() {
        // "sce:use" appears as a word in text/attribute but no
        // element exists — must parse to tell, and pass unchanged.
        let src = "<root description=\"how to sce:use this\"/>";
        let out = expand(src, "inline", None).expect("no sce:use elements");
        assert_eq!(out, src);
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

        let out = expand(&main_src, main_path.to_str().unwrap(), Some(tmp.path()))
            .expect("expansion succeeds");
        assert!(out.contains("_event.port == 80"));
        assert!(!out.contains("<sce:use"));
        assert!(!out.contains("{$port}"));
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
        let out = expand(&main_src, main_path.to_str().unwrap(), Some(tmp.path())).unwrap();
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
        let out = expand(&main_src, main_path.to_str().unwrap(), Some(tmp.path())).unwrap();
        assert!(out.contains(r#"n="1""#));
        assert!(out.contains(r#"n="2""#));
        assert_eq!(out.matches("<marker").count(), 2);
    }

    #[test]
    fn missing_template_attribute_is_own_variant() {
        let tmp = TempDir::new().unwrap();
        let main_src = r#"<root xmlns:sce="http://sce.dev/ext"><sce:use/></root>"#;
        let main_path = write(tmp.path(), "main.xml", main_src);
        let err = expand(main_src, main_path.to_str().unwrap(), Some(tmp.path()))
            .unwrap_err();
        assert!(matches!(err.0, TemplateError::MissingTemplateAttribute));
    }

    #[test]
    fn empty_template_attribute_is_missing_template_attribute() {
        let tmp = TempDir::new().unwrap();
        let main_src = r#"<root xmlns:sce="http://sce.dev/ext"><sce:use template=""/></root>"#;
        let main_path = write(tmp.path(), "main.xml", main_src);
        let err = expand(main_src, main_path.to_str().unwrap(), Some(tmp.path()))
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
        let err = expand(&main_src, main_path.to_str().unwrap(), Some(tmp.path()))
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
        let err = expand(&main_src, main_path.to_str().unwrap(), Some(tmp.path()))
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
        let err = expand(main_src, main_path.to_str().unwrap(), Some(tmp.path()))
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
        let err = expand(main_src, main_path.to_str().unwrap(), Some(tmp.path()))
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
        let err = expand(main_src, main_path.to_str().unwrap(), Some(tmp.path()))
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
        let err = expand(main_src, main_path.to_str().unwrap(), Some(tmp.path()))
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
        let out = expand(main_src, main_path.to_str().unwrap(), Some(tmp.path())).unwrap();
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
        let out = expand(main_src, main_path.to_str().unwrap(), Some(tmp.path())).unwrap();
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
        let out = expand(main_src, main_path.to_str().unwrap(), Some(tmp.path())).unwrap();
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
        let err = expand(main_src, main_path.to_str().unwrap(), Some(tmp.path()))
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
}
