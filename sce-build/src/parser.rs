// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCXML Parser — ports scxml_parser.py using roxmltree.
// Parses W3C SCXML files into SCXMLModel for code generation.

use crate::model::*;
use crate::scxml_semantic::{ScxmlSemanticError, UnsupportedDatamodelKind};
use crate::DocumentLabel;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

/// Reserved `<sce:context id="...">` names, derived from the C++
/// codegen template at first access.
///
/// The C++ codegen emits `using {Id}Type = ...;` on the generated
/// state-machine class (see `tools/codegen/templates/state_machine.jinja2`).
/// Identifiers whose `capitalize`-d form would collide with such an
/// alias must be rejected at parse time so the collision never
/// reaches template rendering or C++ compilation.
///
/// # Single source of truth
///
/// The template is the authoritative source. On first access this
/// `LazyLock` scans the template text for literal `using {Id}Type =`
/// aliases and publishes the lowercased prefixes as the reserved
/// list. Adding a new `using FooType = ...` line to the template is
/// therefore sufficient to reserve `foo` — there is no parallel
/// const to update, and no drift can exist between the template's
/// alias set and the reserved list because they are mechanically
/// the same set.
///
/// # Regex extraction shape
///
/// `using\s+([A-Z][A-Za-z0-9_]*)Type\s*=` captures only literal
/// identifier aliases. Jinja2 expressions such as
/// `using {{ ctx.id | capitalize }}Type = ...` begin with `{`, which
/// does not match the leading `[A-Z]`, so per-context aliases that
/// come from user SCXML are excluded automatically. Conditional
/// template blocks (`{% if ... %}using FooType = ...{% endif %}`)
/// are still visible in the source — reserving a future-enabled
/// alias's id from day one is the correct behaviour.
///
/// # Comparison semantics
///
/// Callers lowercase the context id before checking membership
/// because Jinja2's `capitalize` filter lowercases every character
/// after the first (`"POLICY".capitalize() == "Policy"`), so
/// `policy`, `Policy`, and `POLICY` all generate the same
/// `PolicyType` alias and therefore collide identically.
///
/// # Static-slice shape
///
/// The reserved list is published as `&'static [&'static str]` so
/// [`crate::forge::error::ValidationError::ReservedContextId`]'s
/// `reserved: &'static [&'static str]` field accepts it without
/// conversion — the diagnostic wire format stays unchanged. Leaking
/// is bounded by the template size (static), so the leak does not
/// grow at runtime.
pub static RESERVED_CONTEXT_IDS: LazyLock<&'static [&'static str]> = LazyLock::new(|| {
    const TEMPLATE_SRC: &str = include_str!("../../tools/codegen/templates/state_machine.jinja2");
    let re = regex::Regex::new(r"using\s+([A-Z][A-Za-z0-9_]*)Type\s*=")
        .expect("RESERVED_CONTEXT_IDS regex must compile");
    let mut ids: Vec<String> = re
        .captures_iter(TEMPLATE_SRC)
        .map(|c| c[1].to_ascii_lowercase())
        .collect();
    ids.sort();
    ids.dedup();
    let leaked: Vec<&'static str> = ids
        .into_iter()
        .map(|s| &*Box::leak(s.into_boxed_str()))
        .collect();
    Box::leak(leaked.into_boxed_slice())
});

pub struct SCXMLParser {
    document_order_counter: u32,
    /// XML node id -> document-order rank for every state element, filled
    /// by [`SCXMLParser::assign_document_order`] before the states pass.
    document_order_by_node: BTreeMap<usize, u32>,
    invoke_counter: u32,
    hybrid_invoke_counter: u32,
    send_counter: u32,
    /// §scxml-3.14: every `<invoke>` id must be document-unique.
    /// Both author-supplied and auto-generated ids feed this set so
    /// the author-shadows-auto-counter case (e.g. `<invoke id="_invoke_0">`
    /// followed by an idless invoke whose auto counter hits 0) is
    /// caught alongside plain author duplicates.
    invoke_ids_seen: BTreeSet<String>,
    /// Canonical paths of every external file (xi:include target,
    /// sce:use template fragment) consumed by the most recent
    /// `parse_file` call. Populated by [`expand_preprocessors`] and
    /// surfaced via [`SCXMLParser::preprocessor_deps`] so the codegen
    /// CLI can extend the `--write-deps` depfile with the actual
    /// fragment inputs. `parse_string` leaves it empty (no fs access).
    /// Cleared at the start of every `parse_file` so successive parses
    /// do not accumulate stale entries.
    preprocessor_deps: Vec<PathBuf>,
    /// Operator-configured `--include-dir` search path threaded into
    /// the XInclude / `sce:use` resolvers by [`parse_file`] (see
    /// [`expand_preprocessors`]). Empty by default, so `new()` parsers
    /// resolve fragments exactly as `absolute → base → cwd`. Set via
    /// [`SCXMLParser::with_include_dirs`]. `parse_string` ignores it
    /// (no fs access).
    ///
    /// [`parse_file`]: SCXMLParser::parse_file
    include_dirs: Vec<PathBuf>,
}

/// Run the XInclude + `sce:template` preprocessors on raw SCXML
/// content and return the post-expansion text together with a
/// [`PositionMap`] that remaps expanded-byte coordinates back to
/// author source coordinates.
///
/// Shared by [`SCXMLParser::parse_file`] (which then feeds the
/// expanded text into [`SCXMLParser::parse_impl`] and remaps any
/// downstream diagnostic via the returned map) and by the
/// `sce-codegen expand` subcommand (which prints the expanded text
/// to stdout for the SSOT parity harness —
/// `tests/w3c_template_parity/` consumes the same bytes the
/// codegen pipeline consumes).
///
/// Extracting this into a free function keeps the preprocessor
/// sequence single-source: any future third pass, or any change
/// to the xinclude/template ordering, is picked up by both the
/// codegen consumer and the parity harness without a second edit.
/// This SSOT guarantee holds at the Rust-side boundary; the
/// cross-language SSOT guarantee is enforced by the C++ harness
/// driver diffing canonicalised outputs.
///
/// `extra_dirs` is the operator-configured `--include-dir` search
/// path, threaded unchanged into both expanders so `<xi:include>` and
/// `<sce:use>` resolve fragments by name against a shared directory
/// list instead of only a depth-coupled relative path. The C++
/// runtime mirrors the same precedence (see
/// `PugiXMLDocument::setIncludeDirs`).
pub fn expand_preprocessors(
    content: &str,
    scxml_path: &str,
    base_dir: Option<&Path>,
    extra_dirs: &[PathBuf],
) -> Result<
    (String, crate::position_map::PositionMap, Vec<PathBuf>),
    crate::forge::error::Located<crate::forge::error::ForgeError>,
> {
    // XInclude parity with `PugiXMLDocument::processXInclude`
    // (sce/src/parsing/PugiXMLParser.cpp:212). The C++ runtime
    // expands `<xi:include>` at parse time; without this step
    // the AOT code generator consumes a different effective
    // document than the interpreter, silently producing state
    // machines that diverge from runtime behaviour.
    //
    // The expander also returns a `PositionMap` keyed by expanded
    // bytes — every subsequent diagnostic that fires against the
    // expanded document (XSD line numbers, roxmltree row/col,
    // semantic validation) gets remapped at the parse-impl
    // boundary so `location.{file, row, col}` points at the
    // author source, not at in-memory expanded coordinates.
    // Expander-internal errors (MissingHref, NotFound, ...)
    // already report positions in the pre-expansion outer file,
    // so they wrap directly without going through the map.
    let (included, xinclude_map, xinclude_deps) = crate::xinclude::expand(
        content, scxml_path, base_dir, extra_dirs,
    )
    .map_err(|(err, loc)| {
        use crate::forge::error::{ForgeError, Located, XmlError};
        Located::new(
            ForgeError::Xml(XmlError::XInclude(err)),
            scxml_path,
            Some(loc.row),
            Some(loc.col),
        )
    })?;

    // `sce:template` expansion runs immediately after XInclude
    // so templates see a post-XInclude document. The C++
    // Interpreter performs the same string-level expansion
    // (`sce/src/parsing/TemplateExpander.cpp`); the template
    // parity harness keeps both sides byte-equivalent.
    //
    // The expander composes `xinclude_map` with its own entries
    // (File origins for template-body bytes, CallSite origins
    // for `{$param}` splices per SCE_ACCEPTED_
    // SUBSET.md §2.9) and returns a `final_map` that replaces
    // `xinclude_map` for post-expansion remapping — every
    // emitted byte, wherever it came from, traces back to a
    // source file the author can open.
    let (expanded, final_map, template_deps) =
        crate::template::expand(&included, scxml_path, base_dir, extra_dirs, &xinclude_map)
            .map_err(|(err, loc)| {
                use crate::forge::error::{ForgeError, Located, XmlError};
                // The template expander stamps `loc` against `included`
                // (the post-XInclude bytes). Resolving the byte through
                // `xinclude_map` traces the diagnostic back to the
                // author file — host or `xi:include`'d fragment — so a
                // `<sce:use>` failure inside a fragment surfaces with
                // fragment-file coordinates instead of host-file
                // post-XInclude coordinates. Mirrors
                // the C++ side's `inputMap.lookup` at the useLocation
                // stamp.
                let byte = crate::position_map::rowcol_to_offset(&included, loc.row, loc.col);
                let origin = xinclude_map.lookup(byte);
                let origin_path = origin.file.to_string_lossy().into_owned();
                Located::new(
                    ForgeError::Xml(XmlError::Template(err)),
                    if origin_path.is_empty() {
                        scxml_path
                    } else {
                        origin_path.as_str()
                    },
                    Some(origin.row),
                    Some(origin.col),
                )
            })?;

    // Concatenate xinclude → template open order so the depfile
    // mirrors the actual preprocessor pipeline order. Both expanders
    // already de-duplicate via canonicalisation at their own boundary
    // (cycle detection's `stack`), but the same canon path can still
    // appear in both lists if a `<xi:include>`'d fragment also
    // arrives via `<sce:use>` in another document — `write_depfile`
    // collapses duplicates at the sink, so order > uniqueness here.
    let mut deps = xinclude_deps;
    deps.extend(template_deps);

    Ok((expanded, final_map, deps))
}

/// Refuse a document that still carries a preprocessor directive.
///
/// [`expand_preprocessors`] consumes every `<xi:include>` and
/// `<sce:use>` it is given, so one surviving into a parsed tree means
/// the pass never ran. That is a caller-side mistake, and without this
/// check an invisible one: both the statechart and forge parsers select
/// children by tag name and have no else-branch, so the directive is
/// skipped in silence and whatever it was carrying never arrives. A
/// `lookup` with `sce:default` then answers for the missing key, which
/// reads as a correct table from every side — compiler, generated code,
/// runtime — and a statechart loses whole states just as quietly.
///
/// Held at both parse entries rather than at one: the file-based entries
/// expand for their callers, so only the in-memory ones can arrive in
/// this state, and they exist on both pipelines.
///
/// This is deliberately not an XSD rule. `<sce:use>` is a declared
/// element and the elements admitting it are `xs:any
/// processContents="lax"`, so the schema calls an unexpanded document
/// valid by construction; making it invalid there would also make the
/// editor-integration and template-authoring cases invalid. The
/// document tree is the layer that can tell "not yet expanded" from
/// "not expandable", so the check lives here.
///
/// The predicates come from the two expanders rather than from a copy
/// kept here, so the guard rejects exactly the shapes expansion would
/// have consumed — including `<include>` written without its namespace
/// prefix, which [`crate::xinclude`] accepts for C++ runtime parity.
pub fn reject_unexpanded_directives(
    root: &roxmltree::Node,
    doc_name: &str,
) -> Result<(), crate::forge::error::Located<crate::forge::error::ForgeError>> {
    use crate::forge::error::{ForgeError, Located, XmlError};

    for node in root.descendants() {
        let element = if crate::template::is_sce_use_element(&node) {
            "sce:use"
        } else if crate::xinclude::is_xinclude_element(&node) {
            // Report the prefixed spelling even when the author wrote
            // the bare `<include>`: the prefixed form is what the docs
            // and the fix both name.
            "xi:include"
        } else {
            continue;
        };

        let pos = node.document().text_pos_at(node.range().start);
        return Err(Located::new(
            ForgeError::Xml(XmlError::PreprocessorNotRun {
                element: element.to_string(),
            }),
            doc_name,
            Some(pos.row),
            Some(pos.col),
        ));
    }

    Ok(())
}

/// SCE Protocol-Synthesis RFC §synth-5-O: capture the post-preprocessor
/// source position of an XML element for the SCE-MAP traceability
/// chain. Templates lower the returned [`SourceLocation`] to a
/// per-backend marker (`#line` / `//line` / `// SCE-MAP:` / `#[doc]`)
/// above the function header the IR node lowers to.
///
/// `source_name` is the document label the outer parser threads down —
/// for [`SCXMLParser::parse_file`] the path the caller named, so that
/// `location.file` on a diagnostic is something the consumer can open.
/// Artifacts need the other spelling; see [`artifact_label`].
#[inline]
fn source_location_of(
    node: &roxmltree::Node,
    source_name: &str,
) -> Option<crate::forge::error::SourceLocation> {
    let pos = node.document().text_pos_at(node.range().start);
    Some(crate::forge::error::SourceLocation {
        file: artifact_label(source_name),
        line: Some(pos.row),
        col: Some(pos.col),
    })
}

/// The artifact-facing spelling of a document label: its basename.
///
/// Two consumers read a document label and they need different
/// spellings. A *diagnostic* names the document so a consumer can open
/// it, which means the path the caller supplied — `main.scxml` alone
/// is openable only from the one directory it happens to sit in, and
/// the CLI-boundary validators have always emitted the full path, so a
/// consumer reading one stream saw two conventions with nothing to
/// tell them apart. An *artifact* — an SCE-MAP marker, a provenance
/// record that lands in committed generated code — cannot carry a
/// path: it would bake one machine's checkout into the tree and move
/// whenever the checkout does.
///
/// Deriving here rather than threading a second parameter through
/// every parse function keeps one value flowing and puts the choice at
/// the two places that actually differ.
#[inline]
fn artifact_label(source_name: &str) -> String {
    Path::new(source_name)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(source_name)
        .to_string()
}

/// Resolve the `datamodel` attribute to the data model SCE will use.
///
/// `None` is an absent attribute, whose value the spec leaves
/// platform-specific; [`Datamodel::default`] is where SCE makes that choice
/// and says why.
///
/// The two rejections are the whole reason this returns a `Result`. Before
/// it existed the attribute was read into a `String` that nothing
/// subsequently consulted, so `datamodel="xpath"` and `datamodel="typo"`
/// both compiled — their expressions handed to whatever script engine the
/// deployment happened to inject, in a language the document never named.
fn resolve_declared_datamodel(declared: Option<&str>) -> Result<Datamodel, ScxmlSemanticError> {
    let Some(declared) = declared else {
        return Ok(Datamodel::default());
    };
    match declared {
        "null" => Ok(Datamodel::Null),
        "ecmascript" => Ok(Datamodel::EcmaScript),
        other => Err(ScxmlSemanticError::UnsupportedDatamodel {
            declared: other.to_string(),
            // §scxml-3.2 names `xpath` as a data model; SCE has no XPath
            // expression pipeline, so it is refused as unimplemented
            // rather than as nonsense. Any other token is a value the
            // spec permits a platform to define and SCE has not.
            kind: if other == "xpath" {
                UnsupportedDatamodelKind::Unimplemented
            } else {
                UnsupportedDatamodelKind::Undefined
            },
            supported: vec![
                Datamodel::Null.as_str().to_string(),
                Datamodel::EcmaScript.as_str().to_string(),
            ],
        }),
    }
}

/// Elements that need the underlying data model the Null model does not
/// have — a place to declare a variable, store into, or iterate.
///
/// The spec withholds "the elements defined in 5 Data Model and Data
/// Manipulation" wholesale, and SCE is deliberately narrower: the elements
/// listed here are refused, while `<donedata>`, `<content>` and `<param>`
/// are admitted when they carry only literals. That is an SCE extension to
/// the Null data model, recorded as one in `docs/SCE_ACCEPTED_SUBSET.md`,
/// and it is drawn where it is because the Null model withholds four
/// *languages* rather than syntax: a literal payload names no expression
/// in any of them, so refusing it would deny an author a construct that
/// needs nothing the model lacks. An expression on any of those three
/// elements is still refused — by the attribute rules below, under the
/// sub-section that actually withholds the language.
const ELEMENTS_NEEDING_A_DATA_MODEL: &[&str] = &["datamodel", "data", "assign", "foreach"];

/// Attributes that hold a value expression or a location expression,
/// paired with the language each one needs and the rule that withholds it.
const EXPRESSION_ATTRIBUTES: &[(&str, &str, &str)] = &[
    ("expr", "a value expression language", "B.1.4"),
    ("srcexpr", "a value expression language", "B.1.4"),
    ("targetexpr", "a value expression language", "B.1.4"),
    ("delayexpr", "a value expression language", "B.1.4"),
    ("eventexpr", "a value expression language", "B.1.4"),
    ("typeexpr", "a value expression language", "B.1.4"),
    ("location", "a location expression language", "B.1.3"),
    ("idlocation", "a location expression language", "B.1.3"),
];

/// Is `cond` the whole of the Null data model's boolean expression
/// language?
///
/// The boolean expression language consists of the In predicate only, and
/// has the form `In(id)`. Anything else — a comparison, a conjunction,
/// even a negated `In()` — is a language the
/// model does not have. The rule is deliberately literal: widening it to
/// "contains an In()" would readmit `In('a') && x > 1`, which is the
/// value expression language B-1-4 withholds.
fn is_null_datamodel_condition(cond: &str) -> bool {
    // §scxml-B-1-2: "The boolean expression language consists of the In
    // predicate only. It has the form 'In(id)'."
    let c = cond.trim();
    let Some(inner) = c.strip_prefix("In(").and_then(|r| r.strip_suffix(')')) else {
        return false;
    };
    // A single state reference, quoted or bare. Nested parentheses would
    // mean a call or a sub-expression, neither of which B-1-2 admits.
    !inner.contains(['(', ')', ',']) && !inner.trim().is_empty()
}

/// Does this `<script>` carry native host code rather than data model
/// script text?
///
/// SCE spells a native action as `<script><cpp>…</cpp></script>` (or
/// `<kt>`, or the `urn:sce:cpp` / `urn:sce:kotlin` namespaces) and lowers
/// it straight into the generated host language — see the `is_cpp_function`
/// branch in `parse_executable_content`. Such an element names no data
/// model expression at all.
///
/// Requires *every* element child to be native: a `<script>` mixing native
/// blocks with script text still needs the language for the text.
fn is_native_script_block(node: &roxmltree::Node) -> bool {
    let mut saw_native = false;
    for child in node.children().filter(|n| n.is_element()) {
        let n = child.tag_name().name();
        let ns = child.tag_name().namespace();
        if n == "cpp" || n == "kt" || ns == Some("urn:sce:cpp") || ns == Some("urn:sce:kotlin") {
            saw_native = true;
        } else {
            return false;
        }
    }
    // Text alongside the native blocks is data model script.
    saw_native
        && node
            .children()
            .filter(|n| n.is_text())
            .all(|n| n.text().unwrap_or("").trim().is_empty())
}

/// Refuse constructs the declared data model has no language for.
///
/// Only the Null data model withholds anything today, and the spec says
/// what: `In()` is the entire boolean expression language, there is no
/// location or value expression language, no scripting language, and
/// `<foreach>` plus the data-manipulation elements are unsupported. Each
/// rule is reported
/// under its own sub-section because they are separate absences — an
/// author who wrote `<param expr=…>` needs B-1-4, one who wrote `<script>`
/// needs B-1-5, and telling both of them "the null data model is empty"
/// names none of it.
///
/// Walks only SCXML-namespace elements: `sce:` extensions are governed by
/// `docs/SCE_ACCEPTED_SUBSET.md`, not by Appendix B. A nested `<scxml>`
/// (an inline `<invoke>` document) is skipped with its subtree — it
/// declares its own data model and is validated when it is parsed as the
/// document it is.
fn enforce_datamodel_languages(
    root: &roxmltree::Node,
    datamodel: Datamodel,
) -> Result<(), ScxmlSemanticError> {
    // §scxml-B-1 withholds four languages one sub-section at a time:
    // §scxml-B-1-1 the underlying data model, §scxml-B-1-2 every boolean
    // expression but `In(id)`, §scxml-B-1-3 location expressions,
    // §scxml-B-1-4 value expressions, §scxml-B-1-5 scripting. §scxml-B-1-7
    // withholds `<foreach>` and the §scxml-5 elements wholesale; the
    // narrowing SCE draws against that one is on
    // `ELEMENTS_NEEDING_A_DATA_MODEL`.
    if datamodel != Datamodel::Null {
        return Ok(());
    }
    fn owning_state(node: &roxmltree::Node) -> String {
        let mut cur = node.parent();
        while let Some(n) = cur {
            if n.tag_name().name() == "state" || n.tag_name().name() == "parallel" {
                return n.attribute("id").unwrap_or("").to_string();
            }
            cur = n.parent();
        }
        String::new()
    }

    let mut stack: Vec<roxmltree::Node> = root.children().filter(|n| n.is_element()).collect();
    while let Some(node) = stack.pop() {
        if node.tag_name().namespace() != root.tag_name().namespace() {
            continue;
        }
        let name = node.tag_name().name();
        // A nested inline document owns its own datamodel declaration.
        if name == "scxml" {
            continue;
        }

        // `<script><cpp>…</cpp></script>` / `<kt>` is SCE's native host
        // action (§2.11), not the data model's scripting language: the
        // parser lowers it to host code and no script engine evaluates
        // it. §B-1-5 withholds a scripting language, and a document that
        // uses none is entitled to say so — rejecting this would push
        // honestly engine-free documents onto the scripting tier to
        // satisfy a rule about a language they never used.
        if name == "script" && is_native_script_block(&node) {
            continue;
        }

        // A `<script>` that is not a native block carries data model
        // script text, which B-1-5 has no language for.
        if name == "script" {
            return Err(ScxmlSemanticError::NullDatamodelForbidsConstruct {
                construct: "<script>".to_string(),
                needs: "a scripting language".to_string(),
                rule: "B.1.5".to_string(),
                state: owning_state(&node),
            });
        }

        if ELEMENTS_NEEDING_A_DATA_MODEL.contains(&name) {
            return Err(ScxmlSemanticError::NullDatamodelForbidsConstruct {
                construct: format!("<{name}>"),
                needs: "the underlying data model it declares or writes to".to_string(),
                rule: "B.1.1".to_string(),
                state: owning_state(&node),
            });
        }

        for (attr, needs, rule) in EXPRESSION_ATTRIBUTES {
            if let Some(value) = node.attribute(*attr) {
                return Err(ScxmlSemanticError::NullDatamodelForbidsConstruct {
                    construct: format!("{attr}=\"{value}\""),
                    needs: (*needs).to_string(),
                    rule: (*rule).to_string(),
                    state: owning_state(&node),
                });
            }
        }

        if let Some(cond) = node.attribute("cond") {
            if !is_null_datamodel_condition(cond) {
                return Err(ScxmlSemanticError::NullDatamodelForbidsConstruct {
                    construct: format!("cond=\"{cond}\""),
                    needs: "a boolean expression language beyond In()".to_string(),
                    rule: "B.1.2".to_string(),
                    state: owning_state(&node),
                });
            }
        }

        stack.extend(node.children().filter(|n| n.is_element()));
    }
    Ok(())
}

/// Collect `<sce:unresolved>` markers attached to `node` — both
/// the attribute form (`sce:unresolved="id"`,
/// `sce:unresolved-reason="..."`, `sce:unresolved-candidates="a b c"`)
/// and the child-element form
/// (`<sce:unresolved id reason candidates/>`). Multiple element-form
/// children produce multiple markers; the attribute form contributes
/// at most one. The parser silently
/// collects; `--strict-unresolved` lifts to a build-failing error
/// via [`crate::provenance::check_strict_unresolved`].
fn collect_sce_unresolved(
    node: &roxmltree::Node,
    source_name: &str,
) -> Vec<crate::provenance::UnresolvedMarker> {
    use crate::forge::error::SourceLocation;
    use crate::forge::model::SCE_NAMESPACE;
    use crate::provenance::UnresolvedMarker;
    let mut markers: Vec<UnresolvedMarker> = Vec::new();
    if let Some(id) = node.attribute((SCE_NAMESPACE, "unresolved")) {
        if !id.is_empty() {
            let reason = node
                .attribute((SCE_NAMESPACE, "unresolved-reason"))
                .map(|s| s.to_string());
            let candidates = node
                .attribute((SCE_NAMESPACE, "unresolved-candidates"))
                .map(|raw| {
                    raw.split_whitespace()
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let pos = node.document().text_pos_at(node.range().start);
            markers.push(UnresolvedMarker {
                id: id.to_string(),
                reason,
                candidates,
                location: Some(SourceLocation {
                    file: artifact_label(source_name),
                    line: Some(pos.row),
                    col: Some(pos.col),
                }),
            });
        }
    }
    for child in node.children().filter(|c| c.is_element()) {
        if child.tag_name().namespace() != Some(SCE_NAMESPACE)
            || child.tag_name().name() != "unresolved"
        {
            continue;
        }
        let id = match child.attribute("id") {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => continue,
        };
        let reason = child.attribute("reason").map(|s| s.to_string());
        let candidates = child
            .attribute("candidates")
            .map(|raw| {
                raw.split_whitespace()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let pos = child.document().text_pos_at(child.range().start);
        markers.push(UnresolvedMarker {
            id,
            reason,
            candidates,
            location: Some(SourceLocation {
                file: artifact_label(source_name),
                line: Some(pos.row),
                col: Some(pos.col),
            }),
        });
    }
    markers
}

/// Propagate block-level `sce:req` IDs (from `<onentry>` or
/// `<onexit>`) onto every action in that block, skipping ids the
/// action already carries (an inner action may have its own
/// `sce:req` that overlaps with the block's). Order is preserved:
/// inner ids stay first, inherited block-level ids appended.
fn inherit_req(block_req: &[crate::provenance::RequirementId], block: &mut [crate::model::Action]) {
    if block_req.is_empty() {
        return;
    }
    for action in block.iter_mut() {
        for r in block_req {
            if !action.req.contains(r) {
                action.req.push(r.clone());
            }
        }
    }
}

/// Read the optional `sce:req="ID1 ID2 ..."` attribute and return
/// the whitespace-separated requirement IDs. Returns `Ok(vec![])`
/// when the attribute is absent. Rejects the first duplicate token
/// on a single node with `ValidationError::DuplicateRequirementId`
/// — opaque token by design, but
/// duplicates mask a missing annotation in downstream req-coverage
/// NDJSON so they are caught here.
///
/// `element_label_fn` is invoked only on the duplicate error path
/// so callers can build the author-facing description (e.g.
/// `<state id="armed">`) without paying for the format on the
/// happy path.
fn collect_sce_req(
    node: &roxmltree::Node,
    element_label_fn: impl FnOnce() -> String,
    source_name: &str,
) -> Result<
    Vec<crate::provenance::RequirementId>,
    crate::forge::error::Located<crate::forge::error::ForgeError>,
> {
    use crate::forge::error::{Located, ValidationError};
    use crate::forge::model::SCE_NAMESPACE;
    use crate::provenance::RequirementId;
    use std::collections::HashSet;
    let raw = match node.attribute((SCE_NAMESPACE, "req")) {
        Some(s) => s,
        None => return Ok(Vec::new()),
    };
    let mut seen: HashSet<String> = HashSet::new();
    let mut out: Vec<RequirementId> = Vec::new();
    for tok in RequirementId::split(raw) {
        if !seen.insert(tok.to_string()) {
            let pos = node.document().text_pos_at(node.range().start);
            return Err(Located::new(
                ValidationError::DuplicateRequirementId {
                    element: element_label_fn(),
                    id: tok.to_string(),
                }
                .into(),
                source_name,
                Some(pos.row),
                Some(pos.col),
            ));
        }
        out.push(RequirementId(tok.to_string()));
    }
    Ok(out)
}

/// Read the optional `sce:unhandled` attribute — the events this
/// state deliberately does not handle.
///
/// Also rejects the withdrawn `sce:exhaustive` attribute by name. That
/// rejection is not tidiness: an unrecognised `sce:` attribute on a
/// statechart element is accepted and ignored, so a document still
/// carrying the old parent-level opt-out would lose its exemption
/// silently and either start failing somewhere else or — on a parent
/// whose gaps have since been closed — keep building while its
/// annotation says something the build no longer honours.
///
/// Token rules, each of which exists to keep the declaration
/// checkable against the literal gap set the validator computes:
///
///   * Whitespace-separated literal event names, declaration order
///     preserved.
///   * Present but empty rejects — an attribute that declares nothing
///     is a mis-edit, not an exemption.
///   * Wildcards (`*`, `.*`, `foo.*`) reject. The gap set is always
///     literal; letting a wildcard in would give this one attribute a
///     second matching semantics to be read under.
///   * Duplicates reject. A repeated token cannot mean more than the
///     single token does, so it is an author error rather than a
///     shorthand.
fn parse_sce_unhandled(
    node: &roxmltree::Node,
    element_label_fn: impl Fn() -> String,
    source_name: &str,
) -> Result<Vec<String>, crate::forge::error::Located<crate::forge::error::ForgeError>> {
    use crate::forge::error::{Located, ValidationError};
    use crate::forge::model::SCE_NAMESPACE;

    let pos_of = || node.document().text_pos_at(node.range().start);
    let reject = |attr: &str, value: String, expected: String| {
        let pos = pos_of();
        Located::new(
            ValidationError::InvalidAttribute {
                element: element_label_fn(),
                attr: attr.to_string(),
                value,
                expected,
            }
            .into(),
            source_name,
            Some(pos.row),
            Some(pos.col),
        )
    };

    if let Some(withdrawn) = node.attribute((SCE_NAMESPACE, "exhaustive")) {
        return Err(reject(
            "sce:exhaustive",
            withdrawn.to_string(),
            "sce:unhandled=\"<event names>\" on each child that deliberately \
             leaves the event unhandled — the parent-level opt-out was \
             withdrawn because it silenced gaps its author never saw"
                .to_string(),
        ));
    }

    let raw = match node.attribute((SCE_NAMESPACE, "unhandled")) {
        Some(s) => s,
        None => return Ok(Vec::new()),
    };

    let mut out: Vec<String> = Vec::new();
    for tok in raw.split_whitespace() {
        if tok == "*" || tok == ".*" || tok.ends_with(".*") || tok.contains('*') {
            return Err(reject(
                "sce:unhandled",
                tok.to_string(),
                "a literal event name — wildcards cannot be checked against \
                 the literal gap set"
                    .to_string(),
            ));
        }
        if out.iter().any(|seen| seen == tok) {
            return Err(reject(
                "sce:unhandled",
                tok.to_string(),
                "each event named at most once".to_string(),
            ));
        }
        out.push(tok.to_string());
    }
    if out.is_empty() {
        return Err(reject(
            "sce:unhandled",
            raw.to_string(),
            "at least one event name".to_string(),
        ));
    }
    Ok(out)
}

impl Default for SCXMLParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Pre-extracted attribute bundle for [`SCXMLParser::parse_mesh_rpc_invoke`].
/// Five fields the caller already pulled from distinct attribute paths
/// (`id`, the parser-synthesised `_N` field suffix, `src`, `srcexpr`,
/// `idlocation`) bundled so the parse signature stays under clippy's
/// 7-arg ceiling without losing the per-field meaning.
struct MeshRpcInvokeAttrs {
    invoke_id: String,
    field_suffix: String,
    src: String,
    srcexpr: String,
    idlocation: String,
}

impl SCXMLParser {
    pub fn new() -> Self {
        Self {
            document_order_counter: 0,
            document_order_by_node: BTreeMap::new(),
            invoke_counter: 0,
            hybrid_invoke_counter: 0,
            send_counter: 0,
            invoke_ids_seen: BTreeSet::new(),
            preprocessor_deps: Vec::new(),
            include_dirs: Vec::new(),
        }
    }

    /// Configure the `--include-dir` search path used to resolve
    /// `<xi:include href="...">` and `<sce:use template="...">`
    /// fragments by name. Directories are tried in declaration order
    /// after the including file's own directory and before the cwd
    /// fallback (see [`expand_preprocessors`]). Consumes and returns
    /// `self` so it chains off [`SCXMLParser::new`]:
    ///
    /// ```ignore
    /// let mut parser = SCXMLParser::new().with_include_dirs(dirs);
    /// let model = parser.parse_file(path)?;
    /// ```
    pub fn with_include_dirs(mut self, dirs: Vec<PathBuf>) -> Self {
        self.include_dirs = dirs;
        self
    }

    /// Canonical paths of every external file consumed by the most
    /// recent [`parse_file`] (xi:include targets, sce:use template
    /// fragments). Empty after [`parse_string`] (no fs access) and
    /// before the first [`parse_file`] succeeds.
    ///
    /// Consumed by `sce-codegen --write-deps` to extend the Make-style
    /// depfile with fragment inputs so CMake/Ninja invalidate
    /// generated artifacts when a fragment changes.
    ///
    /// [`parse_file`]: SCXMLParser::parse_file
    /// [`parse_string`]: SCXMLParser::parse_string
    pub fn preprocessor_deps(&self) -> &[PathBuf] {
        &self.preprocessor_deps
    }

    /// Parse an SCXML file from disk.
    ///
    /// The error type is `Located<ForgeError>`: location is part of the
    /// error contract — every failure ties back to the file path so
    /// downstream diagnostics (CLI NDJSON, build scripts, consumers) do
    /// not have to attach file context after the fact. I/O errors
    /// carry the path as `ForgeError::Io`; XML / validation errors
    /// use the file stem as the location label to match the naming
    /// W3C batch diagnostics expect.
    pub fn parse_file(
        &mut self,
        scxml_path: &str,
    ) -> Result<SCXMLModel, crate::forge::error::Located<crate::forge::error::ForgeError>> {
        use crate::forge::error::{ForgeError, Located, XmlError};
        let content = std::fs::read_to_string(scxml_path).map_err(|e| {
            // §wire-W4 D2: distinguish "file not found" from generic
            // I/O failure so the wire surface can route the
            // parser-entry retry strategy. Other I/O failures
            // (permission denied, busy, etc.) keep flowing through
            // `ForgeError::Io` so the distinction stays semantically
            // meaningful — the dispatch lambda in
            // `Diagnostic_test.cpp::ParseErrorConsumer` keys on
            // `xml/file-not-found` for path-retry, not on a generic
            // I/O bucket.
            if e.kind() == std::io::ErrorKind::NotFound {
                Located::new(
                    ForgeError::Xml(XmlError::FileNotFound {
                        path: scxml_path.to_string(),
                    }),
                    scxml_path,
                    None,
                    None,
                )
            } else {
                Located::new(
                    ForgeError::Io {
                        path: Path::new(scxml_path).to_path_buf(),
                        source: e,
                    },
                    scxml_path,
                    None,
                    None,
                )
            }
        })?;
        let name = Path::new(scxml_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        // The document label diagnostics carry is the path the caller
        // named, not its basename. `location.file` is what a consumer
        // opens to apply a fix, and it feeds the `id` hash — so a
        // producer that shortens it cannot share a dedup key with one
        // that does not, which is exactly how the parse-time
        // diagnostics diverged from the CLI-boundary ones inside this
        // very binary. Artifacts still get the basename, derived at
        // the two sites that need it (`artifact_label`).
        let diag_label = scxml_path.to_string();
        let base_dir = Path::new(scxml_path).parent().map(|p| p.to_path_buf());

        // Clear any deps captured by a previous parse on this parser
        // before driving the preprocessors — `preprocessor_deps`
        // describes the *current* file's transitive inputs only.
        self.preprocessor_deps.clear();
        // Clone the configured search path so the `&self` borrow ends
        // before `parse_impl` takes `&mut self` below; the list is tiny
        // (operator-supplied include dirs) and cloned once per parse.
        let extra_dirs = self.include_dirs.clone();
        let (expanded, final_map, deps) =
            expand_preprocessors(&content, scxml_path, base_dir.as_deref(), &extra_dirs)?;
        self.preprocessor_deps = deps;

        self.parse_impl(
            &expanded,
            DocumentLabel {
                identifier: &name,
                diagnostic_label: &diag_label,
            },
            base_dir.as_deref(),
        )
        .map(|mut model| {
            // The positions the parse recorded index into `expanded`.
            // Hand the model the mapping so validators running after
            // this point can report authored coordinates — the error
            // path already gets that through `remap_post_expansion`,
            // and the success path carries positions that outlive it.
            model.authored_positions = Some(crate::model::AuthoredPositions {
                expanded: expanded.clone(),
                map: final_map.clone(),
            });
            model
        })
        .map_err(|err| remap_post_expansion(err, &expanded, &final_map))
    }

    /// Parse SCXML from a string (no filesystem access).
    /// Suitable for WASM and in-memory code generation.
    ///
    /// In-memory callers carry no filesystem extension to distinguish,
    /// so `name` plays both roles of [`DocumentLabel`] — model identifier
    /// (`SCXMLModel.name`) and diagnostic label (`location.file` on every
    /// emitted record).
    pub fn parse_string(
        &mut self,
        content: &str,
        name: &str,
    ) -> Result<SCXMLModel, crate::forge::error::Located<crate::forge::error::ForgeError>> {
        self.parse_impl(content, DocumentLabel::symmetric(name), None)
    }

    /// Parse SCXML from a string with distinct identifier vs diagnostic-
    /// label. Used by the inline-`<content>` synth-invoke path so the
    /// child's model identifier (synth name, extension-free, drives
    /// template symbols) and diagnostic file label (synth name + `.scxml`,
    /// matches the pre-refactor on-disk-synth byte goldens) are both
    /// authoritative without crossing the [`DocumentLabel::symmetric`]
    /// contract.
    pub fn parse_string_with_label(
        &mut self,
        content: &str,
        label: DocumentLabel<'_>,
    ) -> Result<SCXMLModel, crate::forge::error::Located<crate::forge::error::ForgeError>> {
        self.parse_impl(content, label, None)
    }

    /// Parse every sibling Forge document this statechart imports, keyed
    /// by file stem — the registry shape both canonical per-statechart
    /// resolvers
    /// ([`crate::forge::event_schema_check::resolve_imported_event_schemas`]
    /// and [`crate::forge::event_schema_check::resolve_imported_enums`])
    /// consume. Returns `(event_schemas_by_stem, enums_by_stem)`.
    ///
    /// Mirrors the build orchestrator's forge-doc parse — file stem as
    /// [`DocumentLabel::identifier`], so a parsed model's `name` equals
    /// its stem — then hands the registries to the canonical resolvers so
    /// the stem→event-name and stem→alias resolution keeps a single
    /// source of truth.
    ///
    /// EventSchema and Enum are resolved in the same walk because the
    /// receive-/send-side typecheck needs both: an enum-typed field's
    /// literal-width narrowing resolves the field's `enum:<alias>` against
    /// the statechart's own Enum imports. Walking only the EventSchema
    /// half would leave this parse path's validators strictly weaker than
    /// the multi-doc build's — the exact divergence class this seam
    /// exists to prevent.
    ///
    /// Best-effort: an unreadable or non-Forge sibling is silently skipped,
    /// mirroring the resolvers' conservative-defensive skip. A skipped
    /// sibling leaves its events schemaless, which keeps them on the
    /// dynamic `_event.data` baseline — the typed path is never entered on
    /// a schema the parser could not read, so codegen and the validators
    /// stay in agreement about what is in scope.
    fn parse_imported_forge_siblings(
        model: &SCXMLModel,
        base_dir: &Path,
    ) -> (
        std::collections::BTreeMap<String, crate::forge::model::EventSchemaModel>,
        std::collections::BTreeMap<String, crate::forge::model::EnumModel>,
    ) {
        use crate::forge::model::{ForgeDocument, ForgeKind};
        let mut schemas: std::collections::BTreeMap<String, crate::forge::model::EventSchemaModel> =
            std::collections::BTreeMap::new();
        let mut enums: std::collections::BTreeMap<String, crate::forge::model::EnumModel> =
            std::collections::BTreeMap::new();
        for import in &model.forge_imports {
            if !matches!(import.kind, ForgeKind::EventSchema | ForgeKind::Enum) {
                continue;
            }
            let Ok(content) = std::fs::read_to_string(base_dir.join(&import.src)) else {
                continue;
            };
            let Some(stem) = Path::new(&import.src).file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let basename = Path::new(&import.src)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(stem);
            let label = DocumentLabel {
                identifier: stem,
                diagnostic_label: basename,
            };
            match crate::forge::parser::parse_forge(&content, label) {
                Ok(Some(ForgeDocument::EventSchema(schema))) => {
                    schemas.entry(schema.name.clone()).or_insert(schema);
                }
                Ok(Some(ForgeDocument::Enum(em))) => {
                    enums.entry(em.name.clone()).or_insert(em);
                }
                _ => {}
            }
        }
        (schemas, enums)
    }

    /// Two-role label contract — see [`DocumentLabel`]. `label.identifier`
    /// is the pure identifier stored in [`SCXMLModel::name`] (flows into
    /// template symbols, must be extension-free). `label.diagnostic_label`
    /// is the file label used by XSD `source_label`, every outer
    /// `Located::new(..., diagnostic_label, ...)` raise-site, and every
    /// helper `source_name` parameter threaded downstream. Should carry
    /// the full basename so `location.file` on NDJSON records is enough
    /// for downstream tooling to open the source without guessing the
    /// suffix.
    fn parse_impl(
        &mut self,
        content: &str,
        label: DocumentLabel<'_>,
        base_dir: Option<&Path>,
    ) -> Result<SCXMLModel, crate::forge::error::Located<crate::forge::error::ForgeError>> {
        use crate::forge::error::{ForgeError, Located, XmlError};
        let DocumentLabel {
            identifier: name,
            diagnostic_label: diag_label,
        } = label;

        // W3C SCXML + sce: namespace schema validation. Runs before any
        // structural parsing so malformed documents fail fast at the
        // system boundary with libxml2's line/column diagnostics. The
        // schema (`schemas/sce-forge.xsd`) is permissive for W3C SCXML
        // structural elements (xs:any lax) and strict for sce:* — pure
        // statechart documents pass through trivially while inline forge
        // kinds on <data> still get their sce: attributes validated.
        // Not silently skipped when validation cannot run. `validate_or_skip`
        // used to return `Ok(())` for both "validated clean" and "no schema
        // reachable", so a build that stopped validating at the system boundary
        // reported the same success as one that validated — the exact
        // degradation this gate exists to prevent. It now returns which case it
        // is, and a non-validating parse says so once per process on stderr.
        // Once, not per document: N identical lines on a batch run train the
        // reader to ignore them, which is silence with extra steps.
        let outcome =
            crate::forge::xsd_validator::validate_or_skip(content, diag_label).map_err(|errs| {
                Located::new(
                    ForgeError::Xml(XmlError::SchemaValidation(errs)),
                    diag_label,
                    None,
                    None,
                )
            })?;
        crate::forge::xsd_validator::warn_if_not_validated(outcome);

        let doc = roxmltree::Document::parse(content).map_err(|e| {
            // roxmltree reports row/col for every error via `pos()`;
            // passing it through preserves actionable location data
            // instead of fabricating `(1, 1)` at the parser boundary.
            let pos = e.pos();
            Located::new(
                ForgeError::Xml(XmlError::Parse(e.to_string())),
                diag_label,
                Some(pos.row),
                Some(pos.col),
            )
        })?;
        let root = doc.root_element();

        // §wire-W4 D2: catch the previously-silent failure mode where
        // the SCXML pipeline is asked to compile a non-SCXML document
        // (root tag isn't `<scxml>` in the SCXML namespace). Without
        // this check, `parse_states` walks an unrecognised tree and
        // yields an empty model — a `feedback_silently_broken_hooks.md`
        // situation. The `classify_document` router upstream sends
        // `<sce:codec>` etc. to the Forge pipeline before they reach
        // here, so this guard only fires for genuinely-misclassified
        // or hand-mangled input. The namespace gate (parallels
        // `sce/src/parsing/SCXMLParser.cpp::parseInternal`) rejects
        // `<framework:scxml>` foreign-NS roots that share the local
        // name; XSD validation upstream catches most of these but
        // this is defense-in-depth and documents the invariant in
        // code at the model-construction boundary.
        if root.tag_name().name() != "scxml" || !is_scxml_ns(&root) {
            return Err(Located::new(
                ForgeError::Xml(XmlError::WrongRootElement {
                    found: root.tag_name().name().to_string(),
                }),
                diag_label,
                None,
                None,
            ));
        }

        // A forge document wears the same `<scxml>` root, so the guard
        // above cannot see it. Left alone it parses to a stateless model
        // and the author is told "No state nodes found in SCXML document"
        // — a repair instruction pointing at the one thing that is not
        // wrong. SCE_ERROR_CONTRACT.md §4.1 makes `sce:kind` the routing
        // key, and `classify_document` is the primitive; the CLI asks it
        // and the library entries did not, so only build.rs consumers
        // got the misleading answer.
        if let Ok(Some(kind)) = crate::forge::parser::detect_kind_from_node(&root) {
            if kind != crate::forge::model::ForgeKind::Statechart {
                return Err(Located::new(
                    crate::forge::error::ValidationError::WrongPipeline {
                        kind,
                        pipeline: crate::Pipeline::Scxml,
                    }
                    .into(),
                    diag_label,
                    None,
                    None,
                ));
            }
        }

        // The same precondition the forge parse entry holds: a document
        // reaching model construction has been through expansion.
        // `parse_file` runs the expander itself, so only the in-memory
        // entries can arrive unexpanded — and `parse_states` selects
        // children by tag name, so a surviving directive is skipped and
        // the states it was carrying are absent from a model that
        // reports no error.
        reject_unexpanded_directives(&root, diag_label)?;

        // §scxml-3.6: Get initial attribute
        let mut initial = root.attribute("initial").unwrap_or("").to_string();
        if initial.is_empty() {
            // Default to first child state in document order
            for child in root.children() {
                if child.is_element() && is_scxml_state_element(&child) {
                    if let Some(id) = child.attribute("id") {
                        initial = id.to_string();
                        break;
                    }
                }
            }
        }

        // SCE Protocol-Synthesis RFC §synth-5-J-2 + §synth-5-L:
        // `<scxml sce:capacity="N">` declares the
        // per-document event-queue capacity. Two-pass extraction:
        // (1) read the namespaced attribute via the SCE_NAMESPACE
        // URI; (2) if present, parse as u32 and reject zero / non-
        // numeric values with `validation/invalid-attribute`. Absent
        // ⇒ `None`, deploy.yaml `default_event_queue_capacity`
        // fallback applies later in the toolchain (populator hook
        // mirrors the `cache_platform` precedent in
        // `compile_forge_with_deploy`). The value feeds the
        // `EVENT_QUEUE_CAPACITY` bound of the heapless event queue
        // in `--no-std` emission.
        let event_queue_capacity =
            match root.attribute((crate::forge::model::SCE_NAMESPACE, "capacity")) {
                None => None,
                Some(raw) => match raw.parse::<u32>() {
                    Ok(n) if n > 0 => Some(n),
                    _ => {
                        return Err(crate::forge::error::Located::new(
                            crate::forge::error::ValidationError::InvalidAttribute {
                                element: "scxml".to_string(),
                                attr: "sce:capacity".to_string(),
                                value: raw.to_string(),
                                expected: "positive u32".to_string(),
                            }
                            .into(),
                            diag_label,
                            None,
                            None,
                        ));
                    }
                },
            };

        // SCE Protocol-Synthesis RFC §synth-5-O: anchor the model at the
        // `<scxml>` root element's post-preprocessor position. Codegen
        // templates lower this to the top-level SCE-MAP marker above
        // the generated state machine. XInclude / sce:template
        // expansion already remapped row/col onto the included source
        // via `expand_preprocessors` → `remap_post_expansion`, so the
        // recorded line points at the source the author actually
        // wrote, not the post-expansion outer document.
        let root_pos = root.document().text_pos_at(root.range().start);
        let root_source_location = Some(crate::forge::error::SourceLocation {
            file: artifact_label(diag_label),
            line: Some(root_pos.row),
            col: Some(root_pos.col),
        });

        // Capture `<sce:import>` declarations on the statechart root.
        // The same parser pass used by Forge documents (codec / lookup
        // / event-schema / …) so the wire shape and rejection
        // semantics (missing `src`, missing `kind`, unknown kind,
        // duplicate alias) stay byte-identical with the Forge side.
        //
        // Per-statechart visibility lets validators decide which
        // schemas are in-scope for THIS document, replacing the
        // legacy single-global-registry approach where every kind
        // declaration anywhere in the build became visible to every
        // statechart. Downstream consumers:
        //
        //   * receive-side EventSchema typecheck
        //     (`forge::event_schema_check::check`) — filters to
        //     event-schema imports declared on this statechart so a
        //     schemaless statechart keeps the dynamic `_event.data`
        //     baseline even when other statecharts in the same build
        //     declare schemas.
        //   * send-side EventSchema typecheck
        //     (`forge::event_schema_check::check_send_side`) — same
        //     filter, applied at `<send>` / `<raise>` payload sites.
        //   * mesh cross-machine validator
        //     (`mesh::deploy::validate_event_schemas_cross_machine`)
        //     — compares per-machine import visibility so cross-
        //     machine sends whose sender and receiver declare
        //     different schemas (or only one side declares one)
        //     surface as `mesh/event-schema-mismatch`.
        let forge_imports = crate::forge::parser::parse_imports(&root, diag_label)?;

        let mut model = SCXMLModel {
            name: name.to_string(),
            scxml_name: root.attribute("name").unwrap_or("").to_string(),
            initial,
            binding: root.attribute("binding").unwrap_or("early").to_string(),
            datamodel: resolve_declared_datamodel(root.attribute("datamodel"))
                .map_err(|e| crate::forge::error::Located::new(e.into(), diag_label, None, None))?,
            event_queue_capacity,
            source_location: root_source_location,
            forge_imports,
            ..Default::default()
        };

        // §scxml-B-1: refuse constructs the declared data model has no
        // language for, before anything downstream reads them. Placed
        // ahead of every other parse step because the alternative is to
        // build a model whose expressions were never in a language the
        // document named — the state the `datamodel` attribute existed to
        // prevent and, until now, did not.
        enforce_datamodel_languages(&root, model.datamodel)
            .map_err(|e| crate::forge::error::Located::new(e.into(), diag_label, None, None))?;

        // Parse datamodel
        self.parse_datamodel(&root, &mut model, diag_label)?;

        // Parse global scripts
        self.parse_global_scripts(&root, &mut model, base_dir, diag_label);

        // Parse Named Context declarations (must be before states for transforms)
        self.parse_sce_contexts(&root, &mut model, diag_label)?;

        // Top-level `<sce:driver
        // href="..."/>` references. Resolution against `deploy.yaml`'s
        // `platform.driver_root` happens at compile-model time; this
        // pass only captures the verbatim author-written strings and
        // their source positions so a missing `href` surfaces at parse
        // time and the codegen-time `#include` emit has a stable
        // document-order index.
        self.parse_sce_drivers(&root, &mut model, diag_label)?;

        // Capture every top-level
        // `<sce:session-role kind="..."/>` declaration on the SCXML
        // root. The orchestrator reads `declared_session_
        // roles` to drive the cross-doc listener-pair join (it
        // replaced an earlier `Accepting.*` substate string-match).
        // This pass
        // surfaces parse-time `scxml/unknown-session-role-kind` (kind
        // outside the v1 vocabulary) and `scxml/duplicate-session-
        // role-declaration` (same kind declared twice on one doc).
        self.parse_sce_session_roles(&root, &mut model, diag_label)?;

        // Rank state elements by document position before parsing them —
        // `parse_states` visits one element name at a time, so it cannot
        // number a mixed sibling set correctly on its own.
        self.assign_document_order(&root);

        // Parse states recursively
        self.parse_states(&root, None, &mut model, base_dir, diag_label)?;

        // SCE Protocol-Synthesis RFC §synth-5-E — `<sce:on-sample>`
        // structural validators run immediately after the states pass
        // so the diagnostic surfaces before any downstream derivation
        // (feature detection, parallel-region computation, etc.) can
        // mis-interpret a malformed extension. Three checks in fixed
        // order:
        //   1. placement (parent must be `<state>` or `<parallel>`),
        //   2. per-state link uniqueness,
        //   3. event-name vs W3C internal-event-prefix collision.
        // First failure short-circuits — the early author-facing
        // signal beats accumulating multiple structural diagnostics
        // for the same well-known authoring mistake.
        validate_on_sample_placement(&root, diag_label)?;
        validate_on_sample_uniqueness(&model, diag_label)?;
        validate_on_sample_event_names(&model, diag_label)?;
        validate_on_sample_callback_paths(&model, diag_label)?;

        // Session-role naming guard —
        // an SCXML carrying any `Accepting` or `Accepting.*` state id
        // claims the canonical session-FSM accept-side state shape;
        // it must declare the role explicitly via
        // `<sce:session-role kind="accept-side"/>` or it is either a
        // missing role declaration or a name collision. The walker
        // (formerly the substate-driven join in
        // `resolve_listener_links`) lives here so the partial-claim
        // surfaces at parse time, independently of any deploy.yaml.
        validate_axis3_accept_side_state_naming(&model, diag_label)?;

        // Feature detection
        self.detect_features(&mut model);

        // Named Context validation
        self.validate_context_usage(&model, diag_label)?;

        // Resolve deep initial state
        self.resolve_deep_initial(&mut model);

        // §scxml-3.13: Apply parallel initial overrides
        self.apply_parallel_initial_overrides(&mut model);

        // Resolve history targets
        self.resolve_history_targets(&mut model);

        // Compute history leaf targets (resolve default_target to leaf state)
        let history_ids: Vec<String> = model.history_states.keys().cloned().collect();
        for hid in &history_ids {
            if let Some(info) = model.history_states.get(hid) {
                let leaf = model.resolve_to_leaf(&info.default_target);
                let info_mut = model.history_states.get_mut(hid).unwrap();
                info_mut.leaf_target = leaf;
            }
        }

        // Compute initial_leaf
        if !model.initial.is_empty() {
            model.initial_leaf = model.resolve_to_leaf(&model.initial);
        }

        // Compute parallel regions
        self.compute_parallel_regions(&mut model);

        // Detect transition/entry/exit actions, hierarchy
        self.detect_transition_actions(&mut model);
        self.detect_entry_exit_actions(&mut model);
        self.detect_hierarchy(&mut model);

        // Add done.state events
        self.add_done_state_events(&mut model);

        // Set invoke event flags
        self.set_invoke_event_flags(&mut model);

        // Collect child→parent events
        self.collect_child_to_parent_events(&mut model, base_dir);

        // Parse initial_children
        self.parse_initial_children(&mut model);

        // EventSchema MCU native lowering
        // — resolve `<sce:import kind="event-schema">` siblings into the
        // `event_name → EventSchemaModel` map BEFORE the script-engine
        // analyzer runs, so a transition guard reading a typed
        // `_event.data.<field>` whose event carries an imported schema is
        // recognised as natively lowerable (no runtime script engine —
        // the form no_std MCU targets require). The in-memory
        // `parse_string` path (WASM) passes `base_dir = None` and has no
        // sibling files to follow, so it keeps the dynamic `_event.data`
        // String baseline.
        //
        // [`SCXMLModel::imported_event_schemas`] is written here and
        // nowhere else, and it is the map every downstream typed-path
        // consumer reads: the script-engine analyzer's lowerability
        // verdict, the `native_typed_inject_events` switchboard, and all
        // six backend payload builders. Resolution is therefore the seam
        // that *admits* a document to the typed path, so it is also where
        // the typed path's validators must run — against this very map,
        // not a separately-resolved one.
        //
        // Validating here rather than in the build orchestrator is
        // load-bearing, not incidental. Codegen entry points that parse a
        // single document (`sce-codegen generate`) never reach
        // `compile_scxml_with_imports`, so validators hung off the
        // orchestrator do not run for them: an unresolvable
        // `_event.data.<field>` was lowered into a payload-struct field
        // access that does not exist, and the defect surfaced as a
        // compile error in the *generated* code with nothing pointing back
        // at the offending `cond`. Any future entry point that parses a
        // document is now validated by construction.
        if let Some(dir) = base_dir {
            let (schemas_by_stem, enums_by_stem) = Self::parse_imported_forge_siblings(&model, dir);
            model.imported_event_schemas =
                crate::forge::event_schema_check::resolve_imported_event_schemas(
                    &model,
                    &schemas_by_stem,
                );
            let imported_enums =
                crate::forge::event_schema_check::resolve_imported_enums(&model, &enums_by_stem);
            crate::forge::event_schema_check::check(
                &model,
                &model.imported_event_schemas,
                &imported_enums,
                diag_label,
            )?;
            crate::forge::event_schema_check::check_send_side(
                &model,
                &model.imported_event_schemas,
                &imported_enums,
                diag_label,
            )?;
            // §scxml-G-7 — `<sce:action>` native host dispatch is
            // engine-free by definition, so a non-conforming construct is
            // a rejection rather than a degradation to the script engine.
            // It reads the same map, so it belongs to the same seam.
            crate::forge::native_action::validate(
                &model,
                &model.imported_event_schemas,
                diag_label,
            )?;
        }

        // SCE script-engine requirement — single source of truth. See
        // [`crate::script_engine_analyzer`]. Must run before the
        // `needs_nonstatic_method` derivation below (which reads the
        // flag) and after every parse step that populates the model
        // elements the analyzer walks (variables, states, invokes,
        // donedata). Parser sub-routines no longer set this flag; each
        // former write site is now a [`NeedsScriptEngineCause`] variant.
        // One traversal, both results. The flag is the boolean projection
        // of the cause list, so deriving them from a single `analyze` call
        // makes `needs_script_engine == !script_engine_causes.is_empty()`
        // true by construction — a later pass cannot mutate the model into
        // a state where the flag and its own explanation disagree.
        model.script_engine_causes = crate::script_engine_analyzer::analyze(&model);
        model.needs_script_engine = !model.script_engine_causes.is_empty();

        // Compute needs_nonstatic_method
        model.needs_nonstatic_method = model.needs_script_engine
            || model.has_scxml_invoke()
            || model.has_parent_communication
            || model.has_parallel_states
            || model.uses_in_predicate
            || !model.context_objects.is_empty();

        // Document rejection → redirect initial to "pass"
        if model.document_rejected {
            if let Some(state_id) = model
                .states
                .iter()
                .find(|(_, s)| s.is_final && s.id.to_lowercase() == "pass")
                .map(|(id, _)| id.clone())
            {
                model.initial = state_id.clone();
                model.initial_leaf = state_id;
            }
        }

        // Assign transition indices
        for state in model.states.values_mut() {
            for (i, trans) in state.transitions.iter_mut().enumerate() {
                trans.transition_index = i;
            }
        }

        // State-level `invokes` is authoritative; `SCXMLModel.invokes` is a
        // template-visible flat view. Build it once, here, after all
        // per-state mutations are finalised.
        model.refresh_invokes_view();

        Ok(model)
    }

    fn parse_datamodel(
        &mut self,
        root: &roxmltree::Node,
        model: &mut SCXMLModel,
        source_name: &str,
    ) -> Result<(), crate::forge::error::Located<crate::forge::error::ForgeError>> {
        use crate::forge::model::SCE_NAMESPACE;

        for child in root.children() {
            if !child.is_element() || local_name(&child) != "datamodel" {
                continue;
            }
            for data in child.children() {
                if !data.is_element() || local_name(&data) != "data" {
                    continue;
                }

                // SCE Forge: detect inline kind on <data sce:kind="..."> — classify
                // as InlineKind instead of Variable (single XML parse, no re-parsing).
                if let Some(kind_attr) = data.attribute((SCE_NAMESPACE, "kind")) {
                    // Unknown/non-inline kind — fall through to variable.
                    if let Some(inline) =
                        Self::try_parse_inline_kind(&data, kind_attr, source_name)?
                    {
                        model.inline_kinds.push(inline);
                        continue;
                    }
                }

                let var_id = data.attribute("id").unwrap_or("").to_string();
                let expr = data.attribute("expr").unwrap_or("").to_string();
                let src = data.attribute("src").unwrap_or("").to_string();

                let content = if src.is_empty() {
                    if data.children().any(|c| c.is_element()) {
                        // Serialize child elements as XML string
                        let mut xml = String::new();
                        for c in data.children().filter(|c| c.is_element()) {
                            xml.push_str(&serialize_node(&c));
                        }
                        xml
                    } else {
                        data.text().unwrap_or("").trim().to_string()
                    }
                } else {
                    String::new()
                };

                model.variables.push(Variable {
                    id: var_id,
                    expr,
                    src,
                    content,
                    var_type: String::new(),
                    source_location: source_location_of(&data, source_name),
                });
                // `needs_script_engine` is derived post-parse by
                // [`crate::script_engine_analyzer`] —
                // [`NeedsScriptEngineCause::DatamodelVariableInit`].
            }
        }
        Ok(())
    }

    /// SCE Forge: attempt to parse a <data sce:kind="..."> element as an inline kind.
    /// Returns `Ok(Some(kind))` on success, `Ok(None)` for unknown/non-inline kinds
    /// (fall through to variable), `Err` for recognized inline kinds with invalid content.
    ///
    /// Errors are fatal: a document that opted into `sce:kind=...` has
    /// asserted intent about the shape of this `<data>`, so silently
    /// demoting a malformed inline kind to an ECMAScript variable
    /// would mask the author's intent and hand the generator a type
    /// the rest of the pipeline was never told about (see the
    /// `feedback_silently_broken_hooks` memory). Errors flow through
    /// `Located<ForgeError>` with roxmltree-derived row/col so consumers
    /// receive the same precision the codec-field parser already
    /// delivers; the already-located codec-field error is propagated
    /// unchanged (no `.map_err(|l| l.error)` unwrap).
    fn try_parse_inline_kind(
        data: &roxmltree::Node,
        kind_attr: &str,
        source_name: &str,
    ) -> Result<
        Option<crate::forge::model::InlineKind>,
        crate::forge::error::Located<crate::forge::error::ForgeError>,
    > {
        use crate::forge::error::{Located, ValidationError};
        use crate::forge::model::*;

        // Lift a leaf `ValidationError` into a `Located<ForgeError>`
        // anchored at the given node. `text_pos_at(range().start)`
        // recovers libxml2-style (row, col) for any node the
        // roxmltree document owns — the same mechanism the forge
        // codec-field parser uses, so inline-kind diagnostics reach
        // NDJSON with matching precision.
        let locate_at = |node: &roxmltree::Node,
                         err: ValidationError|
         -> Located<crate::forge::error::ForgeError> {
            let pos = node.document().text_pos_at(node.range().start);
            Located::new(err.into(), source_name, Some(pos.row), Some(pos.col))
        };
        let locate = |err: ValidationError| locate_at(data, err);

        let kind = match ForgeKind::from_attr(kind_attr) {
            Some(k) => k,
            None => return Ok(None), // Unknown kind — treat as regular variable
        };
        if !kind.is_inline_eligible() {
            return Ok(None); // Stateful kind — cannot be inline, treat as variable
        }

        let id = data
            .attribute("id")
            .ok_or_else(|| {
                locate(ValidationError::MissingAttribute {
                    element: format!("inline <data sce:kind=\"{kind}\">"),
                    attr: "id".to_string(),
                })
            })?
            .to_string();

        let sce_attr = |local: &str| -> Option<String> {
            data.attribute((SCE_NAMESPACE, local))
                .map(|s| s.to_string())
        };

        let inline_data = match kind {
            ForgeKind::Lookup => {
                let input_id = sce_attr("input").unwrap_or_default();
                let default_value = sce_attr("default").unwrap_or_default();

                let mut entries = Vec::new();
                for child in data.children().filter(|n| n.is_element()) {
                    if child.tag_name().name() == "entry"
                        && child.tag_name().namespace() == Some(SCE_NAMESPACE)
                    {
                        let key = child
                            .attribute("key")
                            .ok_or_else(|| {
                                locate_at(
                                    &child,
                                    ValidationError::MissingAttribute {
                                        element: format!("<sce:entry> in inline lookup '{id}'"),
                                        attr: "key".to_string(),
                                    },
                                )
                            })?
                            .to_string();
                        let value = child
                            .attribute("value")
                            .ok_or_else(|| {
                                locate_at(
                                    &child,
                                    ValidationError::MissingAttribute {
                                        element: format!("<sce:entry> in inline lookup '{id}'"),
                                        attr: "value".to_string(),
                                    },
                                )
                            })?
                            .to_string();
                        entries.push(LookupEntry { key, value });
                    }
                }
                if entries.is_empty() {
                    return Err(locate(ValidationError::EmptyCollection {
                        kind: ForgeKind::Lookup,
                        what: format!("<sce:entry> (inline lookup '{id}')"),
                    }));
                }

                let final_default = if default_value.is_empty() {
                    entries[0].value.clone()
                } else {
                    default_value
                };

                InlineKindData::Lookup {
                    input_id,
                    entries,
                    default_value: final_default,
                }
            }
            ForgeKind::Condition => {
                let expr = data
                    .attribute("expr")
                    .ok_or_else(|| {
                        locate(ValidationError::MissingAttribute {
                            element: format!("inline condition '{id}' <data>"),
                            attr: "expr".to_string(),
                        })
                    })?
                    .to_string();
                InlineKindData::Condition { expr }
            }
            ForgeKind::Codec => {
                let default_endian = sce_attr("default-endian")
                    .and_then(|s| Endian::from_attr(&s))
                    .unwrap_or(Endian::Big);

                let mut fields = Vec::new();
                for child in data.children().filter(|n| n.is_element()) {
                    if child.tag_name().name() == "field"
                        && child.tag_name().namespace() == Some(SCE_NAMESPACE)
                    {
                        // `parse_codec_field_from_node` already returns
                        // `Located<ForgeError>`. Propagate it through
                        // `?` so the codec parser's row/col data
                        // reaches the wire contract; unwrapping it
                        // here would discard location information
                        // the leaf already computed.
                        fields.push(crate::forge::parser::parse_codec_field_from_node(
                            &child,
                            "<inline codec>",
                        )?);
                    }
                }
                if fields.is_empty() {
                    return Err(locate(ValidationError::EmptyCollection {
                        kind: ForgeKind::Codec,
                        what: format!("<sce:field> (inline codec '{id}')"),
                    }));
                }

                InlineKindData::Codec {
                    fields,
                    default_endian,
                }
            }
            ForgeKind::Transform => {
                let expr = data
                    .attribute("expr")
                    .ok_or_else(|| {
                        locate(ValidationError::MissingAttribute {
                            element: format!("inline transform '{id}' <data>"),
                            attr: "expr".to_string(),
                        })
                    })?
                    .to_string();
                let type_str = sce_attr("type").ok_or_else(|| {
                    locate(ValidationError::MissingAttribute {
                        element: format!("inline transform '{id}' <data>"),
                        attr: "sce:type".to_string(),
                    })
                })?;
                let output_type = SceType::from_attr(&type_str).ok_or_else(|| {
                    locate(ValidationError::InvalidAttribute {
                        element: format!("inline transform '{id}' <data>"),
                        attr: "sce:type".to_string(),
                        value: type_str.clone(),
                        expected: "uint8|uint16|uint32|uint64|int8|int16|int32|int64|float32|float64|bool|string|bytes"
                            .to_string(),
                    })
                })?;

                InlineKindData::Transform {
                    inputs: Vec::new(),
                    expr,
                    output_type,
                }
            }
            _ => return Ok(None),
        };

        Ok(Some(InlineKind {
            id,
            data: inline_data,
        }))
    }

    fn parse_global_scripts(
        &mut self,
        root: &roxmltree::Node,
        model: &mut SCXMLModel,
        base_dir: Option<&Path>,
        source_name: &str,
    ) {
        for child in root.children() {
            if !child.is_element() || local_name(&child) != "script" {
                continue;
            }
            let src = child.attribute("src").unwrap_or("").to_string();
            let mut content = child.text().unwrap_or("").to_string();

            // §scxml-5.8: "A conformant SCXML document MUST specify
            // either the 'src' attribute or child content, but not
            // both." Both halves of that sentence reject the document:
            // neither is the empty `<script/>`, both is a `src` with a
            // body beside it. Only the first was implemented, so a
            // document naming a script twice over parsed cleanly and
            // the `src` silently won.
            if src.is_empty() && content.trim().is_empty() {
                model.document_rejected = true;
                continue;
            }
            if !src.is_empty() && !content.trim().is_empty() {
                model.document_rejected = true;
                continue;
            }

            if !src.is_empty() {
                if let Some(dir) = base_dir {
                    let normalized = src
                        .strip_prefix("file://")
                        .or_else(|| src.strip_prefix("file:"))
                        .unwrap_or(&src);
                    let script_path = dir.join(normalized);
                    match std::fs::read_to_string(&script_path) {
                        Ok(c) => content = c,
                        Err(_) => {
                            model.document_rejected = true;
                            continue;
                        }
                    }
                } else {
                    // No filesystem access (WASM) — skip external scripts.
                    // Record the document fact so the analyzer can surface
                    // [`NeedsScriptEngineCause::UnresolvedExternalScript`]
                    // even though `global_scripts` stays empty.
                    model.has_unresolved_external_script = true;
                    continue;
                }
            }

            model.global_scripts.push(Action {
                action_type: "script".to_string(),
                content: content.trim().to_string(),
                // §synth-5-O: this `Action` is emission-eligible, so
                // `forge::provenance` requires the coordinate. The
                // top-level `<script>` does not travel through
                // `parse_executable_content_single` — the site that
                // stamps every other action — so the stamp has to
                // happen here or the node reaches codegen with a
                // silent `None`.
                source_location: source_location_of(&child, source_name),
                ..Default::default()
            });
            // [`NeedsScriptEngineCause::GlobalScript`] —
            // derived post-parse from `model.global_scripts`.
        }
    }

    /// Pin every state element's `document_order` to its position in the
    /// document, keyed by XML node identity.
    ///
    /// Pre-order traversal IS document order for XML, so this walk hands
    /// out the same dense numbering the parse-order counter used to, only
    /// in the right order. It ranks every state element it meets and
    /// recurses through all of them, so the set it numbers is a superset
    /// of what [`Self::parse_states`] (which skips id-less elements and
    /// does not descend into them) later looks up.
    fn assign_document_order(&mut self, elem: &roxmltree::Node) {
        // §scxml-3.2 and §scxml-3.3 both make "the first child state in
        // document order" the default initial state, and the rank is emitted
        // into generated code as the state's document-order index (the
        // conflict-resolution and exit-set order every backend reads). But
        // `parse_states` visits one element name at a time — every <state>,
        // then every <final>, then every <parallel> — so a counter bumped as
        // elements are parsed ranks a mixed sibling set by element name
        // instead of by position, and a <parallel> written before a <state>
        // came out second. Hence a separate pass over document positions.
        for child in scxml_children_any_of(elem, STATE_ELEMENT_TAGS) {
            self.document_order_by_node
                .insert(child.id().get_usize(), self.document_order_counter);
            self.document_order_counter += 1;
            self.assign_document_order(&child);
        }
    }

    /// Document-order rank of a state element, assigned by
    /// [`Self::assign_document_order`]. Indexing is deliberate: that pass
    /// ranks a superset of the elements reaching this lookup, so a miss is
    /// a broken invariant rather than a document the parser should guess a
    /// rank for — and rank 0 would claim "first in document order".
    fn document_order_of(&self, elem: &roxmltree::Node) -> u32 {
        self.document_order_by_node[&elem.id().get_usize()]
    }

    /// Threaded `source_name` lets leaf validation errors
    /// (`parse_invoke` mesh-rpc reserved-param rules) construct
    /// `Located<ForgeError>` records with the same file label the
    /// top-level parse used, without reaching back to `model.name` at
    /// every call site.
    fn parse_states(
        &mut self,
        parent_elem: &roxmltree::Node,
        parent_id: Option<&str>,
        model: &mut SCXMLModel,
        base_dir: Option<&Path>,
        source_name: &str,
    ) -> Result<(), crate::forge::error::Located<crate::forge::error::ForgeError>> {
        // Parse <state> elements
        for child in scxml_children(parent_elem, "state") {
            let state_id = match child.attribute("id") {
                Some(id) => id.to_string(),
                None => continue,
            };

            let mut state = State {
                id: state_id.clone(),
                initial: child.attribute("initial").unwrap_or("").to_string(),
                parent: parent_id.map(|s| s.to_string()),
                document_order: self.document_order_of(&child),
                source_location: source_location_of(&child, source_name),
                ..Default::default()
            };
            state.req =
                collect_sce_req(&child, || format!("<state id=\"{state_id}\">"), source_name)?;
            state.unresolved = collect_sce_unresolved(&child, source_name);
            state.unhandled =
                parse_sce_unhandled(&child, || format!("<state id=\"{state_id}\">"), source_name)?;

            // Parse transitions
            for trans_elem in scxml_children(&child, "transition") {
                let transition = self.parse_transition(&trans_elem, model, source_name)?;
                // Collect event names
                if !transition.event.is_empty() {
                    let ev = &transition.event;
                    if ev != "*" && ev != ".*" && ev != "_*" {
                        for e in ev.split_whitespace() {
                            if e != "*" && e != ".*" && e != "_*" && !e.ends_with(".*") {
                                model.events.insert(e.to_string());
                            }
                        }
                    }
                }
                state.transitions.push(transition);
            }

            // Parse onentry blocks
            for entry_elem in scxml_children(&child, "onentry") {
                let req = collect_sce_req(
                    &entry_elem,
                    || format!("<onentry> in <state id=\"{state_id}\">"),
                    source_name,
                )?;
                let mut block = self.parse_executable_content(&entry_elem, model, source_name)?;
                inherit_req(&req, &mut block);
                if !block.is_empty() {
                    state.on_entry_blocks.push(block);
                }
            }

            // Parse onexit blocks
            for exit_elem in scxml_children(&child, "onexit") {
                let req = collect_sce_req(
                    &exit_elem,
                    || format!("<onexit> in <state id=\"{state_id}\">"),
                    source_name,
                )?;
                let mut block = self.parse_executable_content(&exit_elem, model, source_name)?;
                inherit_req(&req, &mut block);
                if !block.is_empty() {
                    state.on_exit_blocks.push(block);
                }
            }

            // Parse <initial> transition
            if let Some(initial_elem) = scxml_child(&child, "initial") {
                if let Some(initial_trans) = scxml_child(&initial_elem, "transition") {
                    state.initial_transition_actions =
                        self.parse_executable_content(&initial_trans, model, source_name)?;
                    if state.initial.is_empty() {
                        if let Some(target) = initial_trans.attribute("target") {
                            state.initial = target.to_string();
                        }
                    }
                }
            }

            // Parse state-level datamodel
            for dm_elem in scxml_children(&child, "datamodel") {
                for data in scxml_children(&dm_elem, "data") {
                    let content = data.text().unwrap_or("").trim().to_string();
                    state.datamodel.push(Variable {
                        id: data.attribute("id").unwrap_or("").to_string(),
                        expr: data.attribute("expr").unwrap_or("").to_string(),
                        src: data.attribute("src").unwrap_or("").to_string(),
                        content,
                        var_type: String::new(),
                        source_location: source_location_of(&data, source_name),
                    });
                }
            }

            // Parse invokes — parse_invoke returns the typed Invoke variant
            // directly. Unsupported classifications yield `Ok(None)` and
            // are skipped silently; reserved-param violations on
            // `<invoke type="sce:mesh-rpc">` short-circuit via `?`.
            for invoke_elem in scxml_children(&child, "invoke") {
                if let Some(invoke) =
                    self.parse_invoke(&invoke_elem, model, &state_id, base_dir, source_name)?
                {
                    state.invokes.push(invoke);
                }
            }

            // SCE Protocol-Synthesis RFC §synth-5-E:
            // `<sce:on-sample>` is valid inside `<state>` and `<parallel>` only.
            // The AST nodes are collected here; a separate placement
            // validator (`validate_on_sample_placement`) walks the rest of
            // the document for stray nodes outside these two parents.
            collect_on_sample_blocks(&child, &mut state.on_sample_blocks);

            model.states.insert(state_id.clone(), state);

            // Recurse into child states
            self.parse_states(&child, Some(&state_id), model, base_dir, source_name)?;

            // Set default initial (first child in document order)
            let state = model.states.get_mut(&state_id).unwrap();
            if state.initial.is_empty() {
                let first_child = model
                    .states
                    .iter()
                    .filter(|(_, s)| s.parent.as_deref() == Some(&state_id))
                    .filter(|(id, _)| !model.history_states.contains_key(id.as_str()))
                    .min_by_key(|(_, s)| s.document_order);
                if let Some((child_id, _)) = first_child {
                    let child_id = child_id.clone();
                    model.states.get_mut(&state_id).unwrap().initial = child_id;
                }
            }
        }

        // Parse <final> elements
        for child in scxml_children(parent_elem, "final") {
            let final_id = match child.attribute("id") {
                Some(id) => id.to_string(),
                None => continue,
            };

            let mut state = State {
                id: final_id.clone(),
                is_final: true,
                parent: parent_id.map(|s| s.to_string()),
                document_order: self.document_order_of(&child),
                source_location: source_location_of(&child, source_name),
                ..Default::default()
            };
            state.req =
                collect_sce_req(&child, || format!("<final id=\"{final_id}\">"), source_name)?;
            state.unresolved = collect_sce_unresolved(&child, source_name);
            // A `<final>` is excluded from the exhaustiveness comparison
            // (it has no transition surface), so it can never be a
            // non-handler and any declaration here is stale. Reading the
            // attribute is what makes that verdict happen: an element
            // that skipped this call would swallow both the declaration
            // and the withdrawn `sce:exhaustive` in silence.
            state.unhandled =
                parse_sce_unhandled(&child, || format!("<final id=\"{final_id}\">"), source_name)?;

            for entry_elem in scxml_children(&child, "onentry") {
                let req = collect_sce_req(
                    &entry_elem,
                    || format!("<onentry> in <final id=\"{final_id}\">"),
                    source_name,
                )?;
                let mut block = self.parse_executable_content(&entry_elem, model, source_name)?;
                inherit_req(&req, &mut block);
                if !block.is_empty() {
                    state.on_entry_blocks.push(block);
                }
            }
            for exit_elem in scxml_children(&child, "onexit") {
                let req = collect_sce_req(
                    &exit_elem,
                    || format!("<onexit> in <final id=\"{final_id}\">"),
                    source_name,
                )?;
                let mut block = self.parse_executable_content(&exit_elem, model, source_name)?;
                inherit_req(&req, &mut block);
                if !block.is_empty() {
                    state.on_exit_blocks.push(block);
                }
            }

            // Parse donedata
            if let Some(dd_elem) = scxml_child(&child, "donedata") {
                state.donedata = Some(self.parse_donedata(&dd_elem, model.datamodel));
            }

            model.states.insert(final_id, state);
        }

        // Parse <parallel> elements
        for child in scxml_children(parent_elem, "parallel") {
            let parallel_id = match child.attribute("id") {
                Some(id) => id.to_string(),
                None => continue,
            };

            let mut state = State {
                id: parallel_id.clone(),
                is_parallel: true,
                parent: parent_id.map(|s| s.to_string()),
                document_order: self.document_order_of(&child),
                source_location: source_location_of(&child, source_name),
                ..Default::default()
            };
            state.req = collect_sce_req(
                &child,
                || format!("<parallel id=\"{parallel_id}\">"),
                source_name,
            )?;
            state.unresolved = collect_sce_unresolved(&child, source_name);
            // A `<parallel>` carrying transitions is a sibling in the
            // exhaustiveness comparison exactly like a `<state>`, so it
            // can be a non-handler and needs the same way to say the gap
            // is deliberate.
            state.unhandled = parse_sce_unhandled(
                &child,
                || format!("<parallel id=\"{parallel_id}\">"),
                source_name,
            )?;

            for trans_elem in scxml_children(&child, "transition") {
                let transition = self.parse_transition(&trans_elem, model, source_name)?;
                if !transition.event.is_empty() {
                    let ev = &transition.event;
                    if ev != "*" && ev != ".*" && ev != "_*" {
                        for e in ev.split_whitespace() {
                            if e != "*" && e != ".*" && e != "_*" && !e.ends_with(".*") {
                                model.events.insert(e.to_string());
                            }
                        }
                    }
                }
                state.transitions.push(transition);
            }

            for entry_elem in scxml_children(&child, "onentry") {
                let req = collect_sce_req(
                    &entry_elem,
                    || format!("<onentry> in <parallel id=\"{parallel_id}\">"),
                    source_name,
                )?;
                let mut block = self.parse_executable_content(&entry_elem, model, source_name)?;
                inherit_req(&req, &mut block);
                if !block.is_empty() {
                    state.on_entry_blocks.push(block);
                }
            }
            for exit_elem in scxml_children(&child, "onexit") {
                let req = collect_sce_req(
                    &exit_elem,
                    || format!("<onexit> in <parallel id=\"{parallel_id}\">"),
                    source_name,
                )?;
                let mut block = self.parse_executable_content(&exit_elem, model, source_name)?;
                inherit_req(&req, &mut block);
                if !block.is_empty() {
                    state.on_exit_blocks.push(block);
                }
            }

            // SCE Protocol-Synthesis RFC §synth-5-E:
            // `<sce:on-sample>` valid inside `<parallel>` symmetric to
            // `<state>` above. The single helper keeps the two arms in
            // lockstep so a future placement-rule extension touches one
            // call site, not two.
            collect_on_sample_blocks(&child, &mut state.on_sample_blocks);

            model.states.insert(parallel_id.clone(), state);
            model.has_parallel_states = true;
            self.parse_states(&child, Some(&parallel_id), model, base_dir, source_name)?;
        }

        // Parse <history> elements
        for child in scxml_children(parent_elem, "history") {
            let history_id = match child.attribute("id") {
                Some(id) => id.to_string(),
                None => continue,
            };
            let history_type = child.attribute("type").unwrap_or("shallow").to_string();

            let mut default_target = String::new();
            let mut default_actions = Vec::new();
            for trans_elem in scxml_children(&child, "transition") {
                if let Some(target) = trans_elem.attribute("target") {
                    default_target = target.to_string();
                    default_actions =
                        self.parse_executable_content(&trans_elem, model, source_name)?;
                    break;
                }
            }

            // §scxml-3.10.2: the default `<transition>` child is
            // required. Rejecting here rather than dropping the element
            // is what keeps the model faithful to the document — a
            // silently discarded `<history>` left every transition
            // naming it to fall through to `State::<HistoryId>`, a
            // variant the generated enum never declares because a
            // history pseudostate is not a state. The document then
            // passed `check`, passed `generate`, and failed in the
            // consumer's compiler.
            if default_target.is_empty() {
                let parent = parent_id.unwrap_or("").to_string();
                let mut siblings: Vec<&crate::model::State> = model
                    .states
                    .values()
                    .filter(|s| s.parent.as_deref() == parent_id)
                    .collect();
                siblings.sort_by_key(|s| s.document_order);
                return Err(crate::forge::error::Located::new(
                    crate::scxml_semantic::ScxmlSemanticError::HistoryDefaultTransitionMissing {
                        history_id,
                        parent_id: parent,
                        available: siblings.into_iter().map(|s| s.id.clone()).collect(),
                    }
                    .into(),
                    source_name,
                    None,
                    None,
                ));
            }
            model
                .history_default_targets
                .insert(history_id.clone(), default_target.clone());
            model.history_states.insert(
                history_id,
                HistoryInfo {
                    parent: parent_id.unwrap_or("").to_string(),
                    history_type,
                    leaf_target: String::new(), // resolved later
                    default_target,
                    default_actions,
                },
            );
            model.has_history_states = true;
        }
        Ok(())
    }

    fn parse_transition(
        &mut self,
        elem: &roxmltree::Node,
        model: &mut SCXMLModel,
        source_name: &str,
    ) -> Result<Transition, crate::forge::error::Located<crate::forge::error::ForgeError>> {
        let cond = elem
            .attribute("cond")
            .or_else(|| elem.attribute("expr"))
            .unwrap_or("")
            .to_string();

        let mut is_cpp_condition = false;
        let mut is_kt_condition = false;
        let mut is_pure_in = false;
        let mut cond_cpp = String::new();
        let mut cond_cpp_transformed = String::new();
        let mut cond_kt = String::new();

        if let Some(stripped) = cond.strip_prefix("cpp:") {
            is_cpp_condition = true;
            cond_cpp = stripped.to_string();
            cond_cpp_transformed = if !model.context_object_ids.is_empty() {
                transform_cpp_code_with_named_contexts(&cond_cpp, &model.context_object_ids)
            } else {
                cond_cpp.clone()
            };
        } else if let Some(stripped) = cond.strip_prefix("kt:") {
            is_kt_condition = true;
            cond_kt = if !model.context_object_ids.is_empty() {
                transform_kt_code_with_named_contexts(stripped, &model.context_object_ids)
            } else {
                stripped.to_string()
            };
        } else if !cond.is_empty() && is_pure_in_predicate(&cond) {
            is_pure_in = true;
            cond_cpp = convert_in_to_cpp(&cond);
            cond_kt = convert_in_to_kotlin(&cond);
        }
        // Decided only for the author's own language: a `cpp:` / `kt:`
        // condition is target source, and an `In()` predicate has an arm
        // of its own on every backend.
        let cond_constant = if is_cpp_condition || is_kt_condition || is_pure_in {
            None
        } else {
            crate::ecmascript::constant_truthiness(&cond)
        };

        let mut transition = Transition {
            event: elem.attribute("event").unwrap_or("").to_string(),
            target: elem.attribute("target").unwrap_or("").to_string(),
            cond,
            cond_cpp,
            cond_cpp_transformed,
            is_pure_in_predicate: is_pure_in,
            is_cpp_condition,
            cond_kt,
            is_kt_condition,
            cond_constant,
            transition_type: elem.attribute("type").unwrap_or("external").to_string(),
            source_location: source_location_of(elem, source_name),
            ..Default::default()
        };
        transition.req = collect_sce_req(
            elem,
            || {
                let event_attr = elem.attribute("event").unwrap_or("");
                let target_attr = elem.attribute("target").unwrap_or("");
                format!(
                    "<transition{}{}>",
                    if event_attr.is_empty() {
                        String::new()
                    } else {
                        format!(" event=\"{event_attr}\"")
                    },
                    if target_attr.is_empty() {
                        String::new()
                    } else {
                        format!(" target=\"{target_attr}\"")
                    },
                )
            },
            source_name,
        )?;
        transition.unresolved = collect_sce_unresolved(elem, source_name);

        transition.actions = self.parse_executable_content(elem, model, source_name)?;

        // Detect guard conditions requiring In() predicate. The
        // script-engine side of this check is re-evaluated post-parse by
        // [`crate::script_engine_analyzer`] —
        // [`NeedsScriptEngineCause::TransitionGuard`].
        if !transition.cond.is_empty()
            && !transition.is_cpp_condition
            && !transition.is_kt_condition
        {
            let (_needs_se, has_in) = check_expression_needs(&transition.cond);
            if has_in {
                model.uses_in_predicate = true;
            }
        }

        Ok(transition)
    }

    fn parse_executable_content(
        &mut self,
        parent: &roxmltree::Node,
        model: &mut SCXMLModel,
        source_name: &str,
    ) -> Result<Vec<Action>, crate::forge::error::Located<crate::forge::error::ForgeError>> {
        let mut actions = Vec::new();
        for child in parent.children() {
            if let Some(action) =
                self.parse_executable_content_single(&child, model, source_name)?
            {
                actions.push(action);
            }
        }
        Ok(actions)
    }

    fn parse_send_action(
        &mut self,
        elem: &roxmltree::Node,
        action: &mut Action,
        model: &mut SCXMLModel,
        source_name: &str,
    ) -> Result<(), crate::forge::error::Located<crate::forge::error::ForgeError>> {
        action.event = elem.attribute("event").unwrap_or("").to_string();
        action.eventexpr = elem.attribute("eventexpr").unwrap_or("").to_string();
        action.target = elem.attribute("target").unwrap_or("").to_string();
        action.targetexpr = elem.attribute("targetexpr").unwrap_or("").to_string();
        action.send_type = elem.attribute("type").unwrap_or("").to_string();
        action.typeexpr = elem.attribute("typeexpr").unwrap_or("").to_string();
        action.delay = elem.attribute("delay").unwrap_or("").to_string();
        action.delayexpr = elem.attribute("delayexpr").unwrap_or("").to_string();
        action.delay_ms = parse_delay_to_ms(&action.delay);
        action.id = elem.attribute("id").unwrap_or("").to_string();
        action.idlocation = elem.attribute("idlocation").unwrap_or("").to_string();
        action.namelist = elem.attribute("namelist").unwrap_or("").to_string();

        if action.id.is_empty() {
            action.auto_send_id = format!("__send_{}", self.send_counter);
            self.send_counter += 1;
        } else {
            action.auto_send_id = action.id.clone();
        }

        if action.target == "#_parent" {
            model.has_parent_communication = true;
        } else if action.target == "#_child" {
            model.has_child_communication = true;
        }

        // [`NeedsScriptEngineCause::SendNamelist`] / `SendParamExpr` —
        // derived post-parse by [`crate::script_engine_analyzer`] from
        // the `namelist` attribute and each param's `expr`/`is_static_literal`.

        // Parse <param> children
        for param_elem in scxml_children(elem, "param") {
            let param_expr = param_elem.attribute("expr").unwrap_or("").to_string();
            let is_static_literal = is_static_string_literal(&param_expr);
            let static_value = if is_static_literal {
                extract_static_string_literal(&param_expr)
            } else {
                String::new()
            };
            action.params.push(Param {
                name: param_elem.attribute("name").unwrap_or("").to_string(),
                expr: param_expr,
                location: param_elem.attribute("location").unwrap_or("").to_string(),
                is_static_literal,
                static_value,
                source_location: source_location_of(&param_elem, source_name),
            });
        }

        // Parse <content>
        if let Some(content_elem) = scxml_child(elem, "content") {
            action.contentexpr = content_elem.attribute("expr").unwrap_or("").to_string();
            if content_elem.children().any(|c| c.is_element()) {
                let mut xml = String::new();
                for c in content_elem.children().filter(|c| c.is_element()) {
                    xml.push_str(&serialize_node(&c));
                }
                action.content = xml.trim().to_string();
            } else {
                action.content = content_elem.text().unwrap_or("").trim().to_string();
            }
        }

        // Dynamic expressions
        if !action.eventexpr.is_empty()
            || !action.targetexpr.is_empty()
            || !action.delayexpr.is_empty()
            || !action.typeexpr.is_empty()
        {
            model.has_dynamic_expressions = true;
            // [`NeedsScriptEngineCause::SendDynamicAttr`] —
            // derived post-parse by [`crate::script_engine_analyzer`].
        }

        if !action.targetexpr.is_empty() {
            model.events.insert("error.communication".to_string());
        }

        if !action.event.is_empty() {
            model.events.insert(action.event.clone());
        } else if !action.content.is_empty() && action.eventexpr.is_empty() {
            // §scxml-C-2: content-only send (test 520) - empty event name
            model.events.insert(String::new());
        }

        // BasicHTTP send detection
        if action.send_type == "http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor"
            && (action.target.starts_with("http://") || action.target.starts_with("https://"))
        {
            model.needs_http_send = true;
        }

        // SCE_MESH.md §13 path B — SCXML purity: sce:qos / sce:pattern /
        // sce:reply-event / sce:reply-timeout / sce:deadline / sce:priority
        // attributes on <send> were removed. Stage 2 enforcement: presence
        // of any removed attribute is a hard build error
        // (`ValidationError::RemovedAttribute`). No migration tolerance
        // remains — third-party documents must migrate before building
        // against Session E1 or later.
        use crate::forge::error::{Located, ValidationError};
        use crate::forge::model::SCE_NAMESPACE;
        for removed_attr in [
            "qos",
            "pattern",
            "reply-event",
            "reply-timeout",
            "deadline",
            "priority",
        ] {
            if elem.attribute((SCE_NAMESPACE, removed_attr)).is_some() {
                let pos = elem.document().text_pos_at(elem.range().start);
                return Err(Located::new(
                    ValidationError::RemovedAttribute {
                        attribute: format!("sce:{removed_attr}"),
                        event: if action.event.is_empty() {
                            None
                        } else {
                            Some(action.event.clone())
                        },
                    }
                    .into(),
                    &model.name,
                    Some(pos.row),
                    Some(pos.col),
                ));
            }
        }
        Ok(())
    }

    /// §scxml-G-7: parse a Custom Action Element
    /// `<sce:action name="op"><sce:arg expr="..."/>...</sce:action>` into a
    /// native host-trait dispatch action.
    ///
    /// The `name` attribute names a host operation that codegen lowers to a
    /// direct call on a generated `Actions` trait method — no script engine.
    /// Each ordered `<sce:arg expr="...">` child supplies one positional
    /// argument expression; codegen lowers it through the typed-expression
    /// pipeline (the path event-schema guards already use), deriving the
    /// trait parameter types from the triggering event's payload schema. A
    /// non-statically-lowerable argument is rejected at codegen time by the
    /// same `expression/*` machinery — this construct is engine-free by
    /// definition, so it never silently falls back to a runtime engine.
    fn parse_native_action(
        &mut self,
        elem: &roxmltree::Node,
        action: &mut Action,
        source_name: &str,
    ) -> Result<(), crate::forge::error::Located<crate::forge::error::ForgeError>> {
        use crate::forge::error::{Located, ValidationError};
        use crate::forge::model::SCE_NAMESPACE;

        let require_attr = |present: bool, element: &str, attr: &str, node: &roxmltree::Node| {
            if present {
                return Ok(());
            }
            let pos = node.document().text_pos_at(node.range().start);
            Err(Located::new(
                ValidationError::MissingAttribute {
                    element: element.into(),
                    attr: attr.into(),
                }
                .into(),
                source_name,
                Some(pos.row),
                Some(pos.col),
            ))
        };

        let name = elem.attribute("name").unwrap_or("").trim().to_string();
        require_attr(!name.is_empty(), "<sce:action>", "name", elem)?;

        action.action_type = "native_action".to_string();
        action.native_action_name = name;

        for arg in elem.children().filter(|c| {
            c.is_element()
                && c.tag_name().name() == "arg"
                && c.tag_name().namespace() == Some(SCE_NAMESPACE)
        }) {
            let expr = arg.attribute("expr").unwrap_or("").trim().to_string();
            require_attr(!expr.is_empty(), "<sce:arg>", "expr", &arg)?;
            action.params.push(Param {
                name: arg.attribute("name").unwrap_or("").to_string(),
                expr,
                location: String::new(),
                is_static_literal: false,
                static_value: String::new(),
                source_location: source_location_of(&arg, source_name),
            });
        }

        Ok(())
    }

    fn parse_if_action(
        &mut self,
        elem: &roxmltree::Node,
        action: &mut Action,
        model: &mut SCXMLModel,
        source_name: &str,
    ) -> Result<(), crate::forge::error::Located<crate::forge::error::ForgeError>> {
        let cond = elem.attribute("cond").unwrap_or("").to_string();
        let mut is_pure_in = false;
        let mut cond_cpp = String::new();
        let mut cond_kt = String::new();
        if !cond.is_empty() {
            // [`NeedsScriptEngineCause::IfCondition`] is derived post-parse
            // by [`crate::script_engine_analyzer`]; we only surface the
            // In()-predicate side of the check here.
            let (_needs_se, has_in) = check_expression_needs(&cond);
            if has_in {
                model.uses_in_predicate = true;
            }
            if is_pure_in_predicate(&cond) {
                is_pure_in = true;
                cond_cpp = convert_in_to_cpp(&cond);
                cond_kt = convert_in_to_kotlin(&cond);
            }
        }

        action.cond_constant = if is_pure_in {
            None
        } else {
            crate::ecmascript::constant_truthiness(&cond)
        };
        action.cond = cond;
        action.cond_cpp = cond_cpp;
        action.cond_kt = cond_kt;
        action.is_pure_in_predicate = is_pure_in;

        let mut current_branch: usize = 0; // 0 = then, 1+ = elseif, usize::MAX = else
        for child in elem.children() {
            if !child.is_element() {
                continue;
            }
            let tag = local_name(&child);
            match tag.as_str() {
                "elseif" => {
                    let ei_cond = child.attribute("cond").unwrap_or("").to_string();
                    if !ei_cond.is_empty() {
                        // [`NeedsScriptEngineCause::ElseIfCondition`] —
                        // derived post-parse by
                        // [`crate::script_engine_analyzer`]. In()-predicate
                        // is still surfaced here for
                        // `uses_in_predicate` gating.
                        let (_needs_se, has_in) = check_expression_needs(&ei_cond);
                        if has_in {
                            model.uses_in_predicate = true;
                        }
                    }
                    let ei_pure_in = !ei_cond.is_empty() && is_pure_in_predicate(&ei_cond);
                    let ei_cpp = if ei_pure_in {
                        convert_in_to_cpp(&ei_cond)
                    } else {
                        String::new()
                    };
                    let ei_kt = if ei_pure_in {
                        convert_in_to_kotlin(&ei_cond)
                    } else {
                        String::new()
                    };
                    let ei_constant = if ei_pure_in {
                        None
                    } else {
                        crate::ecmascript::constant_truthiness(&ei_cond)
                    };
                    action.elseif_branches.push(ElseIfBranch {
                        cond: ei_cond,
                        cond_cpp: ei_cpp,
                        cond_kt: ei_kt,
                        is_pure_in_predicate: ei_pure_in,
                        cond_constant: ei_constant,
                        actions: Vec::new(),
                    });
                    current_branch = action.elseif_branches.len(); // 1-indexed
                }
                "else" => {
                    current_branch = usize::MAX;
                }
                _ => {
                    // Parse the nested action
                    let nested_actions =
                        self.parse_executable_content_single(&child, model, source_name)?;
                    if let Some(nested) = nested_actions {
                        match current_branch {
                            0 => action.then_actions.push(nested),
                            usize::MAX => action.else_actions.push(nested),
                            n => {
                                if let Some(branch) = action.elseif_branches.get_mut(n - 1) {
                                    branch.actions.push(nested);
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn parse_executable_content_single(
        &mut self,
        child: &roxmltree::Node,
        model: &mut SCXMLModel,
        source_name: &str,
    ) -> Result<Option<Action>, crate::forge::error::Located<crate::forge::error::ForgeError>> {
        if !child.is_element() {
            return Ok(None);
        }
        let tag = local_name(child);
        let mut action = Action {
            action_type: tag.clone(),
            source_location: source_location_of(child, source_name),
            ..Default::default()
        };
        action.req = collect_sce_req(child, || format!("<{tag}>"), source_name)?;
        action.unresolved = collect_sce_unresolved(child, source_name);
        match tag.as_str() {
            "raise" => {
                action.event = child.attribute("event").unwrap_or("").to_string();
                if !action.event.is_empty() {
                    model.events.insert(action.event.clone());
                    // Authoritative internal-signal capture: every `<raise>`
                    // in the document — including inside `<finalize>` (which
                    // is stringified to JS below and invisible to any later
                    // action-tree walk) — flows through this one arm. Record
                    // the event so the externally-drivable surface can exclude
                    // it. See `SCXMLModel::raised_events`.
                    model.raised_events.insert(action.event.clone());
                }
            }
            "send" => self.parse_send_action(child, &mut action, model, source_name)?,
            "assign" => {
                action.location = child.attribute("location").unwrap_or("").to_string();
                action.expr = child.attribute("expr").unwrap_or("").to_string();
                if child.children().any(|c| c.is_element()) {
                    // §scxml-5.4: Serialize with c14n (canonical XML)
                    let mut xml = String::new();
                    for c in child.children().filter(|c| c.is_element()) {
                        let part = serialize_node_c14n(&c);
                        let normalized: Vec<&str> = part.split_whitespace().collect();
                        xml.push_str(&normalized.join(" "));
                    }
                    action.content = xml;
                }
                // [`NeedsScriptEngineCause::AssignAction`] — every
                // `<assign>` routes through the engine-bound helper.
            }
            "log" => {
                action.label = child.attribute("label").unwrap_or("").to_string();
                action.expr = child.attribute("expr").unwrap_or("").to_string();
                // [`NeedsScriptEngineCause::LogExpr`] is derived post-parse
                // by [`crate::script_engine_analyzer`] when `expr` is non-empty.
            }
            "script" => {
                // Check for <cpp> or <kt> native code child elements (same as parse_executable_content)
                let mut found_native = false;
                for sc in child.children().filter(|n| n.is_element()) {
                    let sc_name = sc.tag_name().name();
                    if sc_name == "cpp" || sc.tag_name().namespace() == Some("urn:sce:cpp") {
                        let cpp_code = sc.text().unwrap_or("").to_string();
                        action.is_cpp_function = true;
                        action.content = cpp_code.clone();
                        action.content_transformed = if !model.context_object_ids.is_empty() {
                            transform_cpp_code_with_named_contexts(
                                &cpp_code,
                                &model.context_object_ids,
                            )
                        } else {
                            cpp_code
                        };
                        found_native = true;
                        break;
                    } else if sc_name == "kt" || sc.tag_name().namespace() == Some("urn:sce:kotlin")
                    {
                        let kt_code = sc.text().unwrap_or("").to_string();
                        action.is_kt_function = true;
                        action.content = kt_code.clone();
                        action.content_kt = if !model.context_object_ids.is_empty() {
                            transform_kt_code_with_named_contexts(
                                &kt_code,
                                &model.context_object_ids,
                            )
                        } else {
                            kt_code
                        };
                        found_native = true;
                        break;
                    }
                }
                if !found_native {
                    action.content = child.text().unwrap_or("").to_string();
                    // [`NeedsScriptEngineCause::InlineScriptAction`] —
                    // inline `<script>` body requires runtime evaluation.
                }
            }
            "cancel" => {
                action.sendid = child.attribute("sendid").unwrap_or("").to_string();
                action.sendidexpr = child.attribute("sendidexpr").unwrap_or("").to_string();
                // §scxml-6.3: <cancel> MUST have either sendid or
                // sendidexpr. Mirrors the invoke-duplicate-id check
                // (parse_invoke) — a parse-time ValidationError emit
                // routes through the same Located + Diagnostic pipeline,
                // reusing wire code `validation/require-either` per
                // W4 D4 fold (concept identity over namespace
                // duplication). 5-backend cancel templates may safely
                // assume at least one attribute is non-empty post-parse.
                if action.sendid.is_empty() && action.sendidexpr.is_empty() {
                    let pos = child.document().text_pos_at(child.range().start);
                    return Err(crate::forge::error::Located::new(
                        crate::forge::error::ValidationError::RequireEither {
                            element: "<cancel>".into(),
                            alternatives: vec!["sendid".into(), "sendidexpr".into()],
                        }
                        .into(),
                        source_name,
                        Some(pos.row),
                        Some(pos.col),
                    ));
                }
                // [`NeedsScriptEngineCause::CancelExpr`] — derived post-parse
                // by [`crate::script_engine_analyzer`] from `sendidexpr`.
            }
            "foreach" => {
                // [`NeedsScriptEngineCause::ForeachAction`] — every
                // `<foreach>` iterates a runtime expression.
                action.array = child.attribute("array").unwrap_or("").to_string();
                action.item = child.attribute("item").unwrap_or("").to_string();
                action.index = child.attribute("index").unwrap_or("").to_string();
                action.actions = self.parse_executable_content(child, model, source_name)?;
            }
            "if" => self.parse_if_action(child, &mut action, model, source_name)?,
            "action" => {
                // §scxml-G-7: Custom Action Element. Only the SCE
                // namespace claims `<action>`; a foreign `<action>` from
                // another vocabulary is ignored exactly like any other
                // unrecognised element (falls through to `Ok(None)`).
                use crate::forge::model::SCE_NAMESPACE;
                if child.tag_name().namespace() != Some(SCE_NAMESPACE) {
                    return Ok(None);
                }
                self.parse_native_action(child, &mut action, source_name)?;
            }
            _ => return Ok(None),
        }
        Ok(Some(action))
    }

    /// §scxml-6.4: Parse `<invoke>` into the typed [`Invoke`] sum.
    ///
    /// Returns `None` if the element neither resolves to a static SCXML
    /// session (`src` / inline `<content><scxml>`) nor to a hybrid session
    /// (`srcexpr` / `contentexpr`). The caller pushes the result directly
    /// onto `state.invokes`; there is no intermediate JSON representation.
    /// Parse a single `<invoke>` element into a typed [`Invoke`] variant.
    ///
    /// Returns `Ok(Some(_))` for a successful classification (scxml,
    /// hybrid, or sce:mesh-rpc), `Ok(None)` for unsupported invoke
    /// types that the parser skips silently, and `Err(_)` for SCE
    /// Mesh §9.5 reserved-param rule violations on
    /// `<invoke type="sce:mesh-rpc">`. The error surfaces the source
    /// label (`source_name`) and roxmltree row/col so downstream
    /// NDJSON diagnostics land with the same precision as every other
    /// parser-stage failure.
    fn parse_invoke(
        &mut self,
        elem: &roxmltree::Node,
        model: &mut SCXMLModel,
        state_id: &str,
        base_dir: Option<&Path>,
        source_name: &str,
    ) -> Result<Option<Invoke>, crate::forge::error::Located<crate::forge::error::ForgeError>> {
        // §scxml-6.4.1: Generate invoke ID if not provided. Auto-ids carry
        // a leading underscore by spec convention; templates building
        // identifiers (`child_<suffix>`) consume `field_suffix` instead so the
        // leading underscore does not double up.
        let mut invoke_id = elem.attribute("id").unwrap_or("").to_string();
        if invoke_id.is_empty() {
            invoke_id = format!("_invoke_{}", self.invoke_counter);
            self.invoke_counter += 1;
        }
        let field_suffix = invoke_id.trim_start_matches('_').to_string();

        // §scxml-3.14: `<invoke>` id must be document-unique. Downstream
        // identity axes — AOT `done.invoke.<id>` / `error.invoke.<id>` event
        // matching, `idlocation` datamodel assignment (§6.4.2), mesh
        // `active_invokes_` keying — all assume this; a silent duplicate
        // collapses lifecycle event delivery and mis-cancels in-flight work.
        // Author-supplied ids and auto-counter ids share one set so the
        // shadow case (`<invoke id="_invoke_0">` racing a later auto-gen)
        // is caught alongside plain duplicates.
        if !self.invoke_ids_seen.insert(invoke_id.clone()) {
            let pos = elem.document().text_pos_at(elem.range().start);
            return Err(crate::forge::error::Located::new(
                crate::forge::error::ValidationError::DuplicateId {
                    kind: crate::forge::model::ForgeKind::Statechart,
                    what: "<invoke id>".into(),
                    id: invoke_id,
                }
                .into(),
                source_name,
                Some(pos.row),
                Some(pos.col),
            ));
        }

        let invoke_type = elem.attribute("type").unwrap_or("").to_string();
        let src = elem.attribute("src").unwrap_or("").to_string();
        let srcexpr = elem.attribute("srcexpr").unwrap_or("").to_string();
        let idlocation = elem.attribute("idlocation").unwrap_or("").to_string();
        let autoforward = elem.attribute("autoforward").unwrap_or("false") == "true";
        if autoforward {
            // §scxml-6.4 / test229: stamp the SM-wide predicate so the
            // c11 codegen emits `forward_to_autoforward_children` and its
            // process_event_queues call site. Per-invoke `autoforward` flag
            // (assigned below into `InvokeSessionCommon::autoforward`) drives
            // the per-child arm inside that helper.
            model.has_autoforward_invoke = true;
        }
        let namelist = elem.attribute("namelist").unwrap_or("").to_string();

        // SCE Mesh §9.5: <invoke type="sce:mesh-rpc"> — short-lived RPC
        // layered on W3C invoke lifecycle. Parsed through a dedicated
        // path because reserved `_mesh_*` `<param>` names are structural
        // metadata (strip from payload, populate envelope fields) rather
        // than author data, and the static/hybrid classifier below
        // would otherwise silently drop the invoke (scxml_type=false).
        if invoke_type == "sce:mesh-rpc" {
            let info = self.parse_mesh_rpc_invoke(
                elem,
                state_id,
                source_name,
                MeshRpcInvokeAttrs {
                    invoke_id,
                    field_suffix,
                    src: src.clone(),
                    srcexpr: srcexpr.clone(),
                    idlocation,
                },
            )?;
            return Ok(Some(Invoke::MeshRpc(info)));
        }

        let mut contentexpr = String::new();
        let mut has_inline_scxml = false;
        let mut inline_scxml_text = String::new();

        // Parse inline <content>
        if let Some(content_elem) = scxml_child(elem, "content") {
            contentexpr = content_elem.attribute("expr").unwrap_or("").to_string();

            // Check for inline <scxml> child element (static content)
            if let Some(scxml_child_elem) = scxml_child(&content_elem, "scxml") {
                has_inline_scxml = true;
                // Extract inline SCXML text via roxmltree range
                let doc_text = scxml_child_elem.document().input_text();
                let range = scxml_child_elem.range();
                inline_scxml_text = doc_text[range].to_string();
            }
        }

        // Parse <param> children. Static invokes retain a static-literal
        // optimisation flag so codegen can inline literal values.
        let mut static_params: Vec<Param> = Vec::new();
        let mut hybrid_params: Vec<Param> = Vec::new();
        for param in scxml_children(elem, "param") {
            let name = param.attribute("name").unwrap_or("").to_string();
            let expr = param.attribute("expr").unwrap_or("").to_string();
            let location = param.attribute("location").unwrap_or("").to_string();
            let is_sl = is_static_string_literal(&expr);
            let param_at = source_location_of(&param, source_name);
            static_params.push(Param {
                name: name.clone(),
                expr: expr.clone(),
                location: location.clone(),
                is_static_literal: is_sl,
                static_value: if is_sl {
                    extract_static_string_literal(&expr)
                } else {
                    String::new()
                },
                source_location: param_at.clone(),
            });
            hybrid_params.push(Param {
                name,
                expr,
                location,
                source_location: param_at,
                ..Default::default()
            });
        }

        // Parse <finalize>
        let mut finalize_content = String::new();
        if let Some(finalize_elem) = scxml_child(elem, "finalize") {
            let finalize_actions =
                self.parse_executable_content(&finalize_elem, model, source_name)?;
            finalize_content = actions_to_javascript(&finalize_actions);
        }

        // §scxml-6.4: Classify invoke type
        let has_static_child = !src.is_empty() || has_inline_scxml;
        let scxml_type = invoke_type.is_empty()
            || invoke_type == "scxml"
            || invoke_type == "http://www.w3.org/TR/scxml"
            || invoke_type == "http://www.w3.org/TR/scxml/";

        let is_static_invoke =
            scxml_type && srcexpr.is_empty() && contentexpr.is_empty() && has_static_child;
        let is_hybrid_invoke = scxml_type && (!srcexpr.is_empty() || !contentexpr.is_empty());

        if is_hybrid_invoke {
            // [`NeedsScriptEngineCause::HybridInvoke`] — the hybrid
            // lifecycle resolves `srcexpr`/`contentexpr` at runtime.
            let idx = self.hybrid_invoke_counter;
            self.hybrid_invoke_counter += 1;
            let invoke_req =
                collect_sce_req(elem, || format!("<invoke id=\"{invoke_id}\">"), source_name)?;
            let invoke_unresolved = collect_sce_unresolved(elem, source_name);
            return Ok(Some(Invoke::Hybrid(HybridInvokeInfo {
                common: InvokeSessionCommon {
                    base: InvokeBase {
                        source_location: source_location_of(elem, source_name),
                        invoke_id,
                        field_suffix,
                        state_name: state_id.to_string(),
                        params: hybrid_params,
                        idlocation,
                        req: invoke_req,
                        provenance: Vec::new(),
                        unresolved: invoke_unresolved,
                    },
                    child_name: format!("{}_hybrid{idx}", model.name),
                    autoforward,
                    ..Default::default()
                },
                srcexpr,
                contentexpr,
            })));
        }

        if is_static_invoke {
            // §scxml-6.4.1: `namelist` requires datamodel evaluation.
            // [`NeedsScriptEngineCause::StaticInvokeNamelist`] —
            // derived post-parse by [`crate::script_engine_analyzer`].

            // §scxml-6.4: Inline `<content><scxml>` and external `src="..."`
            // resolve to a concrete child reference eagerly, at parse time.
            //
            // Inline `<content>`: the parser pre-parses the inner SCXML into
            // a structured submodel and stashes it on
            // [`ScxmlInvokeInfo::inline_child`]. The Mesh §9.6.6 naming
            // convention still applies — `common.child_name` is set to the
            // synth name and `src` rewritten to `#<synth>` so peer-classifier
            // (`classify_remote_scxml_invokes`) and codegen continue to
            // treat it as a canonical mesh peer reference — but the synth
            // child no longer materialises as a sibling `.scxml` file in
            // the parent's source directory. Disk emission is a codegen
            // concern, not a parser side-effect.
            //
            // WASM (`base_dir == None`) takes the same in-memory path; the
            // historical empty-`child_name` skip is gone because inline
            // parsing has no filesystem dependency.
            let (resolved_src, resolved_child_name, inline_child_model, inline_child_source_xml) =
                if has_inline_scxml && !inline_scxml_text.is_empty() {
                    // SCE Mesh §9.6.6 rule 1: synthesised machine name is
                    // `<parent_machine_id>__sce_synth_invoke__<invoke_id>`.
                    // `field_suffix` is the invoke_id with its leading
                    // underscore trimmed (line ~1438), so author ids map
                    // verbatim and the auto-generated `_invoke_N` ids
                    // (§scxml-6.4.1 §3.14 — SCE emits one when `id` is
                    // absent) produce `invoke_N` rather than the triple
                    // underscore block `__sce_synth_invoke___invoke_N`.
                    let synth_name = format!(
                        "{}{}{}",
                        model.name,
                        crate::mesh::deploy::SYNTH_INVOKE_INFIX,
                        field_suffix,
                    );
                    let inline_with_ns = if !inline_scxml_text.contains("xmlns=") {
                        inline_scxml_text.replacen(
                            "<scxml",
                            "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\"",
                            1,
                        )
                    } else {
                        inline_scxml_text.clone()
                    };
                    let xml_content = format!("<?xml version=\"1.0\"?>\n\n{inline_with_ns}");
                    // Recursive parse uses a fresh parser instance — sharing
                    // `self` would cross-contaminate document_order_counter /
                    // invoke_counter between parent and child. Asymmetric
                    // [`DocumentLabel`]: identifier = synth name (extension-
                    // free, drives template symbols), diagnostic label =
                    // `<synth>.scxml` (matches the historical on-disk file
                    // path so SCE-MAP markers + NDJSON `location.file` stay
                    // byte-stable against the pre-refactor goldens).
                    let synth_diag_label = format!("{synth_name}.scxml");
                    let inline_child = match SCXMLParser::new().parse_string_with_label(
                        &xml_content,
                        DocumentLabel::asymmetric(&synth_name, &synth_diag_label),
                    ) {
                        Ok(m) => Some(Box::new(m)),
                        Err(e) => {
                            eprintln!(
                                "Warning: Failed to parse inline <content> for invoke {invoke_id} (synth={synth_name}): {e:?}"
                            );
                            None
                        }
                    };
                    // SCE Mesh §9.6.6 rule 2: the rewritten `<invoke>`
                    // carries the canonical `#<machine>` mesh peer
                    // reference so `classify_remote_scxml_invokes`
                    // treats the synth peer through the same axis as
                    // author-declared peers. `child_name` carries the
                    // synth identifier for local child session codegen
                    // (`invoke_methods.jinja2` etc.) and for
                    // `parse_child_metadata` below. `xml_content` rides
                    // on the invoke so codegen can re-emit it to `-o`
                    // for downstream consumers (`inject_partition_context_
                    // for` + CMake stage-3 synth codegen + W3C
                    // `process_children_<N>.cmake`).
                    (
                        format!("#{synth_name}"),
                        synth_name,
                        inline_child,
                        Some(xml_content),
                    )
                } else if !src.is_empty() {
                    let stripped = src.replace("file:", "");
                    let child_name = Path::new(&stripped)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_string())
                        .unwrap_or_default();
                    (src.clone(), child_name, None, None)
                } else {
                    (src.clone(), String::new(), None, None)
                };

            let invoke_req =
                collect_sce_req(elem, || format!("<invoke id=\"{invoke_id}\">"), source_name)?;
            let invoke_unresolved = collect_sce_unresolved(elem, source_name);
            let mut scxml_info = ScxmlInvokeInfo {
                common: InvokeSessionCommon {
                    base: InvokeBase {
                        source_location: source_location_of(elem, source_name),
                        invoke_id,
                        field_suffix,
                        state_name: state_id.to_string(),
                        params: static_params,
                        idlocation,
                        req: invoke_req,
                        provenance: Vec::new(),
                        unresolved: invoke_unresolved,
                    },
                    child_name: resolved_child_name,
                    autoforward,
                    ..Default::default()
                },
                finalize_content,
                src: resolved_src,
                namelist,
                inline_child: inline_child_model,
                inline_child_xml: inline_child_source_xml,
                remote_mesh_target: None,
                remote_mesh_transport: None,
            };

            // Populate child-side metadata (script-engine flag, datamodel
            // variable list). For inline children the model is already
            // parsed in-memory; external (`src=`) children still resolve
            // through `base_dir` on disk.
            if let Some(inline_model) = scxml_info.inline_child.as_deref() {
                populate_child_metadata_from_model(inline_model, &mut scxml_info.common);
            } else if let Some(scxml_dir) = base_dir {
                if !scxml_info.common.child_name.is_empty() {
                    let child_scxml_path =
                        scxml_dir.join(format!("{}.scxml", scxml_info.common.child_name));
                    parse_child_metadata(&child_scxml_path, &mut scxml_info.common);
                }
            }

            return Ok(Some(Invoke::Scxml(scxml_info)));
        }

        // §scxml-6.4.1: a `type` naming no processor this platform
        // implements. The spec defines the case — "MUST place
        // error.execution in the internal event queue" — so the document
        // is valid SCXML with defined meaning, not an author error the
        // compiler may reject. Carrying it as a typed variant is what
        // makes the backends emit that raise; the earlier `Ok(None)` here
        // dropped the `<invoke>` from the model outright, so AOT produced
        // no observable at all where the Interpreter produced one.
        //
        // `typeexpr` resolves the type at runtime, so a document carrying
        // one cannot be classified statically and is left alone.
        if !scxml_type && elem.attribute("typeexpr").is_none() {
            let invoke_req =
                collect_sce_req(elem, || format!("<invoke id=\"{invoke_id}\">"), source_name)?;
            let invoke_unresolved = collect_sce_unresolved(elem, source_name);
            return Ok(Some(Invoke::Unsupported(UnsupportedInvokeInfo {
                base: InvokeBase {
                    source_location: source_location_of(elem, source_name),
                    invoke_id,
                    field_suffix,
                    state_name: state_id.to_string(),
                    // Inert: no session starts, so no `<param>` is ever
                    // delivered. Carried anyway so the AST export reports
                    // what the author wrote rather than an empty invoke.
                    params: static_params,
                    idlocation,
                    req: invoke_req,
                    provenance: Vec::new(),
                    unresolved: invoke_unresolved,
                },
                invoke_type,
            })));
        }

        // A supported type that resolves to no child at all — `<invoke
        // type="scxml">` with neither `src`, inline `<content>`, `srcexpr`
        // nor `contentexpr`. §scxml-6.4.1 requires one of them, so this is
        // a malformed `<invoke>` rather than an unsupported processor. It
        // stays a silent skip here; making it loud is a separate axis from
        // the type-support boundary above and needs its own diagnostic.
        Ok(None)
    }

    // ── parse_mesh_rpc_invoke attribute bundle ──
    //
    // Pre-parsed `<invoke type="sce:mesh-rpc">` attributes grouped into
    // a single argument so the call doesn't trip clippy's 7-arg ceiling
    // — these five fields are extracted together by the caller from
    // distinct XML attribute paths (`id`, generated `_N` suffix,
    // `src`, `srcexpr`, `idlocation`) and stay together as the
    // identifying tuple for the invoke through validation.

    /// SCE Mesh §9.5: parse an `<invoke type="sce:mesh-rpc">` element.
    ///
    /// Enforces the reserved-`_mesh_*` `<param>` rules at parse time
    /// (missing required, duplicate, unknown prefix, invalid deadline)
    /// and strips the reserved names from the payload passed to the
    /// envelope. All four rule variants surface as the same
    /// [`ValidationError::MeshRpcReservedParam`] variant — the repair
    /// surface is uniform ("rename or retype your `<param>`"), and a
    /// single `DiagnosticCode` keeps the wire catalog tight.
    fn parse_mesh_rpc_invoke(
        &mut self,
        elem: &roxmltree::Node,
        state_id: &str,
        source_name: &str,
        attrs: MeshRpcInvokeAttrs,
    ) -> Result<MeshRpcInvokeInfo, crate::forge::error::Located<crate::forge::error::ForgeError>>
    {
        use crate::forge::error::{Located, ValidationError};
        let MeshRpcInvokeAttrs {
            invoke_id,
            field_suffix,
            src,
            srcexpr,
            idlocation,
        } = attrs;

        let locate = |err: ValidationError| -> Located<crate::forge::error::ForgeError> {
            let pos = elem.document().text_pos_at(elem.range().start);
            Located::new(err.into(), source_name, Some(pos.row), Some(pos.col))
        };

        // SCE Mesh §9.5 exactly-one rule: `src` and `srcexpr` are
        // mutually exclusive on `<invoke type="sce:mesh-rpc">`. Both
        // absent or both present is a build-time hard error. The
        // validation runs *before* constructing [`MeshRpcTarget`] so the
        // sum type is only built on well-formed inputs — its two
        // variants map 1:1 to the surviving cases.
        let src_present = !src.is_empty();
        let srcexpr_present = !srcexpr.is_empty();
        if !src_present && !srcexpr_present {
            return Err(locate(ValidationError::MeshRpcMissingTarget));
        }
        if src_present && srcexpr_present {
            return Err(locate(ValidationError::MeshRpcDuplicateTarget));
        }
        let target = if src_present {
            crate::model::MeshRpcTarget::Src { src }
        } else {
            // SCE Mesh §9.5 runtime target resolution — the `srcexpr`
            // entry block emitted by codegen calls `evaluateExpression`.
            // [`NeedsScriptEngineCause::MeshRpcSrcExpr`] is derived
            // post-parse by [`crate::script_engine_analyzer`] when the
            // invoke's [`MeshRpcTarget::SrcExpr`] variant is chosen here.
            crate::model::MeshRpcTarget::SrcExpr { srcexpr }
        };

        // First pass: scan every `<param>` to detect reserved-name
        // violations and extract `_mesh_event` / `_mesh_deadline_ms`.
        // Only after all four rules pass do we construct the final
        // struct, so author errors report before the downstream
        // payload is assembled.
        let mut mesh_event: Option<String> = None;
        let mut mesh_event_count = 0usize;
        let mut deadline_ms: Option<u64> = None;
        let mut deadline_count = 0usize;
        let mut payload_params: Vec<Param> = Vec::new();

        for param in scxml_children(elem, "param") {
            let name = param.attribute("name").unwrap_or("").to_string();
            let expr = param.attribute("expr").unwrap_or("").to_string();
            let location = param.attribute("location").unwrap_or("").to_string();

            if name == "_mesh_event" {
                mesh_event_count += 1;
                // Rule 1: exactly-one. Even the first duplicate fires
                // eagerly so the diagnostic pinpoints the second
                // occurrence rather than waiting for the end of the
                // loop to count and lose the document order signal.
                if mesh_event_count > 1 {
                    return Err(locate(ValidationError::MeshRpcReservedParam {
                        param: "_mesh_event".into(),
                        detail: "<param name=\"_mesh_event\"> must appear exactly once".into(),
                    }));
                }
                mesh_event = Some(extract_static_string_literal(&expr));
            } else if name == "_mesh_deadline_ms" {
                deadline_count += 1;
                if deadline_count > 1 {
                    return Err(locate(ValidationError::MeshRpcReservedParam {
                        param: "_mesh_deadline_ms".into(),
                        detail: "<param name=\"_mesh_deadline_ms\"> may appear at most once".into(),
                    }));
                }
                // §9.5: `_mesh_deadline_ms` is an integer in
                // milliseconds. The literal may be quoted (`expr="'50'"`)
                // or bare (`expr="50"`); both resolve to a non-negative
                // decimal integer string via the existing static-literal
                // extractor.
                let raw = if is_static_string_literal(&expr) {
                    extract_static_string_literal(&expr)
                } else {
                    expr.trim().to_string()
                };
                match raw.parse::<u64>() {
                    Ok(v) => deadline_ms = Some(v),
                    Err(_) => {
                        return Err(locate(ValidationError::MeshRpcReservedParam {
                            param: "_mesh_deadline_ms".into(),
                            detail: format!(
                                "value '{raw}' is not a non-negative integer (milliseconds)"
                            ),
                        }));
                    }
                }
            } else if name.starts_with("_mesh_") {
                return Err(locate(ValidationError::MeshRpcReservedParam {
                    param: name.clone(),
                    detail: "unknown _mesh_* name is reserved for future envelope metadata".into(),
                }));
            } else {
                let is_sl = is_static_string_literal(&expr);
                payload_params.push(Param {
                    name,
                    expr: expr.clone(),
                    location,
                    is_static_literal: is_sl,
                    static_value: if is_sl {
                        extract_static_string_literal(&expr)
                    } else {
                        String::new()
                    },
                    source_location: source_location_of(&param, source_name),
                });
            }
        }

        let mesh_event = mesh_event.ok_or_else(|| {
            locate(ValidationError::MeshRpcReservedParam {
                param: "_mesh_event".into(),
                detail: "required <param name=\"_mesh_event\"> is missing".into(),
            })
        })?;

        let invoke_req =
            collect_sce_req(elem, || format!("<invoke id=\"{invoke_id}\">"), source_name)?;
        let invoke_unresolved = collect_sce_unresolved(elem, source_name);
        Ok(MeshRpcInvokeInfo {
            base: InvokeBase {
                source_location: source_location_of(elem, source_name),
                invoke_id,
                field_suffix,
                state_name: state_id.to_string(),
                params: payload_params,
                idlocation,
                req: invoke_req,
                provenance: Vec::new(),
                unresolved: invoke_unresolved,
            },
            target,
            mesh_event,
            deadline_ms,
        })
    }

    fn parse_donedata(&mut self, elem: &roxmltree::Node, datamodel: Datamodel) -> DoneData {
        let mut dd = DoneData::default();

        // §scxml-5.7: Parse <param> elements.
        // [`NeedsScriptEngineCause::DonedataParam`] is derived post-parse
        // by [`crate::script_engine_analyzer`] from `DoneData.params`.
        for child in scxml_children(elem, "param") {
            dd.params.push(DoneDataParam {
                name: child.attribute("name").unwrap_or("").to_string(),
                expr: child.attribute("expr").map(|s| s.to_string()),
                location: child.attribute("location").map(|s| s.to_string()),
            });
        }

        // §scxml-5.5 + 5.6 + Appendix B.2.2:
        //   - `<content expr="X"/>` → Expression (MUST be evaluated against
        //     the datamodel at runtime — script engine required).
        //   - `<content>text</content>` with ECMAScript datamodel →
        //     InlineText, which takes the ECMAScript data model appendix's
        //     ordered readings the way inline `<data>` text does: an
        //     expression if it is one
        //     (`21` yields number 21, `'foo'` yields string "foo", which
        //     W3C tests 529 and 294 require), and otherwise the string.
        //     Emitting it as an Expression instead made the second reading
        //     unreachable — `<content>inline payload</content>` lowered to
        //     an expression that could only raise.
        //   - `<content>text</content>` with the "null" datamodel → Literal:
        //     the children are used **as the content value**, no evaluation,
        //     no script engine required. This is the `sce_base`-linkable
        //     path for native-only (`cpp:` / `kt:`) state machines.
        //   - Omitted → None.
        if let Some(content_elem) = scxml_child(elem, "content") {
            if let Some(expr) = content_elem.attribute("expr") {
                dd.content = crate::model::DoneDataContent::Expression(expr.to_string());
            } else if let Some(text) = content_elem.text() {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    dd.content = if datamodel == Datamodel::Null {
                        crate::model::DoneDataContent::Literal(trimmed.to_string())
                    } else {
                        crate::model::DoneDataContent::InlineText(trimmed.to_string())
                    };
                }
            }
        }

        dd
    }

    // ── Feature detection ────────────────────────────────

    fn detect_features(&mut self, model: &mut SCXMLModel) {
        for state in model.states.values() {
            if state.is_parallel {
                model.has_parallel_states = true;
            }
            if state.parent.is_some() {
                model.has_hierarchy = true;
            }
            // Note: `uses_in_predicate` is already set during
            // `parse_transition` via `check_expression_needs`; no redundant
            // re-check here. `needs_script_engine` is derived post-parse
            // by [`crate::script_engine_analyzer`].
            // W3C SCXML: detect _event.* usage in guard conditions (transitions + if/elseif)
            for trans in &state.transitions {
                if trans.cond.contains("_event.") {
                    model.has_event_metadata = true;
                }
                if actions_contain_event_metadata(&trans.actions) {
                    model.has_event_metadata = true;
                }
            }
            for block in state
                .on_entry_blocks
                .iter()
                .chain(state.on_exit_blocks.iter())
            {
                if actions_contain_event_metadata(block) {
                    model.has_event_metadata = true;
                }
            }
        }
    }

    fn resolve_deep_initial(&self, model: &mut SCXMLModel) {
        if model.initial.is_empty() {
            return;
        }
        // §scxml-3.13: Check for space-separated parallel initial states
        let initial_states: Vec<&str> = model.initial.split_whitespace().collect();
        if initial_states.len() > 1 {
            // Multiple initial states (parallel entry) — verify all exist
            if initial_states.iter().all(|s| model.states.contains_key(*s)) {
                return; // Keep space-separated format
            }
            // Fallback: treat as single state
        }
        // Single initial state — resolve to leaf by following initial chain
        let mut current = initial_states[0].to_string();
        for _ in 0..20 {
            let state = match model.states.get(&current) {
                Some(s) => s,
                None => break,
            };
            if !state.initial.is_empty() && model.states.contains_key(&state.initial) {
                current = state.initial.clone();
            } else {
                break;
            }
        }
        // §scxml-3.6: Update model.initial to the resolved leaf state
        model.initial = current;
    }

    fn apply_parallel_initial_overrides(&self, model: &mut SCXMLModel) {
        // §scxml-3.6 / §scxml-3.13: Multi-target initial declarations enter every
        // listed descendant simultaneously. Walk up from each target
        // through all ancestors up to (but not crossing) the multi-target
        // origin state, overriding each compound ancestor's `initial` to
        // the child on the path so codegen-time `state_get_initial_child`
        // routes to the path leaf rather than the doc-order default.
        // Skip parallel ancestors (they enter every region automatically).
        // Two scopes covered:
        //   - Root `<scxml initial="A B">` (origin = the document root).
        //   - Nested `<state initial="A B">` (origin = that state). cpp
        //     handles the nested case via `enterDeepInitialTargets_*` +
        //     `StateEntryHelper::enterDeepTargets`, which builds an entry
        //     chain per target. C11 collapses both scopes to per-ancestor
        //     `initial` overrides so the existing chain walk-up reaches
        //     every leaf without a multi-target dispatch helper.
        let mut overrides: Vec<(String, String)> = Vec::new();
        let collect = |targets: &[String],
                       stop_at: Option<&str>,
                       out: &mut Vec<(String, String)>| {
            for state_id in targets {
                if !model.states.contains_key(state_id) {
                    continue;
                }
                let mut current = state_id.clone();
                loop {
                    let parent_id = match model.states.get(&current).and_then(|s| s.parent.clone())
                    {
                        Some(p) if model.states.contains_key(&p) => p,
                        _ => break,
                    };
                    if Some(parent_id.as_str()) == stop_at {
                        break;
                    }
                    let is_parallel = model.states.get(&parent_id).is_some_and(|s| s.is_parallel);
                    if !is_parallel {
                        out.push((parent_id.clone(), current.clone()));
                    }
                    current = parent_id;
                }
            }
        };

        // Root-scope multi-target.
        let root_targets: Vec<String> = if model.initial.is_empty() {
            Vec::new()
        } else {
            let parts: Vec<String> = model.initial.split_whitespace().map(String::from).collect();
            if parts.len() > 1 {
                parts
            } else {
                Vec::new()
            }
        };
        if !root_targets.is_empty() {
            collect(&root_targets, None, &mut overrides);
        }

        // Nested-scope multi-target — every state whose own `initial` lists
        // more than one target. Snapshot ids first so we can iterate while
        // mutating later.
        let nested_origins: Vec<(String, Vec<String>)> = model
            .states
            .iter()
            .filter_map(|(id, s)| {
                let parts: Vec<String> = s.initial.split_whitespace().map(String::from).collect();
                if parts.len() > 1 && parts.iter().all(|t| model.states.contains_key(t)) {
                    Some((id.clone(), parts))
                } else {
                    None
                }
            })
            .collect();
        for (origin_id, targets) in &nested_origins {
            collect(targets, Some(origin_id.as_str()), &mut overrides);
        }

        // Apply collected overrides. A path child takes precedence — the
        // first override wins per ancestor (root-scope first, nested
        // afterwards), preserving the path determined by the outer
        // multi-target. Conflicts inside one scope can't arise because
        // each path through a parallel ancestor enters its own region.
        let mut applied: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for (parent_id, child_id) in overrides {
            if applied.contains(&parent_id) {
                continue;
            }
            if let Some(parent) = model.states.get_mut(&parent_id) {
                parent.initial = child_id;
            }
            applied.insert(parent_id);
        }

        // After overrides, collapse model.initial to the first state of a
        // root-scope multi-target so the chain entry helper has a single
        // leaf to start from (sibling regions enter via parallel
        // expansion).
        if !root_targets.is_empty() {
            model.initial = root_targets[0].clone();
        }
    }

    fn resolve_history_targets(&self, model: &mut SCXMLModel) {
        // §scxml-3.11: Resolve history default targets to leaf states
        let mut history_leaf_targets: BTreeMap<String, String> = BTreeMap::new();
        for (history_id, history_info) in &model.history_states {
            let default_target = &history_info.default_target;
            let leaf = model.resolve_to_leaf(default_target);
            history_leaf_targets.insert(history_id.clone(), leaf);
        }

        let history_defaults = model.history_default_targets.clone();
        for state in model.states.values_mut() {
            // Resolve initial that points to history state
            if !state.initial.is_empty() {
                if let Some(default_target) = history_defaults.get(&state.initial) {
                    let history_id = state.initial.clone();
                    state.initial_history_id = history_id.clone();
                    state.initial_history_default_target = default_target.clone();
                    if let Some(info) = model.history_states.get(&history_id) {
                        state.initial_history_default_actions = info.default_actions.clone();
                    }
                    state.initial = default_target.clone();
                }
            }
            // Resolve transition targets that point to history states
            for trans in &mut state.transitions {
                if !trans.target.is_empty() {
                    if let Some(default_target) = history_defaults.get(&trans.target) {
                        trans.history_target = Some(trans.target.clone());
                        // §scxml-3.11: Resolved leaf target for the Kotlin backend
                        trans.history_leaf_target =
                            history_leaf_targets.get(&trans.target).cloned();
                        trans.target = default_target.clone();
                    }
                }
            }
        }
    }

    fn compute_parallel_regions(&self, model: &mut SCXMLModel) {
        let parallel_ids: Vec<String> = model
            .states
            .iter()
            .filter(|(_, s)| s.is_parallel)
            .map(|(id, _)| id.clone())
            .collect();
        for pid in parallel_ids {
            let children: Vec<String> = model
                .states
                .iter()
                .filter(|(_, s)| s.parent.as_deref() == Some(&pid))
                .map(|(id, _)| id.clone())
                .collect();
            model.parallel_regions.insert(pid, children);
        }
    }

    fn detect_transition_actions(&self, model: &mut SCXMLModel) {
        for state in model.states.values() {
            for trans in &state.transitions {
                if !trans.actions.is_empty() {
                    model.has_transition_actions = true;
                    return;
                }
            }
        }
    }

    fn detect_entry_exit_actions(&self, model: &mut SCXMLModel) {
        for state in model.states.values() {
            if !model.has_entry_actions
                && (!state.on_entry_blocks.is_empty()
                    || state.has_scxml_invoke()
                    || state.has_hybrid_invoke()
                    || state.has_mesh_rpc_invoke()
                    || state.has_unsupported_invoke()
                    || !state.datamodel.is_empty()
                    || !state.initial_transition_actions.is_empty()
                    || !state.initial_history_id.is_empty()
                    || (state.is_final && (state.donedata.is_some() || state.parent.is_some())))
            {
                model.has_entry_actions = true;
            }
            if !model.has_exit_actions
                && (!state.on_exit_blocks.is_empty()
                    || state.has_scxml_invoke()
                    || state.has_hybrid_invoke()
                    || state.has_mesh_rpc_invoke()
                    // §scxml-6.4.1: the pending entry must be cancellable
                    // when the state exits before the macrostep ends.
                    || state.has_unsupported_invoke())
            {
                model.has_exit_actions = true;
            }
            if model.has_entry_actions && model.has_exit_actions {
                break;
            }
        }
    }

    fn detect_hierarchy(&self, model: &mut SCXMLModel) {
        model.has_hierarchy = model.states.values().any(|s| s.parent.is_some());
    }

    /// Register the `done.state.*` events the generated machines raise.
    ///
    /// Two producers, not one. A compound state raises
    /// `done.state.<id>` when a `<final>` child is entered — that is the
    /// direct-parent rule. A `<parallel>` raises `done.state.<id>` when
    /// *every* region has reached a final, and a parallel owns no `<final>`
    /// of its own: its finals sit one level down, inside the regions. Walking
    /// only to the direct parent therefore never reaches it.
    ///
    /// It stayed invisible because the event was still *raised*: the C++ and
    /// C11 emitters raise `Done_state_<parallel>` at the same site they raise
    /// the region's, so the generated code referenced an enumerator this
    /// function never declared and did not compile. `check` reported the
    /// document acceptable for every backend, because acceptance is decided
    /// before anything is compiled.
    ///
    /// The grandparent condition here is deliberately the emitters' own —
    /// `entry_exit_actions.jinja2` guards its raise on
    /// `states[state.parent].parent and states[...].is_parallel`. A rule that
    /// merely resembled it would drift; this one is the same rule.
    fn add_done_state_events(&self, model: &mut SCXMLModel) {
        let parent_ids_with_finals: Vec<String> = model
            .states
            .values()
            .filter(|s| s.is_final)
            .filter_map(|s| s.parent.clone())
            .collect();
        let mut event_names: Vec<String> = Vec::new();
        for parent_id in parent_ids_with_finals {
            event_names.push(format!("done.state.{parent_id}"));
            let enclosing_parallel = model
                .states
                .get(&parent_id)
                .and_then(|region| region.parent.as_ref())
                .filter(|id| model.states.get(*id).is_some_and(|s| s.is_parallel));
            if let Some(parallel_id) = enclosing_parallel {
                event_names.push(format!("done.state.{parallel_id}"));
            }
        }
        for event_name in event_names {
            model.events.insert(event_name);
        }
    }

    /// §scxml-6.4: Only add done.invoke.{id} events if transitions actually reference them.
    /// Matches Python _set_invoke_event_flags() behavior.
    fn set_invoke_event_flags(&self, model: &mut SCXMLModel) {
        // Build set of done.invoke.* events actually used in transitions
        let mut used_done_invoke_events = std::collections::BTreeSet::new();
        for state in model.states.values() {
            for trans in &state.transitions {
                if trans.event.starts_with("done.invoke.") {
                    used_done_invoke_events.insert(trans.event.clone());
                }
            }
        }

        // Set the use_specific_event flag on every Scxml/Hybrid invoke whose
        // id shows up in a `done.invoke.<id>` transition. The matching
        // specific event is also added to the model's event set so the
        // generated Event enum exposes it. `state.invokes` is the single
        // owner — no model-level mirror to keep in sync.
        let mut specific_events_to_add: Vec<String> = Vec::new();
        for state in model.states.values_mut() {
            for invoke in state.invokes.iter_mut() {
                let common: &mut InvokeSessionCommon = match invoke {
                    Invoke::Scxml(i) => &mut i.common,
                    Invoke::Hybrid(i) => &mut i.common,
                    // Neither owns an SCXML child session, so neither has
                    // an `InvokeSessionCommon` to enrich.
                    Invoke::MeshRpc(_) | Invoke::Unsupported(_) => continue,
                };
                if common.invoke_id.is_empty() {
                    continue;
                }
                let specific = format!("done.invoke.{}", common.invoke_id);
                common.use_specific_event = used_done_invoke_events.contains(&specific);
                if common.use_specific_event {
                    specific_events_to_add.push(specific);
                }
            }
        }
        for ev in specific_events_to_add {
            model.events.insert(ev);
        }
    }

    /// §scxml-6.2: Collect events from child state machines that send to parent (#_parent).
    /// Auto-adds child-to-parent events to parent Event enum for compile-time type safety.
    /// Also stamps `InvokeSessionCommon::child_has_send_to_parent` per invoke so
    /// codegen can gate the parent_sm / parent_dispatch wiring at child spawn time
    /// (§scxml-6.4 — required for test226/240/241/243/244/245/276).
    ///
    /// Inline `<content>` children are walked in-memory from
    /// [`ScxmlInvokeInfo::inline_child`]; external `src="…"` children are
    /// re-parsed from disk under `base_dir`. WASM (`base_dir == None`) still
    /// resolves inline children fully — only external `src=` invokes are
    /// silently skipped there.
    fn collect_child_to_parent_events(&self, model: &mut SCXMLModel, base_dir: Option<&Path>) {
        if !model.has_scxml_invoke() {
            return;
        }
        let mut parsed_children: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        let mut child_send_to_parent: std::collections::HashMap<String, bool> =
            std::collections::HashMap::new();

        // Snapshot ScxmlInvokeInfo (clone) so the parent model can be
        // mutated below (events insertion + per-invoke flag stamping)
        // without borrow conflicts. The cloned `inline_child` Box is the
        // structurally identical child model — no re-parse required.
        let invokes: Vec<ScxmlInvokeInfo> = model
            .states
            .values()
            .flat_map(|s| s.iter_scxml_invokes().cloned())
            .collect();
        for si in &invokes {
            if si.child_name.is_empty() || parsed_children.contains(&si.child_name) {
                continue;
            }
            parsed_children.insert(si.child_name.clone());

            // Inline children carry their parsed model on the invoke; external
            // children load from `base_dir` on demand. `owned_external` keeps
            // the disk-parsed model alive long enough for the `&SCXMLModel`
            // borrow used by the walker below.
            let owned_external;
            let child_model_ref: &SCXMLModel = if let Some(inline) = si.inline_child.as_deref() {
                inline
            } else {
                let Some(scxml_dir) = base_dir else { continue };
                let child_scxml_path = scxml_dir.join(format!("{}.scxml", si.child_name));
                if !child_scxml_path.exists() {
                    continue;
                }
                match SCXMLParser::new().parse_file(&child_scxml_path.to_string_lossy()) {
                    Ok(m) => {
                        owned_external = m;
                        &owned_external
                    }
                    Err(_) => continue,
                }
            };

            // Scan child for <send target="#_parent" event="xxx"> actions
            let mut child_parent_events = std::collections::BTreeSet::new();
            for child_state in child_model_ref.states.values() {
                // Check entry/exit actions
                for block in child_state
                    .on_entry_blocks
                    .iter()
                    .chain(child_state.on_exit_blocks.iter())
                {
                    collect_parent_send_events(block, &mut child_parent_events);
                }
                // Check transition actions
                for trans in &child_state.transitions {
                    collect_parent_send_events(&trans.actions, &mut child_parent_events);
                }
                // Check initial transition actions
                collect_parent_send_events(
                    &child_state.initial_transition_actions,
                    &mut child_parent_events,
                );
            }

            child_send_to_parent.insert(si.child_name.clone(), !child_parent_events.is_empty());

            // Add collected events to parent's event set
            for event in child_parent_events {
                model.events.insert(event);
            }
        }

        // §scxml-6.4: stamp child_has_send_to_parent on every Scxml/Hybrid
        // invoke so codegen knows to wire parent_sm / parent_dispatch before
        // child spawn. Hybrid mirrors the same flag because the spawned child
        // is a regular SCXML session whose parent-routing surface is identical.
        for state in model.states.values_mut() {
            for invoke in state.invokes.iter_mut() {
                let common = match invoke {
                    Invoke::Scxml(i) => &mut i.common,
                    Invoke::Hybrid(i) => &mut i.common,
                    // No child session spawns, so there is no
                    // parent-routing surface to mirror.
                    Invoke::MeshRpc(_) | Invoke::Unsupported(_) => continue,
                };
                if let Some(&has) = child_send_to_parent.get(&common.child_name) {
                    common.child_has_send_to_parent = has;
                }
            }
        }
    }

    fn parse_initial_children(&self, model: &mut SCXMLModel) {
        // §scxml-3.6: Parse initial attribute into list of children for ALL states
        for state in model.states.values_mut() {
            if !state.initial.is_empty() {
                state.initial_children =
                    state.initial.split_whitespace().map(String::from).collect();
            }
        }
    }

    // ── Named Context (sce:context) ─────────────────────────

    /// Parse <sce:context> elements for Named Context declarations.
    fn parse_sce_contexts(
        &self,
        root: &roxmltree::Node,
        model: &mut SCXMLModel,
        source_name: &str,
    ) -> Result<(), crate::forge::error::Located<crate::forge::error::ForgeError>> {
        use crate::forge::error::{Located, ValidationError};
        use crate::forge::model::SCE_NAMESPACE;
        for child in root.children().filter(|n| n.is_element()) {
            let is_sce_context = child.tag_name().namespace() == Some(SCE_NAMESPACE)
                && child.tag_name().name() == "context";
            if !is_sce_context {
                continue;
            }
            let ctx_id = child
                .attribute("id")
                .ok_or_else(|| {
                    Located::new(
                        ValidationError::MissingAttribute {
                            element: "<sce:context>".to_string(),
                            attr: "id".to_string(),
                        }
                        .into(),
                        source_name,
                        None,
                        None,
                    )
                })?
                .to_string();
            if model.context_object_ids.contains(&ctx_id) {
                return Err(Located::new(
                    ValidationError::DuplicateContextObject { id: ctx_id }.into(),
                    source_name,
                    None,
                    None,
                ));
            }
            let ctx_id_lower = ctx_id.to_ascii_lowercase();
            if RESERVED_CONTEXT_IDS.iter().any(|&r| r == ctx_id_lower) {
                return Err(Located::new(
                    ValidationError::ReservedContextId {
                        id: ctx_id,
                        reserved: *RESERVED_CONTEXT_IDS,
                    }
                    .into(),
                    source_name,
                    None,
                    None,
                ));
            }
            let cpp_type = child
                .attribute(("urn:sce:cpp", "type"))
                .unwrap_or("")
                .to_string();
            let cpp_include = child
                .attribute(("urn:sce:cpp", "include"))
                .unwrap_or("")
                .to_string();
            let kt_type = child
                .attribute(("urn:sce:kotlin", "type"))
                .unwrap_or("")
                .to_string();
            model.context_objects.push(ContextObject {
                id: ctx_id.clone(),
                cpp_type,
                cpp_include,
                kt_type,
            });
            model.context_object_ids.insert(ctx_id);
        }
        Ok(())
    }

    /// Collect every top-level
    /// `<sce:driver href="..."/>` element on the SCXML root. Each
    /// declaration is captured in document order with its source
    /// position so codegen can `#include` the resolved path into the
    /// C11 `*_sm.c` and so a missing `href` surfaces
    /// `validation/invalid-attribute` (via [`ValidationError::
    /// MissingAttribute`]) before downstream stages.
    ///
    /// `href` resolution against `deploy.yaml`'s `platform.driver_root`
    /// happens later at compile-model time — this method only stores
    /// the verbatim author-written string. Strays under non-root
    /// elements are ignored by design: the SCXML root content model
    /// is `xs:any namespace="##any" lax`, mirroring the W3C
    /// extension-element convention, so the parser walks only root
    /// children.
    fn parse_sce_drivers(
        &self,
        root: &roxmltree::Node,
        model: &mut SCXMLModel,
        source_name: &str,
    ) -> Result<(), crate::forge::error::Located<crate::forge::error::ForgeError>> {
        use crate::forge::error::{Located, SourceLocation, ValidationError};
        use crate::forge::model::SCE_NAMESPACE;
        use crate::model::DriverRef;
        let mut document_order: u32 = 0;
        for child in root.children().filter(|n| n.is_element()) {
            let is_sce_driver = child.tag_name().namespace() == Some(SCE_NAMESPACE)
                && child.tag_name().name() == "driver";
            if !is_sce_driver {
                continue;
            }
            let href = child.attribute("href").ok_or_else(|| {
                let pos = child.document().text_pos_at(child.range().start);
                Located::new(
                    ValidationError::MissingAttribute {
                        element: "<sce:driver>".to_string(),
                        attr: "href".to_string(),
                    }
                    .into(),
                    source_name,
                    Some(pos.row),
                    Some(pos.col),
                )
            })?;
            let pos = child.document().text_pos_at(child.range().start);
            let source_location = Some(SourceLocation {
                file: artifact_label(source_name),
                line: Some(pos.row),
                col: Some(pos.col),
            });
            model.driver_refs.push(DriverRef {
                href: href.to_string(),
                resolved_path: None,
                document_order,
                source_location,
            });
            document_order = document_order.saturating_add(1);
        }
        Ok(())
    }

    /// Listener-role declaration parsing — collect every top-level
    /// `<sce:session-role kind="..."/>` element on the SCXML root.
    /// Each declaration nominates one
    /// [`crate::model::SessionRoleKind`] variant; the orchestrator
    /// (`crate::resolve_listener_links`) joins these against
    /// deploy.yaml `LinkConfig.role` declarations.
    ///
    /// Parse-time guarantees:
    /// - `kind` attribute is required (missing fires
    ///   `validation/missing-attribute` via [`ValidationError::
    ///   MissingAttribute`]).
    /// - `kind` value is in the v1 closed set
    ///   ([`SessionRoleKind::all_wire_names`]); anything else fires
    ///   [`ValidationError::ScxmlUnknownSessionRoleKind`] with the
    ///   vocabulary list as `expected`.
    /// - The same kind declared twice on one document fires
    ///   [`ValidationError::ScxmlDuplicateSessionRoleDeclaration`].
    ///   Set semantics — multiple distinct kinds are permitted
    ///   (forward-compat for an initiator-side v2 entry alongside
    ///   accept-side; v1 has only one variant so the parse path is
    ///   exercised but the multi-kind case has no codegen consumer).
    ///
    /// Strays under non-root elements are ignored by design (same
    /// rationale as `parse_sce_drivers`); the SCXML root
    /// content model is `xs:any namespace="##any" lax`.
    fn parse_sce_session_roles(
        &self,
        root: &roxmltree::Node,
        model: &mut SCXMLModel,
        source_name: &str,
    ) -> Result<(), crate::forge::error::Located<crate::forge::error::ForgeError>> {
        use crate::forge::error::{Located, ValidationError};
        use crate::forge::model::SCE_NAMESPACE;
        use crate::model::SessionRoleKind;
        for child in root.children().filter(|n| n.is_element()) {
            let is_sce_session_role = child.tag_name().namespace() == Some(SCE_NAMESPACE)
                && child.tag_name().name() == "session-role";
            if !is_sce_session_role {
                continue;
            }
            let pos = child.document().text_pos_at(child.range().start);
            let raw_kind = child.attribute("kind").ok_or_else(|| {
                Located::new(
                    ValidationError::MissingAttribute {
                        element: "<sce:session-role>".to_string(),
                        attr: "kind".to_string(),
                    }
                    .into(),
                    source_name,
                    Some(pos.row),
                    Some(pos.col),
                )
            })?;
            let kind = SessionRoleKind::parse(raw_kind).ok_or_else(|| {
                Located::new(
                    ValidationError::ScxmlUnknownSessionRoleKind {
                        kind: raw_kind.to_string(),
                        allowed: SessionRoleKind::all_wire_names()
                            .iter()
                            .map(|s| s.to_string())
                            .collect(),
                    }
                    .into(),
                    source_name,
                    Some(pos.row),
                    Some(pos.col),
                )
            })?;
            if !model.declared_session_roles.insert(kind) {
                return Err(Located::new(
                    ValidationError::ScxmlDuplicateSessionRoleDeclaration {
                        kind: kind.as_str().to_string(),
                    }
                    .into(),
                    source_name,
                    Some(pos.row),
                    Some(pos.col),
                ));
            }
        }
        Ok(())
    }

    /// Validate that cpp:/kt: code referencing objects has <sce:context> declarations.
    fn validate_context_usage(
        &self,
        model: &SCXMLModel,
        source_name: &str,
    ) -> Result<(), crate::forge::error::Located<crate::forge::error::ForgeError>> {
        use crate::forge::error::{Located, ValidationError};
        let missing_context = |site: &str, detail: String| {
            Located::new(
                ValidationError::MissingContext {
                    site: site.to_string(),
                    detail,
                }
                .into(),
                source_name,
                None,
                None,
            )
        };
        if !model.context_object_ids.is_empty() {
            return Ok(());
        }
        static RE_OBJ: LazyLock<regex::Regex> =
            LazyLock::new(|| regex::Regex::new(r"\b([a-zA-Z_][a-zA-Z0-9_]*)\s*\.").unwrap());
        let re_obj = &*RE_OBJ;
        for state in model.states.values() {
            for trans in &state.transitions {
                if trans.is_cpp_condition && re_obj.is_match(&trans.cond_cpp) {
                    return Err(missing_context(
                        "cpp: condition",
                        format!(
                            "'{}' references objects but no <sce:context> declarations found",
                            trans.cond_cpp
                        ),
                    ));
                }
                if trans.is_kt_condition && re_obj.is_match(&trans.cond_kt) {
                    return Err(missing_context(
                        "kt: condition",
                        format!(
                            "'{}' references objects but no <sce:context> declarations found",
                            trans.cond_kt
                        ),
                    ));
                }
            }
            // Check entry/exit blocks AND transition actions for native code references
            let all_actions = state
                .on_entry_blocks
                .iter()
                .chain(state.on_exit_blocks.iter())
                .flat_map(|block| block.iter())
                .chain(state.transitions.iter().flat_map(|t| t.actions.iter()));
            for action in all_actions {
                if action.is_cpp_function && re_obj.is_match(&action.content) {
                    return Err(missing_context(
                        "<cpp>",
                        "action references objects but no <sce:context> declarations found"
                            .to_string(),
                    ));
                }
                if action.is_kt_function && re_obj.is_match(&action.content) {
                    return Err(missing_context(
                        "<kt>",
                        "action references objects but no <sce:context> declarations found"
                            .to_string(),
                    ));
                }
            }
        }
        Ok(())
    }
}

// ── Helper functions ────────────────────────────────

/// Recursively check if any if/elseif conditions within actions reference _event.*.
fn actions_contain_event_metadata(actions: &[Action]) -> bool {
    for action in actions {
        if action.action_type == "if" {
            if action.cond.contains("_event.") {
                return true;
            }
            if actions_contain_event_metadata(&action.then_actions) {
                return true;
            }
            for branch in &action.elseif_branches {
                if branch.cond.contains("_event.") {
                    return true;
                }
                if actions_contain_event_metadata(&branch.actions) {
                    return true;
                }
            }
            if actions_contain_event_metadata(&action.else_actions) {
                return true;
            }
        } else if action.action_type == "foreach" && actions_contain_event_metadata(&action.actions)
        {
            return true;
        }
    }
    false
}

// ── Named Context transforms ────────────────────────

/// Transform C++ code: replace declared context IDs with pointer dereference.
/// e.g., "hardware.powerOff()" → "this->hardware_->powerOff()"
fn transform_cpp_code_with_named_contexts(
    code: &str,
    declared_ids: &std::collections::BTreeSet<String>,
) -> String {
    let (code_with_placeholders, literals) = protect_context_strings(code);
    let mut result = code_with_placeholders;
    // Build single alternation regex for all context IDs (typically 2-3 IDs)
    let alternatives: Vec<String> = declared_ids.iter().map(|id| regex::escape(id)).collect();
    if !alternatives.is_empty() {
        let pattern = regex::Regex::new(&format!(r"\b({})\s*\.", alternatives.join("|"))).unwrap();
        result = pattern
            .replace_all(&result, |caps: &regex::Captures| {
                format!("this->{}_->", &caps[1])
            })
            .to_string();
    }
    restore_context_strings(&result, &literals)
}

/// Transform Kotlin code: replace declared context IDs with camelCase.
/// e.g., "my_hardware.powerOff()" → "myHardware.powerOff()"
fn transform_kt_code_with_named_contexts(
    code: &str,
    declared_ids: &std::collections::BTreeSet<String>,
) -> String {
    let (code_with_placeholders, literals) = protect_context_strings(code);
    let mut result = code_with_placeholders;
    // Build mapping of ids that need renaming, then single alternation regex
    let renames: Vec<(String, String)> = declared_ids
        .iter()
        .map(|id| (id.clone(), id_to_camel_case(id)))
        .filter(|(id, camel)| id != camel)
        .collect();
    if !renames.is_empty() {
        let alternatives: Vec<String> = renames.iter().map(|(id, _)| regex::escape(id)).collect();
        let pattern = regex::Regex::new(&format!(r"\b({})\b", alternatives.join("|"))).unwrap();
        result = pattern
            .replace_all(&result, |caps: &regex::Captures| {
                let matched = &caps[1];
                renames
                    .iter()
                    .find(|(id, _)| id == matched)
                    .map_or_else(|| matched.to_string(), |(_, camel)| camel.clone())
            })
            .to_string();
    }
    restore_context_strings(&result, &literals)
}

/// Convert snake_case/kebab-case identifier to camelCase (delegates to filters::to_camel_case).
fn id_to_camel_case(name: &str) -> String {
    crate::filters::to_camel_case(name.to_string())
}

/// Protect string literals in Named Context code transforms.
/// Distinct from the ECMAScript path, which no longer protects literals at
/// all: `crate::ecmascript` tokenizes the source, so a string is a token
/// rather than a region a later textual pass has to be kept out of.
fn protect_context_strings(code: &str) -> (String, Vec<String>) {
    static RE_STRING: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r#""(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*'"#).unwrap());
    let mut literals = Vec::new();
    let result = RE_STRING.replace_all(code, |caps: &regex::Captures| {
        let idx = literals.len();
        literals.push(caps[0].to_string());
        format!("__STRING_PLACEHOLDER_{idx}__")
    });
    (result.to_string(), literals)
}

/// Restore string literals from Named Context placeholders.
fn restore_context_strings(code: &str, literals: &[String]) -> String {
    let mut result = code.to_string();
    for (i, literal) in literals.iter().enumerate() {
        let placeholder = format!("__STRING_PLACEHOLDER_{i}__");
        result = result.replace(&placeholder, literal);
    }
    result
}

/// §scxml-6.5: Convert finalize actions to JavaScript code string.
/// Supports: assign, script, log, if/elseif/else.
fn actions_to_javascript(actions: &[Action]) -> String {
    if actions.is_empty() {
        return String::new();
    }
    let mut js_lines = Vec::new();
    for action in actions {
        match action.action_type.as_str() {
            "assign" => {
                if !action.location.is_empty() && !action.expr.is_empty() {
                    js_lines.push(format!("{} = {};", action.location, action.expr));
                }
            }
            "script" => {
                if !action.content.is_empty() {
                    js_lines.push(action.content.clone());
                }
            }
            "log" => {
                if !action.expr.is_empty() {
                    let log_msg = if !action.label.is_empty() {
                        format!("\"{}: \" + {}", action.label, action.expr)
                    } else {
                        action.expr.clone()
                    };
                    js_lines.push(format!("console.log({log_msg});"));
                }
            }
            "if" if !action.cond.is_empty() => {
                js_lines.push(format!("if ({}) {{", action.cond));
                if !action.then_actions.is_empty() {
                    let then_js = actions_to_javascript(&action.then_actions);
                    if !then_js.is_empty() {
                        js_lines.push(format!("  {then_js}"));
                    }
                }
                js_lines.push("}".to_string());
                for elseif in &action.elseif_branches {
                    if !elseif.cond.is_empty() {
                        js_lines.push(format!("else if ({}) {{", elseif.cond));
                        if !elseif.actions.is_empty() {
                            let elseif_js = actions_to_javascript(&elseif.actions);
                            if !elseif_js.is_empty() {
                                js_lines.push(format!("  {elseif_js}"));
                            }
                        }
                        js_lines.push("}".to_string());
                    }
                }
                if !action.else_actions.is_empty() {
                    js_lines.push("else {".to_string());
                    let else_js = actions_to_javascript(&action.else_actions);
                    if !else_js.is_empty() {
                        js_lines.push(format!("  {else_js}"));
                    }
                    js_lines.push("}".to_string());
                }
            }
            _ => {}
        }
    }
    js_lines.join(" ")
}

fn local_name(node: &roxmltree::Node) -> String {
    node.tag_name().name().to_string()
}

fn is_scxml_state_element(node: &roxmltree::Node) -> bool {
    let name = node.tag_name().name();
    matches!(name, "state" | "parallel" | "final")
}

/// True when `node`'s namespace is the W3C SCXML namespace.
///
/// §scxml-3.5 requires every SCXML document to declare the
/// namespace on the root, and `xsd_validator::validate_or_skip`
/// (run on every input at `parse_impl` boundary) rejects documents
/// that omit the declaration before this predicate is ever consulted.
/// A `None` namespace at this point therefore signals a foreign-NS
/// element inside an otherwise-valid SCXML document, not a malformed
/// root — strict `Some(SCXML_NAMESPACE)` is correct.
fn is_scxml_ns(node: &roxmltree::Node<'_, '_>) -> bool {
    node.tag_name().namespace() == Some(crate::model::SCXML_NAMESPACE)
}

/// Find SCXML-namespaced children with a given local name.
///
/// Filters by both local name AND namespace — a foreign-namespace
/// element whose local name collides with a W3C element (e.g.
/// `<framework:onentry>`) is correctly skipped. Lenient on missing
/// namespace via [`is_scxml_ns`].
fn scxml_children<'a>(
    parent: &'a roxmltree::Node<'a, 'a>,
    tag: &'a str,
) -> impl Iterator<Item = roxmltree::Node<'a, 'a>> {
    parent
        .children()
        .filter(move |c| c.is_element() && c.tag_name().name() == tag && is_scxml_ns(c))
}

/// The element names that carry a state in the model, in the order the
/// spec lists them. Shared by the document-order pre-pass and the
/// per-name parse loops so the two cannot drift apart.
const STATE_ELEMENT_TAGS: &[&str] = &["state", "final", "parallel"];

/// SCXML-namespaced children matching ANY of the given local names, in
/// document order.
///
/// Calling [`scxml_children`] once per name and chaining the results
/// groups siblings by element name, which is not document order for a
/// mixed set. See [`SCXMLParser::assign_document_order`] for why that
/// distinction is observable.
fn scxml_children_any_of<'a>(
    parent: &'a roxmltree::Node<'a, 'a>,
    tags: &'a [&'a str],
) -> impl Iterator<Item = roxmltree::Node<'a, 'a>> {
    parent
        .children()
        .filter(move |c| c.is_element() && is_scxml_ns(c) && tags.contains(&c.tag_name().name()))
}

/// Find first SCXML-namespaced child with a given local name. See
/// [`scxml_children`] for the namespace-filter contract.
fn scxml_child<'a>(
    parent: &'a roxmltree::Node<'a, 'a>,
    tag: &str,
) -> Option<roxmltree::Node<'a, 'a>> {
    parent
        .children()
        .find(|c| c.is_element() && c.tag_name().name() == tag && is_scxml_ns(c))
}

/// SCE Protocol-Synthesis RFC §synth-5-E helper: collect all `<sce:on-sample>`
/// children of a `<state>` or `<parallel>` element into the supplied
/// vector, in document order. The namespace check (`SCE_NAMESPACE`)
/// distinguishes `<sce:on-sample>` from a hypothetical W3C-namespace
/// `<on-sample>` (the latter is not an SCXML element today, but
/// future spec work may collide).
///
/// Required attributes (`link`, `event`) are recorded as authored;
/// missing values land as empty strings and are caught by the
/// downstream structural validators (`validate_on_sample_*` in
/// [`crate::scxml_semantic`]). Document order is the per-state index
/// inside `on_sample_blocks`, used as a stable diagnostic key when
/// source line numbers aren't available.
fn collect_on_sample_blocks(parent: &roxmltree::Node, out: &mut Vec<crate::model::OnSampleNode>) {
    use crate::forge::model::SCE_NAMESPACE;
    for child in parent.children() {
        if !child.is_element() {
            continue;
        }
        if child.tag_name().namespace() != Some(SCE_NAMESPACE) {
            continue;
        }
        if child.tag_name().name() != "on-sample" {
            continue;
        }
        let link = child.attribute("link").unwrap_or("").to_string();
        let event = child.attribute("event").unwrap_or("").to_string();
        // The `callback` attribute is optional. When absent, codegen
        // synthesizes a default dispatch shim (backwards-compat with
        // the link/event-only `<sce:on-sample>` shape). When present,
        // [`validate_on_sample_callback_paths`] enforces the
        // language-prefixed Rust path subset before any downstream
        // consumer sees it.
        let callback = child.attribute("callback").map(|s| s.to_string());
        let document_order = out.len() as u32;
        out.push(crate::model::OnSampleNode {
            link,
            event,
            callback,
            document_order,
        });
    }
}

/// Listener-role migration-helper —
/// fires `scxml/accept-side-states-without-role-declaration` when an
/// SCXML carries any state-id matching the canonical session-FSM
/// accept-side prefix (`Accepting` or `Accepting.*` per the trailing-
/// dot guard of `accepting_substate_present`) but has no top-level
/// `<sce:session-role kind="accept-side"/>` declaration. Repurposes
/// the predicate behaviour that was previously consumed by
/// `resolve_listener_links`'s substate-walker join; the prefix-match
/// matrix is unchanged — only the consumer flipped.
///
/// The check is parser-internal (no deploy.yaml needed) so the
/// diagnostic surfaces on any SCXML the build sees, regardless of
/// whether the document participates in a deploy listener pair.
/// `offending_ids` ride the diagnostic in document-order (the natural
/// `BTreeMap::keys` iteration on `SCXMLModel.states`) so the message
/// is deterministic across rebuilds.
fn validate_axis3_accept_side_state_naming(
    model: &crate::model::SCXMLModel,
    diag_label: &str,
) -> Result<(), crate::forge::error::Located<crate::forge::error::ForgeError>> {
    use crate::forge::error::{Located, ValidationError};
    use crate::model::SessionRoleKind;
    if model
        .declared_session_roles
        .contains(&SessionRoleKind::AcceptSide)
    {
        // Author claimed the role — naming is sanctioned.
        return Ok(());
    }
    let offending_ids: Vec<String> = model
        .states
        .keys()
        .filter(|id| *id == "Accepting" || id.starts_with("Accepting."))
        .cloned()
        .collect();
    if offending_ids.is_empty() {
        return Ok(());
    }
    Err(Located::new(
        ValidationError::ScxmlAcceptSideStatesWithoutRoleDeclaration { offending_ids }.into(),
        diag_label,
        None,
        None,
    ))
}

/// SCE Protocol-Synthesis RFC §synth-5-E `<sce:on-sample>` placement validator.
/// Walks the entire document looking for `<sce:on-sample>` elements
/// whose immediate parent is **not** `<state>` or `<parallel>`. Such
/// strays are silently ignored by [`collect_on_sample_blocks`] (it
/// only inspects children of the two valid parents); without this
/// validator they would land as unrecorded SCXML noise. The
/// diagnostic carries a descriptive XML path so authors see exactly
/// which boundary was crossed.
fn validate_on_sample_placement(
    root: &roxmltree::Node,
    diag_label: &str,
) -> Result<(), crate::forge::error::Located<crate::forge::error::ForgeError>> {
    use crate::forge::error::{Located, ValidationError};
    use crate::forge::model::SCE_NAMESPACE;
    for desc in root.document().descendants() {
        if !desc.is_element() {
            continue;
        }
        if desc.tag_name().namespace() != Some(SCE_NAMESPACE) {
            continue;
        }
        if desc.tag_name().name() != "on-sample" {
            continue;
        }
        // The parent is the element directly enclosing this node;
        // root document nodes have no parent (this branch never fires
        // for the root, but guard explicitly so the helper is safe to
        // call on any subtree).
        let parent = match desc.parent_element() {
            Some(p) => p,
            None => continue,
        };
        let parent_name = parent.tag_name().name();
        if parent_name == "state" || parent_name == "parallel" {
            continue;
        }
        // Build a descriptive path by climbing parents — each step
        // emits the local tag name. The walk stops at the document
        // root so the path stays bounded even for deeply nested trees.
        let mut chain: Vec<&str> = Vec::new();
        let mut cursor = Some(parent);
        while let Some(node) = cursor {
            chain.push(node.tag_name().name());
            cursor = node.parent_element();
        }
        chain.reverse();
        let path = if chain.is_empty() {
            "<root>".to_string()
        } else {
            chain.join(" > ")
        };
        return Err(Located::new(
            ValidationError::OnSampleInvalidParent {
                path,
                actual_parent: parent_name.to_string(),
            }
            .into(),
            diag_label,
            None,
            None,
        ));
    }
    Ok(())
}

/// SCE Protocol-Synthesis RFC §synth-5-E `<sce:on-sample>` uniqueness validator.
/// Each `<sce:on-sample link="X">` block must appear at most once per
/// state — duplicate registrations on the same link compete for the
/// same RX callback slot at runtime, producing undefined dispatch
/// order. The check is per-state; the same link may appear in
/// multiple distinct states (different subscriptions per state is
/// the canonical fan-out pattern).
fn validate_on_sample_uniqueness(
    model: &crate::model::SCXMLModel,
    diag_label: &str,
) -> Result<(), crate::forge::error::Located<crate::forge::error::ForgeError>> {
    use crate::forge::error::{Located, ValidationError};
    use std::collections::BTreeSet;
    // Iterate states in deterministic order so the diagnostic surface
    // for parallel duplicate-finding matches across runs.
    let mut state_ids: Vec<&String> = model.states.keys().collect();
    state_ids.sort();
    for state_id in state_ids {
        let state = &model.states[state_id];
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        for block in &state.on_sample_blocks {
            if !seen.insert(block.link.as_str()) {
                return Err(Located::new(
                    ValidationError::OnSampleLinkDuplicateInState {
                        state_id: state_id.clone(),
                        link: block.link.clone(),
                    }
                    .into(),
                    diag_label,
                    None,
                    None,
                ));
            }
        }
    }
    Ok(())
}

/// SCE Protocol-Synthesis RFC §synth-5-E `<sce:on-sample>` event-name conflict
/// validator. §scxml-5.10 reserves the `error.*` and `done.*`
/// event-name families for built-in lifecycle events; an author
/// dispatching a sample-arrival event into one of these families
/// would silently crosstalk the W3C-prescribed semantics. The check
/// is conservative — the prefix match (`event` either equals
/// `error`/`done` or starts with `error.`/`done.`) covers both
/// bare-name use ("error") and dotted descendants ("error.io.foo").
fn validate_on_sample_event_names(
    model: &crate::model::SCXMLModel,
    diag_label: &str,
) -> Result<(), crate::forge::error::Located<crate::forge::error::ForgeError>> {
    use crate::forge::error::{Located, ValidationError};
    let mut state_ids: Vec<&String> = model.states.keys().collect();
    state_ids.sort();
    for state_id in state_ids {
        let state = &model.states[state_id];
        for block in &state.on_sample_blocks {
            let event = block.event.as_str();
            let reserved_prefix = if event == "error" || event.starts_with("error.") {
                Some("error.")
            } else if event == "done" || event.starts_with("done.") {
                Some("done.")
            } else {
                None
            };
            if let Some(reserved) = reserved_prefix {
                return Err(Located::new(
                    ValidationError::OnSampleEventNameConflict {
                        event: event.to_string(),
                        reserved_prefix: reserved.to_string(),
                    }
                    .into(),
                    diag_label,
                    None,
                    None,
                ));
            }
        }
    }
    Ok(())
}

/// SCE Protocol-Synthesis RFC §synth-5-E callback path validator
/// (`pool/sample-callback-signature-non-borrow`, spec lines 1516-1519).
/// When `<sce:on-sample callback="...">` is present (an extern
/// reference), enforce the language-prefixed Rust
/// path subset before any downstream consumer ever sees the value:
///
/// ```text
/// callback ::= "rust:" segment ("::" segment)*
/// segment  ::= NCName-equivalent ("crate" | "self" | "super" | identifier)
/// ```
///
/// SCE-side detection here is path syntax; the
/// signature-shape check (borrow vs owned) flows through rustc at
/// user-crate compile time. Both shapes raise the
/// same spec-verbatim diagnostic code; today only the path-syntax arm
/// is reachable. SCE does not inspect the signature itself because
/// rustc already rejects the mismatch when the user crate compiles —
/// a second implementation here could only duplicate or drift from
/// the compiler's answer.
///
/// `feedback_silently_broken_hooks.md` compliance: every variant of
/// the syntax check is reachable from authoring inputs (an unknown
/// `cpp:` prefix, a leading `::`, a malformed segment, an empty
/// path), so the diagnostic is real today. Forward-compat hook for
/// future signature inspection layers on top of A2's surface
/// without renaming the code.
fn validate_on_sample_callback_paths(
    model: &crate::model::SCXMLModel,
    diag_label: &str,
) -> Result<(), crate::forge::error::Located<crate::forge::error::ForgeError>> {
    use crate::forge::error::{Located, ValidationError};
    let mut state_ids: Vec<&String> = model.states.keys().collect();
    state_ids.sort();
    for state_id in state_ids {
        let state = &model.states[state_id];
        for block in &state.on_sample_blocks {
            let Some(callback) = block.callback.as_deref() else {
                continue;
            };
            if let Some(reason) = classify_on_sample_callback_path(callback) {
                return Err(Located::new(
                    ValidationError::PoolSampleCallbackSignatureNonBorrow {
                        state_id: state_id.clone(),
                        link: block.link.clone(),
                        callback: callback.to_string(),
                        reason,
                    }
                    .into(),
                    diag_label,
                    None,
                    None,
                ));
            }
        }
    }
    Ok(())
}

/// Classify an `<sce:on-sample callback="...">` value against the
/// accepted path subset for its declared language. Returns `None` for
/// accepted inputs; returns `Some(reason)` keyed by the failure mode so
/// the diagnostic message can name the specific authoring mistake.
///
/// Two language axes are accepted, one per backend that has a lowering
/// for the callback:
///
/// ```text
/// callback ::= "rust:" rust_path | "c:" c_identifier
/// rust_path ::= segment ("::" segment)*
/// segment   ::= "crate" | "self" | "super" | identifier
/// ```
///
/// `c:` takes a bare identifier because C has no namespaces — a `::`
/// under `c:` is an author reaching for a Rust path on the wrong axis,
/// and reporting it as `MalformedPath` names that directly rather than
/// letting `app::on_scout` reach the C call site as a syntax error in
/// generated code.
///
/// Checks (first failure wins, in order):
/// 1. Empty value → `EmptyPath`.
/// 2. Missing language prefix or unknown prefix → `UnknownLanguagePrefix`.
///    The `prefix` field carries the parsed prefix (or `""` when the
///    colon is absent).
/// 3. Path body empty after the prefix → `EmptyPath`.
/// 4. Path body has a leading `::`, trailing `::`, or empty segment, or
///    carries any `::` at all under `c:` → `MalformedPath`.
/// 5. Any segment fails its language's identifier subset →
///    `MalformedSegment`.
fn classify_on_sample_callback_path(raw: &str) -> Option<crate::forge::error::CallbackPathReason> {
    use crate::forge::error::CallbackPathReason;
    if raw.is_empty() {
        return Some(CallbackPathReason::EmptyPath);
    }
    let (prefix, body) = match raw.split_once(':') {
        Some(t) => t,
        None => {
            return Some(CallbackPathReason::UnknownLanguagePrefix {
                prefix: String::new(),
            });
        }
    };
    if !matches!(prefix, "rust" | "c") {
        return Some(CallbackPathReason::UnknownLanguagePrefix {
            prefix: prefix.to_string(),
        });
    }
    if body.is_empty() {
        return Some(CallbackPathReason::EmptyPath);
    }
    if body.starts_with("::") || body.ends_with("::") {
        return Some(CallbackPathReason::MalformedPath);
    }
    if prefix == "c" && body.contains("::") {
        // A path separator on the C axis: C resolves one flat
        // identifier, so this is a Rust path declared under `c:`.
        return Some(CallbackPathReason::MalformedPath);
    }
    let segments: Vec<&str> = body.split("::").collect();
    for segment in &segments {
        if segment.is_empty() {
            // `a::::b` — split between two `::` produces an empty arm.
            return Some(CallbackPathReason::MalformedPath);
        }
        // `crate` / `self` / `super` are Rust path keywords, not C
        // identifiers; `is_c_identifier` admits the plain-identifier
        // subset both languages share and nothing else.
        let accepted = match prefix {
            "c" => is_c_identifier(segment),
            _ => is_rust_path_segment(segment),
        };
        if !accepted {
            return Some(CallbackPathReason::MalformedSegment {
                segment: (*segment).to_string(),
            });
        }
    }
    None
}

/// True iff `seg` is a C identifier: an ASCII letter or `_` followed by
/// ASCII alphanumerics or `_`.
///
/// Deliberately narrower than C's own grammar, which admits `$` on some
/// implementations and universal character names on all of them. The
/// value here is emitted verbatim into generated C, so the subset is
/// the portable intersection rather than what any one compiler accepts.
fn is_c_identifier(seg: &str) -> bool {
    let mut chars = seg.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// True iff `seg` is a valid Rust path segment per the accepted
/// subset: either one of the path keywords (`crate` / `self` /
/// `super`) or an NCName-equivalent identifier (ASCII letter or `_`,
/// then letters / digits / `_`). The Rust language admits `r#`-raw
/// identifiers and Unicode identifiers; the validator keeps the subset narrow
/// because `<sce:on-sample callback>` author input rarely needs them
/// and the wider grammar amplifies the validator's surface for no
/// observed authoring benefit.
fn is_rust_path_segment(seg: &str) -> bool {
    if matches!(seg, "crate" | "self" | "super") {
        return true;
    }
    let mut chars = seg.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// SCE Protocol-Synthesis RFC §synth-5-E `<sce:on-sample>` cross-ref
/// validator. Walks every state's `on_sample_blocks` and looks each
/// `link=` reference up in the supplied [`SceCrossDocRegistry`]
/// (built once per build by walking every parsed `.forge` file).
/// Two diagnostics emerge:
///
/// * `scxml/on-sample-link-not-declared` — name not present in any
///   `.forge` file in the build. The candidate list rides
///   `Fix::ReplaceOneOf` over the registry's declared link kind
///   names (sorted) so authors can pick a legal link or extend the
///   build with the missing forge file.
/// * `scxml/on-sample-link-wrong-kind` — name resolves to a forge
///   artifact whose kind is not `link` (today the only kind that
///   backs the on-sample subscriber contract). Wired
///   forward-compat: the single-variant `ScxmlDocKind` registry
///   only ever stores `Link`, so the validator's match never reaches
///   the `Some(non-Link)` arm in production until a future
///   cross-registry generalization grows the enum. Forward-compat
///   wiring mirrors the stage_pool `mesh/deploy-stage-pool-wrong-kind`
///   precedent.
///
/// Runs as a post-parse pass — `parse_string` produces the
/// `SCXMLModel` first, the build pipeline assembles the
/// [`SceCrossDocRegistry`] from every parsed `.forge` file, then this
/// validator checks each declared reference against the registry.
/// Structural validators
/// (`validate_on_sample_placement` / `_uniqueness` / `_event_names`)
/// already gated the reference set; any block that reaches this
/// validator has cleared the structural gates.
pub fn validate_on_sample_link_references(
    model: &crate::model::SCXMLModel,
    link_registry: &crate::forge::cross_doc_registry::SceCrossDocRegistry,
    pool_registry: &crate::forge::pool_registry::ForgePoolRegistry,
    diag_label: &str,
) -> Result<(), crate::forge::error::Located<crate::forge::error::ForgeError>> {
    use crate::forge::cross_doc_registry::ScxmlDocKind;
    use crate::forge::error::{Located, ValidationError};
    use crate::forge::pool_registry::ForgePoolKind;
    let mut state_ids: Vec<&String> = model.states.keys().collect();
    state_ids.sort();
    for state_id in state_ids {
        let state = &model.states[state_id];
        for block in &state.on_sample_blocks {
            match link_registry.lookup(&block.link) {
                Some(ScxmlDocKind::Link) => {} // canonical case
                Some(other_kind) => {
                    // RFC §synth-5-E `scxml/on-sample-link-
                    // wrong-kind`. The name resolves but the resolved
                    // doc is not a link (today: statechart or worker
                    // per the worker-outbox registry extension).
                    // The original on-sample diagnostic was wired
                    // forward-compat for exactly this growth point —
                    // see error.rs `OnSampleLinkWrongKind` doc-comment.
                    let candidates = link_registry.names_of_kind(ScxmlDocKind::Link);
                    return Err(Located::new(
                        ValidationError::OnSampleLinkWrongKind {
                            state_id: state_id.clone(),
                            link: block.link.clone(),
                            actual_kind: other_kind.as_str().to_string(),
                            candidates,
                        }
                        .into(),
                        diag_label,
                        None,
                        None,
                    ));
                }
                None => {
                    let candidates = link_registry.names_of_kind(ScxmlDocKind::Link);
                    return Err(Located::new(
                        ValidationError::OnSampleLinkNotDeclared {
                            state_id: state_id.clone(),
                            link: block.link.clone(),
                            candidates,
                        }
                        .into(),
                        diag_label,
                        None,
                        None,
                    ));
                }
            }

            // RFC §synth-5-E stage-pool gate: `pool/sample-take-without-
            // stage-pool`. Every state that subscribes to a link
            // (`<sce:on-sample link="X">`) MUST be backed by a link
            // whose `<sce:stage-pool>` declares where `Sample::take()`
            // copies into. Absence is the canonical reason for the
            // sce-link-runtime `LinkConfig` to wire the
            // `PanicOnTakeHook` default — the diagnostic surfaces
            // that mismatch at codegen time so callbacks that ever
            // call `take()` aren't silently routed to a panic.
            // Borrow-only callbacks remain perfectly legal: they
            // just never trigger this check (they don't write
            // `<sce:on-sample>`-driven subscribers that escape the
            // borrow lifetime).
            if link_registry.lookup_stage_pool(&block.link).is_none() {
                let candidates = pool_registry.names_of_kind(ForgePoolKind::BufferPool);
                return Err(Located::new(
                    ValidationError::PoolSampleTakeWithoutStagePool {
                        state_id: state_id.clone(),
                        link: block.link.clone(),
                        candidates,
                    }
                    .into(),
                    diag_label,
                    None,
                    None,
                ));
            }
        }
    }
    Ok(())
}

/// XML node serialization matching Python lxml etree.tostring(method='xml').
/// Includes namespace declarations and uses self-closing for empty elements.
fn serialize_node(node: &roxmltree::Node) -> String {
    serialize_node_inner(node, None, false)
}

/// XML node serialization matching Python lxml etree.tostring(method='c14n').
/// Always uses explicit close tags (never self-closing).
fn serialize_node_c14n(node: &roxmltree::Node) -> String {
    serialize_node_inner(node, None, true)
}

fn serialize_node_inner(node: &roxmltree::Node, parent_ns: Option<&str>, c14n: bool) -> String {
    if !node.is_element() {
        return node.text().unwrap_or("").to_string();
    }
    let tag = node.tag_name().name();
    let mut result = format!("<{tag}");

    // Include namespace declaration only if it differs from parent
    let my_ns = node.tag_name().namespace();
    if my_ns != parent_ns {
        if let Some(ns) = my_ns {
            result.push_str(&format!(" xmlns=\"{ns}\""));
        } else if parent_ns.is_some() {
            result.push_str(" xmlns=\"\"");
        }
    }

    for attr in node.attributes() {
        result.push_str(&format!(" {}=\"{}\"", attr.name(), attr.value()));
    }

    // Check if element has any meaningful children (elements or non-empty text)
    let has_children = node
        .children()
        .any(|c| c.is_element() || (c.is_text() && !c.text().unwrap_or("").trim().is_empty()));

    if !has_children && !c14n {
        // Self-closing tag for empty elements (matches lxml method='xml')
        result.push_str("/>");
    } else {
        result.push('>');
        for child in node.children() {
            if child.is_element() {
                result.push_str(&serialize_node_inner(&child, my_ns, c14n));
            } else if child.is_text() {
                result.push_str(child.text().unwrap_or(""));
            }
        }
        result.push_str(&format!("</{tag}>"));
    }
    result
}

/// §scxml-5.9.2: Check if expression is pure In() predicate
pub(crate) fn is_pure_in_predicate(cond: &str) -> bool {
    static RE_IN_CALL: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r#"In\(['"][^'"]+['"]\)"#).unwrap());

    let trimmed = cond.trim();
    if trimmed.is_empty() {
        return false;
    }
    if !trimmed.contains("In(") {
        return false;
    }
    // Check if expression consists only of In() calls, &&, ||, !, (, ), whitespace
    let cleaned = RE_IN_CALL.replace_all(trimmed, "TRUE");
    let cleaned = cleaned
        .replace("&&", " ")
        .replace("||", " ")
        .replace(['!', '(', ')'], " ");
    cleaned.split_whitespace().all(|w| w == "TRUE")
}

/// Shared regex for In() predicate with capture group
static RE_IN_PREDICATE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r#"In\(['"]([^'"]+)['"]\)"#).unwrap());

/// Convert In() predicate to C++ isStateActive calls
fn convert_in_to_cpp(cond: &str) -> String {
    RE_IN_PREDICATE
        .replace_all(cond, r#"this->isStateActive("$1")"#)
        .to_string()
}

/// Convert In() predicate to Kotlin isStateActive calls
fn convert_in_to_kotlin(cond: &str) -> String {
    RE_IN_PREDICATE
        .replace_all(cond, r#"isStateActive("$1")"#)
        .to_string()
}

/// Whether a condition has to be evaluated by the data model, and
/// whether it uses the `In()` predicate.
///
/// The single classifier: [`crate::script_engine_analyzer`] replays it
/// on a fully-parsed model rather than reimplementing the boundary, and
/// every backend's guard arm is chosen by its answer.
///
/// Four conditions do not need the data model, and the list is closed
/// rather than sampled:
///
/// * a `cpp:` / `kt:` condition — the author wrote target-language
///   source, and only that backend lowers it;
/// * a pure `In(...)` predicate — the specification answers it from the
///   active configuration alone;
/// * a condition the frontend decides at build time, which
///   [`crate::ecmascript::constant_truthiness`] folds to a boolean the
///   emitters print as their own literal;
/// * a typed `_event.data` guard under an imported EventSchema, decided
///   by the analyzer after this because it needs the schema.
///
/// Everything else needs the data model. This used to be decided by
/// substring — a list of operators, quote characters and reserved words
/// — whose fallthrough was *native*, so a condition the list did not
/// recognise was emitted as target-language source. `cond="1"` became
/// Rust `if 1 {`, `cond="x"` became `if x {`, and `cond="Math.abs(1)"`
/// reached the backend without ever passing the frontend that owns the
/// name `Math`. All three generated cleanly and reported nothing.
pub(crate) fn check_expression_needs(cond: &str) -> (bool, bool) {
    if cond.trim().is_empty() {
        return (false, false);
    }
    if cond.starts_with("cpp:") || cond.starts_with("kt:") {
        return (false, false);
    }
    let has_in = cond.contains("In(");
    if is_pure_in_predicate(cond) {
        return (false, has_in);
    }
    // A mixed `In(...) && x` reads the configuration *and* the data
    // model, so the data model evaluates the whole of it.
    if has_in {
        return (true, true);
    }
    // §scxml-5.9: a `cond` is evaluated in the data model, and one the
    // Processor cannot evaluate raises `error.execution` and reads as
    // false. Both halves are the data model's to perform, so everything
    // this far down goes to it — unless SCE decided the value itself,
    // in which case there is nothing left to evaluate.
    (
        crate::ecmascript::constant_truthiness(cond).is_none(),
        false,
    )
}

fn parse_delay_to_ms(delay: &str) -> i64 {
    let trimmed = delay.trim();
    if trimmed.is_empty() {
        return 0;
    }
    if let Some(s) = trimmed.strip_suffix("ms") {
        s.trim().parse().unwrap_or(0)
    } else if let Some(s) = trimmed.strip_suffix('s') {
        s.trim()
            .parse::<f64>()
            .map(|v| (v * 1000.0) as i64)
            .unwrap_or(0)
    } else {
        // Bare number: default to seconds (common in W3C test suite)
        trimmed
            .parse::<f64>()
            .map(|v| (v * 1000.0) as i64)
            .unwrap_or(0)
    }
}

fn is_static_string_literal(expr: &str) -> bool {
    let trimmed = expr.trim();
    trimmed.len() >= 2
        && ((trimmed.starts_with('\'') && trimmed.ends_with('\''))
            || (trimmed.starts_with('"') && trimmed.ends_with('"')))
}

fn extract_static_string_literal(expr: &str) -> String {
    let trimmed = expr.trim();
    if trimmed.len() >= 2 {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        String::new()
    }
}

/// Scan actions for <send target="#_parent" event="xxx"> and collect event names.
fn collect_parent_send_events(actions: &[Action], events: &mut std::collections::BTreeSet<String>) {
    for action in actions {
        if action.action_type == "send" && action.target == "#_parent" && !action.event.is_empty() {
            events.insert(action.event.clone());
        }
        // Recurse into nested actions (if/then/else, foreach)
        collect_parent_send_events(&action.then_actions, events);
        collect_parent_send_events(&action.else_actions, events);
        collect_parent_send_events(&action.actions, events);
        for branch in &action.elseif_branches {
            collect_parent_send_events(&branch.actions, events);
        }
    }
}

/// §scxml-6.2 (test187/207): detect whether the child SCXML carries
/// any `<send delay="...">` / `<send delayexpr="...">`. The child's
/// codegen emits a scheduler queue + `_tick` entry point only when
/// this returns `true`; the parent's invoke driver mirrors the gate so
/// it knows whether to call the child's `_tick` from `_drive_active_children`.
fn child_has_delayed_send(child_model: &SCXMLModel) -> bool {
    fn walk(actions: &[Action]) -> bool {
        for action in actions {
            if action.action_type == "send"
                && (!action.delay.is_empty() || !action.delayexpr.is_empty())
            {
                return true;
            }
            if walk(&action.then_actions)
                || walk(&action.else_actions)
                || walk(&action.actions)
                || action.elseif_branches.iter().any(|b| walk(&b.actions))
            {
                return true;
            }
        }
        false
    }
    for state in child_model.states.values() {
        for block in state
            .on_entry_blocks
            .iter()
            .chain(state.on_exit_blocks.iter())
        {
            if walk(block) {
                return true;
            }
        }
        for trans in &state.transitions {
            if walk(&trans.actions) {
                return true;
            }
        }
        if walk(&state.initial_transition_actions) {
            return true;
        }
    }
    false
}

/// Parse a child SCXML file to extract metadata (needs_script_engine, datamodel vars).
///
/// The fields written (`child_needs_script_engine`, `child_datamodel_vars`)
/// are session-only — they live on [`InvokeSessionCommon`]. Both
/// `ScxmlInvokeInfo` and `HybridInvokeInfo` expose this via `&mut si.common`.
/// Populate child-side invoke metadata from an already-parsed
/// [`SCXMLModel`]. Shared by the in-memory inline-child path
/// (`parse_invoke` → `ScxmlInvokeInfo::inline_child`) and the on-disk
/// external path ([`parse_child_metadata`]), so both invoke flavours
/// derive the same `child_needs_script_engine` / `child_datamodel_vars`
/// / `child_needs_event_scheduler` from the same walker.
fn populate_child_metadata_from_model(child_model: &SCXMLModel, common: &mut InvokeSessionCommon) {
    common.child_needs_script_engine = child_model.needs_script_engine;
    common.child_datamodel_vars =
        Some(child_model.variables.iter().map(|v| v.id.clone()).collect());
    // §scxml-6.2 (test187/207): mirror the child's own scheduler
    // requirement. The child's codegen emits `_tick` only when
    // its scheduler queue is non-empty; the parent's invoke driver
    // must know whether that entry point exists at template time.
    // The model's `needs_event_scheduler` field is set by the
    // analyzer (post-parse), so we re-derive here by walking the
    // child's <send> actions for any `delay` / `delayexpr`.
    common.child_needs_event_scheduler = child_has_delayed_send(child_model);
}

fn parse_child_metadata(child_path: &Path, common: &mut InvokeSessionCommon) {
    if !child_path.exists() {
        common.child_needs_script_engine = true;
        common.child_datamodel_vars = Some(Vec::new());
        return;
    }
    match SCXMLParser::new().parse_file(&child_path.to_string_lossy()) {
        Ok(child_model) => populate_child_metadata_from_model(&child_model, common),
        Err(_) => {
            common.child_needs_script_engine = true;
            common.child_datamodel_vars = Some(Vec::new());
        }
    }
}

// ══════════════════════════════════════════════════════════════
// ── Post-expansion diagnostic coordinate remapping ──────────
// ══════════════════════════════════════════════════════════════

/// Translate an expanded-document `Located<ForgeError>` back to
/// author source coordinates using the xinclude expansion map.
///
/// Applied at the `parse_impl` boundary so every validator /
/// emitter in the pipeline stays oblivious to the map — they emit
/// expanded coordinates (as today), and this single function
/// rewrites them on the way out. No-op when the map is identity,
/// which keeps the common case (documents without `<xi:include>`)
/// byte-identical to the pre-map behaviour.
///
/// Also walks into `XmlError::SchemaValidation` so every
/// per-record libxml2 line carried by `XsdErrors` remaps too —
/// those entries are the multi-record container the diagnostic
/// emitter iterates, and the outer `Located`'s single (line, col)
/// is `None/None` for schema validation, so without walking in
/// the XSD lines would stay in expanded coordinates.
fn remap_post_expansion(
    mut err: crate::forge::error::Located<crate::forge::error::ForgeError>,
    expanded_text: &str,
    map: &crate::position_map::PositionMap,
) -> crate::forge::error::Located<crate::forge::error::ForgeError> {
    use crate::forge::error::{ForgeError, XmlError};
    if map.is_identity() {
        return err;
    }

    // ── outer (line, col) on the Located wrapper ────────────
    if let (Some(line), Some(col)) = (err.location.line, err.location.col) {
        let offset = crate::position_map::rowcol_to_offset(expanded_text, line, col);
        let src = map.lookup(offset);
        err.location.file = src.file.display().to_string();
        err.location.line = Some(src.row);
        err.location.col = Some(src.col);
    }

    // ── XsdErrors multi-record container (per-record lines) ─
    if let ForgeError::Xml(XmlError::SchemaValidation(ref mut xsd)) = err.error {
        for record in &mut xsd.diagnostics {
            if let Some(line) = record.line {
                // libxml2 reports a 1-based line and sometimes a
                // column; resolve at the column it gave us, or
                // column 1 when it is missing.
                let col = record.col.unwrap_or(1);
                let offset = crate::position_map::rowcol_to_offset(expanded_text, line, col);
                let src = map.lookup(offset);
                record.line = Some(src.row);
                if record.col.is_some() {
                    record.col = Some(src.col);
                }
            }
        }
        // Keep the container's `source_label` consistent with the
        // rewritten outer location — downstream NDJSON emission
        // pulls the per-record file from this field, so leaving it
        // as the expanded-document label would mis-attribute each
        // XsdDiag.
        xsd.source_label = err.location.file.clone();
    }

    err
}

// ══════════════════════════════════════════════════════════════
// ── Unit tests ───────────────────────────────────────────────
// ══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    // ── parse_delay_to_ms ────────────────────────────────────

    #[test]
    fn delay_empty_string() {
        assert_eq!(parse_delay_to_ms(""), 0);
    }

    #[test]
    fn delay_whitespace_only() {
        assert_eq!(parse_delay_to_ms("   "), 0);
    }

    #[test]
    fn delay_milliseconds() {
        assert_eq!(parse_delay_to_ms("500ms"), 500);
    }

    #[test]
    fn delay_milliseconds_with_whitespace() {
        assert_eq!(parse_delay_to_ms("  100 ms"), 100);
    }

    #[test]
    fn delay_zero_ms() {
        assert_eq!(parse_delay_to_ms("0ms"), 0);
    }

    #[test]
    fn delay_seconds_integer() {
        assert_eq!(parse_delay_to_ms("2s"), 2000);
    }

    #[test]
    fn delay_seconds_fractional() {
        assert_eq!(parse_delay_to_ms("1.5s"), 1500);
    }

    #[test]
    fn delay_seconds_with_whitespace() {
        assert_eq!(parse_delay_to_ms("  3 s"), 3000);
    }

    #[test]
    fn delay_bare_number_treated_as_seconds() {
        assert_eq!(parse_delay_to_ms("5"), 5000);
    }

    #[test]
    fn delay_bare_fractional_as_seconds() {
        assert_eq!(parse_delay_to_ms("0.5"), 500);
    }

    #[test]
    fn delay_invalid_numeric_returns_zero() {
        assert_eq!(parse_delay_to_ms("abc"), 0);
    }

    #[test]
    fn delay_invalid_ms_suffix_returns_zero() {
        assert_eq!(parse_delay_to_ms("xyzms"), 0);
    }

    #[test]
    fn delay_negative_ms() {
        assert_eq!(parse_delay_to_ms("-100ms"), -100);
    }

    // ── is_pure_in_predicate ─────────────────────────────────

    #[test]
    fn pure_in_single() {
        assert!(is_pure_in_predicate("In('s1')"));
    }

    #[test]
    fn pure_in_double_quotes() {
        assert!(is_pure_in_predicate("In(\"s1\")"));
    }

    #[test]
    fn pure_in_conjunction() {
        assert!(is_pure_in_predicate("In('s1') && In('s2')"));
    }

    #[test]
    fn pure_in_disjunction() {
        assert!(is_pure_in_predicate("In('s1') || In('s2')"));
    }

    #[test]
    fn pure_in_negation() {
        assert!(is_pure_in_predicate("!In('s1')"));
    }

    #[test]
    fn pure_in_complex_boolean() {
        assert!(is_pure_in_predicate("(In('s1') && !In('s2')) || In('s3')"));
    }

    #[test]
    fn not_pure_in_empty() {
        assert!(!is_pure_in_predicate(""));
    }

    #[test]
    fn not_pure_in_no_in_call() {
        assert!(!is_pure_in_predicate("x > 5"));
    }

    #[test]
    fn not_pure_in_mixed_with_ecmascript() {
        assert!(!is_pure_in_predicate("In('s1') && x > 5"));
    }

    #[test]
    fn not_pure_in_bare_variable() {
        assert!(!is_pure_in_predicate("In('s1') && someVar"));
    }

    // ── convert_in_to_cpp / convert_in_to_kotlin ─────────────

    #[test]
    fn convert_in_cpp_single() {
        assert_eq!(
            convert_in_to_cpp("In('active')"),
            "this->isStateActive(\"active\")"
        );
    }

    #[test]
    fn convert_in_cpp_conjunction() {
        assert_eq!(
            convert_in_to_cpp("In('s1') && In('s2')"),
            "this->isStateActive(\"s1\") && this->isStateActive(\"s2\")"
        );
    }

    #[test]
    fn convert_in_cpp_double_quotes() {
        assert_eq!(
            convert_in_to_cpp("In(\"running\")"),
            "this->isStateActive(\"running\")"
        );
    }

    #[test]
    fn convert_in_kotlin_single() {
        assert_eq!(
            convert_in_to_kotlin("In('active')"),
            "isStateActive(\"active\")"
        );
    }

    #[test]
    fn convert_in_kotlin_negation() {
        assert_eq!(
            convert_in_to_kotlin("!In('idle')"),
            "!isStateActive(\"idle\")"
        );
    }

    // ── check_expression_needs ───────────────────────────────

    #[test]
    fn expr_needs_empty() {
        assert_eq!(check_expression_needs(""), (false, false));
    }

    #[test]
    fn expr_needs_cpp_prefix_no_engine() {
        assert_eq!(check_expression_needs("cpp:someExpr"), (false, false));
    }

    #[test]
    fn expr_needs_kt_prefix_no_engine() {
        assert_eq!(check_expression_needs("kt:someExpr"), (false, false));
    }

    #[test]
    fn expr_needs_pure_in_no_engine() {
        let (needs_engine, has_in) = check_expression_needs("In('s1')");
        assert!(!needs_engine);
        assert!(has_in);
    }

    #[test]
    fn expr_needs_mixed_in_requires_engine() {
        let (needs_engine, has_in) = check_expression_needs("In('s1') && x > 5");
        assert!(needs_engine);
        assert!(has_in);
    }

    #[test]
    fn expr_needs_typeof_requires_engine() {
        let (needs_engine, _) = check_expression_needs("typeof x === 'number'");
        assert!(needs_engine);
    }

    #[test]
    fn expr_needs_event_dot_requires_engine() {
        let (needs_engine, _) = check_expression_needs("_event.data");
        assert!(needs_engine);
    }

    #[test]
    fn expr_needs_underscore_system_var() {
        let (needs_engine, _) = check_expression_needs("_sessionid");
        assert!(needs_engine);
    }

    #[test]
    fn expr_needs_comparison_operators() {
        assert!(check_expression_needs("x == 5").0);
        assert!(check_expression_needs("x != 5").0);
        assert!(check_expression_needs("x === 5").0);
        assert!(check_expression_needs("x !== 5").0);
        assert!(check_expression_needs("x < 5").0);
        assert!(check_expression_needs("x > 5").0);
        assert!(check_expression_needs("x <= 5").0);
        assert!(check_expression_needs("x >= 5").0);
    }

    #[test]
    fn expr_needs_logical_operators() {
        assert!(check_expression_needs("a && b").0);
        assert!(check_expression_needs("a || b").0);
    }

    /// A string literal is decided here, not by the data model.
    ///
    /// This asserted the opposite while the classifier looked for a
    /// quote character: a literal was sent to an engine because it
    /// *contained* something the list recognised. ECMA-262 9.2 gives it
    /// a value with nothing bound, so W3C test 449 — `cond="'foo'"`,
    /// "test that ecmascript objects are converted to booleans inside
    /// cond" — is a pure-static machine.
    #[test]
    fn expr_string_literal_is_decided_here() {
        assert!(!check_expression_needs("'hello'").0);
        assert!(!check_expression_needs("\"hello\"").0);
        assert!(!check_expression_needs("''").0);
    }

    #[test]
    fn expr_needs_reserved_keyword() {
        assert!(check_expression_needs("return").0);
        assert!(check_expression_needs("if(true)").0);
    }

    /// A name that merely begins with a reserved word is an identifier,
    /// and an identifier needs the data model like any other.
    ///
    /// The boundary this checks used to decide something else: a name
    /// that did *not* match the keyword list fell through to "native"
    /// and was emitted as target-language source. `ifelse` is a
    /// perfectly good `<data id>`, and the question a classifier can
    /// answer about it is not whether it looks like a keyword.
    #[test]
    fn expr_a_name_beginning_with_a_keyword_is_still_a_name() {
        assert!(check_expression_needs("ifelse").0);
        assert!(check_expression_needs("if_something").0);
    }

    /// A bare identifier needs the data model.
    ///
    /// This test asserted the defect in as many words — *"a bare
    /// identifier without operators/keywords should not need engine"* —
    /// and what that licensed was Rust `if myVariable {`, naming
    /// nothing, with `check` answering exit 0. A name is precisely what
    /// only the data model holds.
    #[test]
    fn expr_a_bare_identifier_needs_the_datamodel() {
        assert_eq!(check_expression_needs("myVariable"), (true, false));
    }

    // ── is_static_string_literal / extract_static_string_literal ─

    #[test]
    fn static_string_single_quotes() {
        assert!(is_static_string_literal("'hello'"));
        assert_eq!(extract_static_string_literal("'hello'"), "hello");
    }

    #[test]
    fn static_string_double_quotes() {
        assert!(is_static_string_literal("\"hello\""));
        assert_eq!(extract_static_string_literal("\"hello\""), "hello");
    }

    #[test]
    fn static_string_with_whitespace() {
        assert!(is_static_string_literal("  'hello'  "));
        assert_eq!(extract_static_string_literal("  'hello'  "), "hello");
    }

    #[test]
    fn static_string_empty_quoted() {
        assert!(is_static_string_literal("''"));
        assert_eq!(extract_static_string_literal("''"), "");
    }

    #[test]
    fn static_string_mismatched_quotes() {
        assert!(!is_static_string_literal("'hello\""));
        assert!(!is_static_string_literal("\"hello'"));
    }

    #[test]
    fn static_string_too_short() {
        assert!(!is_static_string_literal("x"));
        assert!(!is_static_string_literal(""));
    }

    #[test]
    fn static_string_no_quotes() {
        assert!(!is_static_string_literal("hello"));
    }

    // ── protect_context_strings / restore_context_strings ─────

    #[test]
    fn protect_restore_roundtrip() {
        let code = r#"hardware.powerOff("reason") && sensor.read('temp')"#;
        let (protected, literals) = protect_context_strings(code);
        assert!(!protected.contains("\"reason\""));
        assert!(!protected.contains("'temp'"));
        assert!(protected.contains("__STRING_PLACEHOLDER_0__"));
        assert!(protected.contains("__STRING_PLACEHOLDER_1__"));
        let restored = restore_context_strings(&protected, &literals);
        assert_eq!(restored, code);
    }

    #[test]
    fn protect_no_strings() {
        let code = "hardware.powerOff()";
        let (protected, literals) = protect_context_strings(code);
        assert_eq!(protected, code);
        assert!(literals.is_empty());
    }

    #[test]
    fn protect_escaped_quotes() {
        let code = r#"x.call("escaped \" quote")"#;
        let (protected, literals) = protect_context_strings(code);
        assert_eq!(literals.len(), 1);
        let restored = restore_context_strings(&protected, &literals);
        assert_eq!(restored, code);
    }

    // ── transform_cpp_code_with_named_contexts ───────────────

    #[test]
    fn cpp_transform_simple_context() {
        let mut ids = BTreeSet::new();
        ids.insert("hardware".to_string());
        let result = transform_cpp_code_with_named_contexts("hardware.powerOff()", &ids);
        assert_eq!(result, "this->hardware_->powerOff()");
    }

    #[test]
    fn cpp_transform_multiple_contexts() {
        let mut ids = BTreeSet::new();
        ids.insert("hw".to_string());
        ids.insert("sensor".to_string());
        let result = transform_cpp_code_with_named_contexts("hw.reset() && sensor.read()", &ids);
        assert_eq!(result, "this->hw_->reset() && this->sensor_->read()");
    }

    #[test]
    fn cpp_transform_preserves_string_literals() {
        let mut ids = BTreeSet::new();
        ids.insert("hw".to_string());
        let result = transform_cpp_code_with_named_contexts(r#"hw.log("hw.error")"#, &ids);
        // "hw.error" inside quotes must NOT be transformed
        assert_eq!(result, r#"this->hw_->log("hw.error")"#);
    }

    #[test]
    fn cpp_transform_empty_ids() {
        let ids = BTreeSet::new();
        let result = transform_cpp_code_with_named_contexts("hardware.powerOff()", &ids);
        assert_eq!(result, "hardware.powerOff()");
    }

    // ── transform_kt_code_with_named_contexts ────────────────

    #[test]
    fn kt_transform_snake_to_camel() {
        let mut ids = BTreeSet::new();
        ids.insert("my_hardware".to_string());
        let result = transform_kt_code_with_named_contexts("my_hardware.powerOff()", &ids);
        assert_eq!(result, "myHardware.powerOff()");
    }

    #[test]
    fn kt_transform_already_camel_no_change() {
        let mut ids = BTreeSet::new();
        ids.insert("hardware".to_string());
        // "hardware" → camelCase is still "hardware", no rename needed
        let result = transform_kt_code_with_named_contexts("hardware.powerOff()", &ids);
        assert_eq!(result, "hardware.powerOff()");
    }

    #[test]
    fn kt_transform_preserves_string_literals() {
        let mut ids = BTreeSet::new();
        ids.insert("my_obj".to_string());
        let result = transform_kt_code_with_named_contexts(r#"my_obj.call("my_obj")"#, &ids);
        // Inside quotes should not be transformed
        assert_eq!(result, r#"myObj.call("my_obj")"#);
    }

    // ── actions_to_javascript ────────────────────────────────

    #[test]
    fn actions_to_js_empty() {
        assert_eq!(actions_to_javascript(&[]), "");
    }

    #[test]
    fn actions_to_js_assign() {
        let action = Action {
            action_type: "assign".to_string(),
            location: "x".to_string(),
            expr: "5".to_string(),
            ..Default::default()
        };
        assert_eq!(actions_to_javascript(&[action]), "x = 5;");
    }

    #[test]
    fn actions_to_js_script() {
        let action = Action {
            action_type: "script".to_string(),
            content: "doSomething();".to_string(),
            ..Default::default()
        };
        assert_eq!(actions_to_javascript(&[action]), "doSomething();");
    }

    #[test]
    fn actions_to_js_log_with_label() {
        let action = Action {
            action_type: "log".to_string(),
            label: "debug".to_string(),
            expr: "x".to_string(),
            ..Default::default()
        };
        assert_eq!(
            actions_to_javascript(&[action]),
            "console.log(\"debug: \" + x);"
        );
    }

    #[test]
    fn actions_to_js_log_without_label() {
        let action = Action {
            action_type: "log".to_string(),
            expr: "x".to_string(),
            ..Default::default()
        };
        assert_eq!(actions_to_javascript(&[action]), "console.log(x);");
    }

    #[test]
    fn actions_to_js_if_then_else() {
        let action = Action {
            action_type: "if".to_string(),
            cond: "x > 0".to_string(),
            then_actions: vec![Action {
                action_type: "assign".to_string(),
                location: "y".to_string(),
                expr: "1".to_string(),
                ..Default::default()
            }],
            else_actions: vec![Action {
                action_type: "assign".to_string(),
                location: "y".to_string(),
                expr: "0".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let result = actions_to_javascript(&[action]);
        assert!(result.contains("if (x > 0) {"));
        assert!(result.contains("y = 1;"));
        assert!(result.contains("else {"));
        assert!(result.contains("y = 0;"));
    }

    #[test]
    fn actions_to_js_multiple_actions() {
        let actions = vec![
            Action {
                action_type: "assign".to_string(),
                location: "x".to_string(),
                expr: "1".to_string(),
                ..Default::default()
            },
            Action {
                action_type: "assign".to_string(),
                location: "y".to_string(),
                expr: "2".to_string(),
                ..Default::default()
            },
        ];
        assert_eq!(actions_to_javascript(&actions), "x = 1; y = 2;");
    }

    #[test]
    fn actions_to_js_skips_empty_assign() {
        let action = Action {
            action_type: "assign".to_string(),
            location: "".to_string(),
            expr: "5".to_string(),
            ..Default::default()
        };
        assert_eq!(actions_to_javascript(&[action]), "");
    }

    // ── actions_contain_event_metadata ────────────────────────

    #[test]
    fn event_metadata_empty_actions() {
        assert!(!actions_contain_event_metadata(&[]));
    }

    #[test]
    fn event_metadata_in_if_cond() {
        let action = Action {
            action_type: "if".to_string(),
            cond: "_event.data > 0".to_string(),
            ..Default::default()
        };
        assert!(actions_contain_event_metadata(&[action]));
    }

    #[test]
    fn event_metadata_in_nested_then() {
        let inner = Action {
            action_type: "if".to_string(),
            cond: "_event.type == 'error'".to_string(),
            ..Default::default()
        };
        let outer = Action {
            action_type: "if".to_string(),
            cond: "x > 0".to_string(),
            then_actions: vec![inner],
            ..Default::default()
        };
        assert!(actions_contain_event_metadata(&[outer]));
    }

    #[test]
    fn event_metadata_in_elseif_branch() {
        let action = Action {
            action_type: "if".to_string(),
            cond: "x > 0".to_string(),
            elseif_branches: vec![ElseIfBranch {
                cond: "_event.name".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        };
        assert!(actions_contain_event_metadata(&[action]));
    }

    #[test]
    fn event_metadata_in_foreach() {
        let inner = Action {
            action_type: "if".to_string(),
            cond: "_event.data".to_string(),
            ..Default::default()
        };
        let foreach = Action {
            action_type: "foreach".to_string(),
            actions: vec![inner],
            ..Default::default()
        };
        assert!(actions_contain_event_metadata(&[foreach]));
    }

    #[test]
    fn event_metadata_no_match() {
        let action = Action {
            action_type: "if".to_string(),
            cond: "x > 0".to_string(),
            then_actions: vec![Action {
                action_type: "assign".to_string(),
                location: "y".to_string(),
                expr: "1".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        };
        assert!(!actions_contain_event_metadata(&[action]));
    }

    // ── collect_parent_send_events ───────────────────────────

    #[test]
    fn collect_parent_events_basic() {
        let action = Action {
            action_type: "send".to_string(),
            target: "#_parent".to_string(),
            event: "done".to_string(),
            ..Default::default()
        };
        let mut events = BTreeSet::new();
        collect_parent_send_events(&[action], &mut events);
        assert_eq!(events.len(), 1);
        assert!(events.contains("done"));
    }

    #[test]
    fn collect_parent_events_ignores_non_parent() {
        let action = Action {
            action_type: "send".to_string(),
            target: "#other".to_string(),
            event: "done".to_string(),
            ..Default::default()
        };
        let mut events = BTreeSet::new();
        collect_parent_send_events(&[action], &mut events);
        assert!(events.is_empty());
    }

    #[test]
    fn collect_parent_events_ignores_empty_event() {
        let action = Action {
            action_type: "send".to_string(),
            target: "#_parent".to_string(),
            event: "".to_string(),
            ..Default::default()
        };
        let mut events = BTreeSet::new();
        collect_parent_send_events(&[action], &mut events);
        assert!(events.is_empty());
    }

    #[test]
    fn collect_parent_events_nested_in_if() {
        let send = Action {
            action_type: "send".to_string(),
            target: "#_parent".to_string(),
            event: "child.done".to_string(),
            ..Default::default()
        };
        let if_action = Action {
            action_type: "if".to_string(),
            cond: "true".to_string(),
            then_actions: vec![send],
            ..Default::default()
        };
        let mut events = BTreeSet::new();
        collect_parent_send_events(&[if_action], &mut events);
        assert!(events.contains("child.done"));
    }

    #[test]
    fn collect_parent_events_deduplicates() {
        let a1 = Action {
            action_type: "send".to_string(),
            target: "#_parent".to_string(),
            event: "same".to_string(),
            ..Default::default()
        };
        let a2 = a1.clone();
        let mut events = BTreeSet::new();
        collect_parent_send_events(&[a1, a2], &mut events);
        assert_eq!(events.len(), 1);
    }

    // ── parse_string (integration-level) ─────────────────────

    #[test]
    fn parse_minimal_scxml() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <state id="s1"/>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        assert_eq!(model.initial, "s1");
        assert!(model.states.contains_key("s1"));
    }

    #[test]
    fn parse_with_datamodel() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1" datamodel="ecmascript">
            <datamodel>
                <data id="x" expr="0"/>
                <data id="y" expr="'hello'"/>
            </datamodel>
            <state id="s1"/>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        assert_eq!(model.variables.len(), 2);
        assert_eq!(model.variables[0].id, "x");
        assert_eq!(model.variables[1].id, "y");
    }

    #[test]
    fn parse_transitions() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <state id="s1">
                <transition event="go" target="s2"/>
            </state>
            <final id="s2"/>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        let s1 = &model.states["s1"];
        assert_eq!(s1.transitions.len(), 1);
        assert_eq!(s1.transitions[0].event, "go");
        assert_eq!(s1.transitions[0].target, "s2");
    }

    #[test]
    fn parse_parallel_states() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="p">
            <parallel id="p">
                <state id="r1"><state id="r1a"/></state>
                <state id="r2"><state id="r2a"/></state>
            </parallel>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        assert!(model.states["p"].is_parallel);
        assert!(model.has_parallel_states);
    }

    #[test]
    fn parse_entry_exit_actions() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <state id="s1">
                <onentry><log expr="'entering'"/></onentry>
                <onexit><log expr="'exiting'"/></onexit>
            </state>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        let s1 = &model.states["s1"];
        assert!(!s1.on_entry_blocks.is_empty());
        assert!(!s1.on_exit_blocks.is_empty());
    }

    #[test]
    fn parse_history_state() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <state id="s1">
                <history id="h1" type="deep">
                    <transition target="s1a"/>
                </history>
                <state id="s1a"/>
                <state id="s1b"/>
            </state>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        assert!(model.has_history_states);
        assert!(model.history_states.contains_key("h1"));
        assert_eq!(model.history_states["h1"].history_type, "deep");
    }

    #[test]
    fn parse_send_action_attributes() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <state id="s1">
                <onentry>
                    <send event="timer" delay="500ms"/>
                </onentry>
            </state>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        let entry = &model.states["s1"].on_entry_blocks[0];
        assert_eq!(entry[0].event, "timer");
        assert_eq!(entry[0].delay_ms, 500);
    }

    #[test]
    fn parse_if_elseif_else() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1" datamodel="ecmascript">
            <state id="s1">
                <onentry>
                    <if cond="x > 0">
                        <log expr="'positive'"/>
                    <elseif cond="x == 0"/>
                        <log expr="'zero'"/>
                    <else/>
                        <log expr="'negative'"/>
                    </if>
                </onentry>
            </state>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        let entry = &model.states["s1"].on_entry_blocks[0];
        let if_action = &entry[0];
        assert_eq!(if_action.action_type, "if");
        assert_eq!(if_action.elseif_branches.len(), 1);
        assert!(!if_action.else_actions.is_empty());
    }

    // ── Error path tests ─────────────────────────────────────

    #[test]
    fn error_invalid_xml() {
        let mut parser = SCXMLParser::new();
        let result = parser.parse_string("<not valid xml", "test");
        assert!(result.is_err());
        // May fail at XSD validation or XML parse — either error path is valid
    }

    #[test]
    fn error_empty_input() {
        let mut parser = SCXMLParser::new();
        let result = parser.parse_string("", "test");
        assert!(result.is_err());
    }

    #[test]
    fn error_non_scxml_root() {
        let mut parser = SCXMLParser::new();
        let result = parser.parse_string("<html><body/></html>", "test");
        // XSD validation rejects non-SCXML root — should not panic
        assert!(result.is_err());
    }

    #[test]
    fn graceful_state_without_id() {
        // States without id are skipped (no panic)
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <state id="s1"/>
            <state/>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        // Only s1 should be in the model, the id-less state is skipped
        assert!(model.states.contains_key("s1"));
        assert_eq!(
            model.states.len(),
            1,
            "id-less state should be skipped, got: {:?}",
            model.states.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn graceful_transition_empty_target() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <state id="s1">
                <transition event="go"/>
            </state>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        let s1 = &model.states["s1"];
        assert_eq!(s1.transitions.len(), 1);
        assert_eq!(s1.transitions[0].target, "");
    }

    #[test]
    fn graceful_transition_nonexistent_target() {
        // Target references a state that doesn't exist
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <state id="s1">
                <transition event="go" target="nonexistent"/>
            </state>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        // Should not panic — just stores the target string
        let model = parser.parse_string(scxml, "test").unwrap();
        assert_eq!(model.states["s1"].transitions[0].target, "nonexistent");
    }

    #[test]
    fn graceful_empty_datamodel() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <datamodel/>
            <state id="s1"/>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        assert!(model.variables.is_empty());
    }

    #[test]
    fn graceful_data_without_id() {
        // <data> without id — should be skipped gracefully
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <datamodel>
                <data expr="5"/>
                <data id="x" expr="10"/>
            </datamodel>
            <state id="s1"/>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        // Only the data element with id="x" should be parsed
        assert!(
            model.variables.iter().any(|v| v.id == "x"),
            "variable x should exist"
        );
    }

    #[test]
    fn graceful_deeply_nested_states() {
        // 5 levels of state nesting — should parse correctly
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="a">
            <state id="a">
                <state id="b">
                    <state id="c">
                        <state id="d">
                            <state id="e"/>
                        </state>
                    </state>
                </state>
            </state>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        assert_eq!(model.states.len(), 5);
        assert_eq!(model.states["e"].parent, Some("d".to_string()));
        assert_eq!(model.states["d"].parent, Some("c".to_string()));
        assert!(model.has_hierarchy);
    }

    #[test]
    fn graceful_send_with_missing_event() {
        // <send> without event attribute — should not panic
        let scxml = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <state id="s1">
                <onentry>
                    <send target="#_parent"/>
                </onentry>
            </state>
        </scxml>"##;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        let entry = &model.states["s1"].on_entry_blocks[0];
        assert_eq!(entry[0].action_type, "send");
        assert_eq!(entry[0].event, "");
    }

    #[test]
    fn graceful_foreach_action() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1" datamodel="ecmascript">
            <state id="s1">
                <onentry>
                    <foreach array="items" item="x" index="i">
                        <log expr="x"/>
                    </foreach>
                </onentry>
            </state>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        let entry = &model.states["s1"].on_entry_blocks[0];
        assert_eq!(entry[0].action_type, "foreach");
        assert_eq!(entry[0].array, "items");
        assert_eq!(entry[0].item, "x");
        assert_eq!(entry[0].index, "i");
    }

    #[test]
    fn graceful_cancel_action() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <state id="s1">
                <onentry>
                    <cancel sendid="timer1"/>
                </onentry>
            </state>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        let entry = &model.states["s1"].on_entry_blocks[0];
        assert_eq!(entry[0].action_type, "cancel");
        assert_eq!(entry[0].sendid, "timer1");
    }

    /// §scxml-6.3 enforcement — `<cancel>` MUST carry sendid or
    /// sendidexpr. The both-empty shape is a parse-time hard error,
    /// emitted as `ValidationError::RequireEither` (wire code
    /// `validation/require-either` per W4 D4 fold). 5-backend cancel
    /// templates rely on this guarantee.
    #[test]
    fn cancel_without_sendid_or_sendidexpr_rejected() {
        use crate::forge::error::{ForgeError, ValidationError};
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <state id="s1">
                <onentry>
                    <cancel/>
                </onentry>
            </state>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let err = parser.parse_string(scxml, "test").unwrap_err();
        match err.error {
            ForgeError::Validation(boxed) => match *boxed {
                ValidationError::RequireEither {
                    element,
                    alternatives,
                } => {
                    assert_eq!(element, "<cancel>");
                    assert_eq!(
                        alternatives,
                        vec!["sendid".to_string(), "sendidexpr".to_string()]
                    );
                }
                other => {
                    panic!("expected ValidationError::RequireEither for <cancel/>, got: {other:?}")
                }
            },
            other => {
                panic!("expected ValidationError::RequireEither for <cancel/>, got: {other:?}")
            }
        }
    }

    #[test]
    fn graceful_raise_action() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <state id="s1">
                <onentry>
                    <raise event="internal.event"/>
                </onentry>
            </state>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        let entry = &model.states["s1"].on_entry_blocks[0];
        assert_eq!(entry[0].action_type, "raise");
        assert_eq!(entry[0].event, "internal.event");
    }

    /// `raised_events` is the authoritative internal-signal capture: every
    /// `<raise>` — including one nested in a `<finalize>` block, which is
    /// stringified to JS at parse time and thereafter invisible to an
    /// action-tree walk — is recorded so the trust-boundary surface can
    /// exclude it. Regression for the finalize-completeness gap: without
    /// the parse-time capture, `fin_ev` would leak into the externally-
    /// drivable set (an internal signal wrongly forgeable).
    #[test]
    fn raised_events_captures_finalize_and_onentry_raises() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <state id="s1">
                <onentry><raise event="entry_ev"/></onentry>
                <invoke type="http://www.w3.org/TR/scxml/" id="c" src="child.scxml">
                    <finalize><raise event="fin_ev"/></finalize>
                </invoke>
            </state>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        assert!(
            model.raised_events.contains("entry_ev"),
            "onentry raise must be captured; got {:?}",
            model.raised_events
        );
        assert!(
            model.raised_events.contains("fin_ev"),
            "finalize-nested raise must be captured (stringified to JS, so \
             only the parse-time capture sees it); got {:?}",
            model.raised_events
        );
    }

    #[test]
    fn default_initial_follows_document_order_not_element_name() {
        // §scxml-3.3: the default initial state is the first child state in
        // document order. `parse_states` visits one element name at a time, so
        // before `assign_document_order` ranked elements by position the
        // <state> child won regardless of where it was written — this document
        // resolved to `cs` instead of `cf`.
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="c">
            <state id="c">
                <final id="cf"/>
                <state id="cs"/>
            </state>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        assert_eq!(
            model.states["c"].initial, "cf",
            "<final> written first is the first child state in document order"
        );
        assert!(
            model.states["cf"].document_order < model.states["cs"].document_order,
            "document_order must follow document position: cf={} cs={}",
            model.states["cf"].document_order,
            model.states["cs"].document_order
        );
    }

    /// The rank is emitted into generated code as each state's
    /// document-order index, so a mixed sibling set must come out in
    /// written order across all three state element names.
    #[test]
    fn document_order_ranks_mixed_siblings_in_written_order() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="c">
            <state id="c">
                <final id="f1"/>
                <parallel id="p1">
                    <state id="pa"/>
                    <state id="pb"/>
                </parallel>
                <state id="s2"/>
            </state>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();

        let mut ranked: Vec<(&str, u32)> = ["c", "f1", "p1", "pa", "pb", "s2"]
            .iter()
            .map(|id| (*id, model.states[*id].document_order))
            .collect();
        ranked.sort_by_key(|(_, order)| *order);

        // Pre-order: the parallel's own children precede the sibling
        // written after it, which is what document order means for a tree.
        let ids: Vec<&str> = ranked.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, vec!["c", "f1", "p1", "pa", "pb", "s2"]);
    }

    #[test]
    fn graceful_multiple_transitions() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <state id="s1">
                <transition event="a" target="s2"/>
                <transition event="b" target="s3"/>
                <transition event="c" target="s1"/>
            </state>
            <state id="s2"/>
            <state id="s3"/>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        assert_eq!(model.states["s1"].transitions.len(), 3);
        assert_eq!(model.states["s1"].transitions[0].event, "a");
        assert_eq!(model.states["s1"].transitions[1].event, "b");
        assert_eq!(model.states["s1"].transitions[2].event, "c");
    }

    #[test]
    fn graceful_final_state_with_donedata() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <state id="s1">
                <transition event="done" target="final"/>
            </state>
            <final id="final">
                <donedata>
                    <param name="result" expr="42"/>
                </donedata>
            </final>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        assert!(model.states["final"].is_final);
    }

    #[test]
    fn error_duplicate_context_id() {
        use crate::forge::error::{ForgeError, ValidationError};
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                        xmlns:sce="http://sce.dev/ext"
                        version="1.0" initial="s1">
            <sce:context id="hw"/>
            <sce:context id="hw"/>
            <state id="s1"/>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let err = parser
            .parse_string(scxml, "test")
            .expect_err("duplicate <sce:context id=\"hw\"> must fail validation");
        assert!(
            matches!(
                err.error,
                ForgeError::Validation(ref boxed)
                    if matches!(**boxed, ValidationError::DuplicateContextObject { ref id } if id == "hw"),
            ),
            "expected ValidationError::DuplicateContextObject(id=\"hw\"), got: {:?}",
            err.error,
        );
    }

    #[test]
    fn error_reserved_context_id_lowercase() {
        use crate::forge::error::{ForgeError, ValidationError};
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                        xmlns:sce="http://sce.dev/ext"
                        version="1.0" initial="s1">
            <sce:context id="policy"/>
            <state id="s1"/>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let err = parser
            .parse_string(scxml, "test")
            .expect_err("reserved <sce:context id=\"policy\"> must fail validation");
        assert!(
            matches!(
                err.error,
                ForgeError::Validation(ref boxed)
                    if matches!(**boxed, ValidationError::ReservedContextId { ref id, .. } if id == "policy"),
            ),
            "expected ValidationError::ReservedContextId(id=\"policy\"), got: {:?}",
            err.error,
        );
    }

    #[test]
    fn error_reserved_context_id_case_insensitive() {
        // Jinja2 `capitalize` lowercases every char after the first,
        // so `Policy` and `POLICY` both generate `PolicyType` — the
        // guard must catch every case variant.
        use crate::forge::error::{ForgeError, ValidationError};
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                        xmlns:sce="http://sce.dev/ext"
                        version="1.0" initial="s1">
            <sce:context id="POLICY"/>
            <state id="s1"/>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let err = parser
            .parse_string(scxml, "test")
            .expect_err("case-variant reserved id must fail validation");
        assert!(
            matches!(
                err.error,
                ForgeError::Validation(ref boxed)
                    if matches!(**boxed, ValidationError::ReservedContextId { ref id, .. } if id == "POLICY"),
            ),
            "expected ValidationError::ReservedContextId(id=\"POLICY\"), got: {:?}",
            err.error,
        );
    }

    #[test]
    fn error_inline_kind_missing_id_is_fatal() {
        // Malformed inline kind (`sce:kind="lookup"` without `id`) must
        // fail the whole parse. Silently demoting it to a JS variable
        // would mask the author's declared intent and hand the
        // generator a data type it was never told about
        // (feedback_silently_broken_hooks).
        use crate::forge::error::{ForgeError, ValidationError};
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                        xmlns:sce="http://sce.dev/ext"
                        version="1.0" initial="s1">
            <datamodel>
                <data sce:kind="lookup">
                    <sce:entry key="1" value="one"/>
                </data>
            </datamodel>
            <state id="s1"/>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let err = parser
            .parse_string(scxml, "test")
            .expect_err("malformed inline lookup must fail hard");
        assert!(
            matches!(
                err.error,
                ForgeError::Validation(ref boxed)
                    if matches!(**boxed, ValidationError::MissingAttribute { ref attr, .. } if attr == "id"),
            ),
            "expected ValidationError::MissingAttribute(attr=\"id\"), got: {:?}",
            err.error,
        );
        assert_eq!(
            err.location.file, "test",
            "located error must carry source name"
        );
        assert!(
            err.location.line.is_some() && err.location.col.is_some(),
            "inline-kind leaf errors must carry roxmltree row/col, got: {:?}",
            err.location,
        );
    }

    #[test]
    fn error_missing_context_on_cpp_condition() {
        // cpp: condition references `hw.ready` but no <sce:context id="hw">
        // is declared — parse must fail with MissingContext, not the
        // semantically wrong IncompatibleAttributes.
        use crate::forge::error::{ForgeError, ValidationError};
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                        xmlns:sce="http://sce.dev/ext"
                        version="1.0" initial="s1">
            <state id="s1">
                <transition cond="cpp:hw.ready()" target="s2"/>
            </state>
            <state id="s2"/>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let err = parser
            .parse_string(scxml, "test")
            .expect_err("cpp: condition without <sce:context> must fail validation");
        assert!(
            matches!(
                err.error,
                ForgeError::Validation(ref boxed)
                    if matches!(**boxed, ValidationError::MissingContext { ref site, .. } if site == "cpp: condition"),
            ),
            "expected ValidationError::MissingContext(site=\"cpp: condition\"), got: {:?}",
            err.error,
        );
    }

    #[test]
    fn graceful_default_initial_from_first_state() {
        // No initial attribute — default to first child state
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0">
            <state id="first"/>
            <state id="second"/>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        assert_eq!(model.initial, "first");
    }

    #[test]
    fn graceful_whitespace_in_initial() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="  s1  ">
            <state id="s1"/>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        // Parser may trim or not — should not panic
        let _model = parser.parse_string(scxml, "test").unwrap();
    }

    // ── InvokeBase.field_suffix derivation ───────────────────────────

    /// Helper: parse an SCXML fragment with a single `<invoke>` and return its
    /// (invoke_id, field_suffix) pair.
    fn first_invoke_ids(scxml: &str) -> (String, String) {
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        let invoke = model
            .states
            .values()
            .find_map(|s| s.invokes.first())
            .expect("expected at least one invoke");
        let base = match invoke {
            Invoke::Scxml(info) => &info.common.base,
            Invoke::Hybrid(info) => &info.common.base,
            Invoke::MeshRpc(info) => &info.base,
            Invoke::Unsupported(info) => &info.base,
        };
        (base.invoke_id.clone(), base.field_suffix.clone())
    }

    #[test]
    fn invoke_field_suffix_strips_auto_id_leading_underscore() {
        // No `id` attribute → parser auto-generates `_invoke_0` per §scxml-6.4.1.
        // field_suffix drops the leading underscore so `child_` + suffix is
        // `child_invoke_0`, not `child__invoke_0`.
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s">
                <invoke type="scxml" src="child.scxml"/>
            </state>
        </scxml>"#;
        let (invoke_id, field_suffix) = first_invoke_ids(scxml);
        assert_eq!(invoke_id, "_invoke_0");
        assert_eq!(field_suffix, "invoke_0");
    }

    #[test]
    fn invoke_field_suffix_preserves_user_supplied_id() {
        // User-supplied id has no leading underscore — round-trips unchanged.
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s">
                <invoke id="invokedChild" type="scxml" src="child.scxml"/>
            </state>
        </scxml>"#;
        let (invoke_id, field_suffix) = first_invoke_ids(scxml);
        assert_eq!(invoke_id, "invokedChild");
        assert_eq!(field_suffix, "invokedChild");
    }

    // ── §scxml-3.14 invoke-id uniqueness ─────────────────────────

    #[test]
    fn invoke_id_duplicate_author_rejected() {
        // Two parallel regions with the same author-supplied <invoke id>.
        // §scxml-3.14 forbids duplicate ids; the parser must surface this as
        // ValidationError::DuplicateId rather than let the collision reach
        // AOT event matching or the mesh active_invokes_ map.
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="p">
            <parallel id="p">
                <state id="a">
                    <invoke id="motor_call" type="scxml" src="child.scxml"/>
                </state>
                <state id="b">
                    <invoke id="motor_call" type="scxml" src="child.scxml"/>
                </state>
            </parallel>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let err = parser
            .parse_string(scxml, "test")
            .expect_err("duplicate <invoke id> must reject");
        use crate::forge::error::{ForgeError, ValidationError};
        match err.error {
            ForgeError::Validation(boxed) => match *boxed {
                ValidationError::DuplicateId { what, id, .. } => {
                    assert_eq!(what, "<invoke id>");
                    assert_eq!(id, "motor_call");
                }
                other => panic!("expected DuplicateId for <invoke id>, got: {other:?}"),
            },
            other => panic!("expected DuplicateId for <invoke id>, got: {other:?}"),
        }
    }

    #[test]
    fn invoke_id_auto_counter_parallel_regions_unique() {
        // Two parallel regions, both with idless <invoke>. Auto-counter
        // yields `_invoke_0` and `_invoke_1` — no collision, parses clean.
        // Guards against over-rejection once duplicate detection landed.
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="p">
            <parallel id="p">
                <state id="a">
                    <invoke type="scxml" src="child.scxml"/>
                </state>
                <state id="b">
                    <invoke type="scxml" src="child.scxml"/>
                </state>
            </parallel>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser
            .parse_string(scxml, "test")
            .expect("auto-id parallel invokes must parse clean");
        // Two distinct invoke ids collected across the parallel's children.
        let ids: Vec<String> = model
            .states
            .values()
            .flat_map(|s| s.invokes.iter())
            .map(|i| match i {
                Invoke::Scxml(info) => info.common.base.invoke_id.clone(),
                Invoke::Hybrid(info) => info.common.base.invoke_id.clone(),
                Invoke::MeshRpc(info) => info.base.invoke_id.clone(),
                Invoke::Unsupported(info) => info.base.invoke_id.clone(),
            })
            .collect();
        assert_eq!(ids.len(), 2);
        assert_ne!(ids[0], ids[1], "auto-counter must yield distinct ids");
    }

    #[test]
    fn invoke_id_author_shadows_auto_counter_rejected() {
        // Author picks `_invoke_0` explicitly; a subsequent idless invoke
        // would take counter=0 and land on the same id. Must reject so the
        // shadow case cannot slip past the uniqueness gate.
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s">
                <invoke id="_invoke_0" type="scxml" src="a.scxml"/>
                <invoke type="scxml" src="b.scxml"/>
            </state>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let err = parser
            .parse_string(scxml, "test")
            .expect_err("author-shadows-auto-counter must reject");
        use crate::forge::error::{ForgeError, ValidationError};
        match err.error {
            ForgeError::Validation(boxed) => match *boxed {
                ValidationError::DuplicateId { what, id, .. } => {
                    assert_eq!(what, "<invoke id>");
                    assert_eq!(id, "_invoke_0");
                }
                other => panic!("expected DuplicateId for <invoke id>, got: {other:?}"),
            },
            other => panic!("expected DuplicateId for <invoke id>, got: {other:?}"),
        }
    }

    // ── <invoke type="sce:mesh-rpc"> reserved-param rules (§9.5) ────

    /// Helper: parse a fragment and pull the first [`Invoke::MeshRpc`].
    fn first_mesh_rpc_invoke(scxml: &str) -> MeshRpcInvokeInfo {
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        let invoke = model
            .states
            .values()
            .find_map(|s| s.invokes.first())
            .expect("expected at least one invoke");
        match invoke {
            Invoke::MeshRpc(info) => info.clone(),
            other => panic!("expected MeshRpc invoke, got {other:?}"),
        }
    }

    /// Helper: parse a fragment, expect `MeshRpcReservedParam`, return the
    /// `(param, detail)` pair. Any other outcome is a test failure.
    fn first_mesh_rpc_violation(scxml: &str) -> (String, String) {
        use crate::forge::error::{ForgeError, ValidationError};
        let mut parser = SCXMLParser::new();
        let err = parser.parse_string(scxml, "test").unwrap_err();
        match err.error {
            ForgeError::Validation(boxed) => match *boxed {
                ValidationError::MeshRpcReservedParam { param, detail } => (param, detail),
                other => panic!("expected MeshRpcReservedParam, got {other:?}"),
            },
            other => panic!("expected MeshRpcReservedParam, got {other:?}"),
        }
    }

    #[test]
    fn mesh_rpc_invoke_happy_path_populates_envelope_fields() {
        // Extra `#` on the raw-string delimiter because `src="#motor"`
        // literally contains a `"#` byte sequence that would otherwise
        // terminate the standard `r#"..."#` form early.
        let scxml = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s">
                <invoke type="sce:mesh-rpc" src="#motor">
                    <param name="_mesh_event" expr="'service.request.compute_force'"/>
                    <param name="_mesh_deadline_ms" expr="'250'"/>
                    <param name="torque" expr="'42'"/>
                </invoke>
            </state>
        </scxml>"##;
        let info = first_mesh_rpc_invoke(scxml);
        assert_eq!(info.target.src_literal(), Some("#motor"));
        assert_eq!(info.mesh_event, "service.request.compute_force");
        assert_eq!(info.deadline_ms, Some(250));
        // Reserved names are stripped from the payload; author params
        // pass through unchanged.
        assert_eq!(info.base.params.len(), 1);
        assert_eq!(info.base.params[0].name, "torque");
    }

    #[test]
    fn mesh_rpc_srcexpr_promotes_needs_script_engine() {
        // SCE Mesh §9.5: the srcexpr entry block calls
        // `evaluateExpression` unconditionally, so `parse_invoke` must
        // flip `needs_script_engine` on every document that carries a
        // `MeshRpcTarget::SrcExpr`. Without the flag the generated code
        // references a missing `ensureScriptEngine()` helper.
        let scxml = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s">
                <invoke type="sce:mesh-rpc" srcexpr="'#motor'">
                    <param name="_mesh_event" expr="'service.request.ping'"/>
                </invoke>
            </state>
        </scxml>"##;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        assert!(
            model.needs_script_engine,
            "srcexpr invoke must promote needs_script_engine",
        );
    }

    #[test]
    fn mesh_rpc_static_src_does_not_promote_needs_script_engine() {
        // Static `src="#motor"` carries no expression evaluation — the
        // target is a build-time literal resolved through the topology.
        // This document has no other script-engine trigger (no datamodel
        // block, no content, no donedata), so the flag must stay off.
        // Pairs with the previous test to pin both sides of the branch.
        let scxml = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s">
                <invoke type="sce:mesh-rpc" src="#motor">
                    <param name="_mesh_event" expr="'service.request.ping'"/>
                </invoke>
            </state>
        </scxml>"##;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        assert!(
            !model.needs_script_engine,
            "static src invoke without other triggers must not promote needs_script_engine",
        );
    }

    #[test]
    fn mesh_rpc_invoke_rejects_missing_mesh_event() {
        let scxml = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s">
                <invoke type="sce:mesh-rpc" src="#motor">
                    <param name="torque" expr="'42'"/>
                </invoke>
            </state>
        </scxml>"##;
        let (param, detail) = first_mesh_rpc_violation(scxml);
        assert_eq!(param, "_mesh_event");
        assert!(
            detail.contains("required") && detail.contains("_mesh_event"),
            "expected 'required _mesh_event' phrasing, got: {detail}",
        );
    }

    #[test]
    fn mesh_rpc_invoke_rejects_duplicate_mesh_event() {
        let scxml = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s">
                <invoke type="sce:mesh-rpc" src="#motor">
                    <param name="_mesh_event" expr="'x'"/>
                    <param name="_mesh_event" expr="'y'"/>
                </invoke>
            </state>
        </scxml>"##;
        let (param, detail) = first_mesh_rpc_violation(scxml);
        assert_eq!(param, "_mesh_event");
        assert!(
            detail.contains("exactly once"),
            "expected 'exactly once' phrasing, got: {detail}",
        );
    }

    #[test]
    fn mesh_rpc_invoke_rejects_unknown_reserved_prefix() {
        let scxml = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s">
                <invoke type="sce:mesh-rpc" src="#motor">
                    <param name="_mesh_event" expr="'x'"/>
                    <param name="_mesh_oracle" expr="'42'"/>
                </invoke>
            </state>
        </scxml>"##;
        let (param, detail) = first_mesh_rpc_violation(scxml);
        assert_eq!(param, "_mesh_oracle");
        assert!(
            detail.contains("reserved"),
            "expected 'reserved' phrasing, got: {detail}",
        );
    }

    #[test]
    fn mesh_rpc_invoke_rejects_non_integer_deadline() {
        let scxml = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s">
                <invoke type="sce:mesh-rpc" src="#motor">
                    <param name="_mesh_event" expr="'x'"/>
                    <param name="_mesh_deadline_ms" expr="'abc'"/>
                </invoke>
            </state>
        </scxml>"##;
        let (param, detail) = first_mesh_rpc_violation(scxml);
        assert_eq!(param, "_mesh_deadline_ms");
        assert!(
            detail.contains("non-negative integer"),
            "expected 'non-negative integer' phrasing, got: {detail}",
        );
    }

    #[test]
    fn mesh_rpc_invoke_accepts_bare_deadline_literal() {
        // Bare numeric literal (`expr="50"` not `expr="'50'"`) is the
        // idiomatic shape for integer-typed params and should parse
        // through the same path as the quoted form.
        let scxml = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s">
                <invoke type="sce:mesh-rpc" src="#motor">
                    <param name="_mesh_event" expr="'x'"/>
                    <param name="_mesh_deadline_ms" expr="50"/>
                </invoke>
            </state>
        </scxml>"##;
        let info = first_mesh_rpc_invoke(scxml);
        assert_eq!(info.deadline_ms, Some(50));
    }

    // ── §9.5 srcexpr exactly-one rule ───────────────────────────

    #[test]
    fn mesh_rpc_invoke_accepts_srcexpr_alone() {
        let scxml = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s">
                <invoke type="sce:mesh-rpc" srcexpr="'#motor_' + id">
                    <param name="_mesh_event" expr="'x'"/>
                </invoke>
            </state>
        </scxml>"##;
        let info = first_mesh_rpc_invoke(scxml);
        assert!(
            info.target.src_literal().is_none(),
            "SrcExpr variant has no build-time literal"
        );
        match &info.target {
            MeshRpcTarget::SrcExpr { srcexpr } => {
                assert_eq!(srcexpr, "'#motor_' + id");
            }
            other => panic!("expected SrcExpr variant, got {other:?}"),
        }
    }

    #[test]
    fn mesh_rpc_invoke_rejects_missing_target() {
        let scxml = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s">
                <invoke type="sce:mesh-rpc">
                    <param name="_mesh_event" expr="'x'"/>
                </invoke>
            </state>
        </scxml>"##;
        use crate::forge::error::{ForgeError, ValidationError};
        let mut parser = SCXMLParser::new();
        let err = parser.parse_string(scxml, "test").unwrap_err();
        match err.error {
            ForgeError::Validation(boxed) => match *boxed {
                ValidationError::MeshRpcMissingTarget => {}
                other => panic!("expected MeshRpcMissingTarget, got {other:?}"),
            },
            other => panic!("expected MeshRpcMissingTarget, got {other:?}"),
        }
    }

    #[test]
    fn mesh_rpc_invoke_rejects_both_src_and_srcexpr() {
        let scxml = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s">
                <invoke type="sce:mesh-rpc" src="#motor" srcexpr="'#motor'">
                    <param name="_mesh_event" expr="'x'"/>
                </invoke>
            </state>
        </scxml>"##;
        use crate::forge::error::{ForgeError, ValidationError};
        let mut parser = SCXMLParser::new();
        let err = parser.parse_string(scxml, "test").unwrap_err();
        match err.error {
            ForgeError::Validation(boxed) => match *boxed {
                ValidationError::MeshRpcDuplicateTarget => {}
                other => panic!("expected MeshRpcDuplicateTarget, got {other:?}"),
            },
            other => panic!("expected MeshRpcDuplicateTarget, got {other:?}"),
        }
    }

    // ── Session C/D attribute deprecation → Stage 2 hard error ─────

    /// Helper: parse a document and return the first RemovedAttribute
    /// diagnostic payload. Panics with an explanatory message if the
    /// parser succeeded (meaning Stage 2 enforcement is broken).
    fn first_removed_attribute(scxml: &str) -> (String, Option<String>) {
        use crate::forge::error::{ForgeError, ValidationError};
        let mut p = SCXMLParser::new();
        let res = p.parse_string(scxml, "test");
        let err = match res {
            Ok(_) => panic!(
                "parser should reject deprecated sce:* attributes \
                 as hard errors (got Ok)"
            ),
            Err(e) => e,
        };
        match err.error {
            ForgeError::Validation(boxed) => match *boxed {
                ValidationError::RemovedAttribute { attribute, event } => (attribute, event),
                other => panic!("expected RemovedAttribute, got: {other:?}"),
            },
            other => panic!("expected RemovedAttribute, got: {other:?}"),
        }
    }

    #[test]
    fn send_with_sce_qos_is_hard_error() {
        let scxml = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                              xmlns:sce="http://sce.dev/ext"
                              version="1.0" initial="s">
            <state id="s">
                <onentry>
                    <send event="brake.activate" target="#motor" sce:qos="reliable"/>
                </onentry>
            </state>
        </scxml>"##;
        let (attr, event) = first_removed_attribute(scxml);
        assert_eq!(attr, "sce:qos");
        assert_eq!(event.as_deref(), Some("brake.activate"));
    }

    #[test]
    fn send_with_sce_pattern_is_hard_error() {
        let scxml = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                              xmlns:sce="http://sce.dev/ext"
                              version="1.0" initial="s">
            <state id="s">
                <onentry>
                    <send event="svc.call" target="#motor" sce:pattern="request"/>
                </onentry>
            </state>
        </scxml>"##;
        let (attr, _) = first_removed_attribute(scxml);
        assert_eq!(attr, "sce:pattern");
    }

    #[test]
    fn send_with_sce_reply_timeout_is_hard_error() {
        // reply-timeout was missing from the Stage 1 watch list — Stage 2
        // picks it up alongside qos/pattern/reply-event/deadline/priority.
        let scxml = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                              xmlns:sce="http://sce.dev/ext"
                              version="1.0" initial="s">
            <state id="s">
                <onentry>
                    <send event="svc.call" target="#motor" sce:reply-timeout="500"/>
                </onentry>
            </state>
        </scxml>"##;
        let (attr, _) = first_removed_attribute(scxml);
        assert_eq!(attr, "sce:reply-timeout");
    }

    // ── remap_post_expansion ────────────────────────────────

    #[test]
    fn remap_post_expansion_rewrites_outer_line_col_into_included_file() {
        // Hand-crafted PositionMap mimicking xinclude splice:
        //   expanded[0..4)  = "AAA\n"   from outer.xml[0..4)
        //   expanded[4..11) = "BBBBBB\n" from frag.xml[0..7)
        //   expanded[11..14)= "CCC"     from outer.xml[4..7)
        use crate::forge::error::{ForgeError, Located, XmlError};
        use crate::position_map::{Origin, PositionMap};
        use std::path::PathBuf;

        let expanded = "AAA\nBBBBBB\nCCC";
        let mut map = PositionMap::default();
        map.register_file(PathBuf::from("outer.xml"), "AAA\nCCC");
        map.register_file(PathBuf::from("frag.xml"), "BBBBBB\n");
        map.push_entry(
            0,
            4,
            Origin::File {
                path: PathBuf::from("outer.xml"),
                source_offset: 0,
            },
        );
        map.push_entry(
            4,
            11,
            Origin::File {
                path: PathBuf::from("frag.xml"),
                source_offset: 0,
            },
        );
        map.push_entry(
            11,
            14,
            Origin::File {
                path: PathBuf::from("outer.xml"),
                source_offset: 4,
            },
        );

        // Expanded (row 2, col 3) — byte offset 4 + 2 = 6 on the
        // BBBBBB line, which maps to frag.xml (row 1, col 3).
        let err = Located::new(
            ForgeError::Xml(XmlError::Parse("synthetic".to_string())),
            "expanded.scxml",
            Some(2),
            Some(3),
        );
        let remapped = remap_post_expansion(err, expanded, &map);
        assert_eq!(remapped.location.file, "frag.xml");
        assert_eq!(remapped.location.line, Some(1));
        assert_eq!(remapped.location.col, Some(3));
    }

    #[test]
    fn remap_post_expansion_identity_map_is_noop() {
        // Identity map (no xinclude): remap must leave the error
        // byte-identical to prove documents without preprocessor
        // expansion pay no behavioural cost.
        use crate::forge::error::{ForgeError, Located, XmlError};
        use crate::position_map::PositionMap;

        let text = "<root/>";
        let map = PositionMap::identity("main.scxml", text);
        let err = Located::new(
            ForgeError::Xml(XmlError::Parse("synthetic".to_string())),
            "some-diag-label",
            Some(42),
            Some(17),
        );
        let remapped = remap_post_expansion(err, text, &map);
        assert_eq!(remapped.location.file, "some-diag-label");
        assert_eq!(remapped.location.line, Some(42));
        assert_eq!(remapped.location.col, Some(17));
    }

    #[test]
    fn parse_file_remaps_post_expansion_error_into_included_fragment() {
        // End-to-end load-bearing test for the remap wiring: if
        // `parse_file`'s `.map_err(remap_post_expansion ...)` line
        // ever gets deleted, this test fails. Without it, the
        // unit tests above still pass — proving remap_post_expansion
        // works in isolation — but the pipeline would silently
        // emit expanded coordinates, which is exactly the bug this
        // commit closes.
        //
        // Shape: main.scxml has an outer <state> with a single
        // <xi:include> pointing at frag.xml. frag.xml declares two
        // <invoke id="dup"/> — §scxml-3.14 duplicate-id catches
        // this during parse_impl's invoke parse, producing a
        // Located<ValidationError::DuplicateId> whose raw row/col
        // lie in the expanded document. After remap, the location
        // must resolve to frag.xml at the second invoke's real
        // (row, col) inside frag.xml — not at an expanded offset
        // the operator cannot navigate.
        use crate::forge::error::{ForgeError, ValidationError};
        use std::fs;
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");
        let frag_path = tmp.path().join("frag.xml");
        let main_path = tmp.path().join("main.scxml");

        let frag = r#"<fragment>
<invoke id="dup" type="http://www.w3.org/TR/scxml/"/>
<invoke id="dup" type="http://www.w3.org/TR/scxml/"/>
</fragment>"#;
        fs::write(&frag_path, frag).unwrap();

        // Outer file also has boilerplate lines before the include
        // so any confusion between outer row and fragment row
        // produces a visibly wrong answer.
        let main = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
        xmlns:xi="http://www.w3.org/2001/XInclude"
        version="1.0" initial="s">
    <state id="s">
        <xi:include href="frag.xml"/>
    </state>
</scxml>"#;
        fs::write(&main_path, main).unwrap();

        let mut parser = SCXMLParser::new();
        let err = parser
            .parse_file(main_path.to_str().unwrap())
            .expect_err("duplicate <invoke id> must fail");

        // Error kind: DuplicateId for <invoke id>.
        match &err.error {
            ForgeError::Validation(boxed) => match boxed.as_ref() {
                ValidationError::DuplicateId { what, id, .. } => {
                    assert_eq!(what, "<invoke id>");
                    assert_eq!(id, "dup");
                }
                other => panic!("expected DuplicateId, got: {other:?}"),
            },
            other => panic!("expected DuplicateId, got: {other:?}"),
        }

        // Location must be remapped to frag.xml, not to main.scxml
        // or an expanded label.
        assert!(
            err.location.file.ends_with("frag.xml"),
            "expected location.file to end with 'frag.xml' (got {:?})",
            err.location.file,
        );

        // The second <invoke id="dup"/> lives on line 3 of frag.xml
        // (line 1 is <fragment>, line 2 is the first invoke).
        // Without remap, this would report the expanded document's
        // line, which depends on the splice offset and is ≠ 3.
        assert_eq!(err.location.line, Some(3), "expected frag.xml line 3");
    }

    #[test]
    fn parse_file_remaps_post_expansion_error_into_template_body() {
        // End-to-end load-bearing test for the template half of
        // preprocessor coordinate mapping: if `parse_file` ever
        // stops threading `template::expand`'s `PositionMap` into
        // `remap_post_expansion`, this test fails while the unit
        // tests still pass (proving the pipeline wiring — not just
        // the map-building — is exercised).
        //
        // Shape: main.scxml hosts a single `<sce:use>` pointing at
        // t.xml. t.xml declares two `<invoke id="{$id}">` in a
        // row — parameterised over `{$id}` so the same value is
        // spliced twice, producing duplicate invoke ids after
        // expansion. parse_impl's §scxml-3.14 duplicate-id
        // check fires at the second `<invoke>` element's range —
        // bytes that came 1:1 from t.xml, so Origin::File routes
        // the diagnostic back to t.xml's row, not to the expanded
        // document's row. If `parse_file` were to drop template's
        // map and keep feeding `xinclude_map` into
        // `remap_post_expansion`, the lookup would resolve the
        // expanded offset against the post-xinclude document
        // (identity over main.scxml here) and point at some
        // unrelated main.scxml row — the assertion below would
        // then fail.
        //
        // This is the template-half analogue of
        // `parse_file_remaps_post_expansion_error_into_included_fragment`;
        // together they cover the two preprocessor stages sharing
        // the single `remap_post_expansion` boundary at
        // `parse_file`'s `.map_err(...)` line.
        use crate::forge::error::{ForgeError, ValidationError};
        use std::fs;
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");
        let t_path = tmp.path().join("t.xml");
        let main_path = tmp.path().join("main.scxml");

        // Template file — two `<invoke>` declarations share the
        // same `{$id}` param so one call-site binding produces a
        // duplicate. The second `<invoke>` line is the position
        // the remap must resolve to.
        //
        // Lines:
        //   1: <sce:template ...>
        //   2:   <sce:param name="id" required="true"/>
        //   3:   <invoke id="{$id}" type="..."/>
        //   4:   <invoke id="{$id}" type="..."/>
        //   5: </sce:template>
        let template_raw = r#"<sce:template xmlns:sce="http://sce.dev/ext" name="t">
  <sce:param name="id" required="true"/>
  <invoke id="{$id}" type="http://www.w3.org/TR/scxml/"/>
  <invoke id="{$id}" type="http://www.w3.org/TR/scxml/"/>
</sce:template>"#;
        fs::write(&t_path, template_raw).unwrap();

        // Main file — `<sce:use>` lives inside a `<state>` so
        // both invokes parent the same state and the duplicate-id
        // check runs against the state's invoke set.
        let main = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s">
    <state id="s">
        <sce:use template="t.xml" id="dup"/>
    </state>
</scxml>"#;
        fs::write(&main_path, main).unwrap();

        let mut parser = SCXMLParser::new();
        let err = parser
            .parse_file(main_path.to_str().unwrap())
            .expect_err("duplicate <invoke id> must fail");

        match &err.error {
            ForgeError::Validation(boxed) => match boxed.as_ref() {
                ValidationError::DuplicateId { what, id, .. } => {
                    assert_eq!(what, "<invoke id>");
                    assert_eq!(id, "dup");
                }
                other => panic!("expected DuplicateId, got: {other:?}"),
            },
            other => panic!("expected DuplicateId, got: {other:?}"),
        }

        // Location must be remapped to the template file — the
        // duplicate invoke's element bytes came from t.xml, not
        // from main.scxml.
        assert!(
            err.location.file.ends_with("t.xml"),
            "expected location.file to end with 't.xml' (got {:?})",
            err.location.file,
        );
        // The second `<invoke id="{$id}">` lives on line 4 of the
        // template file. Without the template-map thread, the
        // reported line would be whatever row this expanded byte
        // lands at in the in-memory post-template document — which
        // is ≠ 4 because splicing always shifts line numbers.
        assert_eq!(
            err.location.line,
            Some(4),
            "expected t.xml line 4 for the second <invoke>",
        );
    }

    #[test]
    fn parse_file_remaps_post_expansion_error_into_callsite() {
        // End-to-end load-bearing test for the `Origin::CallSite`
        // branch of preprocessor coordinate mapping. The sibling
        // `parse_file_remaps_post_expansion_error_into_template_body`
        // exercises template-body bytes (`Origin::File`); this test
        // exercises the bytes synthesised by `{$param}` substitution.
        // Together they pin both arms of the composition chain
        //   parse_file → template::expand
        //   → substitute_into_template_with_map
        //   → apply_substitution_with_tracking (CallSite emission)
        //   → append_mapped_substring
        //   → outer map → lookup → SourcePos
        // staying wired end-to-end. Without this test, every layer
        // has unit coverage but no consumer depends on the chain
        // keeping CallSite entries intact — exactly the
        // `feedback_built_but_unconsumed.md` hazard.
        //
        // Shape: a unique substring is bound to a required `<sce:use
        // id="...">` param; the template body splices it via
        // `{$id}`. After expansion we locate the marker in the
        // returned text and assert every byte in its range resolves
        // through `PositionMap::lookup` to the *caller's*
        // `<sce:use>` element row/col, not to the template file.
        // Drives `template::expand` directly rather than `parse_file`
        // — most validators report at element-open offsets (which
        // are template-body bytes), so coercing one to fire inside a
        // substituted attribute value is fragile across libxml2 /
        // roxmltree versions. The narrower entry point still walks
        // the same composition primitives.
        //
        // Load-bearing verification (manual, in-session): swap the
        // `Origin::CallSite { ... }` emission inside
        // `template::apply_substitution_with_tracking` for
        // `Origin::File { path: template_path.to_path_buf(), ... }`
        // and re-run; the marker bytes then resolve to `t.xml` and
        // the assertions below fail. Restore to confirm green.
        use crate::position_map::PositionMap;
        use crate::template;
        use std::fs;
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");
        let t_path = tmp.path().join("t.xml");
        let main_path = tmp.path().join("main.scxml");

        let template_raw = r#"<sce:template xmlns:sce="http://sce.dev/ext" name="t">
  <sce:param name="id" required="true"/>
  <marker value="{$id}"/>
</sce:template>"#;
        fs::write(&t_path, template_raw).unwrap();

        // Distinctive marker that a) the user's `find` can locate
        // unambiguously in the expanded output, and b) cannot
        // collide with template-file bytes since the template never
        // contains it literally.
        const MARKER: &str = "ZZZ_UNIQUE_MARKER_ZZZ";
        let main_src = format!(
            "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\"\n\
             \x20\x20\x20\x20xmlns:sce=\"http://sce.dev/ext\"\n\
             \x20\x20\x20\x20version=\"1.0\" initial=\"s\">\n\
             \x20\x20\x20\x20<sce:use template=\"t.xml\" id=\"{MARKER}\"/>\n\
             </scxml>"
        );
        fs::write(&main_path, &main_src).unwrap();

        let input_map = PositionMap::identity(main_path.to_str().unwrap(), &main_src);
        let (expanded, map, _deps) = template::expand(
            &main_src,
            main_path.to_str().unwrap(),
            Some(tmp.path()),
            &[],
            &input_map,
        )
        .expect("template expansion succeeds");

        // The marker must appear exactly once in the expanded
        // output — the `id="..."` value spliced into `{$id}`. Both
        // checks (presence + uniqueness) guard against an expander
        // change that drops or duplicates substitutions.
        let marker_offset = expanded.find(MARKER).expect(
            "marker substring must be present in expanded document — \
             splice path produced no output",
        );
        assert!(
            expanded[marker_offset + MARKER.len()..]
                .find(MARKER)
                .is_none(),
            "marker must be unique in expanded output",
        );

        // The `<sce:use>` element in `main.scxml` lives on row 4
        // (lines 1-3 are the `<scxml>` open spread across three
        // lines), col 5 (4-space indent before `<`). roxmltree's
        // `range().start` points at `<`, and
        // `apply_substitution_with_tracking` records the call-site
        // (row, col) supplied by `expand_impl`'s `doc_loc` — i.e.
        // exactly that position.
        let use_row = 4u32;
        let use_col = 5u32;

        // Sweep the entire marker range, not just the first byte.
        // CallSite collapse means *every* byte in the substituted
        // run must point at the same call-site (row, col); a future
        // refactor that emits per-byte File entries would slip past
        // a single-byte spot-check.
        for byte_in_marker in 0..MARKER.len() {
            let pos = map.lookup(marker_offset + byte_in_marker);
            assert!(
                pos.file.ends_with("main.scxml"),
                "marker byte {byte_in_marker} must remap to caller \
                 (main.scxml), got {:?}",
                pos.file,
            );
            assert_eq!(
                (pos.row, pos.col),
                (use_row, use_col),
                "marker byte {byte_in_marker} must point at <sce:use> \
                 (row={use_row}, col={use_col}); CallSite collapse \
                 violated",
            );
        }
    }

    #[test]
    fn remap_post_expansion_walks_xsd_multi_record_container() {
        // XSD validation is a multi-record container — the outer
        // Located has no (line, col) of its own, but each XsdDiag
        // carries its own line. The remap must descend into the
        // container so per-record lines resolve to source.
        use crate::forge::error::{ForgeError, Located, XmlError};
        use crate::forge::xsd_validator::{XsdDiag, XsdErrors};
        use crate::position_map::{Origin, PositionMap};
        use std::path::PathBuf;

        let expanded = "AAA\nBBBBBB\nCCC";
        let mut map = PositionMap::default();
        map.register_file(PathBuf::from("outer.xml"), "AAA\nCCC");
        map.register_file(PathBuf::from("frag.xml"), "BBBBBB\n");
        map.push_entry(
            0,
            4,
            Origin::File {
                path: PathBuf::from("outer.xml"),
                source_offset: 0,
            },
        );
        map.push_entry(
            4,
            11,
            Origin::File {
                path: PathBuf::from("frag.xml"),
                source_offset: 0,
            },
        );
        map.push_entry(
            11,
            14,
            Origin::File {
                path: PathBuf::from("outer.xml"),
                source_offset: 4,
            },
        );

        let xsd_errs = XsdErrors {
            source_label: "expanded.scxml".to_string(),
            diagnostics: vec![
                XsdDiag {
                    line: Some(2),
                    col: Some(3),
                    message: "violation on frag line".to_string(),
                },
                XsdDiag {
                    line: Some(3),
                    col: None,
                    message: "violation on tail line".to_string(),
                },
            ],
        };
        let err = Located::new(
            ForgeError::Xml(XmlError::SchemaValidation(xsd_errs)),
            "expanded.scxml",
            None,
            None,
        );
        let remapped = remap_post_expansion(err, expanded, &map);

        if let ForgeError::Xml(XmlError::SchemaValidation(ref xsd)) = remapped.error {
            // Record 0: expanded (2, 3) → frag.xml (1, 3).
            assert_eq!(xsd.diagnostics[0].line, Some(1));
            assert_eq!(xsd.diagnostics[0].col, Some(3));
            // Record 1: expanded (3, ?) → outer.xml (2, ?).
            // Col was None on input → stays None (we only rewrite
            // col when one was originally reported).
            assert_eq!(xsd.diagnostics[1].line, Some(2));
            assert_eq!(xsd.diagnostics[1].col, None);
        } else {
            panic!("expected SchemaValidation variant");
        }
    }

    // ── §wire-W4 Stage D: ParseError cross-side drift tests ────────
    //
    // Sister tests to W3's `cpp_xinclude_subtypes_match_rust_diagnostic_codes`
    // and `cpp_xinclude_subtype_code_returns_rust_wire_string` in
    // `sce-build/src/xinclude.rs`. α-strict scope: 2 NEW wire codes
    // (`xml/file-not-found`, `xml/wrong-root-element`) have full Rust
    // producers in this module's `parse_file` / `parse_impl`; the
    // other 3 C++ ParseError leaves (ParseXmlFailed, ParseException,
    // ParseNoRootElement) reuse the existing `xml/parse` wire code
    // because the Rust error model has no distinct producer for those
    // scenarios — Result-based, no exceptions, roxmltree always-has-
    // root.

    /// Pin the 1:1 mapping between Rust `XmlError::FileNotFound` /
    /// `WrongRootElement` variants, the `xml/*` `DiagnosticCode`s
    /// they emit, and the C++ `SCE::parsing::Parse<Variant>` subtypes
    /// declared in `sce/include/parsing/ParseError.h`. Also asserts
    /// the 3 reused-code leaves exist (they share `xml/parse` so they
    /// don't need a Rust XmlError variant — the wire-share is α-strict
    /// design per §wire-W4 D2).
    ///
    /// A commit on any one side that fails to update the other two is
    /// the drift this test catches.
    #[test]
    fn cpp_parse_subtypes_match_rust_diagnostic_codes() {
        // The 2 NEW W4 wire codes paired with their C++ class names.
        let rust_to_cpp_new: &[(&str, &str)] = &[
            ("xml/file-not-found", "ParseFileNotFound"),
            ("xml/wrong-root-element", "ParseWrongRootElement"),
        ];
        assert_eq!(
            rust_to_cpp_new.len(),
            2,
            "Expected 2 NEW α-strict wire codes; update if scope grew"
        );

        // The 3 reused-code leaves (no Rust XmlError variant by α-strict
        // design — see ParseError.h per-leaf comments). They still must
        // exist as ParseError subclasses to compile.
        let reused_code_cpp: &[&str] = &["ParseXmlFailed", "ParseException", "ParseNoRootElement"];

        let expected_cpp: BTreeSet<&str> = rust_to_cpp_new
            .iter()
            .map(|(_, cpp)| *cpp)
            .chain(reused_code_cpp.iter().copied())
            .collect();
        assert_eq!(
            expected_cpp.len(),
            5,
            "Expected 5 distinct ParseError subtypes (α-strict)"
        );

        let hdr = include_str!("../../sce/include/parsing/ParseError.h");
        let re = regex::Regex::new(r"class\s+(Parse\w+)\s*:\s*public\s+ParseError\b").unwrap();
        let mut found: BTreeSet<String> = BTreeSet::new();
        for captures in re.captures_iter(hdr) {
            found.insert(captures[1].to_string());
        }

        assert!(
            !found.is_empty(),
            "sce/include/parsing/ParseError.h must declare at least \
             one `class Parse<Variant> : public ParseError` — if the \
             declaration shape changed, update this drift test in the \
             same commit"
        );

        let found_refs: BTreeSet<&str> = found.iter().map(|s| s.as_str()).collect();
        assert_eq!(
            found_refs, expected_cpp,
            "ParseError subtype drift: C++ header = {:?}, expected \
             (5 leaves: 2 NEW-code + 3 reused-code) = {:?}. \
             Change both sides in the same commit (§wire-W4).",
            found_refs, expected_cpp
        );

        // Cross-check: every NEW Rust `DiagnosticCode` slash-path
        // MUST be spelled as the `serde(rename = \"...\")` literal in
        // `sce-build/src/forge/diagnostic.rs`. Catches a future
        // rename of the wire string on the Rust side without a C++
        // counter-edit.
        let diag = include_str!("forge/diagnostic.rs");
        for (rust_code, cpp_name) in rust_to_cpp_new {
            let needle = format!("\"{}\"", rust_code);
            assert!(
                diag.contains(&needle),
                "DiagnosticCode `{}` (paired with C++ `{}`) is not \
                 declared as a `serde(rename)` literal in \
                 sce-build/src/forge/diagnostic.rs. Keep the wire \
                 name, the Rust variant, and the C++ subtype in \
                 sync — see §wire-W4.",
                rust_code,
                cpp_name
            );
        }
    }

    /// Pin the wire-string return literal inside each C++
    /// `Parse<Variant>` subtype's `code()` body. §wire-W4 makes the
    /// 2 NEW-code leaves return their distinct `xml/*` strings while
    /// the 3 reused-code leaves all return `\"xml/parse\"` — both
    /// halves are pinned so a future rename on either side cannot
    /// drift the JSON wire contract silently.
    ///
    /// The bite: changing `return \"xml/file-not-found\"` to
    /// `return \"xml/file-not-foundXXX\"` in `ParseError.h` reds here
    /// with a pointed `does not contain` diff naming the exact class
    /// and exact missing literal.
    #[test]
    fn cpp_parse_subtype_code_returns_rust_wire_string() {
        // Pair every C++ class with its expected wire string. 2 leaves
        // get NEW codes; 3 share `xml/parse`.
        let class_to_code: &[(&str, &str)] = &[
            ("ParseFileNotFound", "xml/file-not-found"),
            ("ParseWrongRootElement", "xml/wrong-root-element"),
            ("ParseXmlFailed", "xml/parse"),
            ("ParseException", "xml/parse"),
            ("ParseNoRootElement", "xml/parse"),
        ];
        assert_eq!(class_to_code.len(), 5, "α-strict 5-leaf inventory");

        let hdr = include_str!("../../sce/include/parsing/ParseError.h");

        for (cpp_class, expected_code) in class_to_code {
            // Locate the class block. The header's shape (one class
            // per subtype, each terminated with `};`) keeps a forward
            // `find(\"};\")` accurate enough for a drift guard; if a
            // future rewrite nests braces inside a subtype we update
            // this scanner in the same commit.
            let class_marker = format!("class {} : public ParseError", cpp_class);
            let class_start = hdr.find(&class_marker).unwrap_or_else(|| {
                panic!(
                    "class `{}` not found in sce/include/parsing/\
                     ParseError.h — drift in subtype naming, see \
                     `cpp_parse_subtypes_match_rust_diagnostic_codes`",
                    cpp_class
                )
            });
            let body_start = hdr[class_start..].find('{').unwrap() + class_start + 1;
            let body_end_rel = hdr[body_start..].find("};").unwrap();
            let body = &hdr[body_start..body_start + body_end_rel];

            let needle = format!("return \"{}\";", expected_code);
            assert!(
                body.contains(&needle),
                "Class `{}` body does not contain `{}` — the C++ \
                 subtype's `code()` override must return the expected \
                 wire literal exactly so the JSON wire emitted by \
                 `to_json()` agrees with `--error-format=json`. RFC \
                 §W4 / SCE_ERROR_CONTRACT.md §3.",
                cpp_class,
                needle
            );
        }

        // Sanity-count: the header should declare exactly 5 `code()`
        // overrides on leaves + 1 pure-virtual on the base = 6
        // occurrences. A 6th leaf (or a missing one) reds with a
        // count diff rather than silently passing.
        let override_count = hdr
            .matches("std::string_view code() const noexcept override")
            .count();
        assert_eq!(
            override_count, 6,
            "expected 6 `code() const noexcept override` lines in \
             ParseError.h (1 pure-virtual on ParseError + 5 subtype \
             overrides); found {}",
            override_count
        );
    }

    // ── SCE Protocol-Synthesis RFC §synth-5-E — `<sce:on-sample>` structural tests ──
    //
    // These cover the SCXML extension's parser-AST + 3 structural
    // validator surfaces. Cross-ref behaviour (link-not-declared,
    // link-wrong-kind) is gated on the `SceCrossDocRegistry` and
    // is not exercised here.

    fn on_sample_test_doc(states_body: &str) -> String {
        format!(
            r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0"
       initial="running"
       datamodel="ecmascript"
       sce:kind="statechart">
{states_body}
</scxml>"##
        )
    }

    #[test]
    fn on_sample_basic_parses() {
        // The AST node is collected from a valid
        // <state> parent, with link/event recorded verbatim.
        let xml = on_sample_test_doc(
            r##"  <state id="running">
    <sce:on-sample link="scout_link" event="scout.tick"/>
    <transition event="scout.tick" target="running"/>
  </state>"##,
        );
        let model = SCXMLParser::new()
            .parse_string(&xml, "on_sample_basic")
            .expect("parse");
        let running = &model.states["running"];
        assert_eq!(running.on_sample_blocks.len(), 1);
        let block = &running.on_sample_blocks[0];
        assert_eq!(block.link, "scout_link");
        assert_eq!(block.event, "scout.tick");
        assert_eq!(block.document_order, 0);
    }

    #[test]
    fn on_sample_multi_link_per_state_parses() {
        // Multiple distinct links allowed in one
        // state. Document-order indices are 0-based.
        let xml = on_sample_test_doc(
            r##"  <state id="running">
    <sce:on-sample link="scout_link"  event="scout.tick"/>
    <sce:on-sample link="status_link" event="status.tick"/>
    <transition event="scout.tick"  target="running"/>
    <transition event="status.tick" target="running"/>
  </state>"##,
        );
        let model = SCXMLParser::new()
            .parse_string(&xml, "on_sample_multi_link")
            .expect("parse");
        let running = &model.states["running"];
        assert_eq!(running.on_sample_blocks.len(), 2);
        assert_eq!(running.on_sample_blocks[0].link, "scout_link");
        assert_eq!(running.on_sample_blocks[1].link, "status_link");
        assert_eq!(running.on_sample_blocks[0].document_order, 0);
        assert_eq!(running.on_sample_blocks[1].document_order, 1);
    }

    #[test]
    fn on_sample_invalid_parent_at_root_rejected() {
        // The placement rule rejects on-sample blocks
        // outside <state>/<parallel>. Root-level placement is the
        // canonical author-mistake — the diagnostic surfaces the
        // crossing parent name verbatim.
        let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0"
       initial="running"
       datamodel="ecmascript"
       sce:kind="statechart">
  <sce:on-sample link="scout_link" event="scout.tick"/>
  <state id="running">
    <transition event="scout.tick" target="running"/>
  </state>
</scxml>"##;
        let err = SCXMLParser::new()
            .parse_string(xml, "on_sample_root_reject")
            .expect_err("root-level <sce:on-sample> must be rejected");
        let err_str = format!("{:?}", err);
        assert!(
            err_str.contains("OnSampleInvalidParent"),
            "expected OnSampleInvalidParent, got: {err_str}"
        );
        assert!(
            err_str.contains("scxml"),
            "diagnostic should name the crossing parent: {err_str}"
        );
    }

    #[test]
    fn on_sample_invalid_parent_inside_onentry_rejected() {
        // <onentry> inside <state> looks plausible but is not one of
        // the two valid parents — the placement rule is parent-tag
        // exact, not ancestor-loose.
        let xml = on_sample_test_doc(
            r##"  <state id="running">
    <onentry>
      <sce:on-sample link="scout_link" event="scout.tick"/>
    </onentry>
    <transition event="scout.tick" target="running"/>
  </state>"##,
        );
        let err = SCXMLParser::new()
            .parse_string(&xml, "on_sample_onentry_reject")
            .expect_err("<sce:on-sample> inside <onentry> must be rejected");
        let err_str = format!("{:?}", err);
        assert!(
            err_str.contains("OnSampleInvalidParent"),
            "expected OnSampleInvalidParent, got: {err_str}"
        );
        assert!(
            err_str.contains("onentry"),
            "diagnostic should name <onentry> as the crossing parent: {err_str}"
        );
    }

    #[test]
    fn on_sample_duplicate_link_rejected() {
        // Uniqueness: same link declared twice in
        // one state is rejected even though multi-link fan-in is
        // allowed.
        let xml = on_sample_test_doc(
            r##"  <state id="running">
    <sce:on-sample link="scout_link" event="scout.tick"/>
    <sce:on-sample link="scout_link" event="scout.refresh"/>
    <transition event="scout.tick" target="running"/>
  </state>"##,
        );
        let err = SCXMLParser::new()
            .parse_string(&xml, "on_sample_dup_reject")
            .expect_err("duplicate link must be rejected");
        let err_str = format!("{:?}", err);
        assert!(
            err_str.contains("OnSampleLinkDuplicateInState"),
            "expected OnSampleLinkDuplicateInState, got: {err_str}"
        );
        assert!(
            err_str.contains("scout_link"),
            "diagnostic should name the duplicated link: {err_str}"
        );
    }

    #[test]
    fn on_sample_event_name_conflict_with_error_prefix_rejected() {
        // Event names colliding with the W3C SCXML
        // §5.10 internal-event prefix family (error.*, done.*) are
        // rejected. `error.io` is the canonical regression case.
        let xml = on_sample_test_doc(
            r##"  <state id="running">
    <sce:on-sample link="scout_link" event="error.io"/>
    <transition event="error.io" target="running"/>
  </state>"##,
        );
        let err = SCXMLParser::new()
            .parse_string(&xml, "on_sample_event_conflict")
            .expect_err("event name colliding with error.* must be rejected");
        let err_str = format!("{:?}", err);
        assert!(
            err_str.contains("OnSampleEventNameConflict"),
            "expected OnSampleEventNameConflict, got: {err_str}"
        );
        assert!(
            err_str.contains("error."),
            "diagnostic should quote the reserved prefix: {err_str}"
        );
    }

    #[test]
    fn on_sample_event_name_done_prefix_rejected() {
        // Symmetric coverage for the done.* prefix — both halves of
        // the W3C internal-event family must reject.
        let xml = on_sample_test_doc(
            r##"  <state id="running">
    <sce:on-sample link="scout_link" event="done.state.foo"/>
    <transition event="done.state.foo" target="running"/>
  </state>"##,
        );
        let err = SCXMLParser::new()
            .parse_string(&xml, "on_sample_done_conflict")
            .expect_err("event name colliding with done.* must be rejected");
        let err_str = format!("{:?}", err);
        assert!(
            err_str.contains("OnSampleEventNameConflict"),
            "expected OnSampleEventNameConflict, got: {err_str}"
        );
    }

    // ── SCE Protocol-Synthesis RFC §synth-5-E — `<sce:on-sample>` cross-ref ───────
    //
    // Cross-ref validator integrates with the build's
    // SceCrossDocRegistry (populated by walking every parsed `.forge`
    // file). Tests below construct a synthetic SCXMLModel via
    // `parse_string` (so all of the structural validators
    // fire) and then invoke `validate_on_sample_link_references`
    // with various registry shapes.

    fn parse_running_with_link(link: &str) -> crate::model::SCXMLModel {
        let xml = on_sample_test_doc(&format!(
            r##"  <state id="running">
    <sce:on-sample link="{link}" event="scout.tick"/>
    <transition event="scout.tick" target="running"/>
  </state>"##
        ));
        SCXMLParser::new()
            .parse_string(&xml, "cross_ref_test")
            .expect("parse + structural validation")
    }

    /// Build a synthetic forge link document for cross-ref tests. Any
    /// stage-pool reference passed via `stage_pool` lands on the
    /// `LinkModel.stage_pool` field, which `record_document` then
    /// captures into the registry's sparse `stage_pools` map.
    fn make_link_doc(name: &str, stage_pool: Option<&str>) -> crate::forge::model::ForgeDocument {
        use crate::forge::model::{BackpressurePolicy, ForgeDocument, LinkClass, LinkModel};
        ForgeDocument::Link(LinkModel {
            name: name.to_string(),
            class: LinkClass::Udp,
            framer: "scout_frame_codec".to_string(),
            backpressure: BackpressurePolicy::Drop,
            inbound: vec![],
            outbound: vec![],
            rx_pool: None,
            tx_pool: None,
            stage_pool: stage_pool.map(String::from),
            accept_stage_copy_rate: false,
            // Synthetic test helper — not built from a real
            // `<scxml>` element, so §synth-5-O Atomic 0c populates the
            // post-emit walker with the same fixture stub the rest
            // of these tests use.
            source_location: None,
        })
    }

    #[test]
    fn cross_ref_link_resolves_when_registered() {
        // Happy path: the registry knows the link by name AND records
        // a `<sce:stage-pool>` for it (RFC §synth-5-E stage-pool gate) →
        // both checks pass, no diagnostic.
        use crate::forge::cross_doc_registry::SceCrossDocRegistry;
        use crate::forge::pool_registry::ForgePoolRegistry;
        let model = parse_running_with_link("scout_link");
        let mut registry = SceCrossDocRegistry::new();
        registry
            .record_document(&make_link_doc("scout_link", Some("scout_stage_pool")))
            .unwrap();
        let pool_registry = ForgePoolRegistry::new();
        validate_on_sample_link_references(&model, &registry, &pool_registry, "cross_ref_test")
            .expect("registered link with stage_pool resolves cleanly");
    }

    #[test]
    fn cross_ref_link_not_declared_emits_candidates() {
        // Sad path: the registry holds a different name → the
        // unresolved reference surfaces as
        // `OnSampleLinkNotDeclared` with the registry's actual
        // link names (sorted) as `Fix::ReplaceOneOf` candidates.
        use crate::forge::cross_doc_registry::{SceCrossDocRegistry, ScxmlDocKind};
        use crate::forge::pool_registry::ForgePoolRegistry;
        let model = parse_running_with_link("scout_link");
        let mut registry = SceCrossDocRegistry::new();
        registry.record("status_link", ScxmlDocKind::Link).unwrap();
        let pool_registry = ForgePoolRegistry::new();
        let err =
            validate_on_sample_link_references(&model, &registry, &pool_registry, "cross_ref_test")
                .expect_err("unregistered link must be rejected");
        let err_str = format!("{:?}", err);
        assert!(
            err_str.contains("OnSampleLinkNotDeclared"),
            "expected OnSampleLinkNotDeclared, got: {err_str}"
        );
        assert!(
            err_str.contains("scout_link"),
            "diagnostic should name the unresolved link: {err_str}"
        );
        assert!(
            err_str.contains("status_link"),
            "diagnostic should carry candidates: {err_str}"
        );
    }

    #[test]
    fn cross_ref_empty_registry_yields_empty_candidates() {
        // Edge case: no link kinds registered anywhere in the
        // build → diagnostic still fires (NotDeclared) with an
        // empty candidate list, signalling "likely missing
        // .forge file" to the author.
        use crate::forge::cross_doc_registry::SceCrossDocRegistry;
        use crate::forge::pool_registry::ForgePoolRegistry;
        let model = parse_running_with_link("scout_link");
        let registry = SceCrossDocRegistry::new();
        let pool_registry = ForgePoolRegistry::new();
        let err =
            validate_on_sample_link_references(&model, &registry, &pool_registry, "cross_ref_test")
                .expect_err("empty registry → reference must be rejected");
        let err_str = format!("{:?}", err);
        assert!(
            err_str.contains("OnSampleLinkNotDeclared"),
            "expected OnSampleLinkNotDeclared, got: {err_str}"
        );
        // Candidates vec should be empty in the printed Debug form.
        assert!(
            err_str.contains("candidates: []"),
            "empty registry should yield empty candidates list: {err_str}"
        );
    }

    #[test]
    fn cross_ref_no_on_sample_blocks_is_noop() {
        // States without `<sce:on-sample>` produce no validator
        // surface — empty registry is fine.
        use crate::forge::cross_doc_registry::SceCrossDocRegistry;
        use crate::forge::pool_registry::ForgePoolRegistry;
        let xml = on_sample_test_doc(
            r##"  <state id="running">
    <transition event="external" target="running"/>
  </state>"##,
        );
        let model = SCXMLParser::new()
            .parse_string(&xml, "cross_ref_noop")
            .expect("parse");
        let registry = SceCrossDocRegistry::new();
        let pool_registry = ForgePoolRegistry::new();
        validate_on_sample_link_references(&model, &registry, &pool_registry, "cross_ref_noop")
            .expect("states without on-sample blocks need no registry entries");
    }

    // ── SCE Protocol-Synthesis RFC §synth-5-E — stage-pool gate ──
    //
    // The stage-pool gate is a third validator gate after the kind-resolution gates:
    // a registered link without `<sce:stage-pool>` cannot back an
    // `<sce:on-sample>` subscriber that ever reaches `Sample::take()`.
    // Tests below pin both arms (resolves on stage_pool present,
    // rejects on stage_pool absent) plus the candidate list shape.

    #[test]
    fn cross_ref_link_without_stage_pool_emits_take_diagnostic() {
        use crate::forge::cross_doc_registry::SceCrossDocRegistry;
        use crate::forge::pool_registry::{ForgePoolKind, ForgePoolRegistry};
        let model = parse_running_with_link("scout_link");
        let mut registry = SceCrossDocRegistry::new();
        // Link is registered (kind matches) BUT has no `<sce:stage-pool>`
        // — the η' gate fires `pool/sample-take-without-stage-pool`.
        registry
            .record_document(&make_link_doc("scout_link", None))
            .unwrap();
        let mut pool_registry = ForgePoolRegistry::new();
        pool_registry
            .record("scout_stage_pool", ForgePoolKind::BufferPool)
            .unwrap();
        pool_registry
            .record("alt_stage_pool", ForgePoolKind::BufferPool)
            .unwrap();
        let err =
            validate_on_sample_link_references(&model, &registry, &pool_registry, "cross_ref_test")
                .expect_err("on-sample on link without stage_pool must be rejected");
        let err_str = format!("{:?}", err);
        assert!(
            err_str.contains("PoolSampleTakeWithoutStagePool"),
            "expected PoolSampleTakeWithoutStagePool, got: {err_str}"
        );
        assert!(
            err_str.contains("scout_link"),
            "diagnostic should name the link: {err_str}"
        );
        // Candidate list pulls from the pool registry's buffer-pool kind
        // names so authors see a concrete `<sce:stage-pool>` `ref` target.
        assert!(
            err_str.contains("alt_stage_pool"),
            "diagnostic should carry pool-registry candidates: {err_str}"
        );
        assert!(
            err_str.contains("scout_stage_pool"),
            "diagnostic should carry pool-registry candidates: {err_str}"
        );
    }

    #[test]
    fn cross_ref_link_with_stage_pool_no_pool_registry_still_passes() {
        // A1 only inspects the link's stage_pool *presence* — it does
        // not cross-resolve the named pool against the build's pool
        // registry (that's the rx_pool / tx_pool slot-size validator's
        // territory; for stage_pool the pool kind validation defers to
        // the deploy-side `mesh/deploy-stage-pool-*` family). So a
        // link that declares `<sce:stage-pool>` resolves cleanly even
        // when the pool registry is empty.
        use crate::forge::cross_doc_registry::SceCrossDocRegistry;
        use crate::forge::pool_registry::ForgePoolRegistry;
        let model = parse_running_with_link("scout_link");
        let mut registry = SceCrossDocRegistry::new();
        registry
            .record_document(&make_link_doc("scout_link", Some("scout_stage_pool")))
            .unwrap();
        let pool_registry = ForgePoolRegistry::new();
        validate_on_sample_link_references(&model, &registry, &pool_registry, "cross_ref_test")
            .expect("link with stage_pool resolves regardless of pool registry contents");
    }

    // ── SCE Protocol-Synthesis RFC §synth-5-E — `callback="rust:..."` ──
    //
    // The optional `<sce:on-sample callback="rust:crate::path::fn">`
    // attribute pairs with the `validate_on_sample_callback_paths`
    // structural validator that enforces the accepted Rust path subset.
    // Tests below pin: AST round-trip, happy path, four reachable failure
    // arms (empty, unknown prefix, malformed `::`, malformed segment).

    fn parse_running_with_callback(callback: Option<&str>) -> crate::model::SCXMLModel {
        let inner = match callback {
            Some(cb) => format!(
                r##"  <state id="running">
    <sce:on-sample link="scout_link" event="scout.tick" callback="{cb}"/>
    <transition event="scout.tick" target="running"/>
  </state>"##
            ),
            None => r##"  <state id="running">
    <sce:on-sample link="scout_link" event="scout.tick"/>
    <transition event="scout.tick" target="running"/>
  </state>"##
                .to_string(),
        };
        let xml = on_sample_test_doc(&inner);
        SCXMLParser::new()
            .parse_string(&xml, "callback_test")
            .expect("parse + structural validation")
    }

    #[test]
    fn callback_attr_absent_round_trips_as_none() {
        // Absence of `callback=` is a backwards-compat
        // shape with the link/event-only `<sce:on-sample link/event>`
        // — the AST field carries `None` and downstream codegen
        // synthesizes a default dispatch shim.
        let model = parse_running_with_callback(None);
        let state = &model.states["running"];
        assert_eq!(state.on_sample_blocks.len(), 1);
        assert!(state.on_sample_blocks[0].callback.is_none());
    }

    #[test]
    fn callback_attr_present_round_trips_to_ast_field() {
        // Well-formed `rust:crate::module::fn`
        // path lands on the AST field verbatim and clears the
        // structural validator.
        let model = parse_running_with_callback(Some("rust:my_app::on_scout_sample"));
        let state = &model.states["running"];
        assert_eq!(
            state.on_sample_blocks[0].callback.as_deref(),
            Some("rust:my_app::on_scout_sample"),
        );
    }

    #[test]
    fn callback_unknown_language_prefix_rejects() {
        // Today only `rust:` is accepted. `cpp:` raises
        // `pool/sample-callback-signature-non-borrow` with the
        // UnknownLanguagePrefix arm.
        let xml = on_sample_test_doc(
            r##"  <state id="running">
    <sce:on-sample link="scout_link" event="scout.tick" callback="cpp:my_app::on_scout"/>
    <transition event="scout.tick" target="running"/>
  </state>"##,
        );
        let err = SCXMLParser::new()
            .parse_string(&xml, "callback_test")
            .expect_err("non-rust prefix must be rejected");
        let err_str = format!("{:?}", err);
        assert!(
            err_str.contains("PoolSampleCallbackSignatureNonBorrow"),
            "expected PoolSampleCallbackSignatureNonBorrow, got: {err_str}",
        );
        assert!(
            err_str.contains("UnknownLanguagePrefix") && err_str.contains("\"cpp\""),
            "diagnostic should carry the unknown-prefix reason: {err_str}",
        );
    }

    #[test]
    fn callback_missing_colon_rejects_as_unknown_prefix() {
        // No `:` separator at all — the parser surfaces this through
        // the same UnknownLanguagePrefix arm with an empty `prefix`
        // field. Authors typically arrive here by writing the path
        // without the language prefix, which today is required.
        let xml = on_sample_test_doc(
            r##"  <state id="running">
    <sce:on-sample link="scout_link" event="scout.tick" callback="my_app::on_scout"/>
    <transition event="scout.tick" target="running"/>
  </state>"##,
        );
        let err = SCXMLParser::new()
            .parse_string(&xml, "callback_test")
            .expect_err("missing language prefix must be rejected");
        let err_str = format!("{:?}", err);
        assert!(
            err_str.contains("UnknownLanguagePrefix"),
            "expected UnknownLanguagePrefix arm: {err_str}",
        );
    }

    #[test]
    fn callback_leading_double_colon_rejects_as_malformed_path() {
        let xml = on_sample_test_doc(
            r##"  <state id="running">
    <sce:on-sample link="scout_link" event="scout.tick" callback="rust:::on_scout"/>
    <transition event="scout.tick" target="running"/>
  </state>"##,
        );
        let err = SCXMLParser::new()
            .parse_string(&xml, "callback_test")
            .expect_err("leading `::` must be rejected");
        let err_str = format!("{:?}", err);
        assert!(
            err_str.contains("MalformedPath"),
            "expected MalformedPath arm: {err_str}",
        );
    }

    #[test]
    fn callback_invalid_segment_rejects_as_malformed_segment() {
        // Segment with shell metacharacter — caught by the
        // `is_rust_path_segment` ASCII identifier subset.
        let xml = on_sample_test_doc(
            r##"  <state id="running">
    <sce:on-sample link="scout_link" event="scout.tick" callback="rust:my_app::on-scout"/>
    <transition event="scout.tick" target="running"/>
  </state>"##,
        );
        let err = SCXMLParser::new()
            .parse_string(&xml, "callback_test")
            .expect_err("non-identifier segment must be rejected");
        let err_str = format!("{:?}", err);
        assert!(
            err_str.contains("MalformedSegment"),
            "expected MalformedSegment arm: {err_str}",
        );
        assert!(
            err_str.contains("on-scout"),
            "diagnostic should name the offending segment: {err_str}",
        );
    }

    #[test]
    fn callback_empty_value_rejects_as_empty_path() {
        let xml = on_sample_test_doc(
            r##"  <state id="running">
    <sce:on-sample link="scout_link" event="scout.tick" callback=""/>
    <transition event="scout.tick" target="running"/>
  </state>"##,
        );
        let err = SCXMLParser::new()
            .parse_string(&xml, "callback_test")
            .expect_err("empty callback must be rejected");
        let err_str = format!("{:?}", err);
        assert!(
            err_str.contains("EmptyPath"),
            "expected EmptyPath arm: {err_str}",
        );
    }

    #[test]
    fn callback_path_keyword_segments_accepted() {
        // `crate::`, `self::`, `super::` are valid Rust path keywords
        // — the validator must accept them as legal
        // segment shapes (not rejected as non-identifiers).
        for callback in &[
            "rust:crate::on_scout",
            "rust:self::callbacks::on_scout",
            "rust:super::on_scout",
        ] {
            let xml = on_sample_test_doc(&format!(
                r##"  <state id="running">
    <sce:on-sample link="scout_link" event="scout.tick" callback="{callback}"/>
    <transition event="scout.tick" target="running"/>
  </state>"##
            ));
            SCXMLParser::new()
                .parse_string(&xml, "callback_test")
                .unwrap_or_else(|e| {
                    panic!("path keyword `{callback}` should parse cleanly; got: {e:?}")
                });
        }
    }

    // ── `sce:req` requirement-annotation attribute ──────

    #[test]
    fn sce_req_collected_on_state_transition_and_invoke() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                              xmlns:sce="http://sce.dev/ext"
                              version="1.0" initial="armed" datamodel="null">
            <state id="armed" sce:req="REQ_AB_12345 REQ_CD_67890">
                <transition event="go" target="firing" sce:req="REQ_AB_12346"/>
                <invoke type="scxml" src="child.scxml" sce:req="REQ_AB_12347"/>
            </state>
            <state id="firing"/>
        </scxml>"#;
        let model = SCXMLParser::new()
            .parse_string(scxml, "sce_req_basic")
            .expect("parse");
        let armed = &model.states["armed"];
        let req_strings: Vec<&str> = armed.req.iter().map(|r| r.0.as_str()).collect();
        assert_eq!(req_strings, vec!["REQ_AB_12345", "REQ_CD_67890"]);
        let transition = &armed.transitions[0];
        let trans_req: Vec<&str> = transition.req.iter().map(|r| r.0.as_str()).collect();
        assert_eq!(trans_req, vec!["REQ_AB_12346"]);
        let invoke = &armed.invokes[0];
        let base = match invoke {
            crate::model::Invoke::Scxml(info) => &info.common.base,
            _ => panic!("expected Scxml invoke variant"),
        };
        let invoke_req: Vec<&str> = base.req.iter().map(|r| r.0.as_str()).collect();
        assert_eq!(invoke_req, vec!["REQ_AB_12347"]);
    }

    #[test]
    fn sce_req_on_onentry_inherits_to_block_actions() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                              xmlns:sce="http://sce.dev/ext"
                              version="1.0" initial="armed" datamodel="null">
            <state id="armed">
                <onentry sce:req="REQ_AB_12350">
                    <raise event="ev1"/>
                    <raise event="ev2"/>
                </onentry>
            </state>
        </scxml>"#;
        let model = SCXMLParser::new()
            .parse_string(scxml, "sce_req_inherit")
            .expect("parse");
        let armed = &model.states["armed"];
        let block = &armed.on_entry_blocks[0];
        assert_eq!(block.len(), 2);
        for action in block {
            let action_req: Vec<&str> = action.req.iter().map(|r| r.0.as_str()).collect();
            assert_eq!(action_req, vec!["REQ_AB_12350"]);
        }
    }

    #[test]
    fn sce_req_on_action_overrides_inherits_union() {
        // Block-level req inherits onto every action; inner req
        // appears first, then any non-overlapping inherited token.
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                              xmlns:sce="http://sce.dev/ext"
                              version="1.0" initial="armed" datamodel="null">
            <state id="armed">
                <onentry sce:req="BLOCK_REQ">
                    <raise event="ev1" sce:req="INNER_REQ"/>
                </onentry>
            </state>
        </scxml>"#;
        let model = SCXMLParser::new()
            .parse_string(scxml, "sce_req_union")
            .expect("parse");
        let action = &model.states["armed"].on_entry_blocks[0][0];
        let action_req: Vec<&str> = action.req.iter().map(|r| r.0.as_str()).collect();
        assert_eq!(action_req, vec!["INNER_REQ", "BLOCK_REQ"]);
    }

    #[test]
    fn sce_req_duplicate_token_rejected() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                              xmlns:sce="http://sce.dev/ext"
                              version="1.0" initial="armed" datamodel="null">
            <state id="armed" sce:req="REQ_001 REQ_001"/>
        </scxml>"#;
        let err = SCXMLParser::new()
            .parse_string(scxml, "sce_req_dup")
            .expect_err("duplicate sce:req should reject");
        let rendered = format!("{err:?}");
        assert!(
            rendered.contains("DuplicateRequirementId"),
            "expected ValidationError::DuplicateRequirementId in: {rendered}"
        );
        assert!(
            rendered.contains("REQ_001"),
            "expected duplicate id 'REQ_001' in: {rendered}"
        );
    }

    #[test]
    fn sce_req_absent_attribute_leaves_vec_empty() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                              version="1.0" initial="armed" datamodel="null">
            <state id="armed">
                <transition event="go" target="armed"/>
                <onentry><raise event="ev"/></onentry>
            </state>
        </scxml>"#;
        let model = SCXMLParser::new()
            .parse_string(scxml, "sce_req_absent")
            .expect("parse");
        let armed = &model.states["armed"];
        assert!(armed.req.is_empty());
        assert!(armed.transitions[0].req.is_empty());
        assert!(armed.on_entry_blocks[0][0].req.is_empty());
    }
}
