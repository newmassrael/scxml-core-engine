// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C XInclude preprocessing for SCXML source documents.
//
// Pairs with `PugiXMLDocument::processXIncludeRecursive`
// (sce/src/parsing/PugiXMLParser.cpp:239) so the AOT code-generation
// pipeline consumes the same post-expansion document the runtime
// parser consumes. Without this module, a document containing
// `<xi:include href="...">` was expanded at runtime by the interpreter
// but passed through unchanged to the AOT generator — producing a
// state machine that diverges from the runtime-parsed form.
//
// # Supported subset
//
// The C++ runtime implementation is a minimal recursive processor;
// this module reproduces the same minimum:
//
//   * `<xi:include href="...">` or legacy `<include href="...">`
//     (both accepted, matching pugixml's local-name matching).
//   * `href` is resolved absolute-first, then relative to the
//     including file's directory, then relative to the current
//     working directory (mirrors
//     `PugiXMLDocument::resolveFilePath`).
//   * Recursion is bounded by `MAX_XINCLUDE_DEPTH` and cycles are
//     detected via a path stack.
//   * The children of the included document's root element are
//     spliced into the parent in place of the `<xi:include>` node,
//     matching the C++ behaviour at PugiXMLParser.cpp:296-301. The
//     included document itself must be well-formed (single root).
//
// Unsupported XInclude features (pugixml does not implement these
// either and they are rejected explicitly rather than silently
// ignored so divergence cannot surface at runtime):
//
//   * `<xi:fallback>` elements,
//   * `parse="text"` mode,
//   * XPointer expressions (`xpointer=`).
//
// # AOT vs runtime error model
//
// The runtime parser warns and continues when an individual
// `<xi:include>` fails (missing `href`, unresolvable path, malformed
// target). The AOT pipeline hard-errors on every such case: a
// state machine generated from a document whose includes silently
// dropped is strictly broken, and build-time refusal is preferable
// to runtime divergence. The message is tagged with the offending
// node's row/column so downstream diagnostics can surface a
// machine-readable location.

use crate::position_map::{Origin, PositionMap};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Maximum nesting depth for recursive `<xi:include>` expansion.
///
/// Mirrors `PugiXMLDocument::MAX_XINCLUDE_DEPTH` at
/// `sce/include/parsing/PugiXMLParser.h:69`. The value is duplicated
/// across the language boundary (C++ / Rust) because there is no
/// shared header the codegen crate can pull. `xinclude_depth_matches_runtime`
/// in the test module below pins the constant so a future runtime
/// change is caught by a red test rather than silent divergence.
pub const MAX_XINCLUDE_DEPTH: u32 = 10;

/// W3C XInclude 1.0 namespace URI. Exposed for diagnostic messages
/// and for callers that want to assert the namespace explicitly;
/// the expansion itself matches by local name (`include`) to
/// preserve parity with the pugixml implementation, which is
/// lenient about the namespace declaration.
pub const XINCLUDE_NS: &str = "http://www.w3.org/2001/XInclude";

/// Errors raised by the XInclude preprocessor.
///
/// Every variant carries enough context for an operator (or a
/// downstream agent consuming the NDJSON diagnostic stream) to
/// either fix the source document or fix the file layout without
/// reading the crate source. The offending `<xi:include>` node's
/// row and column are attached by [`expand`] via [`Located`], so
/// the variants themselves stay orthogonal to position.
#[derive(Debug, thiserror::Error)]
pub enum XIncludeError {
    /// `<xi:include>` element has no `href` attribute, or its value
    /// is the empty string. The pugixml runtime logs a warning and
    /// skips the element; the AOT pipeline rejects the document so
    /// the build fails where the problem is introduced.
    #[error("<xi:include> missing or empty `href` attribute")]
    MissingHref,

    /// `href` did not resolve against any of the search paths
    /// (absolute, including-file directory, current working
    /// directory). `searched` lists the paths tried so the operator
    /// can pick the right one without guessing.
    #[error("<xi:include href=\"{href}\">: file not found (searched: {searched})")]
    NotFound { href: String, searched: String },

    /// Resolved file exists but could not be read — permission
    /// denied, I/O failure, etc.
    #[error("<xi:include href=\"{href}\">: cannot read: {source}")]
    ReadError {
        href: String,
        #[source]
        source: std::io::Error,
    },

    /// Resolved file was read but is not well-formed XML. `detail`
    /// carries roxmltree's diagnostic so the operator can find the
    /// offending position inside the included file. Position inside
    /// the including document points at the `<xi:include>` node.
    #[error("<xi:include href=\"{href}\">: included file is malformed: {detail}")]
    Malformed { href: String, detail: String },

    /// A cycle has been detected in the inclusion graph: including
    /// the referenced file would revisit a file already on the
    /// current expansion stack. `chain` is the rendered stack (root
    /// → leaf, separated by " → ") for operator diagnosis.
    #[error("<xi:include href=\"{href}\">: cycle detected ({chain})")]
    Cycle { href: String, chain: String },

    /// Recursion exceeded [`MAX_XINCLUDE_DEPTH`]. This catches
    /// pathological (but acyclic) inclusion chains where each
    /// file pulls in another without looping back.
    #[error("<xi:include> nesting exceeds depth limit of {limit}")]
    TooDeep { limit: u32 },

    /// The document uses an XInclude feature that the pugixml
    /// runtime does not implement. Accepting these at codegen
    /// time would produce state machines that differ from
    /// runtime parse — we reject them at the earliest stage.
    #[error("<xi:include href=\"{href}\">: unsupported feature: {feature}")]
    Unsupported { href: String, feature: &'static str },
}

/// Location of an `<xi:include>` inside the source string —
/// 1-based row, 1-based column, matching the
/// [`roxmltree::TextPos`] convention used everywhere else in the
/// forge error pipeline.
#[derive(Debug, Clone, Copy)]
pub struct XIncludeLocation {
    pub row: u32,
    pub col: u32,
}

/// Expand every `<xi:include>` / `<include>` element in `content`.
///
/// `self_path` is the filesystem path of the document supplying
/// `content`. It is added to the cycle-detection stack, used for
/// diagnostic rendering, and used as the `Origin::File` path for
/// outer-content regions in the returned [`PositionMap`].
/// Callers that have no filesystem identity (in-memory documents)
/// should pass a stable label such as the `DocumentLabel`
/// diagnostic string. `base_dir` is the directory
/// `<xi:include href="relative/...">` is resolved against —
/// typically `Path::new(self_path).parent()`.
///
/// Returns the expanded document as an owned `String` suitable
/// for handing to `roxmltree::Document::parse`, plus a
/// [`PositionMap`] mapping every expanded byte back to its source
/// (the outer file, or an included fragment). Documents without
/// any include hit a short-circuit that returns the identity map
/// — no allocation beyond the single `content.to_string()`.
///
/// # Error model
///
/// Errors fire *during* expansion, so their row/col are positions
/// in the pre-expansion `content` — already in source
/// coordinates, never in expanded coordinates. The position map
/// is not used to translate these; callers wrap them directly in
/// `Located` with the `<xi:include>` node's position as reported
/// here. Only *post-expansion* diagnostics (XSD, semantic
/// validation on the expanded tree, expression transpile) need
/// the map.
pub fn expand(
    content: &str,
    self_path: &str,
    base_dir: Option<&Path>,
) -> Result<(String, PositionMap, Vec<PathBuf>), (XIncludeError, XIncludeLocation)> {
    let self_file = PathBuf::from(self_path);
    if !content.contains("include") {
        return Ok((
            content.to_string(),
            PositionMap::identity(self_file, content),
            Vec::new(),
        ));
    }
    let mut stack: Vec<PathBuf> = Vec::new();
    if let Ok(abs) = std::fs::canonicalize(self_path) {
        stack.push(abs);
    } else {
        stack.push(self_file.clone());
    }
    // Every `<xi:include>` we successfully open feeds this collector,
    // which the parse-boundary call site (`expand_preprocessors`)
    // surfaces to the depfile sink. Without it, downstream build
    // systems treat fragment edits as silent no-ops because the only
    // prerequisites recorded in `--write-deps` output are the SCE
    // jinja2 templates and the host SCXML — fragments are invisible.
    let mut deps: Vec<PathBuf> = Vec::new();
    let (out, map) = expand_impl(content, &self_file, base_dir, 0, &mut stack, &mut deps)?;
    Ok((out, map, deps))
}

fn expand_impl(
    content: &str,
    content_file: &Path,
    base_dir: Option<&Path>,
    depth: u32,
    stack: &mut Vec<PathBuf>,
    deps: &mut Vec<PathBuf>,
) -> Result<(String, PositionMap), (XIncludeError, XIncludeLocation)> {
    if depth >= MAX_XINCLUDE_DEPTH {
        return Err((
            XIncludeError::TooDeep {
                limit: MAX_XINCLUDE_DEPTH,
            },
            XIncludeLocation { row: 1, col: 1 },
        ));
    }

    let doc = roxmltree::Document::parse(content).map_err(|e| {
        let pos = e.pos();
        (
            XIncludeError::Malformed {
                href: String::new(),
                detail: e.to_string(),
            },
            XIncludeLocation {
                row: pos.row,
                col: pos.col,
            },
        )
    })?;

    let root = doc.root_element();
    let includes: Vec<roxmltree::Node> = collect_includes(&root);
    if includes.is_empty() {
        // No includes in this content — the output is a 1:1 copy
        // of `content` from `content_file`, so identity is exact.
        return Ok((
            content.to_string(),
            PositionMap::identity(content_file.to_path_buf(), content),
        ));
    }

    // Emit the output by splicing: walk the original byte stream
    // and replace each `<xi:include>` range with the rendered
    // children of the included document's root. Processing in
    // document order (left to right) lets a single cursor serve
    // the splice and keeps the offset math trivial. The position
    // map is built in lock-step with the output, so every emitted
    // byte lands in exactly one entry.
    let mut out = String::with_capacity(content.len());
    let mut cursor = 0usize;
    let mut map = PositionMap::default();
    map.register_file(content_file.to_path_buf(), content);

    for node in includes {
        let range = node.range();

        // Copy the unchanged prefix up to this include node and
        // tag it as an outer-file region.
        if cursor < range.start {
            let out_start = out.len();
            out.push_str(&content[cursor..range.start]);
            map.push_entry(
                out_start,
                out.len(),
                Origin::File {
                    path: content_file.to_path_buf(),
                    source_offset: cursor,
                },
            );
        }

        let loc = doc_loc(&doc, range.start);

        reject_unsupported(&node, &loc)?;

        let href = node
            .attribute("href")
            .filter(|v| !v.is_empty())
            .ok_or((XIncludeError::MissingHref, loc))?;

        let resolved = resolve_href(href, base_dir).map_err(|e| (e, loc))?;

        // Cycle detection: a canonicalised path that is already on
        // the active expansion stack would re-enter its own
        // expansion. Canonicalisation also de-duplicates
        // `./foo.xml`, `foo.xml`, and `../dir/foo.xml` forms so
        // aliased includes of the same target are still caught.
        let canon = std::fs::canonicalize(&resolved).unwrap_or(resolved.clone());
        if stack.contains(&canon) {
            let chain = render_chain(stack, &canon);
            return Err((
                XIncludeError::Cycle {
                    href: href.to_string(),
                    chain,
                },
                loc,
            ));
        }

        let raw = std::fs::read_to_string(&resolved).map_err(|e| {
            (
                XIncludeError::ReadError {
                    href: href.to_string(),
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

        stack.push(canon);
        let nested_base = resolved.parent().map(|p| p.to_path_buf());
        let (expanded, nested_map) = expand_impl(
            &raw,
            &resolved,
            nested_base.as_deref(),
            depth + 1,
            stack,
            deps,
        )
        .map_err(|(err, nested_loc)| {
            // The nested error's row/col references the
            // included file, not the outer one. For the AOT
            // diagnostic we keep the outer-file location
            // (the `<xi:include>` node) because that is
            // where the operator will apply the fix — the
            // inner location is still available via the
            // nested error's own rendering inside `detail`.
            let _ = nested_loc;
            (remap_nested(err, href), loc)
        })?;
        stack.pop();

        let (rendered, rendered_range) =
            render_root_children(&expanded, href).map_err(|e| (e, loc))?;
        let splice_start = out.len();
        out.push_str(&rendered);
        // Compose the nested map for exactly the bytes we just
        // spliced in: clip nested entries to the rendered range,
        // then shift them so they land at `splice_start` in the
        // outer map.
        map.append_mapped_substring(
            &nested_map,
            rendered_range.start,
            rendered_range.end,
            splice_start,
        );

        cursor = range.end;
    }

    // Tail: copy the unchanged suffix after the last include.
    if cursor < content.len() {
        let out_start = out.len();
        out.push_str(&content[cursor..]);
        map.push_entry(
            out_start,
            out.len(),
            Origin::File {
                path: content_file.to_path_buf(),
                source_offset: cursor,
            },
        );
    }

    Ok((out, map))
}

/// Collect every `<xi:include>` / `<include>` element in the
/// document in document order. Nested includes (an include that
/// itself sits inside another include's target file) are handled
/// by the recursive [`expand_impl`] call — this walker only needs
/// to see the top-level shape of the current document.
fn collect_includes<'a, 'input>(
    root: &roxmltree::Node<'a, 'input>,
) -> Vec<roxmltree::Node<'a, 'input>> {
    let mut out = Vec::new();
    collect_includes_into(root, &mut out);
    out
}

fn collect_includes_into<'a, 'input>(
    node: &roxmltree::Node<'a, 'input>,
    out: &mut Vec<roxmltree::Node<'a, 'input>>,
) {
    for child in node.children() {
        if !child.is_element() {
            continue;
        }
        let name = child.tag_name();
        // Match either the proper XInclude namespace or the bare
        // local name `include`. The C++ runtime is lenient on the
        // namespace declaration (PugiXMLParser.cpp:249), so the
        // AOT side accepts the same inputs.
        let is_include = name.name() == "include"
            && (name.namespace() == Some(XINCLUDE_NS) || name.namespace().is_none());
        if is_include {
            out.push(child);
        } else {
            collect_includes_into(&child, out);
        }
    }
}

/// Reject XInclude features that the runtime does not implement.
fn reject_unsupported(
    node: &roxmltree::Node,
    loc: &XIncludeLocation,
) -> Result<(), (XIncludeError, XIncludeLocation)> {
    if let Some(mode) = node.attribute("parse") {
        if mode != "xml" {
            return Err((
                XIncludeError::Unsupported {
                    href: node.attribute("href").unwrap_or("").to_string(),
                    feature: "parse=\"text\" (only parse=\"xml\" is supported)",
                },
                *loc,
            ));
        }
    }
    if node.attribute("xpointer").is_some() {
        return Err((
            XIncludeError::Unsupported {
                href: node.attribute("href").unwrap_or("").to_string(),
                feature: "xpointer selection is not implemented",
            },
            *loc,
        ));
    }
    for child in node.children() {
        if child.is_element()
            && child.tag_name().name() == "fallback"
            && (child.tag_name().namespace() == Some(XINCLUDE_NS)
                || child.tag_name().namespace().is_none())
        {
            return Err((
                XIncludeError::Unsupported {
                    href: node.attribute("href").unwrap_or("").to_string(),
                    feature: "<xi:fallback> alternative content is not implemented",
                },
                *loc,
            ));
        }
    }
    Ok(())
}

/// Resolve `href` to an absolute path using the same precedence
/// as `PugiXMLDocument::resolveFilePath`: absolute → base
/// directory → current working directory. Returns `NotFound` with
/// the search trail on failure so the operator can see which
/// paths were tried.
fn resolve_href(href: &str, base_dir: Option<&Path>) -> Result<PathBuf, XIncludeError> {
    let href_path = Path::new(href);
    let mut tried: Vec<String> = Vec::new();

    if href_path.is_absolute() {
        if href_path.exists() {
            return Ok(href_path.to_path_buf());
        }
        tried.push(href_path.display().to_string());
    } else {
        if let Some(base) = base_dir {
            let candidate = base.join(href_path);
            if candidate.exists() {
                return Ok(candidate);
            }
            tried.push(candidate.display().to_string());
        }
        if href_path.exists() {
            return Ok(href_path.to_path_buf());
        }
        tried.push(href_path.display().to_string());
    }

    Err(XIncludeError::NotFound {
        href: href.to_string(),
        searched: tried.join(", "),
    })
}

/// Render the children of the included document's root element
/// as a textual slice of `expanded`. Mirrors the pugixml
/// behaviour of copying `includedRoot.children()` into the
/// parent: the root element itself is dropped (the document is a
/// fragment wrapper), and every child — element, text, comment —
/// is preserved so SCXML mixed content survives the splice.
///
/// Returns both the rendered string and the `[start, end)` byte
/// range it occupies inside `expanded`. The caller needs the
/// range to compose the nested expansion's `PositionMap` with
/// its own — the children slice is what actually lands in the
/// outer output, and the range tells the map which sub-region to
/// inherit.
fn render_root_children(
    expanded: &str,
    href: &str,
) -> Result<(String, std::ops::Range<usize>), XIncludeError> {
    let doc = roxmltree::Document::parse(expanded).map_err(|e| XIncludeError::Malformed {
        href: href.to_string(),
        detail: e.to_string(),
    })?;
    let root = doc.root_element();
    let children: Vec<_> = root.children().collect();
    if children.is_empty() {
        return Ok((String::new(), 0..0));
    }
    let start = children.first().unwrap().range().start;
    let end = children.last().unwrap().range().end;
    Ok((expanded[start..end].to_string(), start..end))
}

/// Convert a byte offset into a 1-based (row, col) pair using
/// roxmltree's TextPos. Used to tag the offending `<xi:include>`
/// node with a position the diagnostic pipeline can surface.
fn doc_loc(doc: &roxmltree::Document, offset: usize) -> XIncludeLocation {
    let pos = doc.text_pos_at(offset);
    XIncludeLocation {
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

/// Rewrite a nested error's href so the outer diagnostic names
/// the `<xi:include>` the operator sees, not the transitive
/// relationship inside the included chain.
fn remap_nested(err: XIncludeError, outer_href: &str) -> XIncludeError {
    match err {
        XIncludeError::MissingHref
        | XIncludeError::TooDeep { .. }
        | XIncludeError::Cycle { .. } => err,
        XIncludeError::NotFound { searched, .. } => XIncludeError::NotFound {
            href: outer_href.to_string(),
            searched,
        },
        XIncludeError::ReadError { source, .. } => XIncludeError::ReadError {
            href: outer_href.to_string(),
            source,
        },
        XIncludeError::Malformed { detail, .. } => XIncludeError::Malformed {
            href: outer_href.to_string(),
            detail,
        },
        XIncludeError::Unsupported { feature, .. } => XIncludeError::Unsupported {
            href: outer_href.to_string(),
            feature,
        },
    }
}

/// Helper used by the [`HashSet`] cycle-tracking variant in tests
/// that care about visit semantics separate from the live stack.
#[allow(dead_code)]
fn as_set(stack: &[PathBuf]) -> HashSet<PathBuf> {
    stack.iter().cloned().collect()
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
    fn xinclude_depth_matches_runtime() {
        // Guards against silent drift of the Rust-side constant
        // value. Two-way agreement against the C++ header is
        // enforced separately by `cpp_xinclude_expander_matches_rust_shape`
        // below (Phase X B3) — that test reads
        // `sce/include/parsing/XIncludeExpander.h` via `include_str!`
        // and asserts the literal matches.
        assert_eq!(MAX_XINCLUDE_DEPTH, 10);
    }

    #[test]
    fn passthrough_when_no_include_substring() {
        let src = "<root><state id=\"s1\"/></root>";
        let (out, map, _deps) = expand(src, "inline", None).expect("no includes");
        assert_eq!(out, src);
        assert!(map.is_identity());
    }

    #[test]
    fn passthrough_when_include_substring_but_no_element() {
        // "include" appears as a word in an attribute or text but
        // no actual `<xi:include>` element exists — must parse
        // the document to tell, and must pass it through
        // unchanged.
        let src = "<root description=\"please include docs\"/>";
        let (out, map, _deps) = expand(src, "inline", None).expect("no include elements");
        assert_eq!(out, src);
        assert!(map.is_identity());
    }

    #[test]
    fn expands_single_include() {
        let tmp = TempDir::new().unwrap();
        let frag = write(
            tmp.path(),
            "frag.xml",
            r#"<fragment><state id="s1"/><state id="s2"/></fragment>"#,
        );
        let main_src = format!(
            r#"<root><xi:include xmlns:xi="http://www.w3.org/2001/XInclude" href="{}"/></root>"#,
            frag.file_name().unwrap().to_str().unwrap()
        );
        let main_path = write(tmp.path(), "main.xml", &main_src);

        let (out, map, _deps) = expand(&main_src, main_path.to_str().unwrap(), Some(tmp.path()))
            .expect("expansion succeeds");
        // The children of `<fragment>` must be spliced in place
        // of the `<xi:include>` element, dropping the wrapper.
        assert!(out.contains("<state id=\"s1\"/>"));
        assert!(out.contains("<state id=\"s2\"/>"));
        assert!(!out.contains("<xi:include"));
        assert!(!out.contains("<fragment"));
        // Map must reflect that expansion happened — identity
        // would mean the single entry covered the whole output
        // from main.xml, which is wrong for a document with a
        // splice.
        assert!(!map.is_identity());
    }

    #[test]
    fn accepts_bare_include_without_namespace() {
        // Pugixml matches by local name; the AOT side must match
        // the same inputs or documents that work at runtime will
        // fail at build time.
        let tmp = TempDir::new().unwrap();
        let frag = write(tmp.path(), "frag.xml", r#"<f><x/></f>"#);
        let main_src = format!(
            r#"<root><include href="{}"/></root>"#,
            frag.file_name().unwrap().to_str().unwrap()
        );
        let main_path = write(tmp.path(), "main.xml", &main_src);

        let (out, _map, _deps) =
            expand(&main_src, main_path.to_str().unwrap(), Some(tmp.path())).unwrap();
        assert!(out.contains("<x/>"));
        assert!(!out.contains("<include"));
    }

    #[test]
    fn missing_href_is_hard_error() {
        let src = r#"<root><xi:include xmlns:xi="http://www.w3.org/2001/XInclude"/></root>"#;
        let err = expand(src, "inline", None).unwrap_err();
        assert!(matches!(err.0, XIncludeError::MissingHref));
    }

    #[test]
    fn empty_href_is_hard_error() {
        let src =
            r#"<root><xi:include xmlns:xi="http://www.w3.org/2001/XInclude" href=""/></root>"#;
        let err = expand(src, "inline", None).unwrap_err();
        assert!(matches!(err.0, XIncludeError::MissingHref));
    }

    #[test]
    fn nonexistent_href_is_hard_error() {
        let tmp = TempDir::new().unwrap();
        let main_src = r#"<root><xi:include xmlns:xi="http://www.w3.org/2001/XInclude" href="missing.xml"/></root>"#;
        let main_path = write(tmp.path(), "main.xml", main_src);
        let err = expand(main_src, main_path.to_str().unwrap(), Some(tmp.path())).unwrap_err();
        match err.0 {
            XIncludeError::NotFound { href, searched } => {
                assert_eq!(href, "missing.xml");
                assert!(searched.contains("missing.xml"));
            }
            other => panic!("expected NotFound, got {:?}", other),
        }
    }

    #[test]
    fn cycle_is_detected() {
        let tmp = TempDir::new().unwrap();
        // a.xml → b.xml → a.xml
        let a_path = tmp.path().join("a.xml");
        let b_path = tmp.path().join("b.xml");
        fs::write(
            &a_path,
            r#"<wrap><xi:include xmlns:xi="http://www.w3.org/2001/XInclude" href="b.xml"/></wrap>"#,
        )
        .unwrap();
        fs::write(
            &b_path,
            r#"<wrap><xi:include xmlns:xi="http://www.w3.org/2001/XInclude" href="a.xml"/></wrap>"#,
        )
        .unwrap();
        let main_src = fs::read_to_string(&a_path).unwrap();
        let err = expand(&main_src, a_path.to_str().unwrap(), Some(tmp.path())).unwrap_err();
        assert!(matches!(err.0, XIncludeError::Cycle { .. }));
    }

    #[test]
    fn nested_includes_expand_transitively() {
        let tmp = TempDir::new().unwrap();
        let inner = write(tmp.path(), "inner.xml", r#"<g><leaf/></g>"#);
        let middle = write(
            tmp.path(),
            "middle.xml",
            &format!(
                r#"<mid><xi:include xmlns:xi="http://www.w3.org/2001/XInclude" href="{}"/></mid>"#,
                inner.file_name().unwrap().to_str().unwrap()
            ),
        );
        let main_src = format!(
            r#"<root><xi:include xmlns:xi="http://www.w3.org/2001/XInclude" href="{}"/></root>"#,
            middle.file_name().unwrap().to_str().unwrap()
        );
        let main_path = write(tmp.path(), "main.xml", &main_src);

        let (out, _map, _deps) =
            expand(&main_src, main_path.to_str().unwrap(), Some(tmp.path())).unwrap();
        assert!(out.contains("<leaf/>"));
        assert!(!out.contains("<xi:include"));
        assert!(!out.contains("<mid"));
        assert!(!out.contains("<g"));
    }

    #[test]
    fn sibling_includes_of_same_target_are_allowed() {
        // Two top-level includes of the same file do not form a
        // cycle — the stack is pushed then popped for each,
        // matching pugixml's behaviour.
        let tmp = TempDir::new().unwrap();
        let frag = write(tmp.path(), "shared.xml", r#"<f><x/></f>"#);
        let main_src = format!(
            r#"<root>
<xi:include xmlns:xi="http://www.w3.org/2001/XInclude" href="{name}"/>
<xi:include xmlns:xi="http://www.w3.org/2001/XInclude" href="{name}"/>
</root>"#,
            name = frag.file_name().unwrap().to_str().unwrap()
        );
        let main_path = write(tmp.path(), "main.xml", &main_src);

        let (out, _map, _deps) =
            expand(&main_src, main_path.to_str().unwrap(), Some(tmp.path())).unwrap();
        // Both splice points must have been replaced.
        assert_eq!(out.matches("<x/>").count(), 2);
        assert!(!out.contains("<xi:include"));
    }

    #[test]
    fn parse_text_is_rejected() {
        let tmp = TempDir::new().unwrap();
        let frag = write(tmp.path(), "t.txt", "plain text");
        let main_src = format!(
            r#"<root><xi:include xmlns:xi="http://www.w3.org/2001/XInclude" href="{}" parse="text"/></root>"#,
            frag.file_name().unwrap().to_str().unwrap()
        );
        let main_path = write(tmp.path(), "main.xml", &main_src);
        let err = expand(&main_src, main_path.to_str().unwrap(), Some(tmp.path())).unwrap_err();
        assert!(matches!(err.0, XIncludeError::Unsupported { .. }));
    }

    #[test]
    fn xpointer_is_rejected() {
        let tmp = TempDir::new().unwrap();
        let frag = write(tmp.path(), "frag.xml", r#"<f><x/></f>"#);
        let main_src = format!(
            r#"<root><xi:include xmlns:xi="http://www.w3.org/2001/XInclude" href="{}" xpointer="xpointer(//x)"/></root>"#,
            frag.file_name().unwrap().to_str().unwrap()
        );
        let main_path = write(tmp.path(), "main.xml", &main_src);
        let err = expand(&main_src, main_path.to_str().unwrap(), Some(tmp.path())).unwrap_err();
        assert!(matches!(err.0, XIncludeError::Unsupported { .. }));
    }

    #[test]
    fn fallback_is_rejected() {
        let tmp = TempDir::new().unwrap();
        let main_src = r#"<root><xi:include xmlns:xi="http://www.w3.org/2001/XInclude" href="missing.xml"><xi:fallback xmlns:xi="http://www.w3.org/2001/XInclude"><nope/></xi:fallback></xi:include></root>"#;
        let main_path = write(tmp.path(), "main.xml", main_src);
        let err = expand(main_src, main_path.to_str().unwrap(), Some(tmp.path())).unwrap_err();
        assert!(matches!(err.0, XIncludeError::Unsupported { .. }));
    }

    #[test]
    fn location_points_at_include_node() {
        // Whitespace before `<xi:include>` places it on row 2 — the
        // reported location must name that row, not row 1.
        let tmp = TempDir::new().unwrap();
        let main_src =
            "<root>\n    <xi:include xmlns:xi=\"http://www.w3.org/2001/XInclude\"/>\n</root>";
        let main_path = write(tmp.path(), "main.xml", main_src);
        let err = expand(main_src, main_path.to_str().unwrap(), Some(tmp.path())).unwrap_err();
        assert_eq!(err.1.row, 2);
    }

    /// Drift test pinning the C++ `XIncludeExpander` API shape
    /// against the Rust `expand` return type and constants. Phase X
    /// B3 deliverable — see
    /// `claudedocs/rfc-sce-template-phase-x.md` §3 B3. Follows the
    /// `cpp_origin_shape_matches_rust` pattern in
    /// `sce-build/src/position_map.rs::tests`: read the
    /// authoritative C++ header via `include_str!` at test compile
    /// time and regex-scan the declarations.
    ///
    /// What this test pins:
    ///
    ///   * `MAX_XINCLUDE_DEPTH = N` literal in the C++ header
    ///     matches the Rust constant value.
    ///   * `struct XIncludeExpandResult` declares two fields: one
    ///     string-typed named `expanded_text` (mirrors Rust's
    ///     `String`), one `PositionMap`-typed named `positions`.
    ///   * The `expandStringX` free function declaration names its
    ///     three parameters (`content`, `selfPath`, `baseDir`)
    ///     mirroring Rust's `expand(content, self_path, base_dir)`.
    ///
    /// What this test does NOT pin: parameter types (so a
    /// `string_view` ↔ `string` swap stays allowed), member-init
    /// braces, internal helper signatures, error-class hierarchy.
    /// Those are covered by the C++ unit tests in
    /// `tests/parsing/XIncludeExpander_test.cpp` and the
    /// production wiring test there.
    ///
    /// Load-bearing: rename `XIncludeExpandResult::positions` to
    /// `coords` in the C++ header → this test reds with the
    /// missing-struct-shape assertion below.
    #[test]
    fn cpp_xinclude_expander_matches_rust_shape() {
        let hdr = include_str!("../../sce/include/parsing/XIncludeExpander.h");

        // ── Constant agreement: MAX_XINCLUDE_DEPTH ──────────────
        let depth_re =
            regex::Regex::new(r"constexpr\s+unsigned\s+MAX_XINCLUDE_DEPTH\s*=\s*(\d+)\s*;")
                .expect("MAX_XINCLUDE_DEPTH regex must compile");
        let cap = depth_re.captures(hdr).expect(
            "sce/include/parsing/XIncludeExpander.h must declare \
             `constexpr unsigned MAX_XINCLUDE_DEPTH = N;`",
        );
        let cpp_depth: u32 = cap[1]
            .parse()
            .expect("MAX_XINCLUDE_DEPTH literal must parse as u32");
        assert_eq!(
            cpp_depth, MAX_XINCLUDE_DEPTH,
            "C++ MAX_XINCLUDE_DEPTH ({cpp_depth}) must match Rust \
             MAX_XINCLUDE_DEPTH ({}). Update both in the same commit.",
            MAX_XINCLUDE_DEPTH
        );

        // ── Result-struct shape: expanded_text + positions ──────
        // String family allowlist — std::string (current) +
        // std::string_view kept open for a future portability swap.
        let string_family = r"(?:std::string|std::string_view)";
        let map_family = r"PositionMap";
        let result_re = regex::Regex::new(&format!(
            r"struct\s+XIncludeExpandResult\s*\{{\s*{s}\s+expanded_text\s*(?:\{{[^}}]*\}})?\s*;\s*{m}\s+positions\s*(?:\{{[^}}]*\}})?\s*;\s*\}}\s*;",
            s = string_family,
            m = map_family,
        ))
        .expect("XIncludeExpandResult regex must compile");

        assert!(
            result_re.is_match(hdr),
            "sce/include/parsing/XIncludeExpander.h must declare \
             `struct XIncludeExpandResult {{ <string-family> \
             expanded_text; PositionMap positions; }};` matching \
             Rust's `(String, PositionMap)` return tuple from \
             `xinclude::expand`. If the field order or type family \
             changed, update this drift test in the same commit. \
             String family allowlist: {s}.",
            s = string_family,
        );

        // ── Entry-point declaration: parameter names ────────────
        // Loose match on parameter types so `std::string_view` →
        // `std::string` portability swaps do not red the test.
        // Strict match on the three names: `content`, `selfPath`,
        // `baseDir` — those name the Rust-side contract bytes.
        let entry_re = regex::Regex::new(
            r"XIncludeExpandResult\s+expandStringX\s*\([^)]*\bcontent\b[^)]*\bselfPath\b[^)]*\bbaseDir\b[^)]*\)\s*;",
        )
        .expect("expandStringX regex must compile");

        assert!(
            entry_re.is_match(hdr),
            "sce/include/parsing/XIncludeExpander.h must declare \
             `XIncludeExpandResult expandStringX(... content ..., \
             ... selfPath ..., ... baseDir ...);` mirroring Rust's \
             `expand(content, self_path, base_dir)` call shape. If \
             the parameter names changed, update this drift test in \
             the same commit."
        );
    }

    /// Pin the 1:1 mapping between Rust `XIncludeError` variants,
    /// the `xml/xinclude-*` `DiagnosticCode`s they emit, and the C++
    /// `SCE::parsing::XInclude<Variant>` subtypes declared in
    /// `sce/include/parsing/XIncludeError.h`. RFC §W3 milestone in
    /// `claudedocs/rfc-sce-diagnostic-wire-unification.md`.
    ///
    /// Mirrors the W1 sister test
    /// `cpp_template_subtypes_match_rust_diagnostic_codes` in
    /// `sce-build/src/template.rs::tests`. A commit on any one side
    /// that fails to update the other two is the drift this test
    /// catches.
    #[test]
    fn cpp_xinclude_subtypes_match_rust_diagnostic_codes() {
        use std::collections::BTreeSet;

        // Authoritative Rust ground truth — the 7 xml/xinclude-*
        // `DiagnosticCode`s, paired with the C++ class name that must
        // exist in the header. Table-form so an audit reading this
        // test sees exactly which Rust variant maps to which C++
        // class without having to walk two files in parallel.
        let rust_to_cpp: &[(&str, &str)] = &[
            ("xml/xinclude-missing-href", "XIncludeMissingHref"),
            ("xml/xinclude-not-found", "XIncludeNotFound"),
            ("xml/xinclude-read-error", "XIncludeReadError"),
            ("xml/xinclude-cycle", "XIncludeCycle"),
            ("xml/xinclude-too-deep", "XIncludeTooDeep"),
            ("xml/xinclude-malformed", "XIncludeMalformed"),
            ("xml/xinclude-unsupported", "XIncludeUnsupported"),
        ];
        assert_eq!(
            rust_to_cpp.len(),
            7,
            "Expected 7-way mapping; update rust_to_cpp if the \
             DiagnosticCode set grew or shrank"
        );
        let expected_cpp: BTreeSet<&str> = rust_to_cpp.iter().map(|(_, cpp)| *cpp).collect();

        let hdr = include_str!("../../sce/include/parsing/XIncludeError.h");
        let re =
            regex::Regex::new(r"class\s+(XInclude\w+)\s*:\s*public\s+XIncludeExpansionError\b")
                .unwrap();
        let mut found: BTreeSet<String> = BTreeSet::new();
        for captures in re.captures_iter(hdr) {
            found.insert(captures[1].to_string());
        }

        assert!(
            !found.is_empty(),
            "sce/include/parsing/XIncludeError.h must declare at \
             least one `class XInclude<Variant> : public \
             XIncludeExpansionError` — if the declaration shape \
             changed, update this drift test in the same commit"
        );

        let found_refs: BTreeSet<&str> = found.iter().map(|s| s.as_str()).collect();
        assert_eq!(
            found_refs, expected_cpp,
            "XIncludeError subtype drift: C++ header = {:?}, \
             expected (from DiagnosticCode mapping) = {:?}. Change \
             both sides in the same commit — see RFC §W3 \
             (claudedocs/rfc-sce-diagnostic-wire-unification.md).",
            found_refs, expected_cpp
        );

        // Cross-check: every Rust `DiagnosticCode` name MUST be
        // spelled as the `serde(rename = "...")` literal in
        // `sce-build/src/forge/diagnostic.rs`. If a future edit
        // renames a code, this assertion fails with a pointed diff
        // rather than the drift travelling silently through JSON
        // wire contracts.
        let diag = include_str!("forge/diagnostic.rs");
        for (rust_code, cpp_name) in rust_to_cpp {
            let needle = format!("\"{}\"", rust_code);
            assert!(
                diag.contains(&needle),
                "DiagnosticCode `{}` (paired with C++ `{}`) is not \
                 declared as a `serde(rename)` literal in \
                 sce-build/src/forge/diagnostic.rs. Keep the wire \
                 name, the Rust variant, and the C++ subtype in \
                 sync — see RFC §W3.",
                rust_code,
                cpp_name
            );
        }
    }

    /// Pin the wire-string return literal inside each C++
    /// `XInclude<Variant>` subtype's `code()` body. RFC §W3 makes
    /// each subtype override `Diagnostic::code()` to return its
    /// `xml/xinclude-*` wire string; the sister test above pins
    /// **subtype names** between Rust and C++; this one pins the
    /// **wire-string return literal** so a future rename on either
    /// side cannot drift the JSON wire contract silently.
    ///
    /// The bite: changing `return "xml/xinclude-cycle"` to
    /// `return "xml/xinclude-cycleXXX"` in `XIncludeError.h` reds
    /// here with a pointed `does not contain` diff naming the exact
    /// class and exact missing literal.
    #[test]
    fn cpp_xinclude_subtype_code_returns_rust_wire_string() {
        let rust_to_cpp: &[(&str, &str)] = &[
            ("xml/xinclude-missing-href", "XIncludeMissingHref"),
            ("xml/xinclude-not-found", "XIncludeNotFound"),
            ("xml/xinclude-read-error", "XIncludeReadError"),
            ("xml/xinclude-cycle", "XIncludeCycle"),
            ("xml/xinclude-too-deep", "XIncludeTooDeep"),
            ("xml/xinclude-malformed", "XIncludeMalformed"),
            ("xml/xinclude-unsupported", "XIncludeUnsupported"),
        ];
        assert_eq!(rust_to_cpp.len(), 7);

        let hdr = include_str!("../../sce/include/parsing/XIncludeError.h");

        for (rust_code, cpp_class) in rust_to_cpp {
            // Locate the class block. The header's shape (one class
            // per subtype, each terminated with `};`) keeps a forward
            // `find("};")` accurate enough for a drift guard; if a
            // future rewrite nests braces inside a subtype we update
            // this scanner in the same commit.
            let class_marker = format!("class {} : public XIncludeExpansionError", cpp_class);
            let class_start = hdr.find(&class_marker).unwrap_or_else(|| {
                panic!(
                    "class `{}` not found in sce/include/parsing/\
                     XIncludeError.h — drift in subtype naming, see \
                     `cpp_xinclude_subtypes_match_rust_diagnostic_codes`",
                    cpp_class
                )
            });
            let body_start = hdr[class_start..].find('{').unwrap() + class_start + 1;
            let body_end_rel = hdr[body_start..].find("};").unwrap();
            let body = &hdr[body_start..body_start + body_end_rel];

            let needle = format!("return \"{}\";", rust_code);
            assert!(
                body.contains(&needle),
                "Class `{}` body does not contain `{}` — the C++ \
                 subtype's `code()` override must return the Rust \
                 DiagnosticCode wire literal exactly so the JSON \
                 wire emitted by `to_json()` agrees with \
                 `--error-format=json`. RFC §W3 / SCE_ERROR_CONTRACT.md §3.",
                cpp_class,
                needle
            );
        }

        // Sanity-count: the header should declare exactly 7 `code()`
        // overrides on leaves + 1 pure-virtual on the base = 8
        // occurrences. An 8th leaf (or a missing one) reds with a
        // count diff rather than silently passing.
        let override_count = hdr
            .matches("std::string_view code() const noexcept override")
            .count();
        assert_eq!(
            override_count, 8,
            "expected 8 `code() const noexcept override` lines in \
             XIncludeError.h (1 pure-virtual on \
             XIncludeExpansionError + 7 subtype overrides); found {}",
            override_count
        );
    }
}
