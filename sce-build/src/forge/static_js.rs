// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A `datamodel="sce-static"` document, lowered to the ECMAScript document the
//! Interpreter runs (docs/SCE_ACCEPTED_SUBSET.md §2.15,
//! claudedocs/shared-calendar/e7-static-datamodel-design.md D11–D16).
//!
//! The generated backends hold the document's variables to their types and
//! check its integer operations. The Interpreter hands every expression to a
//! script engine that does neither, so it refuses a `sce-static` document
//! (`scxml/unsupported-datamodel`) rather than run it with a meaning the
//! document did not give it. This module is the way to run one anyway: it
//! rewrites the document into an ordinary `datamodel="ecmascript"` one whose
//! every expression is what [`ExprTarget::Js`] lowers it to, calling a small
//! library ([`runtime_expression`]) for the operations the document checks.
//!
//! # The document is edited, not re-rendered
//!
//! There is no model-to-SCXML serialiser, and one would have to cover every
//! element the Interpreter runs — `<send>`, `<invoke>`, `<parallel>`,
//! `<history>`, `<donedata>` — and drop, in silence, whatever it forgot. What
//! changes between the two documents is only their expressions, and the model
//! already keeps the exact place of each expression attribute for its
//! diagnostics ([`crate::attribute_spelling`]). So the rewrite replaces those
//! places in the text the document was parsed from, and every other byte —
//! comments, extension elements, formatting — is the author's.
//!
//! The walk is the one the Kotlin and Rust backends use
//! ([`crate::forge::static_lowering::lower`]): what a `sce-static` document
//! means, and what it is refused for, is decided once. This target only says
//! how the Interpreter's language spells it, and reads the expressions the
//! walk lowered from [`StaticLowering::sites`].
//!
//! # What it does not lower yet
//!
//! Records, lists, calls of imported algorithms, `<sce:action>`, and the
//! executable content the walk does not lower are refused with
//! `generate/unsupported-feature` naming the construct, never passed through:
//! an expression left as the author wrote it would be run by the script engine
//! as ECMAScript, which is the mis-execution the refusal exists to prevent.
//!
//! # An event's data
//!
//! An event's data reaches the Interpreter as untyped JSON, and a generated
//! machine reads it through the event's schema. So a read of a schema field is
//! a call of the library's `field`, which refuses what the generated lift of a
//! payload refuses — no data, a bare value, a missing field, a value of another
//! type or beyond its width — by throwing, and the expression that read it
//! fails as an overflow does. One difference is left standing and stated
//! (docs/SCE_ACCEPTED_SUBSET.md §2.15): a generated machine lifts the payload
//! once when the event is dequeued and raises `error.execution` once for a
//! malformed delivery, and the Interpreter raises one for each expression that
//! reads a field.

use std::ops::Range;
use std::path::PathBuf;

use crate::forge::error::{ForgeError, GenerateError, Located};
use crate::forge::expr::ExprTarget;
use crate::forge::model::SceType;
use crate::forge::static_lowering::{lower, Callee, LoweredSite, StaticTarget};
use crate::forge::type_ctx::StaticScope;
use crate::model::{Action, Datamodel, SCXMLModel};

/// The global of the script engine the library is bound to, so every lowered
/// expression reads `SceStatic.<member>`.
///
/// ⚠ A global, not a `<data>` value: the Interpreter carries a data value
/// between its script engine and the machine as plain data, and an object
/// whose members are functions comes out of that with none of them
/// (`SceStatic.U8.add` was "not a function", measured 2026-09-30). The
/// library is installed by the expression of a `<data>` instead, which writes
/// the global as it runs and leaves the `<data>` holding a flag.
pub(crate) const RUNTIME_GLOBAL: &str = "SceStatic";

/// The id of the `<data>` that installs the library — the document's first, so
/// it runs before any other `<data>` and before every expression.
pub(crate) const RUNTIME_DATA_ID: &str = "SceStaticInstalled";

/// What `_event.data` is read through: the Interpreter's own name for the data
/// of the event being processed. The expression lowerer spells a field of it
/// as a call of the library's `field` (`ExprTarget::Js`).
pub(crate) const PAYLOAD_ACCESSOR: &str = "_event.data";

/// The integer widths the library implements, and so the only ones an
/// operation may be checked at. A width outside this list has no
/// `SceStatic.<I|U><bits>` to call — the lowering refuses it rather than write
/// a call that fails at run time.
pub(crate) const INTEGER_WIDTHS: [u8; 4] = [8, 16, 32, 64];

/// The data model the lowered document declares.
const LOWERED_DATAMODEL: &str = "ecmascript";

/// The library's source as it is kept in the tree, formatted to be read.
const RUNTIME_SOURCE: &str = include_str!("static_js/sce_static.js");

/// `source` without its `/* … */` comments.
fn without_block_comments(source: &str) -> String {
    let mut code = String::with_capacity(source.len());
    let mut rest = source;
    while let Some(open) = rest.find("/*") {
        code.push_str(&rest[..open]);
        let close = rest[open..]
            .find("*/")
            .expect("sce_static.js holds an unterminated comment");
        rest = &rest[open + close + 2..];
    }
    code.push_str(rest);
    code
}

/// The expression of the `<data>` that installs the library: its comments
/// dropped — they explain the source, and would ship inside every lowered
/// document — and then every line trimmed and the lines joined by a single
/// space, so the attribute that carries it reads the same to every XML parser
/// however it normalises a line break. It binds the library to
/// [`RUNTIME_GLOBAL`] and gives the `<data>` a value that survives the
/// Interpreter's script boundary.
///
/// A line comment would swallow the rest of that one line, so the source holds
/// none (`the_library_survives_being_put_on_one_line` holds it to that).
pub(crate) fn runtime_expression() -> String {
    let library = without_block_comments(RUNTIME_SOURCE)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    format!("(function () {{ globalThis.{RUNTIME_GLOBAL} = {library}; return true; }})()")
}

/// The Interpreter's script engine, as a target of the `sce-static` walk.
///
/// A variable is a global of the script engine under its own id, and `In` is
/// the Interpreter's own predicate, so both are spelled as the author wrote
/// them. It has no statements of its own: executable content stays the
/// document's, and only the expressions in it are replaced
/// ([`StaticLowering::sites`]). So the methods that spell a statement answer
/// nothing, and the ones for a construct [`Self::unsupported`] refuses are
/// never asked.
pub struct JsTarget;

/// Why a spelling this target does not have is never asked for.
const REFUSED_BEFORE_THE_WALK: &str = "the construct is refused by JsTarget::unsupported before \
     the walk reaches a spelling for it";

impl StaticTarget for JsTarget {
    fn name(&self) -> &'static str {
        LOWERED_DATAMODEL
    }
    fn callee(&self, _document_name: &str) -> Option<Callee> {
        None
    }
    fn unsupported(&self, model: &SCXMLModel, scope: &StaticScope) -> Option<String> {
        if let Some(variable) = scope.variables.iter().find(|v| {
            v.value_type
                .as_ref()
                .is_some_and(|t| t.record_alias().is_some() || t.list_elem().is_some())
        }) {
            let kind = if variable
                .value_type
                .as_ref()
                .is_some_and(|t| t.record_alias().is_some())
            {
                "record"
            } else {
                "list"
            };
            return Some(format!("the {kind} variable `{}`", variable.id));
        }
        if !model.global_scripts.is_empty() {
            return Some("a top-level <script>".to_string());
        }
        if model.has_invoke() {
            return Some("<invoke>".to_string());
        }
        for state in model.states.values() {
            if state.donedata.is_some() {
                return Some(format!("the <donedata> of state `{}`", state.id));
            }
            let lists = state
                .on_entry_blocks
                .iter()
                .chain(&state.on_exit_blocks)
                .map(Vec::as_slice)
                .chain([
                    state.initial_transition_actions.as_slice(),
                    state.initial_history_default_actions.as_slice(),
                ])
                .chain(state.transitions.iter().map(|t| t.actions.as_slice()));
            for actions in lists {
                if let Some(construct) = first_unsupported_action(actions) {
                    return Some(construct);
                }
            }
        }
        None
    }
    fn expr_target(&self) -> ExprTarget {
        ExprTarget::Js
    }
    fn field_name(&self, id: &str) -> String {
        id.to_string()
    }
    fn field_ref(&self, name: &str) -> String {
        name.to_string()
    }
    fn in_function(&self) -> &'static str {
        "In"
    }
    fn scalar_type(&self, ty: &SceType) -> String {
        ty.as_attr().to_string()
    }
    fn scalar_view(&self, _ty: &SceType) -> Option<String> {
        None
    }
    fn record_type(&self, _machine: &str, _alias: &str) -> String {
        unreachable!("{REFUSED_BEFORE_THE_WALK}")
    }
    fn record_def(
        &self,
        _ty: &str,
        _alias: &str,
        _schema: &crate::forge::model::EventSchemaModel,
    ) -> String {
        unreachable!("{REFUSED_BEFORE_THE_WALK}")
    }
    fn record_field(&self, _id: &str) -> String {
        unreachable!("{REFUSED_BEFORE_THE_WALK}")
    }
    fn record_value(&self, _ty: &str, _fields: &[(String, String)]) -> String {
        unreachable!("{REFUSED_BEFORE_THE_WALK}")
    }
    fn list_type(&self, _elem: &SceType) -> String {
        unreachable!("{REFUSED_BEFORE_THE_WALK}")
    }
    fn list_view(&self, _elem: &SceType) -> Option<String> {
        unreachable!("{REFUSED_BEFORE_THE_WALK}")
    }
    fn list_empty(&self) -> String {
        unreachable!("{REFUSED_BEFORE_THE_WALK}")
    }
    fn assign(&self, _target: &str, _value: &str) -> String {
        String::new()
    }
    fn assign_field(&self, _target: &str, _field: &str, _value: &str) -> String {
        unreachable!("{REFUSED_BEFORE_THE_WALK}")
    }
    fn log(&self, _label: &str, _value: &str) -> String {
        String::new()
    }
    fn append(
        &self,
        _target: &str,
        _capacity: u32,
        _value: &str,
        _overflow: Option<&str>,
    ) -> String {
        unreachable!("{REFUSED_BEFORE_THE_WALK}")
    }
    fn clear(&self, _target: &str) -> String {
        unreachable!("{REFUSED_BEFORE_THE_WALK}")
    }
    fn raise_execution_error(&self, _machine: &str, _message: &str) -> String {
        String::new()
    }
    // The script engine's own channel: a library call that throws stops the
    // statement before it writes, a condition that throws is false, and
    // `error.execution` is raised either way (W3C SCXML 5.9.1, 3.12.2) —
    // the outcome E12 D5 gives the generated backends.
    fn receiving_statement(&self, statement: &str, _failed: &str) -> String {
        statement.to_string()
    }
    fn receiving_condition(&self, value: &str, _failed: &str) -> String {
        value.to_string()
    }
    fn payload_accessor(&self, _event: &str) -> String {
        PAYLOAD_ACCESSOR.to_string()
    }
    // A delivery that carried no payload makes the read throw, so the guard
    // is false and says so: there is no channel to check before it.
    fn payload_guard(&self, _machine: &str, _event: &str, lowered: &str) -> String {
        lowered.to_string()
    }
}

/// The first action of `actions`, or of anything nested in one, that is not
/// executable content the walk lowers — described for a refusal.
///
/// An allowlist: a kind of executable content added to the model is refused
/// here until it is admitted, and never left as the author wrote it.
fn first_unsupported_action(actions: &[Action]) -> Option<String> {
    let mut found = None;
    for action in actions {
        action.walk(&mut |a| {
            if found.is_none() {
                found = unsupported_action(a);
            }
        });
    }
    found
}

fn unsupported_action(action: &Action) -> Option<String> {
    match action.action_type.as_str() {
        "assign" if action.expr.trim().is_empty() => {
            Some("an <assign> that holds its value as content".to_string())
        }
        "assign" | "if" | "log" | "raise" | "cancel" => None,
        "send" if !action.params.is_empty() || !action.contentexpr.is_empty() => {
            Some("a <send> that carries <param> or <content expr>".to_string())
        }
        "send" => None,
        "sce_append" => Some("<sce:append>".to_string()),
        "sce_clear" => Some("<sce:clear>".to_string()),
        "native_action" => Some("<sce:action>".to_string()),
        other => Some(format!("a <{other}> action")),
    }
}

/// One replacement in the text a document was parsed from.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Edit {
    range: Range<usize>,
    text: String,
}

/// `text` as it is written inside an XML attribute value, whichever quote the
/// attribute uses, and so that reading it back gives `text` exactly.
fn xml_attribute_value(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            '\t' => escaped.push_str("&#9;"),
            '\n' => escaped.push_str("&#10;"),
            '\r' => escaped.push_str("&#13;"),
            other => escaped.push(other),
        }
    }
    escaped
}

/// `text` with every edit applied. The edits are disjoint by construction —
/// each is one attribute — and a pair that is not is a defect in the walk that
/// produced them, so it is refused rather than applied in some order.
fn apply(text: &str, mut edits: Vec<Edit>) -> Result<String, GenerateError> {
    edits.sort_by_key(|e| e.range.start);
    if let Some(pair) = edits.windows(2).find(|w| w[0].range.end > w[1].range.start) {
        return Err(GenerateError::unsupported(format!(
            "two lowered expressions claim the same place in the document ({:?} and {:?})",
            pair[0].range, pair[1].range
        )));
    }
    let mut lowered = String::with_capacity(text.len());
    let mut at = 0;
    for edit in &edits {
        lowered.push_str(&text[at..edit.range.start]);
        lowered.push_str(&edit.text);
        at = edit.range.end;
    }
    lowered.push_str(&text[at..]);
    Ok(lowered)
}

/// The replacement each lowered expression asks for. An expression that
/// lowered to what the author wrote is left alone.
fn site_edits(sites: &[LoweredSite]) -> Result<Vec<Edit>, GenerateError> {
    let mut edits = Vec::new();
    for site in sites {
        if site.text.trim() == site.source.trim() {
            continue;
        }
        let Some(written) = site.spelling.as_ref().and_then(|s| s.value()) else {
            return Err(GenerateError::unsupported(format!(
                "`{}` has no place in the document to be rewritten at: the model holds it \
                 as something the document does not spell",
                site.source.trim()
            )));
        };
        edits.push(Edit {
            range: written.range(),
            text: xml_attribute_value(&site.text),
        });
    }
    Ok(edits)
}

const SCXML_NAMESPACE: &str = "http://www.w3.org/2005/07/scxml";

fn is_scxml_element(node: &roxmltree::Node<'_, '_>, name: &str) -> bool {
    node.is_element()
        && node.tag_name().name() == name
        && node.tag_name().namespace() == Some(SCXML_NAMESPACE)
}

/// The edits that turn the document into an ECMAScript one: its data model,
/// and — when an expression calls the library — the `<data>` that installs it,
/// placed first so every other `<data>` and every later expression finds it.
fn document_edits(text: &str, needs_library: bool) -> Result<Vec<Edit>, GenerateError> {
    let document = roxmltree::Document::parse(text).map_err(|e| {
        GenerateError::unsupported(format!(
            "the document cannot be read again to be rewritten: {e}"
        ))
    })?;
    let root = document.root_element();
    let mut edits = Vec::new();
    let declared = root
        .attributes()
        .find(|a| a.name() == "datamodel" && a.namespace().is_none())
        .ok_or_else(|| {
            GenerateError::unsupported("the <scxml> root declares no datamodel".to_string())
        })?;
    edits.push(Edit {
        range: declared.range_value(),
        text: LOWERED_DATAMODEL.to_string(),
    });
    if needs_library {
        let first_data = root
            .children()
            .find(|n| is_scxml_element(n, "datamodel"))
            .and_then(|datamodel| datamodel.children().find(|n| is_scxml_element(n, "data")))
            .ok_or_else(|| {
                GenerateError::unsupported(
                    "the lowered document calls the runtime library and declares no top-level \
                     <data> to install it before the others"
                        .to_string(),
                )
            })?;
        let at = first_data.range().start;
        edits.push(Edit {
            range: at..at,
            text: format!(
                "<data id=\"{RUNTIME_DATA_ID}\" expr=\"{}\"/>\n    ",
                xml_attribute_value(&runtime_expression())
            ),
        });
    }
    Ok(edits)
}

/// `text`, a `sce-static` document, as the ECMAScript document that means the
/// same to the Interpreter. `model` is `text` parsed — the places its
/// expressions were written are places in `text` — and judged.
///
/// A document under any other data model is returned as it is.
fn lower_parsed(
    text: &str,
    model: &SCXMLModel,
    label: &str,
) -> Result<String, Located<ForgeError>> {
    let refuse = |error: GenerateError| Located::in_file(ForgeError::from(error), label);
    if model.datamodel != Datamodel::SceStatic {
        return Ok(text.to_string());
    }
    if let Some(variable) = StaticScope::of(model)
        .into_iter()
        .flat_map(|scope| scope.variables)
        .find(|v| v.id == RUNTIME_GLOBAL || v.id == RUNTIME_DATA_ID)
    {
        return Err(refuse(GenerateError::unsupported(format!(
            "<data id=\"{}\"> takes a name the ecmascript lowering keeps for its runtime \
             library; rename the variable",
            variable.id
        ))));
    }
    let mut lowered = model.clone();
    let machine = crate::filters::to_pascal_case(model.name.clone());
    let lowering = lower(&mut lowered, &machine, &[], &JsTarget).map_err(refuse)?;
    let mut edits = site_edits(&lowering.sites).map_err(refuse)?;
    let needs_library = lowering
        .sites
        .iter()
        .any(|s| s.text.contains(&format!("{RUNTIME_GLOBAL}.")));
    edits.extend(document_edits(text, needs_library).map_err(refuse)?);
    apply(text, edits).map_err(refuse)
}

/// The document `path` names, lowered for the Interpreter — through the same
/// reading `generate` gives it, so `<xi:include>` and `<sce:use>` are expanded
/// and the imports beside it are resolved. The lowered document is the
/// expanded one, and stands alone.
pub fn lower_file(path: &str, include_dirs: Vec<PathBuf>) -> Result<String, Located<ForgeError>> {
    let mut parser = crate::parser::SCXMLParser::new().with_include_dirs(include_dirs);
    let model = parser.parse_file(path)?;
    let expanded = model
        .authored_positions
        .as_ref()
        .map(|positions| positions.expanded.clone())
        .ok_or_else(|| {
            Located::in_file(
                ForgeError::from(GenerateError::unsupported(
                    "the parse kept no text of the document to be rewritten".to_string(),
                )),
                path,
            )
        })?;
    lower_parsed(&expanded, &model, path)
}

/// `text`, a document read from memory, lowered for the Interpreter. Nothing
/// beside it is resolved: no include, no import.
pub fn lower_source(text: &str, name: &str) -> Result<String, Located<ForgeError>> {
    let model = crate::parser::SCXMLParser::new().parse_string(text, name)?;
    lower_parsed(text, &model, name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::expr::CheckedOp;

    /// Whether the library implements `op` as a method of each integer type.
    /// Exhaustive on purpose: a new [`CheckedOp`] breaks this match, and the
    /// author is pointed at the library that has to gain the method.
    fn is_integer_method(op: CheckedOp) -> bool {
        match op {
            CheckedOp::Add
            | CheckedOp::Sub
            | CheckedOp::Mul
            | CheckedOp::Div
            | CheckedOp::Rem
            | CheckedOp::Neg
            | CheckedOp::Narrow => true,
            // A read of a collection, not of an integer type.
            CheckedOp::Index => false,
        }
    }

    const EVERY_OP: [CheckedOp; 8] = [
        CheckedOp::Add,
        CheckedOp::Sub,
        CheckedOp::Mul,
        CheckedOp::Div,
        CheckedOp::Rem,
        CheckedOp::Neg,
        CheckedOp::Narrow,
        CheckedOp::Index,
    ];

    /// The emitter and the library are two halves of one contract: every name
    /// the emitter can write is one the library defines.
    #[test]
    fn the_library_defines_every_name_the_emitter_writes() {
        let text = runtime_expression();
        for op in EVERY_OP {
            let method = format!("{}: function", op.helper());
            if is_integer_method(op) {
                assert!(text.contains(&method), "no `{method}` in the library");
            }
        }
        for member in ["at: function", "round: function", "field: field"] {
            assert!(text.contains(member), "no `{member}` in the library");
        }
        // Every type a payload field is read at is one `field` knows: the
        // integer types by their range, the rest by name.
        for kind in ["float32", "float64", "bool", "string"] {
            assert!(
                text.contains(&format!("'{kind}'")),
                "`field` has no reader for `{kind}`"
            );
        }
        for signed in [false, true] {
            for bits in INTEGER_WIDTHS {
                let prefix = if signed { 'I' } else { 'U' };
                let declared = format!("{prefix}{bits}: integer({signed}, {bits})");
                assert!(text.contains(&declared), "no `{declared}` in the library");
            }
        }
    }

    /// The library is one expression that binds [`RUNTIME_GLOBAL`] and puts
    /// nothing else in the script engine's global scope, and whose value — the
    /// one the `<data>` holds — is a flag: a value with function members does
    /// not survive the Interpreter's script boundary.
    #[test]
    fn the_library_is_one_self_contained_expression() {
        let text = runtime_expression();
        assert!(
            text.starts_with(&format!(
                "(function () {{ globalThis.{RUNTIME_GLOBAL} = (function () {{"
            )),
            "{text}"
        );
        assert!(text.ends_with("})(); return true; })()"), "{text}");
        assert!(!text.contains('\n'));
        assert!(
            !text.contains("/*"),
            "the comments explain the source and do not ship in every document"
        );
    }

    /// The library sits in an XML attribute whose line breaks a parser
    /// normalises, and is joined onto one line above: a `//` comment would
    /// take the rest of the library with it.
    #[test]
    fn the_library_survives_being_put_on_one_line() {
        assert!(
            !RUNTIME_SOURCE.contains("//"),
            "a line comment in sce_static.js comments out everything after it once the \
             lines are joined"
        );
        // No line break is doing the work of a semicolon: a line that ends a
        // statement without one would run into the next once they are joined.
        let code = without_block_comments(RUNTIME_SOURCE);
        for line in code.lines().map(str::trim).filter(|line| !line.is_empty()) {
            assert!(
                line == "})()" || line.ends_with(['{', '}', ';', ',']),
                "`{line}` ends in neither a brace, a comma nor a semicolon"
            );
        }
    }

    const COUNTER: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="counter">
  <!-- kept as written -->
  <datamodel>
    <data id="level" sce:type="uint8" expr="250"/>
    <data id="refusals" sce:type="uint8" expr="0"/>
  </datamodel>
  <state id="idle">
    <transition event="bump" cond="level &lt; 255" target="idle">
      <assign location="level" expr="level + 3"/>
      <if cond="level === 253">
        <log label="at" expr="'253'"/>
      <elseif cond="level > 250"/>
        <assign location="refusals" expr="refusals + 1"/>
      </if>
    </transition>
    <transition event="stop" cond="In('idle')" target="done"/>
  </state>
  <final id="done"/>
</scxml>"#;

    fn lowered(document: &str) -> String {
        lower_source(document, "counter").unwrap_or_else(|e| panic!("does not lower: {e:?}"))
    }

    fn refusal(document: &str) -> String {
        format!(
            "{:?}",
            lower_source(document, "counter").expect_err("must be refused")
        )
    }

    /// The lowered document is the author's, expression by expression: the
    /// data model, the library, and each checked operation differ, and
    /// nothing else does.
    #[test]
    fn only_the_expressions_and_the_data_model_differ() {
        let out = lowered(COUNTER);
        assert!(out.contains(r#"datamodel="ecmascript""#), "{out}");
        assert!(!out.contains(r#"datamodel="sce-static""#), "{out}");
        assert!(
            out.contains(r#"expr="SceStatic.U8.add(level, 3)""#),
            "the assignment is checked at its width: {out}"
        );
        assert!(out.contains("<!-- kept as written -->"), "{out}");
        assert!(
            out.contains(r#"sce:type="uint8" expr="250""#),
            "an initial value that lowered to itself is untouched: {out}"
        );
        // Undoing the two kinds of edit gives the input back byte for byte.
        let undone = out
            .replace(r#"datamodel="ecmascript""#, r#"datamodel="sce-static""#)
            .replace(
                r#"expr="SceStatic.U8.add(level, 3)""#,
                r#"expr="level + 3""#,
            )
            .replace(
                r#"expr="SceStatic.U8.add(refusals, 1)""#,
                r#"expr="refusals + 1""#,
            );
        let library = format!(
            "<data id=\"{RUNTIME_DATA_ID}\" expr=\"{}\"/>\n    ",
            xml_attribute_value(&runtime_expression())
        );
        assert_eq!(undone.replacen(&library, "", 1), COUNTER);
    }

    /// The lowered document is well-formed and its data model is the one the
    /// Interpreter runs: reading it back is what the Interpreter does.
    #[test]
    fn the_lowered_document_reads_back_as_an_ecmascript_document() {
        let out = lowered(COUNTER);
        let document = roxmltree::Document::parse(&out).expect("well-formed");
        let root = document.root_element();
        assert_eq!(root.attribute("datamodel"), Some("ecmascript"));
        let first = root
            .children()
            .find(|n| is_scxml_element(n, "datamodel"))
            .and_then(|d| d.children().find(|n| is_scxml_element(n, "data")))
            .expect("a first <data>");
        assert_eq!(
            first.attribute("id"),
            Some(RUNTIME_DATA_ID),
            "the library is installed before every other <data>"
        );
        assert_eq!(first.attribute("expr"), Some(runtime_expression().as_str()));
        let condition = root
            .descendants()
            .find(|n| is_scxml_element(n, "transition") && n.attribute("event") == Some("bump"))
            .and_then(|t| t.attribute("cond"))
            .expect("the guard");
        assert_eq!(condition, "level < 255", "a guard the walk left as written");
    }

    /// A document under another data model is not this lowering's to touch.
    #[test]
    fn a_document_under_another_data_model_is_returned_as_it_is() {
        let ecmascript = COUNTER
            .replace(r#"datamodel="sce-static""#, r#"datamodel="ecmascript""#)
            .replace(r#" sce:type="uint8""#, "");
        assert_eq!(lowered(&ecmascript), ecmascript);
    }

    #[test]
    fn a_value_inside_a_single_quoted_attribute_is_escaped_for_it() {
        assert_eq!(
            xml_attribute_value("a < 'b' && \"c\""),
            "a &lt; &apos;b&apos; &amp;&amp; &quot;c&quot;"
        );
        let single = COUNTER.replace(r#"expr="level + 3""#, "expr='level + 3'");
        let out = lowered(&single);
        assert!(out.contains("expr='SceStatic.U8.add(level, 3)'"), "{out}");
        roxmltree::Document::parse(&out).expect("well-formed");
    }

    /// Each construct the lowering does not have is named in its refusal, and
    /// the document is not passed through.
    #[test]
    fn what_it_cannot_lower_yet_is_refused_by_name() {
        let with_list = COUNTER.replace(
            r#"<data id="refusals" sce:type="uint8" expr="0"/>"#,
            r#"<data id="refusals" sce:type="uint8" expr="0"/>
    <data id="history" sce:type="list&lt;uint8&gt;" sce:capacity="4"/>"#,
        );
        assert!(refusal(&with_list).contains("the list variable `history`"));
        let with_script = COUNTER.replace(
            r#"<log label="at" expr="'253'"/>"#,
            r#"<script>level = 0;</script>"#,
        );
        assert!(refusal(&with_script).contains("<script>"));
    }

    #[test]
    fn a_variable_may_not_take_the_libraries_names() {
        for name in [RUNTIME_GLOBAL, RUNTIME_DATA_ID] {
            let clash = COUNTER.replace(
                r#"<data id="refusals" sce:type="uint8" expr="0"/>"#,
                &format!(
                    r#"<data id="refusals" sce:type="uint8" expr="0"/>
    <data id="{name}" sce:type="uint8" expr="0"/>"#
                ),
            );
            assert!(refusal(&clash).contains(name), "{name}");
        }
    }

    #[test]
    fn edits_that_overlap_are_refused_not_ordered() {
        let edits = vec![
            Edit {
                range: 0..4,
                text: "a".to_string(),
            },
            Edit {
                range: 2..6,
                text: "b".to_string(),
            },
        ];
        assert!(apply("0123456789", edits).is_err());
        let disjoint = vec![
            Edit {
                range: 4..6,
                text: "B".to_string(),
            },
            Edit {
                range: 0..2,
                text: "A".to_string(),
            },
        ];
        assert_eq!(apply("0123456789", disjoint).unwrap(), "A23B6789");
    }
}
