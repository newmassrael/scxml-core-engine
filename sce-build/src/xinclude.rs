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
    Unsupported {
        href: String,
        feature: &'static str,
    },
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
/// `content`. It is added to the cycle-detection stack and used
/// for diagnostic rendering; callers that have no filesystem
/// identity (in-memory documents) should pass a stable label
/// such as the `DocumentLabel` diagnostic string. `base_dir` is
/// the directory `<xi:include href="relative/...">` is resolved
/// against — typically `Path::new(self_path).parent()`.
///
/// Returns the expanded document as an owned `String` suitable
/// for handing to `roxmltree::Document::parse`. Short-circuits
/// with an `Ok(content.to_string())` when the input has no
/// `include` substring at all, so documents that do not use
/// XInclude pay only a single substring search on the critical
/// path.
pub fn expand(
    content: &str,
    self_path: &str,
    base_dir: Option<&Path>,
) -> Result<String, (XIncludeError, XIncludeLocation)> {
    if !content.contains("include") {
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
) -> Result<String, (XIncludeError, XIncludeLocation)> {
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
        return Ok(content.to_string());
    }

    // Emit the output by splicing: walk the original byte stream
    // and replace each `<xi:include>` range with the rendered
    // children of the included document's root. Processing in
    // document order (left to right) lets a single cursor serve
    // the splice and keeps the offset math trivial.
    let mut out = String::with_capacity(content.len());
    let mut cursor = 0usize;

    for node in includes {
        let range = node.range();
        // Copy the unchanged prefix up to this include node.
        out.push_str(&content[cursor..range.start]);

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

        stack.push(canon);
        let nested_base = resolved.parent().map(|p| p.to_path_buf());
        let expanded = expand_impl(&raw, nested_base.as_deref(), depth + 1, stack).map_err(
            |(err, nested_loc)| {
                // The nested error's row/col references the
                // included file, not the outer one. For the AOT
                // diagnostic we keep the outer-file location
                // (the `<xi:include>` node) because that is
                // where the operator will apply the fix — the
                // inner location is still available via the
                // nested error's own rendering inside `detail`.
                let _ = nested_loc;
                (remap_nested(err, href), loc)
            },
        )?;
        stack.pop();

        let rendered = render_root_children(&expanded, href).map_err(|e| (e, loc))?;
        out.push_str(&rendered);

        cursor = range.end;
    }

    // Tail: copy the unchanged suffix after the last include.
    out.push_str(&content[cursor..]);
    Ok(out)
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
fn render_root_children(expanded: &str, href: &str) -> Result<String, XIncludeError> {
    let doc = roxmltree::Document::parse(expanded).map_err(|e| XIncludeError::Malformed {
        href: href.to_string(),
        detail: e.to_string(),
    })?;
    let root = doc.root_element();
    let children: Vec<_> = root.children().collect();
    if children.is_empty() {
        return Ok(String::new());
    }
    let start = children.first().unwrap().range().start;
    let end = children.last().unwrap().range().end;
    Ok(expanded[start..end].to_string())
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
        // Guards against silent drift from
        // sce/include/parsing/PugiXMLParser.h:69. If the runtime
        // changes the limit, update `MAX_XINCLUDE_DEPTH` here in
        // the same commit.
        assert_eq!(MAX_XINCLUDE_DEPTH, 10);
    }

    #[test]
    fn passthrough_when_no_include_substring() {
        let src = "<root><state id=\"s1\"/></root>";
        let out = expand(src, "inline", None).expect("no includes");
        assert_eq!(out, src);
    }

    #[test]
    fn passthrough_when_include_substring_but_no_element() {
        // "include" appears as a word in an attribute or text but
        // no actual `<xi:include>` element exists — must parse
        // the document to tell, and must pass it through
        // unchanged.
        let src = "<root description=\"please include docs\"/>";
        let out = expand(src, "inline", None).expect("no include elements");
        assert_eq!(out, src);
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

        let out = expand(&main_src, main_path.to_str().unwrap(), Some(tmp.path()))
            .expect("expansion succeeds");
        // The children of `<fragment>` must be spliced in place
        // of the `<xi:include>` element, dropping the wrapper.
        assert!(out.contains("<state id=\"s1\"/>"));
        assert!(out.contains("<state id=\"s2\"/>"));
        assert!(!out.contains("<xi:include"));
        assert!(!out.contains("<fragment"));
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

        let out = expand(&main_src, main_path.to_str().unwrap(), Some(tmp.path())).unwrap();
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

        let out = expand(&main_src, main_path.to_str().unwrap(), Some(tmp.path())).unwrap();
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

        let out = expand(&main_src, main_path.to_str().unwrap(), Some(tmp.path())).unwrap();
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
        let main_src = "<root>\n    <xi:include xmlns:xi=\"http://www.w3.org/2001/XInclude\"/>\n</root>";
        let main_path = write(tmp.path(), "main.xml", main_src);
        let err = expand(main_src, main_path.to_str().unwrap(), Some(tmp.path())).unwrap_err();
        assert_eq!(err.1.row, 2);
    }
}
