// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A `datamodel="sce-static"` document, lowered to the ECMAScript document the
//! Interpreter runs (docs/SCE_ACCEPTED_SUBSET.md §2.15).
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
//! `<sce:action>`, an algorithm with a construct
//! [`crate::forge::static_js_algorithm`] does not spell, an `<invoke>` that is
//! not a child session written inline (one by `src`, a hybrid, a mesh or a
//! host-run one) and the executable content the walk does not lower are refused
//! with `generate/unsupported-feature` naming the
//! construct, never passed through: an expression left as the author wrote it
//! would be run by the script engine as ECMAScript, which is the
//! mis-execution the refusal exists to prevent.
//!
//! # Child sessions
//!
//! An inline child is a `sce-static` document of its own, with variables of its
//! own that the values an `<invoke>` hands it land in, so it is lowered as one
//! and put back where it stands in its parent's text. The values are expressions
//! of the parent: each `<param>` is lowered where it is written, and a string the
//! child bounds carries that bound (`SceStatic.bounded`), so that a value past it
//! fails as it does on the generated backends and is left out. A `namelist` name
//! cannot carry one, and a child that bounds a string it hands is refused by
//! name.
//!
//! # Algorithms
//!
//! An imported algorithm is read from beside the document and lowered to a
//! function installed by the `<data>` that installs the library, as
//! `SceStatic.algorithms.<name>`; a call of it is a call of that. A failure —
//! a checked operation, a `<sce:require>` — is a throw, so the expression that
//! called it fails as an overflow of its own does.
//!
//! # Lists and records
//!
//! A list is an array and a record a plain object, and neither is changed in
//! place: a statement that changes one is an `<assign>` of the whole value,
//! written again (`SceStatic.append`, `SceStatic.set`). So a copy of the
//! machine's state taken earlier keeps what it saw, as the generated backends'
//! does, and a full list fails the way any assignment does. The `<data>` of a
//! list or a record, and the `<sce:append>` and `<sce:clear>` elements, are
//! replaced whole ([`StaticLowering::elements`]).
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

use std::collections::BTreeMap;
use std::ops::Range;
use std::path::{Path, PathBuf};

use crate::forge::error::{ForgeError, GenerateError, Located};
use crate::forge::expr::ExprTarget;
use crate::forge::import_source::ImportSource;
use crate::forge::model::{ForgeDocument, ForgeImport, ForgeKind, ParsedForge, SceType};
use crate::forge::static_js_algorithm;
use crate::forge::static_lowering::{lower, Callee, LoweredElement, LoweredSite, StaticTarget};
use crate::forge::type_ctx::StaticScope;
use crate::forge::types::InferredType;
use crate::model::{Action, Datamodel, SCXMLModel};
use crate::model::{DoneDataContent, Invoke};

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
///
/// The algorithms the document calls are installed beside the library, each as
/// `SceStatic.algorithms.<symbol>`: they are written in the same attribute, so
/// the function a guard calls exists before the first guard is evaluated.
pub(crate) fn runtime_expression(algorithms: &[LoweredAlgorithm]) -> String {
    let library = without_block_comments(RUNTIME_SOURCE)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let installed: String = algorithms
        .iter()
        .map(|a| {
            format!(
                " {RUNTIME_GLOBAL}.algorithms.{} = {};",
                a.symbol, a.function
            )
        })
        .collect();
    format!(
        "(function () {{ globalThis.{RUNTIME_GLOBAL} = {library};{installed} return true; }})()"
    )
}

/// An algorithm the document calls, as the function it becomes
/// ([`crate::forge::static_js_algorithm`]) and the name it is installed under.
#[derive(Debug, Clone)]
pub(crate) struct LoweredAlgorithm {
    pub(crate) symbol: String,
    pub(crate) function: String,
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
///
/// It is built with the algorithms the document calls, already lowered: a call
/// of one it has none for is refused by the walk, where the document is read,
/// and never left as a name the script engine does not define.
pub struct JsTarget {
    /// Each imported algorithm's document name, and the symbol it is installed
    /// under.
    algorithms: BTreeMap<String, String>,
}

impl JsTarget {
    fn new(algorithms: BTreeMap<String, String>) -> Self {
        Self { algorithms }
    }

    /// The `<assign>` of `location` to the expression `expr`, as the element.
    fn assign_element(&self, location: &str, expr: &str) -> String {
        format!(
            "<assign location=\"{}\" expr=\"{}\"/>",
            xml_attribute_value(location),
            xml_attribute_value(expr)
        )
    }
}

impl StaticTarget for JsTarget {
    fn name(&self) -> &'static str {
        LOWERED_DATAMODEL
    }
    // The Interpreter performs `<sce:action>` through the host installed on the
    // machine (`INativeActionHost`), with the arguments its engine computes.
    fn lowers_host_action_arguments(&self) -> bool {
        true
    }
    // The payload is the engine's `_event.data`, read as the JSON it arrived
    // as, and an enum value is the variant's declared name there as it is in
    // the data model: the field needs no type of its own.
    fn payload_enum_fields(&self) -> bool {
        true
    }
    fn payload_enum_read(&self, accessor: &str, field: &str, variants: &[&str]) -> Option<String> {
        let names: Vec<String> = variants.iter().map(|name| format!("'{name}'")).collect();
        Some(format!(
            "{RUNTIME_GLOBAL}.field({accessor}, '{field}', [{}])",
            names.join(", ")
        ))
    }
    fn callee(&self, document_name: &str) -> Option<Callee> {
        let symbol = self.algorithms.get(document_name)?;
        Some(Callee {
            call: format!("{RUNTIME_GLOBAL}.algorithms.{symbol}"),
            // The function travels in the document, so there is nothing to
            // import.
            import: Some(String::new()),
        })
    }
    fn unsupported(&self, model: &SCXMLModel, _scope: &StaticScope) -> Option<String> {
        if !model.global_scripts.is_empty() {
            return Some("a top-level <script>".to_string());
        }
        for state in model.states.values() {
            for invoke in &state.invokes {
                if let Some(construct) = unsupported_invoke(invoke, &state.id) {
                    return Some(construct);
                }
            }
            // The pairs of a `<donedata>` are expressions the walk lowers, in
            // the attribute each is written in, and its inline text is finished
            // to the string it spells, at the place it is written. A
            // `<content expr>` the model let through names a record, which the
            // engine reads as the object it holds, the fields the generated
            // backends carry as pairs; a `<content>` that holds an element has
            // no text to finish and is read by the engine as a document, where
            // the generated backends carry it as a string.
            if let Some(done) = &state.donedata {
                match &done.content {
                    DoneDataContent::InlineText(_) if done.content_text_spelling.is_none() => {
                        return Some(format!(
                            "the <donedata> <content> holding an element, of state `{}`",
                            state.id
                        ));
                    }
                    _ => {}
                }
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
    // A record is a plain object of its schema's fields, written again with a
    // field changed rather than changed in place: a saved or published copy of
    // the machine's state keeps what it saw, as the generated backends' does.
    fn record_type(&self, machine: &str, alias: &str) -> String {
        format!("{machine}{alias}Record")
    }
    // Nothing is declared: the Interpreter's data model holds no types.
    fn record_def(
        &self,
        _ty: &str,
        _alias: &str,
        _schema: &crate::forge::model::EventSchemaModel,
        _enum_types: &std::collections::BTreeMap<String, String>,
    ) -> String {
        String::new()
    }
    fn record_field(&self, id: &str) -> String {
        id.to_string()
    }
    // Parenthesised: `{a: 1}` alone reads as a block.
    fn record_value(&self, _ty: &str, fields: &[(String, String)]) -> String {
        let members: Vec<String> = fields
            .iter()
            .map(|(field, value)| format!("'{field}': {value}"))
            .collect();
        format!("({{{}}})", members.join(", "))
    }
    // The data model holds no types, so an enum value is the variant's
    // declared name: a string, which is also what a saved state holds, and
    // which `Alias.variant` lowers to ([`crate::forge::expr`]). Nothing is
    // declared.
    fn enum_type(&self, _machine: &str, alias: &str) -> Option<String> {
        Some(format!("enum:{alias}"))
    }
    fn list_type(&self, elem: &SceType) -> String {
        format!("list<{}>", elem.as_attr())
    }
    fn list_view(&self, _elem: &SceType) -> Option<String> {
        None
    }
    fn record_list_type(&self, record: &str) -> String {
        format!("list<{record}>")
    }
    fn record_list_view(&self, _record: &str) -> Option<String> {
        None
    }
    fn list_empty(&self) -> String {
        "[]".to_string()
    }
    // The library throws past the bound, which the Interpreter answers as it
    // does any assignment that fails: nothing is written, `error.execution` is
    // raised, and the block ends (§scxml-4.9).
    fn bounded_string(&self, value: &str, capacity: u32) -> String {
        format!("{RUNTIME_GLOBAL}.bounded({value}, {capacity})")
    }
    fn assign(&self, _target: &str, _value: &str) -> String {
        String::new()
    }
    fn assign_field(&self, _target: &str, _field: &str, _value: &str) -> String {
        String::new()
    }
    // The document's `<assign location="rec.field">` becomes an `<assign>` of
    // the whole record, written again with that field changed.
    fn field_assignment(&self, record: &str, field: &str, value: &str) -> Option<(String, String)> {
        Some((
            record.to_string(),
            format!("{RUNTIME_GLOBAL}.set({record}, '{field}', {value})"),
        ))
    }
    // A list or a record is given its initial value by the `<data>` that
    // declares it: the list is empty, and a record is built from its fields.
    fn data_element(&self, id: &str, init: &str) -> Option<String> {
        Some(format!(
            "<data id=\"{}\" expr=\"{}\"/>",
            xml_attribute_value(id),
            xml_attribute_value(init)
        ))
    }
    fn log(&self, _label: &str, _value: &str, _ty: InferredType) -> String {
        String::new()
    }
    // The element that replaces the `<sce:append>`: an `<assign>` of the list,
    // written again with the value at its end. A full list makes the library
    // throw, which the Interpreter answers as it does any assignment that
    // fails: nothing is written, `error.execution` is raised, and the block
    // ends (§scxml-4.9) — the outcome the generated backends give.
    fn append(
        &self,
        target: &str,
        capacity: u32,
        value: &str,
        _value_can_fail: bool,
        _overflow: &str,
        _failed: &str,
    ) -> String {
        self.assign_element(
            target,
            &format!("{RUNTIME_GLOBAL}.append({target}, {capacity}, {value})"),
        )
    }
    fn clear(&self, target: &str) -> String {
        self.assign_element(target, "[]")
    }
    fn raise_execution_error(&self, _machine: &str, _message: &str) -> String {
        String::new()
    }
    // The script engine's own channel: a library call that throws stops the
    // statement before it writes, a condition that throws is false, and
    // `error.execution` is raised either way (§scxml-5.9.1, 3.12.2) — and
    // §scxml-4.9 ends the block, which the Interpreter does for any
    // element that raised. Nothing here is spelled: the document's own
    // executable content is what runs.
    fn receiving_statement(&self, statement: &str, _failed: &str) -> String {
        statement.to_string()
    }
    fn receiving_call(&self, statement: &str, _failed: &str) -> String {
        statement.to_string()
    }
    fn receiving_condition(&self, value: &str, _failed: &str, _flag: &str) -> String {
        value.to_string()
    }
    fn condition_failed_flag(&self, _if_ordinal: u32) -> String {
        String::new()
    }
    fn payload_accessor(&self, _event: &str) -> String {
        PAYLOAD_ACCESSOR.to_string()
    }
    // A delivery that carried no payload makes the read throw, so the guard
    // is false and says so: there is no channel to check before it.
    fn payload_guard(&self, _machine: &str, _event: &str, lowered: &str) -> String {
        lowered.to_string()
    }
    // The Interpreter's own data model reads the lowered expression where the
    // document wrote it, so a `<param>` has no typed value to build.
    fn wire_value(&self, _ty: InferredType, value: &str) -> String {
        value.to_string()
    }
    // An enum value is already the name its enum declares: a variant lowers to
    // that string, and a variable holds it.
    fn enum_wire_name(&self, _alias: &str, value: &str) -> Option<String> {
        Some(value.to_string())
    }
    // The Interpreter's own `<send>` evaluates its `delayexpr` and reads the
    // string as a time, so the expression is lowered in the attribute it is
    // written in.
    fn lowers_delay_expr(&self) -> bool {
        true
    }
}

/// What of an `<invoke>` has no lowering for the Interpreter, described for a
/// refusal: every kind but a child session written inline, since its own
/// document is lowered where it stands ([`child_edits`]) and a `src` is a file
/// the lowering would have to write another one for.
///
/// An allowlist, as [`unsupported_action`] is.
fn unsupported_invoke(invoke: &Invoke, state: &str) -> Option<String> {
    let info = match invoke {
        Invoke::Scxml(info) => info,
        Invoke::Hybrid(_) => return Some(format!("a hybrid <invoke> in state `{state}`")),
        Invoke::MeshRpc(_) => return Some(format!("a mesh <invoke> in state `{state}`")),
        Invoke::Unsupported(_) => {
            return Some(format!("a host-run <invoke> in state `{state}`"));
        }
    };
    if info.inline_child.is_none() || info.inline_child_range.is_none() {
        return Some(format!("an <invoke> by `src` in state `{state}`"));
    }
    // A `namelist` name is handed to the child by the Interpreter as it holds it,
    // with no expression to carry the bound the child declared for a string.
    let declared = info.common.child_static_variables.as_deref().unwrap_or(&[]);
    for name in info.namelist.split_whitespace() {
        let bounded_string = declared.iter().find(|v| v.id == name).is_some_and(|v| {
            v.capacity.is_some()
                && matches!(
                    v.value_type.as_ref().and_then(|t| t.scalar()),
                    Some(SceType::String)
                )
        });
        if bounded_string {
            return Some(format!(
                "the `namelist` name `{name}` of the <invoke> in state `{state}`, which hands \
                 a child a string it bounds"
            ));
        }
    }
    None
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
        // ECMAScript's own `<foreach>` walks the array a list variable is, and
        // declares the item and the index; only its body's expressions are
        // lowered, typed by what the loop variables are.
        "assign" | "if" | "log" | "raise" | "cancel" | "foreach" => None,
        // An inline text is finished to the string it spells at the place it is
        // written; one that holds an element has no text to finish, and an
        // engine reads it as a document where the generated backends carry it
        // as a string.
        "send" if !action.content.trim().is_empty() && action.content_text_spelling.is_none() => {
            Some("a <send> whose <content> holds an element".to_string())
        }
        // The pairs of its `<param>`s are expressions the walk lowers, in the
        // attribute each is written in, and the Interpreter's own `<send>` reads
        // them once, when it runs. A `<content expr>` the judge let through names
        // a record, which that `<send>` reads as the object it holds: the fields
        // the generated backends carry as pairs.
        "send" => None,
        "sce_append" | "sce_clear" => None,
        // A host operation: the Interpreter performs it through its host, and
        // the arguments are expressions the walk lowers.
        "native_action" => None,
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
    // An insertion sorts before an edit that starts where it does: the library
    // goes in front of the first `<data>`, which may itself be replaced.
    edits.sort_by_key(|e| (e.range.start, e.range.end));
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

/// The replacement each lowered element asks for: the element that carries the
/// attribute the walk named, in full, becomes the element it lowered to.
///
/// The element is found by the attribute because that is what the model keeps
/// of where an element was written; the attribute's range lies inside exactly
/// one element, and that element's range is what is replaced.
fn element_edits(text: &str, elements: &[LoweredElement]) -> Result<Vec<Edit>, GenerateError> {
    if elements.is_empty() {
        return Ok(Vec::new());
    }
    let document = roxmltree::Document::parse(text).map_err(|e| {
        GenerateError::unsupported(format!(
            "the document cannot be read again to be rewritten: {e}"
        ))
    })?;
    let mut edits = Vec::new();
    for element in elements {
        let Some(written) = element.anchor.as_ref().and_then(|a| a.value()) else {
            return Err(GenerateError::unsupported(
                "an element has no place in the document to be rewritten at: the model holds it \
                 as something the document does not spell"
                    .to_string(),
            ));
        };
        let range = written.range();
        let owner = document.descendants().find(|node| {
            node.is_element()
                && node.attributes().any(|attribute| {
                    let value = attribute.range_value();
                    value.start <= range.start && range.end <= value.end
                })
        });
        let Some(owner) = owner else {
            return Err(GenerateError::unsupported(format!(
                "no element of the document carries the attribute written at {range:?}, so \
                 there is no element to rewrite"
            )));
        };
        edits.push(Edit {
            range: owner.range(),
            text: element.text.clone(),
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
fn document_edits(
    text: &str,
    needs_library: bool,
    algorithms: &[LoweredAlgorithm],
) -> Result<Vec<Edit>, GenerateError> {
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
                xml_attribute_value(&runtime_expression(algorithms))
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
    base_dir: Option<&Path>,
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
    let LoweredCallees { called, installed } =
        lowered_algorithms(model, base_dir).map_err(refuse)?;
    let target = JsTarget::new(called);
    let mut lowered = model.clone();
    let machine = crate::filters::to_pascal_case(model.name.clone());
    let lowering = lower(&mut lowered, &machine, &target).map_err(refuse)?;
    let mut edits = site_edits(&lowering.sites).map_err(refuse)?;
    edits.extend(element_edits(text, &lowering.elements).map_err(refuse)?);
    let library_call = format!("{RUNTIME_GLOBAL}.");
    let needs_library = lowering
        .sites
        .iter()
        .map(|site| site.text.as_str())
        .chain(lowering.elements.iter().map(|e| e.text.as_str()))
        .any(|text| text.contains(&library_call));
    edits.extend(document_edits(text, needs_library, &installed).map_err(refuse)?);
    edits.extend(child_edits(model, label, base_dir)?);
    apply(text, edits).map_err(refuse)
}

/// The edits that put each inline child session back where it stands in its
/// parent's text, lowered by the same walk as any document: the child is a
/// `sce-static` document of its own, with variables of its own that the values
/// an `<invoke>` hands it land in, and what the Interpreter runs for it is its
/// lowered text. A child under another data model is left as it was written.
///
/// The text the parser wrapped the child in begins with a prologue, which no
/// element has, so it is dropped before the child takes its place.
fn child_edits(
    model: &SCXMLModel,
    label: &str,
    base_dir: Option<&Path>,
) -> Result<Vec<Edit>, Located<ForgeError>> {
    let mut edits = Vec::new();
    for state in model.states.values() {
        for invoke in &state.invokes {
            let Invoke::Scxml(info) = invoke else {
                continue;
            };
            let (Some(child), Some(xml), Some(range)) = (
                info.inline_child.as_deref(),
                info.inline_child_xml.as_deref(),
                info.inline_child_range.clone(),
            ) else {
                continue;
            };
            let child_label = format!("{label}, <invoke id=\"{}\">", info.common.base.invoke_id);
            let lowered = lower_parsed(xml, child, &child_label, base_dir)?;
            if lowered == xml {
                continue;
            }
            let element = lowered
                .strip_prefix(crate::parser::INLINE_CHILD_DECLARATION)
                .unwrap_or(&lowered);
            edits.push(Edit {
                range,
                text: element.to_string(),
            });
        }
    }
    Ok(edits)
}

/// The algorithms a statechart calls, and the functions that have to be
/// installed for those calls to run.
struct LoweredCallees {
    /// Each imported algorithm's document name, and the symbol it is installed
    /// under.
    called: BTreeMap<String, String>,
    /// Every function installed: the algorithms the statechart calls and, once
    /// each, every algorithm those call. A function looks its callee up by name
    /// when it runs, so the order they are installed in does not matter.
    installed: Vec<LoweredAlgorithm>,
}

/// Every algorithm `model` imports, read from beside the document and lowered,
/// with the algorithms those import in turn. The walk asks the target for each
/// call, so one that is not here is refused there.
///
/// Refused here, by name: an import with no directory to be read from, one that
/// cannot be read, one that is not an algorithm, one that calls itself, and
/// every construct of a body that has no lowering yet.
fn lowered_algorithms(
    model: &SCXMLModel,
    base_dir: Option<&Path>,
) -> Result<LoweredCallees, GenerateError> {
    let mut lowered = LoweredCallees {
        called: BTreeMap::new(),
        installed: Vec::new(),
    };
    let Some(scope) = StaticScope::of(model) else {
        return Ok(lowered);
    };
    for callee in &scope.callees {
        let what = |reason: &str| {
            GenerateError::unsupported(format!(
                "`{}(…)`: the algorithm it imports {reason}",
                callee.alias
            ))
        };
        let import = model
            .forge_imports
            .iter()
            .find(|i| i.alias == callee.alias && i.kind == ForgeKind::Algorithm)
            .ok_or_else(|| what("has no <sce:import> this lowering can read"))?;
        let dir = base_dir
            .ok_or_else(|| what("cannot be read: the document was given with no directory"))?;
        let (parsed, callee_dir) =
            read_algorithm(dir, import).ok_or_else(|| what("cannot be read"))?;
        let functions = algorithm_functions(parsed, &callee_dir, &what, &mut Vec::new())?;
        lowered
            .called
            .insert(callee.document_name.clone(), functions[0].symbol.clone());
        install(&mut lowered.installed, functions);
    }
    Ok(lowered)
}

/// The document `import` names, read from `base_dir`, and the directory the
/// imports it makes are resolved from. `None` for one that cannot be read, which
/// the caller words in its own name for the document.
fn read_algorithm(base_dir: &Path, import: &ForgeImport) -> Option<(ParsedForge, PathBuf)> {
    let source = ImportSource::read(base_dir, import).ok()?;
    let parsed = source.parse().ok().flatten()?;
    let dir = source
        .path
        .parent()
        .map_or_else(|| base_dir.to_path_buf(), Path::to_path_buf);
    Some((parsed, dir))
}

/// `more` added to `into`, a symbol installed once.
fn install(into: &mut Vec<LoweredAlgorithm>, more: Vec<LoweredAlgorithm>) {
    for function in more {
        if !into.iter().any(|held| held.symbol == function.symbol) {
            into.push(function);
        }
    }
}

/// The algorithm `parsed` is, as the function it becomes, followed by the
/// functions of every algorithm it imports, and theirs. The first is the
/// document's own. `base_dir` is where its imports are resolved from, and
/// `what` words a refusal in the caller's own name for the document.
///
/// `visiting` holds the symbols being lowered above this one, so an algorithm
/// that imports itself, however far round, is refused rather than followed
/// forever (v1 forbids recursion, SCE_FORGE.md §4.12).
fn algorithm_functions(
    parsed: ParsedForge,
    base_dir: &Path,
    what: &dyn Fn(&str) -> GenerateError,
    visiting: &mut Vec<String>,
) -> Result<Vec<LoweredAlgorithm>, GenerateError> {
    let ForgeDocument::Algorithm(algorithm) = parsed.document else {
        return Err(what("is not an algorithm"));
    };
    let symbol = static_js_algorithm::symbol(&algorithm.name);
    if visiting.contains(&symbol) {
        return Err(what("imports itself, which an algorithm may not"));
    }
    visiting.push(symbol.clone());
    let imports =
        static_js_algorithm::Imports::resolve(&algorithm.name, &parsed.imports, base_dir)?;
    let mut functions = vec![LoweredAlgorithm {
        symbol,
        function: static_js_algorithm::lower(&algorithm, &imports)?,
    }];
    for import in parsed
        .imports
        .iter()
        .filter(|import| import.kind == ForgeKind::Algorithm)
    {
        let (callee, callee_dir) = read_algorithm(base_dir, import)
            .ok_or_else(|| what(&format!("imports `{}`, which cannot be read", import.alias)))?;
        install(
            &mut functions,
            algorithm_functions(callee, &callee_dir, what, visiting)?,
        );
    }
    visiting.pop();
    Ok(functions)
}

/// An algorithm document lowered on its own, for a caller that runs it in a
/// script engine and not from a statechart: the differential check that holds
/// the Interpreter to the numerical conformance cases (E11) is one.
#[derive(Debug, Clone)]
pub struct LoweredAlgorithmDocument {
    /// The name it is installed under: `SceStatic.algorithms.<symbol>`.
    pub symbol: String,
    /// The expression that installs the runtime library and the algorithm —
    /// the one a lowered statechart holds in its first `<data>`.
    pub install: String,
}

/// The algorithm document `document` names — an `sce:std/...` document, or a
/// path — lowered as a statechart's import of it would be, and refused for the
/// same reasons: an import it cannot read, or a construct with no lowering yet.
/// A refusal names the construct. The algorithms it imports are installed
/// beside it.
pub fn lower_algorithm_document(
    document: &str,
) -> Result<LoweredAlgorithmDocument, Located<ForgeError>> {
    let refuse = |error: GenerateError| Located::in_file(ForgeError::from(error), document);
    let path = crate::forge::stdlib::resolve(Path::new("."), document);
    let content = if crate::forge::stdlib::names_standard(&path) {
        crate::forge::stdlib::lookup(&path)
            .map(str::to_string)
            .ok_or_else(|| {
                refuse(GenerateError::unsupported(format!(
                    "`{document}` names no document of the standard library"
                )))
            })?
    } else {
        std::fs::read_to_string(&path).map_err(|error| {
            refuse(GenerateError::unsupported(format!(
                "`{document}` cannot be read: {error}"
            )))
        })?
    };
    let identifier = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("unknown");
    let parsed = crate::forge::parser::parse_forge_with_imports(
        &content,
        crate::DocumentLabel {
            identifier,
            diagnostic_label: document,
        },
    )?
    .ok_or_else(|| {
        refuse(GenerateError::unsupported(format!(
            "`{document}` is a statechart, not an algorithm"
        )))
    })?;
    let what =
        |reason: &str| GenerateError::unsupported(format!("`{document}`: the document {reason}"));
    let base_dir = path
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    let functions =
        algorithm_functions(parsed, &base_dir, &what, &mut Vec::new()).map_err(refuse)?;
    Ok(LoweredAlgorithmDocument {
        symbol: functions[0].symbol.clone(),
        install: runtime_expression(&functions),
    })
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
    lower_parsed(&expanded, &model, path, Path::new(path).parent())
}

/// `text`, a document read from memory, lowered for the Interpreter. Nothing
/// beside it is resolved: no include, no import.
pub fn lower_source(text: &str, name: &str) -> Result<String, Located<ForgeError>> {
    let model = crate::parser::SCXMLParser::new().parse_string(text, name)?;
    lower_parsed(text, &model, name, None)
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
        let text = runtime_expression(&[]);
        for op in EVERY_OP {
            let method = format!("{}: function", op.helper());
            if is_integer_method(op) {
                assert!(text.contains(&method), "no `{method}` in the library");
            }
        }
        for member in [
            "at: function",
            "round: function",
            "field: field",
            "out: function",
            "extend: function",
        ] {
            assert!(text.contains(member), "no `{member}` in the library");
        }
        // The bitwise operators are written as the library's, at the width of
        // the operation.
        for method in ["and", "or", "xor", "not", "shl", "shr", "ushr"] {
            let declared = format!("{method}: function");
            assert!(text.contains(&declared), "no `{declared}` in the library");
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
        let text = runtime_expression(&[]);
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

    /// A caller that compares this engine with a generated backend reads the
    /// failure's name, so each name a backend reports (SCE_FORGE.md 3.4.1) has
    /// a place in the library that throws it, and the one the Interpreter adds
    /// — an integer a Number cannot hold — is named too.
    #[test]
    fn every_failure_of_the_library_names_itself() {
        let code = without_block_comments(RUNTIME_SOURCE);
        for name in [
            "overflow",
            "divide-by-zero",
            "precondition",
            "out-of-range",
            "capacity-exceeded",
            "unrepresentable",
        ] {
            assert!(
                code.contains(&format!(", '{name}');")),
                "no `fail(…, '{name}')` in sce_static.js"
            );
        }
    }

    /// An algorithm document on its own is lowered as a statechart's import of
    /// it would be, under the name it is installed by.
    #[test]
    fn an_algorithm_document_is_lowered_on_its_own() {
        let lowered = lower_algorithm_document("sce:std/time/second_of_day.scxml").expect("lowers");
        assert_eq!(lowered.symbol, "second_of_day");
        assert!(
            lowered
                .install
                .contains("SceStatic.algorithms.second_of_day = function"),
            "{}",
            lowered.install
        );
        assert!(!lowered.install.contains('\n'));
    }

    /// An algorithm that calls another is lowered with it, and with what that
    /// one calls: every function is installed beside the one the document names,
    /// once, under its own symbol.
    #[test]
    fn the_algorithms_an_algorithm_calls_are_installed_beside_it() {
        let lowered =
            lower_algorithm_document("sce:std/time/days_from_civil.scxml").expect("lowers");
        assert_eq!(lowered.symbol, "days_from_civil");
        for symbol in ["days_from_civil", "days_in_month", "is_leap_year"] {
            let installed = format!("SceStatic.algorithms.{symbol} = function");
            assert_eq!(
                lowered.install.matches(&installed).count(),
                1,
                "`{symbol}` is installed once: {}",
                lowered.install
            );
        }
    }

    /// The reasons an import of the document is refused are the reasons the
    /// document is: what is not there is named.
    #[test]
    fn an_algorithm_document_that_is_not_there_is_refused_by_name() {
        let absent = lower_algorithm_document("sce:std/time/no_such_document.scxml")
            .expect_err("no such document");
        assert!(
            format!("{absent:?}").contains("names no document of the standard library"),
            "{absent:?}"
        );
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
            xml_attribute_value(&runtime_expression(&[]))
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
        assert_eq!(
            first.attribute("expr"),
            Some(runtime_expression(&[]).as_str())
        );
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

    /// An insertion goes in front of an edit that starts where it does — the
    /// library is placed before the first `<data>`, which may be replaced too.
    #[test]
    fn an_insertion_goes_before_an_edit_that_starts_at_the_same_place() {
        let edits = vec![
            Edit {
                range: 3..6,
                text: "[replaced]".to_string(),
            },
            Edit {
                range: 3..3,
                text: "[inserted]".to_string(),
            },
        ];
        assert_eq!(
            apply("0123456789", edits).unwrap(),
            "012[inserted][replaced]6789"
        );
    }

    const LISTS_AND_RECORDS: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static" name="shelf">
  <datamodel>
    <data id="picked" sce:type="list&lt;uint8&gt;" sce:capacity="2"/>
    <data id="n" sce:type="uint8" expr="0"/>
  </datamodel>
  <state id="s">
    <transition event="add" type="internal">
      <sce:append target="picked" expr="n + 1"/>
    </transition>
    <transition event="wipe" type="internal">
      <sce:clear target="picked"/>
    </transition>
  </state>
</scxml>"#;

    /// A list is declared by the `<data>` that holds its initial value, and the
    /// element that fills it becomes the `<assign>` that does — the list,
    /// written again with the value at its end, through the library, which
    /// refuses a full one.
    #[test]
    fn a_list_is_declared_empty_and_its_statements_become_assignments() {
        let out = lowered(LISTS_AND_RECORDS);
        assert!(
            out.contains(r#"<data id="picked" expr="[]"/>"#),
            "the list's <data> holds its initial value: {out}"
        );
        assert!(
            out.contains(
                r#"<assign location="picked" expr="SceStatic.append(picked, 2, SceStatic.U8.add(n, 1))"/>"#
            ),
            "{out}"
        );
        assert!(
            out.contains(r#"<assign location="picked" expr="[]"/>"#),
            "{out}"
        );
        assert!(
            !out.contains("<sce:append") && !out.contains("<sce:clear"),
            "{out}"
        );
        // The library is installed before the `<data>` it precedes, which was
        // itself replaced.
        let installed = out.find(RUNTIME_DATA_ID).expect("the library is installed");
        let list = out.find(r#"<data id="picked""#).expect("the list");
        assert!(installed < list, "{out}");
        roxmltree::Document::parse(&out).expect("well-formed");
    }
}
