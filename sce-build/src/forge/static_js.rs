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
//! An algorithm with a construct
//! [`crate::forge::static_js_algorithm`] does not spell (a `<sce:foreach>` over
//! a bounded-collection import), an `<invoke type="scxml">` whose `src` names a
//! Mesh peer rather than a document, a model's generation
//! (`sce-host-model-generate`, docs/adr/0087, section 7.1), a union
//! (docs/adr/0092, the row of this lowering in
//! [`crate::forge::event_schema_check::ComposedLowering::ECMASCRIPT`]), and the executable
//! content the walk does not lower are refused with
//! `generate/unsupported-feature` naming the construct, never passed through:
//! an expression left as the author wrote it would be run by the script engine
//! as ECMAScript, which is the mis-execution the refusal exists to prevent.
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
//! A child named by `src` — a file, or a statechart of the standard library
//! (`sce:std/...`) — has no place in its parent's text, and the Interpreter loads
//! it at run time from beside the invoking document. So it is lowered as a
//! document of its own and written beside the invoking one under its stem
//! ([`lower_file_set`]), and the parent's `src` is rewritten to that name where
//! it says another (`src_edits`). The file is the one the parse read the child
//! from, `<child_name>.scxml` beside the parent — the document the `<param>`s
//! were typed against and the generated backends start — and a library child is
//! read from the library, its own imports resolving inside it. Its arguments are
//! lowered as an inline child's are, held to the same rules.
//!
//! # Hybrid invokes
//!
//! A hybrid `<invoke>` names the document it starts by a value (`srcexpr`)
//! among the documents it declares (`sce:candidates`), each a `sce-static`
//! document beside the invoking one. The Interpreter loads that document at run
//! time, so every candidate is lowered as a document of its own and written
//! beside the invoking one under its stem ([`lower_file_set`]), and the
//! `srcexpr` becomes the library's `candidate` call, which reduces the value to
//! its stem by the rule every engine reads from one table and answers that
//! file's name. A value that names none throws: the attribute cannot be
//! evaluated, `error.execution` is raised and nothing starts. The `<param>`s
//! and the `namelist` go to every candidate, each keeping the names it declares;
//! the Interpreter carries one expression in an attribute, so an argument the
//! candidates take as variables of different types or bounds is refused by
//! name (`site_edits`), as is a `namelist` name that lands in a string a
//! candidate bounds.
//!
//! # Host-run invokes and Mesh requests
//!
//! The Interpreter serves an `<invoke>` a host runs, and a Mesh request
//! (`type="sce:mesh-rpc"`), through the invoker its host registered on the
//! machine, as the generated engines do (docs/adr/0084, D4, D6 and D7). Its own
//! `<invoke>` evaluates the request, so the element stays as written and only
//! its expressions are lowered: each `<param>` and each `namelist` name as a
//! `<send>`'s are, and the `srcexpr` and `<content expr>` to the string each
//! computes. The envelope of a Mesh request (`_mesh_event`, the deadline) is
//! literal and travels as written. A model's generation is the one host-run
//! type refused: the build writes its request's `answer_schema` and the
//! machine's table of answer schemas, and the lowered document carries neither.
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
//!
//! # A composed payload
//!
//! The data of an event whose schema holds a list, a record or an optional
//! value of either (docs/adr/0086) is read whole, by the library's `payload`
//! handed the descriptor of the schema's record, every bound at every depth in
//! it ([`JsTarget::reading_composed_payloads`], spelled by
//! [`crate::forge::composed_lowering::JsComposed`]), and a read at any depth is
//! that value and the fields, elements and lengths after it. A `<foreach>` over
//! a list the data or an enclosing loop's item holds walks that list, a list
//! taken whole into a list variable is the value itself, which nothing changes
//! in place, and a `<send>` carries a composed value by its name as the value
//! its `<param>` writes, a 64-bit integer as the text of its digits. Each
//! transition taken on such an event opens its content with a read of the whole
//! data ([`leading_read_edits`]), so a delivery outside its type runs none of
//! the transition's content, as on the generated backends.

use std::collections::BTreeMap;
use std::ops::Range;
use std::path::{Path, PathBuf};

use crate::attribute_spelling::AttributeSpelling;
use crate::forge::composed_lowering::JsComposed;
use crate::forge::error::{ForgeError, GenerateError, Located};
use crate::forge::expr::ExprTarget;
use crate::forge::import_source::ImportSource;
use crate::forge::model::{
    EntryField, EntryType, EnumModel, ForgeDocument, ForgeImport, ForgeKind, ParsedForge, SceType,
};
use crate::forge::static_js_algorithm;
use crate::forge::static_lowering::{
    lower, Callee, CalleeImport, LiftStatement, LoweredElement, LoweredSite, RenderCall,
    StaticTarget,
};
use crate::forge::type_ctx::{StateArg, StaticCallee, StaticScope};
use crate::forge::types::InferredType;
use crate::model::{Action, Datamodel, SCXMLModel, Variable};
use crate::model::{DoneDataContent, HybridInvokeInfo, Invoke, ScxmlInvokeInfo};

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

/// The data model the lowered document declares, and the name its lowering
/// refuses what it does not lower yet by.
pub(crate) const LOWERED_DATAMODEL: &str = "ecmascript";

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
    /// Each event whose schema holds a composed value, and the read of its
    /// whole data an expression reads a part of it through
    /// ([`Self::reading_composed_payloads`]).
    payloads: BTreeMap<String, String>,
}

impl JsTarget {
    fn new(algorithms: BTreeMap<String, String>) -> Self {
        Self {
            algorithms,
            payloads: BTreeMap::new(),
        }
    }

    /// This target, reading the data of each event of `model` whose schema
    /// holds a composed value (docs/adr/0086, section 6) through the library's
    /// `payload`, handed the descriptor of the schema's record that the walk of
    /// its type spells ([`JsComposed`]): every field read at once, each bound
    /// held at every depth, so a delivery outside its type is refused whole by
    /// the first expression that reads any of it, and none of its content that
    /// reads the data runs (docs/adr/0086, section 4.2). A read is then that
    /// value and the fields, elements and lengths after it, as a generated
    /// machine reads the payload it lifted.
    ///
    /// The Interpreter's script engine reads an event's data in the expression
    /// that names it, so the data is read where each expression reads it, and
    /// once more before anything else a transition taken on the event runs
    /// ([`leading_read_edits`]): a delivery the read refuses runs none of the
    /// transition's content, as a generated machine, which lifts the payload
    /// once when the event is dequeued, runs none of it.
    ///
    /// An event whose read the walk cannot spell (its record no import of the
    /// statechart's) is left out, so its data is `_event.data` as the engine
    /// holds it, and the walk refuses a read of it by name where it declares
    /// what the payload holds, as it refuses one on every backend.
    fn reading_composed_payloads(mut self, model: &SCXMLModel) -> Self {
        use crate::forge::composed_lowering as composed;
        let entries = model.entry_event_schemas();
        let records = composed::StatechartRecords::of(model);
        // An enum is held by the names its variants declare, which is what its
        // reader reads.
        let enums: BTreeMap<String, composed::MachineEnum> = model
            .held_enums()
            .iter()
            .filter_map(|(alias, held)| {
                Some((
                    alias.clone(),
                    composed::MachineEnum {
                        alias: alias.clone(),
                        ty: self.enum_type("", alias)?,
                        variants: held
                            .variants
                            .iter()
                            .map(|v| (v.name.clone(), v.name.clone()))
                            .collect(),
                    },
                ))
            })
            .collect();
        for (event, schema) in &model.imported_event_schemas {
            if entries.contains_key(event) {
                continue;
            }
            let Some(alias) = model
                .imported_records
                .iter()
                .find(|(_, held)| held.name == schema.name)
                .map(|(alias, _)| alias.clone())
            else {
                continue;
            };
            let read = composed::reader(
                &crate::forge::value_type::ValueType::Record { alias },
                None,
                &records,
                &enums,
                &JsComposed,
                PAYLOAD_ACCESSOR,
            );
            if let Ok(descriptor) = read {
                self.payloads.insert(
                    event.clone(),
                    format!("{RUNTIME_GLOBAL}.payload({PAYLOAD_ACCESSOR}, {descriptor})"),
                );
            }
        }
        self
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

/// The type the library's `field` holds an enum field to: the array of the
/// names its variants declare. An enum a composed value holds at any depth is
/// read by the same array ([`JsComposed`]).
pub(crate) fn variant_list(variants: &[&str]) -> String {
    let names: Vec<String> = variants.iter().map(|name| format!("'{name}'")).collect();
    format!("[{}]", names.join(", "))
}

/// The type the library's `field` holds a field whose present value is of `ty`
/// to: the names of its variants for an enum, `variants`, and the type's own
/// name otherwise. `None` for a type `field` has no reader for.
fn field_kind(ty: &SceType, variants: &[&str]) -> Option<String> {
    Some(match ty {
        SceType::Enum(_) => variant_list(variants),
        ty => format!("'{}'", InferredType::from_sce_type(ty).declared_spelling()?),
    })
}

/// The name the function handed to the library's `mapPresent` binds a present
/// value to. What the function spells reads only that name and the library,
/// so no variable of the document it would shadow is read there.
const PRESENT_NAME: &str = "scePresent";

/// `value`, an optional value, with `map` applied to its present value and
/// `null` kept (docs/adr/0055): the library's `mapPresent`, or `value` itself
/// where `map` changes nothing.
fn map_present(value: &str, map: &dyn Fn(&str) -> String) -> String {
    let mapped = map(PRESENT_NAME);
    if mapped == PRESENT_NAME {
        return value.to_string();
    }
    format!(
        "{RUNTIME_GLOBAL}.mapPresent({value}, function ({PRESENT_NAME}) {{ return {mapped}; }})"
    )
}

/// `value` without the library's `out` the expression lowering wrapped it in
/// ([`crate::forge::expr`]): `out` refuses a BigInt the data model cannot hold,
/// and a value that leaves for the wire leaves as the text of its digits, which
/// it can.
fn without_out(value: &str) -> &str {
    value
        .strip_prefix(RUNTIME_GLOBAL)
        .and_then(|rest| rest.strip_prefix(".out("))
        .and_then(|rest| rest.strip_suffix(')'))
        .unwrap_or(value)
}

/// The functions the Interpreter lowers `<alias>.toNumber` and `<alias>.fromNumber`
/// to, as `(to_number, from_number)`, for the enum whose `variants` are each a
/// declared name and the number it declares: the library's `number` and
/// `variant`, bound to the enum's table. A value of an enum is its declared name
/// here, so the two read the table one way and the other, and `variant` throws
/// `out-of-range` for a number no variant declares, which the Interpreter answers
/// as any failed operation. A number beyond what a Number holds exactly is a
/// BigInt, which a variable of the machine refuses as `unrepresentable`.
pub(crate) fn enum_number_symbols(variants: &[(String, i128)]) -> (String, String) {
    const SAFE: i128 = 9_007_199_254_740_991;
    let number = |value: i128| {
        if (-SAFE..=SAFE).contains(&value) {
            value.to_string()
        } else {
            format!("{value}n")
        }
    };
    let numbers: Vec<String> = variants
        .iter()
        .map(|(name, value)| format!("'{name}': {}", number(*value)))
        .collect();
    let names: Vec<String> = variants
        .iter()
        .map(|(name, value)| format!("'{value}': '{name}'"))
        .collect();
    (
        format!(
            "{RUNTIME_GLOBAL}.number.bind(null, {{{}}})",
            numbers.join(", ")
        ),
        format!(
            "{RUNTIME_GLOBAL}.variant.bind(null, {{{}}})",
            names.join(", ")
        ),
    )
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
        Some(format!(
            "{RUNTIME_GLOBAL}.field({accessor}, '{field}', {})",
            variant_list(variants)
        ))
    }
    // The script engine reads the payload a field at a time, so the default a
    // delivery that lacks a field takes is handed to the read of that field:
    // the library's `field` reads it where the data names no such field, and
    // holds it to the type as a delivered value, as the generated lifts do.
    fn payload_revised_read(
        &self,
        accessor: &str,
        field: &EntryField,
        variants: &[&str],
        default: &str,
    ) -> Option<String> {
        // An optional field is read by `payload_optional_read`, its default
        // included (docs/adr/0055), so the field here holds a present value.
        let kind = field_kind(field.sce_type.present_type(), variants)?;
        Some(format!(
            "{RUNTIME_GLOBAL}.field({accessor}, '{}', {kind}, {default})",
            field.id
        ))
    }
    // An optional field is read through `field` with its type written
    // `{optional: T}` (docs/adr/0055, section 4): a delivered `null` is absent, a
    // delivered value is held to `T`, and a delivery that lacks the key is
    // refused, since every field of the first revision is carried. A field a
    // later revision added is handed its default as the fourth argument, `null`
    // when it declares none, which the library reads absent where the data
    // names no such field.
    fn payload_optional_read(
        &self,
        accessor: &str,
        field: &EntryField,
        variants: &[&str],
    ) -> Option<String> {
        let kind = field_kind(field.sce_type.optional_elem()?, variants)?;
        let default = field
            .since
            .and_then(|_| crate::forge::schema_revision::default_wire(field))
            .map(|default| format!(", {default}"))
            .unwrap_or_default();
        Some(format!(
            "{RUNTIME_GLOBAL}.field({accessor}, '{}', {{optional: {kind}}}{default})",
            field.id
        ))
    }
    // An optional value is `null` where it is absent and the value itself where
    // it is present (docs/adr/0055, section 5): the data model holds no types,
    // so the type is named only where a refusal or a declaration reads it, and a
    // present value is lifted as it is.
    fn optional_type(&self, present: &str) -> Option<String> {
        Some(format!("optional<{present}>"))
    }
    fn optional_none(&self, _present: &str) -> Option<String> {
        Some("null".to_string())
    }
    fn optional_some(&self, _present: &str, value: &str) -> Option<String> {
        Some(value.to_string())
    }
    // The present value is handed to a function the library calls only where it
    // is present; what that function throws is the statement's failure.
    fn optional_map(
        &self,
        value: &str,
        _present: &str,
        map: &dyn Fn(&str) -> String,
    ) -> Option<String> {
        Some(map_present(value, map))
    }
    // `null` crosses as JSON `null`, and a present value as its own wire value
    // (a 64-bit integer as the text of its digits): the value leaves the
    // expressions as that, so the `out` that refuses a BigInt the data model
    // cannot hold is not asked, as for a present 64-bit integer.
    fn wire_optional(
        &self,
        value: &str,
        _present: &str,
        wire: &dyn Fn(&str) -> String,
    ) -> Option<String> {
        Some(map_present(without_out(value), wire))
    }
    // A template has no unit here: the library's one `render` is handed the
    // template's parts where the statement stands (docs/adr/0087, section 3.1,
    // item 7), so there is nothing to bring into scope.
    fn text_template(&self, _document: &str) -> Option<Callee> {
        Some(Callee {
            call: String::new(),
            import: CalleeImport::InScope,
        })
    }
    // The template's text as the library's parts, in order: a literal part as
    // the string it is, a trusted text and an enum's name (which a variant
    // lowers to here) as the value, an integer as its decimal digits, and an
    // untrusted text as `{fenced: value}`, which the library puts between the
    // markers with every marker removed.
    fn render_call(&self, render: &RenderCall<'_>) -> Option<String> {
        use crate::forge::model::{HoleKind, HoleTrust, TemplatePart};
        let mut parts = Vec::with_capacity(render.template.parts.len());
        for part in &render.template.parts {
            parts.push(match part {
                TemplatePart::Literal(text) => crate::filters::ecmascript_string_literal(text),
                TemplatePart::Put(id) => {
                    let at = render.template.holes.iter().position(|h| h.id == *id)?;
                    let value = &render.args.get(at)?.value;
                    match render.template.holes[at].kind {
                        HoleKind::Text {
                            trust: HoleTrust::Untrusted,
                            ..
                        } => format!("{{fenced: {value}}}"),
                        HoleKind::Integer(_) => format!("String({value})"),
                        HoleKind::Text { .. } | HoleKind::Enum(_) => value.clone(),
                    }
                }
            });
        }
        let (open, close) = render.template.fence.as_ref().map_or_else(
            || ("''".to_string(), "''".to_string()),
            |f| {
                (
                    crate::filters::ecmascript_string_literal(&f.open),
                    crate::filters::ecmascript_string_literal(&f.close),
                )
            },
        );
        Some(format!(
            "{RUNTIME_GLOBAL}.render([{}], {open}, {close})",
            parts.join(", ")
        ))
    }
    // The element that replaces the `<sce:render>`: an `<assign>` of the
    // variable to the rendering, held to its bound as any string is. A hole that
    // throws is answered as any assignment that fails is: nothing is written,
    // `error.execution` is raised, and the block ends (§scxml-4.9).
    fn render_element(&self, location: &str, value: &str) -> Option<String> {
        Some(self.assign_element(location, value))
    }
    // Only an algorithm is lowered to a function the document carries; a call of
    // another kind is refused by name before the walk ([`lowered_algorithms`]).
    fn callee(&self, callee: &StaticCallee) -> Option<Callee> {
        if !callee.kind.is_algorithm() {
            return None;
        }
        let symbol = self.algorithms.get(&callee.document_name)?;
        Some(Callee {
            call: format!("{RUNTIME_GLOBAL}.algorithms.{symbol}"),
            // The function travels in the document, so there is nothing to
            // import.
            import: CalleeImport::InScope,
        })
    }
    // The Interpreter's records are plain objects of their schema's fields and its
    // lists arrays, which is what the algorithm lowered to ecmascript takes and
    // returns: a call hands it the value the datamodel holds, and needs no adapter.
    // An algorithm only reads what it is handed, so nothing of the machine's state is
    // reached except through what the call returns.
    fn lowers_state_calls(&self) -> bool {
        true
    }
    fn has_state_adapters(&self) -> bool {
        false
    }
    fn state_argument(&self, _machine: &str, _arg: &StateArg, held: &str) -> String {
        held.to_string()
    }
    fn state_adapter_name(&self, _machine: &str, callee: &StaticCallee, _coded: &str) -> String {
        self.callee(callee)
            .expect("an algorithm the document imports travels in it")
            .call
    }
    fn state_list_within(&self, _adapter: &str, value: &str, capacity: u32) -> String {
        format!("{RUNTIME_GLOBAL}.within({value}, {capacity})")
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
            // backends carry as pairs, or one value (a `<send>`'s), which the
            // engine reads as the data it is; a `<content>` that holds an element has
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
    // A composed value is read by the descriptor of its type, copied as it is
    // and written for a `<send>` as the value its `<param>` carries
    // (docs/adr/0086, [`JsComposed`]): one spelling serves both, since the
    // Interpreter has no typed seam a host writes a value into.
    fn composed_spelling<'a>(
        &self,
        _machine: &'a str,
    ) -> Option<Box<dyn crate::forge::composed_lowering::ComposedSpelling + 'a>> {
        Some(Box::new(JsComposed))
    }
    fn composed_send_spelling<'a>(
        &self,
        _machine: &'a str,
    ) -> Option<Box<dyn crate::forge::composed_lowering::ComposedSpelling + 'a>> {
        Some(Box::new(JsComposed))
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
        _schema: &crate::forge::model::EntrySchema,
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
    // The library's two readings of the enum's table ([`enum_number_symbols`]):
    // nothing is declared, since a value is its declared name.
    fn enum_number_symbols(&self, _ty: &str, model: &EnumModel) -> (String, String) {
        let variants: Vec<(String, i128)> = model
            .variants
            .iter()
            .map(|v| (v.name.clone(), v.value))
            .collect();
        enum_number_symbols(&variants)
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
    // A byte string is the text of its bytes, one character to a byte, so the
    // library counts its length; it throws past the bound as it does for a string.
    // A record's byte-string field is the same text in the object that holds it,
    // which `field_assignment` writes again with the field changed, held to the
    // bound its schema declares as a variable's is. A payload is the engine's
    // `_event.data`, read as the JSON it arrived as, and a byte-string field of it
    // is the text of its bytes: one character to a byte, as the wire spells it. It is
    // held to the bound of the place it is written to by the library call every byte
    // string takes, which refuses a character past U+00FF.
    fn bounded_bytes(&self, value: &str, capacity: u32) -> String {
        format!("{RUNTIME_GLOBAL}.boundedBytes({value}, {capacity})")
    }
    // The list an algorithm returned is the array the variable holds, which the
    // library holds to the variable's bound and throws past it, as it does a string.
    fn list_within(
        &self,
        value: &str,
        capacity: u32,
        _symbol: &str,
        _elem: &crate::forge::model::SceType,
    ) -> String {
        format!("{RUNTIME_GLOBAL}.within({value}, {capacity})")
    }
    fn assign(&self, _target: &str, _value: &str) -> String {
        String::new()
    }
    // The Interpreter's own `<send>` generates the id and writes it to the
    // location the attribute names, which is kept as it is written: there is no
    // statement of the lowering's to emit.
    fn fresh_send_id(&self) -> String {
        String::new()
    }
    fn assign_field(&self, _target: &str, _field: &str, _value: &str) -> String {
        String::new()
    }
    // A field of the present value of an optional record is the object the variable
    // holds, written again with that field changed (`field_assignment`) where a test
    // proved it present: like any field, there is no statement of the lowering's to
    // emit (docs/adr/0055, step 9).
    fn assign_present_field(&self, _target: &str, _field: &str, _value: &str) -> Option<String> {
        Some(String::new())
    }
    // A list is the array its variable holds, and a record in it is the plain object it
    // is everywhere else, which a `set` writes again rather than changes: the element
    // the library reads — it throws for an index outside the array — is the value.
    fn record_at(&self, list: &str, index: &str) -> Option<String> {
        Some(format!("{RUNTIME_GLOBAL}.at({list}, {index})"))
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
    // The element that replaces the `<sce:lift>`: a `<script>` that hands the text
    // to the library's `lift` with the type the record is read as, written as the
    // library's `field` takes a type (`{record: …, defaults: …}`), and writes the
    // record or the refusal into the two variables. The library answers rather
    // than throws, so the script raises nothing, as the generated backends' lift
    // does not: a 64-bit integer past what the data model holds exactly is
    // refused as past its width here (`outOfRange`), where a backend whose int64
    // holds it reads it.
    fn lift(&self, lift: &LiftStatement<'_>) -> Option<String> {
        let kind = |ty: &SceType, max: Option<u32>| -> Option<String> {
            Some(match ty {
                SceType::String => format!("{{string: {}}}", max?),
                SceType::Bytes => format!("{{bytes: {}}}", max?),
                SceType::Enum(reference) => {
                    let model = lift.enums.get(&reference.alias)?;
                    let names: Vec<&str> = model.variants.iter().map(|v| v.name.as_str()).collect();
                    variant_list(&names)
                }
                other => field_kind(other, &[])?,
            })
        };
        let schema = lift.record()?;
        let mut fields = Vec::with_capacity(schema.fields.len());
        for field in &schema.fields {
            let declared = match &field.sce_type {
                EntryType::Scalar(ty) => kind(ty, field.max_size)?,
                EntryType::Optional { elem } => {
                    format!("{{optional: {}}}", kind(elem, field.max_size)?)
                }
            };
            fields.push(format!("'{}': {declared}", field.id));
        }
        // The defaults are a JSON object, which is an object literal here.
        let defaults = lift
            .defaults
            .as_ref()
            .map(|json| format!(", defaults: {json}"))
            .unwrap_or_default();
        let path_bound = lift.path_bound()?;
        let refusal = self.record_value(
            "",
            &lift.refusal_members(
                self,
                "sceLift.refusal.code",
                "sceLift.refusal.pointer",
                "sceLift.refusal.bound",
            )?,
        );
        let script = format!(
            "(function (sceLift) {{ if (sceLift.refusal === null) {{ {location} = sceLift.value; \
             {absent} = null; }} else {{ {absent} = {refusal}; }} }})({RUNTIME_GLOBAL}.lift({text}, \
             {{record: {{{fields}}}{defaults}}}, {depth}, {path_bound}));",
            location = lift.location,
            absent = lift.refusal,
            text = lift.text,
            fields = fields.join(", "),
            depth = lift.depth,
        );
        Some(format!("<script>{}</script>", xml_attribute_value(&script)))
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
    // The engine's own `_event.data`, a field of which the expression lowering
    // reads through the library's `field`; the data of an event whose schema
    // holds a composed value is read whole instead, and a part of it is read
    // from that value ([`JsTarget::reading_composed_payloads`]).
    fn payload_accessor(&self, event: &str) -> String {
        self.payloads
            .get(event)
            .cloned()
            .unwrap_or_else(|| PAYLOAD_ACCESSOR.to_string())
    }
    // A delivery that carried no payload makes the read throw, so the guard
    // is false and says so: there is no channel to check before it.
    fn payload_guard(&self, _machine: &str, _event: &str, lowered: &str) -> String {
        lowered.to_string()
    }
    // The Interpreter's own data model reads the lowered expression where the
    // document wrote it, so a `<param>` has no typed value to build. A 64-bit
    // integer is a `BigInt` there, which the engine's data cannot carry as a
    // number: it crosses as the text of its digits.
    fn wire_value(&self, ty: InferredType, value: &str) -> String {
        if ty.is_wide() {
            format!("String({})", without_out(value))
        } else {
            value.to_string()
        }
    }
    // A byte string is already the text of its bytes there, one character to a
    // byte, which is what crosses.
    fn wire_bytes(&self, value: &str) -> Option<String> {
        Some(value.to_string())
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
    // Nor does it need the name of the event it delivers to be known before it
    // runs: its own `<send>` evaluates the `eventexpr` and delivers that name.
    fn lowers_event_expr(&self) -> bool {
        true
    }
    // Its own `<cancel>` evaluates the `sendidexpr` and removes the send that
    // holds the id it computes.
    fn lowers_cancel_expr(&self) -> bool {
        true
    }
    // Its own `<send>` evaluates the `<content expr>` and carries the value as
    // the event's data.
    fn lowers_scalar_content(&self) -> bool {
        true
    }
    // Its own `<invoke>` evaluates the `srcexpr` and loads the document it
    // names, so the attribute becomes the call that reduces the value to its
    // stem and answers the lowered candidate that stem names, written beside the
    // invoking document ([`lower_file_set`]). A value naming no declared document
    // throws: the attribute cannot be evaluated, `error.execution` is raised and
    // nothing starts, as on the generated backends.
    fn lowers_hybrid_invoke(&self) -> bool {
        true
    }
    // Its own `<invoke>` evaluates the `srcexpr` and the `<content expr>` of an
    // invocation a host runs and hands the host what each computes, so each is
    // lowered in the attribute it is written in, to the string it computes
    // (docs/adr/0084, D7). One that cannot be evaluated starts nothing, as on the
    // generated backends.
    fn lowers_host_src_expr(&self) -> bool {
        true
    }
    // Its own `<send>` evaluates the `targetexpr` and routes the value as it
    // routes a written target. The attribute becomes the call that holds the value
    // to the routes the document declares as `sce:targets`: a value that is none
    // of them is the empty target, which the engine answers as it answers any
    // address nobody is at — `error.communication`, nothing sent
    // (docs/adr/0005, decision 3).
    fn target_expr_site(&self, native_target: &str, entries: &[String]) -> Option<String> {
        let names: Vec<String> = entries
            .iter()
            .map(|entry| serde_json::to_string(entry).expect("a string is JSON"))
            .collect();
        Some(format!(
            "{RUNTIME_GLOBAL}.route({native_target}, [{}])",
            names.join(", ")
        ))
    }
    // Its own `<send>` evaluates the `typeexpr` and delivers by the processor the
    // value names, so the attribute becomes the call that holds the value to the
    // types the document declares as `sce:types`: one that is none of them throws,
    // the attribute cannot be evaluated, and `error.execution` is raised with
    // nothing sent (docs/adr/0005, decision 3).
    fn type_expr_site(&self, native_type: &str, entries: &[String]) -> Option<String> {
        let names: Vec<String> = entries
            .iter()
            .map(|entry| serde_json::to_string(entry).expect("a string is JSON"))
            .collect();
        Some(format!(
            "{RUNTIME_GLOBAL}.processor({native_type}, [{}])",
            names.join(", ")
        ))
    }
    fn hybrid_src_site(&self, native_src: &str, stems: &[&str]) -> Option<String> {
        let names: Vec<String> = stems
            .iter()
            .map(|stem| serde_json::to_string(stem).expect("a string is JSON"))
            .collect();
        Some(format!(
            "{RUNTIME_GLOBAL}.candidate({native_src}, [{}])",
            names.join(", ")
        ))
    }
}

/// What of an `<invoke>` has no lowering for the Interpreter, described for a
/// refusal. A child session is lowered when it is written inline — its own
/// document is lowered where it stands ([`child_edits`]) — or named by `src` —
/// its document is lowered as one of its own and written beside the invoking one
/// ([`lower_file_set`]) — and a hybrid one is lowered with its candidates.
///
/// An invocation a host runs and a Mesh request are lowered by the walk, their
/// `<param>`s and `namelist` as a `<send>`'s are: the Interpreter serves each
/// through the invoker its host registered, a Mesh router included, and
/// raises `error.execution` for a type nothing serves (docs/adr/0084, D4, D6
/// and D7). Their requests hand no child a variable it bounds, so the bounded
/// `namelist` refusal below is a child session's alone.
///
/// The one host-run type refused is a model's generation
/// (`sce-host-model-generate`, docs/adr/0087, section 7.1): its request's
/// `answer_schema` is written by the build from `sce:answer`, and the schema it
/// names is resolved in the machine's table of answer schemas, and the lowered
/// document carries neither, so its host would be asked for a text under no
/// schema the machine names.
///
/// An allowlist, as [`unsupported_action`] is.
fn unsupported_invoke(invoke: &Invoke, state: &str) -> Option<String> {
    let info = match invoke {
        Invoke::Scxml(info) => info,
        Invoke::Hybrid(info) => return unsupported_hybrid_invoke(info, state),
        Invoke::Unsupported(info)
            if info.invoke_type == crate::forge::model_answer::MODEL_GENERATE =>
        {
            return Some(format!(
                "a model's generation (the <invoke type=\"{}\"> in state `{state}`, whose \
                 answer schema the build writes from the machine's table of answer schemas)",
                info.invoke_type
            ));
        }
        Invoke::MeshRpc(_) | Invoke::Unsupported(_) => return None,
    };
    // A `src` that names no document — a Mesh peer (`#<name>`) — has nothing to
    // be lowered and written beside the invoking document.
    if info.inline_child_range.is_none() && src_child(info).is_none() {
        return Some(format!(
            "the <invoke> in state `{state}` whose `src` (`{}`) names no document to \
             lower beside it",
            info.src
        ));
    }
    let declared = info.common.child_static_variables.as_deref().unwrap_or(&[]);
    bounded_namelist_name(&info.namelist, declared).map(|name| {
        format!(
            "the `namelist` name `{name}` of the <invoke> in state `{state}`, which hands \
             a child a string it bounds"
        )
    })
}

/// What of a hybrid `<invoke>` has no lowering for the Interpreter. The
/// candidates it may start are documents of their own, lowered beside the
/// invoking one ([`lower_file_set`]), so what is left is what the `<invoke>`
/// hands them: a `namelist` name is carried by the Interpreter as it holds the
/// value, with no expression to carry a bound, so one that lands in a string
/// any candidate bounds is refused.
fn unsupported_hybrid_invoke(info: &HybridInvokeInfo, state: &str) -> Option<String> {
    info.candidates.iter().find_map(|candidate| {
        let declared = candidate.child_static_variables.as_deref().unwrap_or(&[]);
        bounded_namelist_name(&info.namelist, declared).map(|name| {
            format!(
                "the `namelist` name `{name}` of the hybrid <invoke> in state `{state}`, which \
                 hands the candidate `{}` a string it bounds",
                candidate.stem
            )
        })
    })
}

/// The first name of `namelist` that lands in a string `declared` bounds, or in
/// an optional string it bounds (docs/adr/0055, step 10). A `namelist` name is
/// handed to a child by the Interpreter as it holds it, with no expression to
/// carry the bound the child declared for a string.
fn bounded_namelist_name<'a>(namelist: &'a str, declared: &[Variable]) -> Option<&'a str> {
    namelist.split_whitespace().find(|name| {
        declared.iter().find(|v| v.id == *name).is_some_and(|v| {
            let held = v.value_type.as_ref().and_then(|t| {
                t.scalar()
                    .or_else(|| t.optional_elem().and_then(|elem| elem.scalar()))
            });
            v.capacity().is_some() && matches!(held, Some(SceType::String))
        })
    })
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
        // the generated backends carry as pairs; or one value, which it reads as
        // the event's data, as the generated backends carry it whole.
        "send" => None,
        "sce_append" | "sce_clear" => None,
        // A lift is lowered to a `<script>` that calls the library's reader of a
        // text (docs/adr/0087, section 2.1).
        "sce_lift" => None,
        // A render is replaced by an `<assign>` of the library's `render`, handed
        // the template's parts (docs/adr/0087, section 3.1, item 7):
        // `JsTarget::render_call` and `JsTarget::render_element`.
        "sce_render" => None,
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
    let mut edits: Vec<Edit> = Vec::new();
    // The places already lowered, and to what. A hybrid `<invoke>` hands every
    // candidate the arguments it writes once, so the walk lowers each of them
    // once per candidate: the same place, and the same text when the candidates
    // take the value alike. The Interpreter carries one expression in an
    // attribute, so a place the walk lowered two ways has no lowering here.
    let mut lowered: Vec<(Range<usize>, &str)> = Vec::new();
    for site in sites {
        let place = site
            .spelling
            .as_ref()
            .and_then(|s| s.value())
            .map(|written| written.range());
        if let Some(range) = &place {
            if let Some((_, text)) = lowered.iter().find(|(held, _)| held == range) {
                if *text == site.text {
                    continue;
                }
                return Err(GenerateError::unsupported(format!(
                    "`{}` is written once and lowered two ways: the candidates of a hybrid \
                     <invoke> take the value as variables of different types or bounds, and \
                     the Interpreter carries one expression in an attribute",
                    site.source.trim()
                )));
            }
            lowered.push((range.clone(), site.text.as_str()));
        }
        if site.text.trim() == site.source.trim() {
            continue;
        }
        let Some(range) = place else {
            return Err(GenerateError::unsupported(format!(
                "`{}` has no place in the document to be rewritten at: the model holds it \
                 as something the document does not spell",
                site.source.trim()
            )));
        };
        edits.push(Edit {
            range,
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

/// The offset just past the `>` that closes the start tag opened at `start` in
/// `text`, a `>` inside a quoted attribute value passed over; `None` for a tag
/// that does not close.
fn start_tag_end(text: &str, start: usize) -> Option<usize> {
    let mut quote: Option<char> = None;
    for (offset, c) in text.get(start..)?.char_indices() {
        match (quote, c) {
            (Some(open), c) if c == open => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(c),
            (None, '>') => return Some(start + offset + 1),
            (None, _) => {}
        }
    }
    None
}

/// The read that opens the content of each transition taken on an event whose
/// composed payload the machine reads (docs/adr/0086, section 4.2): a
/// `<script>` that reads the whole data ([`JsTarget::reading_composed_payloads`])
/// and keeps nothing of it, inserted before the transition's first element. A
/// delivery outside its type makes it throw, which the Interpreter answers as
/// it does any executable content that fails: `error.execution` is raised and
/// the block ends (§scxml-4.9), so none of the transition's content runs, as a
/// generated machine, which lifts the payload when the event is dequeued, runs
/// none of it. A delivery of its type passes, and every expression after it
/// reads the data as it would have.
///
/// An event is read where `lowered`, the model the walk lowered, has a
/// transition on it whose condition or content reads `_event.data`, which is
/// where a generated machine lifts its payload; a machine that only takes the
/// event reads nothing of it and refuses nothing. A transition with no content
/// has no block to end; one inside an inline child is the child's, lowered with
/// it ([`child_edits`]).
fn leading_read_edits(
    text: &str,
    lowered: &SCXMLModel,
    target: &JsTarget,
) -> Result<Vec<Edit>, GenerateError> {
    let received = lowered.received_events();
    let read: std::collections::BTreeSet<String> = lowered
        .states
        .values()
        .flat_map(|state| &state.transitions)
        .filter(|t| {
            t.content_reads_payload || crate::forge::expr::references_event_data_lexically(&t.cond)
        })
        .map(|t| t.payload_event(&received).to_string())
        .filter(|event| target.payloads.contains_key(event))
        .collect();
    if read.is_empty() {
        return Ok(Vec::new());
    }
    let document = roxmltree::Document::parse(text).map_err(|e| {
        GenerateError::unsupported(format!(
            "the document cannot be read again to be rewritten: {e}"
        ))
    })?;
    let mut edits = Vec::new();
    for node in document
        .descendants()
        .filter(|node| is_scxml_element(node, "transition"))
    {
        let in_a_child = node
            .ancestors()
            .filter(|ancestor| is_scxml_element(ancestor, "scxml"))
            .count()
            > 1;
        let Some(payload) = node
            .attribute("event")
            .map(str::trim)
            .filter(|event| read.contains(*event))
            .and_then(|event| target.payloads.get(event))
        else {
            continue;
        };
        if in_a_child || !node.children().any(|child| child.is_element()) {
            continue;
        }
        let at = start_tag_end(text, node.range().start).ok_or_else(|| {
            GenerateError::unsupported(format!(
                "the <transition event=\"{}\"> has a start tag that does not close, so there is \
                 no place to read its data before its content",
                node.attribute("event").unwrap_or_default()
            ))
        })?;
        edits.push(Edit {
            range: at..at,
            text: format!(
                "<script>{}</script>",
                xml_attribute_value(&format!("{payload};"))
            ),
        });
    }
    Ok(edits)
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
    // A composed value this lowering does not hold yet is refused by name, by
    // the one place that decides it, before this lowering reads the model, as
    // every other backend's is (docs/adr/0086, section 7.3).
    crate::forge::event_schema_check::check_composed_uses(
        model,
        label,
        crate::forge::event_schema_check::ComposedLowering::ECMASCRIPT,
    )?;
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
    let target = JsTarget::new(called).reading_composed_payloads(model);
    let mut lowered = model.clone();
    let machine = crate::filters::to_pascal_case(model.name.clone());
    let lowering = lower(&mut lowered, &machine, &target).map_err(refuse)?;
    let mut edits = site_edits(&lowering.sites).map_err(refuse)?;
    edits.extend(element_edits(text, &lowering.elements).map_err(refuse)?);
    edits.extend(leading_read_edits(text, &lowered, &target).map_err(refuse)?);
    let library_call = format!("{RUNTIME_GLOBAL}.");
    let needs_library = lowering
        .sites
        .iter()
        .map(|site| site.text.as_str())
        .chain(lowering.elements.iter().map(|e| e.text.as_str()))
        .any(|text| text.contains(&library_call));
    edits.extend(document_edits(text, needs_library, &installed).map_err(refuse)?);
    edits.extend(child_edits(model, label, base_dir)?);
    edits.extend(src_edits(model).map_err(refuse)?);
    apply(text, edits).map_err(refuse)
}

/// The edits that make each `<invoke>` of `model` that names its child by `src`
/// name it by the file the child is written as in the set ([`lower_file_set`]):
/// `<stem>.scxml`, beside the invoking document. A `src` that says that already
/// is left as the author wrote it; any other — a path into another directory, a
/// `file:` one, a library name (`sce:std/...`) — is rewritten, so the lowered
/// parent starts the lowered child, and not the document the author named,
/// which the Interpreter would run untyped or not find.
///
/// The stem is kept, so the host that answers for the child
/// (`INativeActionHost::hostForChild`) is asked by the name it was asked by.
fn src_edits(model: &SCXMLModel) -> Result<Vec<Edit>, GenerateError> {
    let mut edits = Vec::new();
    for state in model.states.values() {
        for invoke in &state.invokes {
            let Invoke::Scxml(info) = invoke else {
                continue;
            };
            let Some(child) = src_child(info) else {
                continue;
            };
            let file_name = child.file_name();
            if child.written == file_name {
                continue;
            }
            let Some(written) = child.spelling.value() else {
                return Err(GenerateError::unsupported(format!(
                    "the `src` of the <invoke> in state `{}` has no place in the document to be \
                     rewritten at",
                    state.id
                )));
            };
            edits.push(Edit {
                range: written.range(),
                text: xml_attribute_value(&file_name),
            });
        }
    }
    Ok(edits)
}

/// The child an `<invoke>` names by `src`, as the set it is lowered in holds it
/// ([`lower_file_set`]).
struct SrcChild<'a> {
    /// Where the `src` was written, which [`src_edits`] rewrites.
    spelling: &'a AttributeSpelling,
    /// The `src` as the author wrote it, decoded.
    written: String,
    /// Where the child is read from, against the directory of the document that
    /// invokes it: a library name, read from the library, or the file the parse
    /// read the child from ([`ScxmlInvokeInfo::child_file_name`]) — the document
    /// the invoke's arguments were typed against and the generated backends start.
    source: String,
    /// The stem the child is written under, as `<stem>.scxml`: a file's
    /// `child_name`, or the library document's own stem.
    stem: String,
}

impl SrcChild<'_> {
    /// The name the child is written under in the set, and that the lowered
    /// `<invoke>` names it by.
    fn file_name(&self) -> String {
        format!("{}.scxml", self.stem)
    }
}

/// The child `info` names by `src`, or `None` for one that stands in the text of
/// the document that invokes it (written in `<content>`, lowered where it stands
/// by [`child_edits`]) and for a `src` that names no document — a Mesh peer
/// (`#<name>`), which [`unsupported_invoke`] refuses.
fn src_child(info: &ScxmlInvokeInfo) -> Option<SrcChild<'_>> {
    if info.inline_child_range.is_some() {
        return None;
    }
    let spelling = info.src_spelling.as_ref()?;
    let written = spelling.decoded()?.trim().to_string();
    if written.starts_with('#') {
        return None;
    }
    let (source, stem) = if crate::forge::stdlib::names_standard(Path::new(&written)) {
        let stem = Path::new(&written).file_stem()?.to_str()?.to_string();
        (written.clone(), stem)
    } else {
        (info.child_file_name()?, info.common.child_name.clone())
    };
    Some(SrcChild {
        spelling,
        written,
        source,
        stem,
    })
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
        // A transform, condition, lookup, interpolation or validator a statechart
        // calls (docs/adr/0048) has no function in the Interpreter's library yet,
        // and is refused by its name rather than read as an algorithm.
        if !callee.kind.is_algorithm() {
            return Err(GenerateError::unsupported(format!(
                "`{}(…)`: a call of an imported {} has no ecmascript lowering yet",
                callee.alias,
                callee.kind.forge_kind()
            )));
        }
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
///
/// A document with an `<invoke>` that starts a document of its own — a child
/// named by `src`, or a candidate of a hybrid one — is refused: the lowered
/// children are other files, which [`lower_file_set`] returns.
pub fn lower_file(path: &str, include_dirs: Vec<PathBuf>) -> Result<String, Located<ForgeError>> {
    let mut set = lower_file_set(path, include_dirs)?;
    if set.len() > 1 {
        return Err(Located::in_file(
            ForgeError::from(GenerateError::unsupported(format!(
                "an <invoke> starts a document of its own (a child named by `src`, or a \
                 candidate of a hybrid <invoke>), so the lowering is {} documents that stand \
                 in one directory; ask for the set (`--out-dir`)",
                set.len()
            ))),
            path,
        ));
    }
    Ok(set.remove(0).text)
}

/// A document lowered for the Interpreter, under the name it is read by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoweredDocument {
    /// The file name the document is written as, in the directory the set it is
    /// part of stands in.
    pub file_name: String,
    /// The lowered document.
    pub text: String,
}

/// The document `path` names, lowered as [`lower_file`] lowers it, with every
/// document it may start that does not stand in its text (`Started`) — each
/// lowered the same way, and so the ones those start. The first is the
/// document asked for, under the name it has; `path` may name a statechart of
/// the standard library (`sce:std/...`), which is read from the library.
///
/// The Interpreter loads a child of its own document at run time, from beside
/// the invoking one, so each is written under `<stem>.scxml` and the directory
/// the set stands in together is what a run needs. A hybrid `<invoke>`'s lowered
/// `srcexpr` answers that name for the candidate the value names
/// ([`JsTarget::hybrid_src_site`]), and the `src` of a child named by it is
/// rewritten to that name where it says another (`src_edits`). A document
/// reached twice, by two invokes or two documents of the set, is lowered once.
///
/// Refused by name: two documents, however far apart in the set, with one stem
/// — the name a lowered one is written under, and what it is started by.
pub fn lower_file_set(
    path: &str,
    include_dirs: Vec<PathBuf>,
) -> Result<Vec<LoweredDocument>, Located<ForgeError>> {
    let root = PathBuf::from(path);
    let root_name = root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path)
        .to_string();
    let mut claimed: BTreeMap<String, PathBuf> = BTreeMap::new();
    claimed.insert(root_name.clone(), root.clone());
    let mut pending = std::collections::VecDeque::from([(root, root_name)]);
    let mut documents = Vec::new();
    while let Some((source, file_name)) = pending.pop_front() {
        let label = source.to_string_lossy().into_owned();
        let (text, started) = lower_one(&label, include_dirs.clone())?;
        documents.push(LoweredDocument { file_name, text });
        for Started { source: next, stem } in started {
            let name = format!("{stem}.scxml");
            match claimed.get(&name) {
                Some(held) if same_document(held, &next) => {}
                Some(held) => {
                    return Err(Located::in_file(
                        ForgeError::from(GenerateError::unsupported(format!(
                            "`{}` and `{}` have the one stem `{stem}`: a document a lowered one \
                             starts — a child named by `src`, or a candidate of a hybrid \
                             <invoke> — is written under its stem and started by that name, \
                             so the two cannot stand in one directory",
                            held.display(),
                            next.display(),
                        ))),
                        &label,
                    ));
                }
                None => {
                    claimed.insert(name.clone(), next.clone());
                    pending.push_back((next, name));
                }
            }
        }
    }
    Ok(documents)
}

/// A document a lowered one may start that does not stand in its text: a child
/// named by `src` ([`src_child`]), or a candidate of a hybrid `<invoke>`
/// (`sce:candidates`).
struct Started {
    /// Where it is read from: a file, or a library name (`sce:std/...`).
    source: PathBuf,
    /// The stem it is written under, as `<stem>.scxml`.
    stem: String,
}

/// The document `path` names, lowered, and every document it may start that
/// does not stand in its text ([`started_documents`]).
fn lower_one(
    path: &str,
    include_dirs: Vec<PathBuf>,
) -> Result<(String, Vec<Started>), Located<ForgeError>> {
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
    let text = lower_parsed(&expanded, &model, path, Path::new(path).parent())?;
    let mut started = Vec::new();
    let dir = Path::new(path).parent().unwrap_or_else(|| Path::new("."));
    started_documents(&model, dir, &mut started);
    Ok((text, started))
}

/// Every document an `<invoke>` of `model` may start that does not stand in its
/// text — a child named by `src` and each candidate of a hybrid `<invoke>` —
/// in a child session written inline in it too, which is read from the same
/// directory. `base_dir` is the directory of the document `model` was read
/// from; a library name resolves inside the library, wherever that is
/// (`crate::forge::stdlib::resolve`). A document under another data model is
/// not lowered, so what its invokes start is not either.
fn started_documents(model: &SCXMLModel, base_dir: &Path, into: &mut Vec<Started>) {
    if model.datamodel != Datamodel::SceStatic {
        return;
    }
    for state in model.states.values() {
        for invoke in &state.invokes {
            match invoke {
                Invoke::Hybrid(info) => {
                    into.extend(info.candidates.iter().map(|candidate| Started {
                        source: base_dir.join(&candidate.path),
                        stem: candidate.stem.clone(),
                    }));
                }
                Invoke::Scxml(info) => {
                    match (info.inline_child.as_deref(), &info.inline_child_range) {
                        (Some(child), Some(_)) => started_documents(child, base_dir, into),
                        _ => into.extend(src_child(info).map(|child| Started {
                            source: crate::forge::stdlib::resolve(base_dir, &child.source),
                            stem: child.stem,
                        })),
                    }
                }
                _ => {}
            }
        }
    }
}

/// Whether `a` and `b` name one document: the same path, one the file system
/// resolves to the same place, or one statechart of the standard library
/// however its name is spelled (`crate::forge::stdlib::identity`).
fn same_document(a: &Path, b: &Path) -> bool {
    a == b || crate::forge::stdlib::identity(a) == crate::forge::stdlib::identity(b)
}

/// `text`, a document read from memory, lowered for the Interpreter. Nothing
/// beside it is resolved: no include, no import, and no document an `<invoke>`
/// of it starts, so one that starts a document of its own is refused by name —
/// [`lower_documents`] lowers it with the documents it starts.
pub fn lower_source(text: &str, name: &str) -> Result<String, Located<ForgeError>> {
    let model = crate::parser::SCXMLParser::new().parse_string(text, name)?;
    let mut started = Vec::new();
    started_documents(&model, Path::new("."), &mut started);
    if let Some(Started { stem, .. }) = started.first() {
        return Err(Located::in_file(
            ForgeError::from(GenerateError::unsupported(format!(
                "an <invoke> starts the document `{stem}` of its own, which a document lowered \
                 from memory alone has nothing beside it to lower from; lower the documents it \
                 starts with it"
            ))),
            name,
        ));
    }
    lower_parsed(text, &model, name, None)
}

/// The directory a set of documents held in memory stands in for the lowering
/// that reads them as files. It is a name and no place: nothing is opened there.
const MEMORY_DIR: &str = "/sce-memory";

/// The document `root` names, lowered as [`lower_file_set`] lowers a file, from
/// `documents` held in memory: each is named by its path beside the others, and
/// `root` is one of them. Everything the lowering reads beside the document — an
/// imported algorithm, an include, a child named by `src`, a candidate of a
/// hybrid `<invoke>` — is read
/// from the set, so a caller with no file system (the codegen WASM) gets what a
/// caller with one does. A name the set lacks is a file that is not there.
///
/// The standard library (`sce:std/...`) is built in and needs no entry.
pub fn lower_documents(
    root: &str,
    documents: &BTreeMap<String, String>,
) -> Result<Vec<LoweredDocument>, Located<ForgeError>> {
    let base = Path::new(MEMORY_DIR);
    let held = documents
        .iter()
        .map(|(name, text)| (base.join(name), text.clone()));
    crate::confine::with_documents(held, || {
        lower_file_set(&base.join(root).to_string_lossy(), Vec::new())
    })
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
            // The whole data of an event whose schema holds a composed value
            // (`JsTarget::reading_composed_payloads`).
            "payload: payload",
            "out: function",
            "extend: function",
            "candidate: function",
            // An optional value (docs/adr/0055): `x ?? d`, and a present value
            // held to a bound or written for the wire, absence kept.
            "otherwise: function",
            "mapPresent: function",
        ] {
            assert!(text.contains(member), "no `{member}` in the library");
        }
        // The bitwise operators are written as the library's, at the width of
        // the operation.
        for method in ["and", "or", "xor", "not", "shl", "shr", "ushr"] {
            let declared = format!("{method}: function");
            assert!(text.contains(&declared), "no `{declared}` in the library");
        }
        // A composed type is written as the descriptor docs/adr/0086 section
        // 4.3 names, an object whose one key is its constructor: `field` knows
        // each, among them the union that reads a tag and then its case's record,
        // and reads the part of the type each names. The emitter writes
        // an optional payload field's type `{optional: T}`
        // (`payload_optional_read`), read as absent for a delivered `null` and
        // as `T` at the same path otherwise.
        assert!(
            text.contains("var keys = ['optional', 'list', 'record', 'union', 'string', 'bytes'];"),
            "`field` knows no constructor of a type descriptor"
        );
        assert!(
            text.contains(
                "return value === null ? null : readValue(value, type.optional, path, pointer);"
            ),
            "`field` reads no type written {{optional: T}}"
        );
        for part in [
            "type.list",
            "type.max",
            // A list's lower count (docs/adr/0086, section 15).
            "type.min",
            "type.record",
            "type.defaults",
            "type.string",
            "type.bytes",
        ] {
            assert!(
                text.contains(part),
                "`field` reads no `{part}` of a type descriptor"
            );
        }
        // Every type a payload field is read at is one `field` knows: the
        // integer types by their range, the rest by name.
        for kind in ["float32", "float64", "bool", "string", "bytes"] {
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

    const OPTIONALS: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static" name="optionals">
  <datamodel>
    <data id="due" sce:type="optional&lt;int64&gt;" expr="null"/>
    <data id="other" sce:type="optional&lt;int64&gt;" expr="7"/>
    <data id="title" sce:type="optional&lt;string&gt;" sce:capacity="4" expr="null"/>
    <data id="wider" sce:type="optional&lt;string&gt;" sce:capacity="8" expr="'longer'"/>
    <data id="days" sce:type="int64" expr="0"/>
  </datamodel>
  <state id="s">
    <transition event="write" type="internal">
      <assign location="due" expr="null"/>
      <assign location="title" expr="'abc'"/>
      <assign location="other" expr="due"/>
      <assign location="title" expr="wider"/>
    </transition>
    <transition event="read" type="internal">
      <if cond="due !== null">
        <assign location="days" expr="due.value"/>
      </if>
      <assign location="days" expr="due ?? 0"/>
    </transition>
    <transition event="finish" target="done"/>
  </state>
  <final id="done">
    <donedata>
      <param name="due" expr="due"/>
      <param name="title" expr="title"/>
    </donedata>
  </final>
</scxml>"#;

    /// The `expr` of each element named `name` whose attribute `key` is `value`,
    /// in document order, in the lowered document `out`.
    fn exprs_of(out: &str, name: &str, key: &str, value: &str) -> Vec<String> {
        let document = roxmltree::Document::parse(out).expect("well-formed");
        document
            .descendants()
            .filter(|n| is_scxml_element(n, name) && n.attribute(key) == Some(value))
            .map(|n| n.attribute("expr").unwrap_or_default().to_string())
            .collect()
    }

    /// An optional value is `null` where it is absent and the value itself where
    /// it is present (docs/adr/0055): `null` is left as written, a present
    /// initial value as a value of `T` is, a present text is held to the bound of the place it is
    /// written to, an optional text through `mapPresent`, which keeps `null`, an
    /// optional 64-bit integer leaves the expressions through `out`, a proven
    /// read is the value, and `??` is the library's `otherwise`.
    #[test]
    fn an_optional_value_is_null_or_the_value_itself() {
        let out = lowered(OPTIONALS);
        assert_eq!(exprs_of(&out, "data", "id", "due"), ["null"], "{out}");
        // A literal landing as the present value of an optional 64-bit integer
        // leaves through `out`, as one landing in an `int64` place does.
        assert_eq!(
            exprs_of(&out, "data", "id", "other"),
            ["SceStatic.out(7)"],
            "{out}"
        );
        assert_eq!(
            exprs_of(&out, "assign", "location", "due"),
            ["null"],
            "{out}"
        );
        assert_eq!(
            exprs_of(&out, "assign", "location", "title"),
            [
                "SceStatic.bounded('abc', 4)",
                "SceStatic.mapPresent(wider, function (scePresent) { return \
                 SceStatic.bounded(scePresent, 4); })"
            ],
            "{out}"
        );
        assert_eq!(
            exprs_of(&out, "assign", "location", "other"),
            ["SceStatic.out(due)"],
            "{out}"
        );
        assert_eq!(
            exprs_of(&out, "assign", "location", "days"),
            [
                "SceStatic.out(due)",
                "SceStatic.out(SceStatic.otherwise(due, function () { return 0; }))"
            ],
            "{out}"
        );
        let document = roxmltree::Document::parse(&out).expect("well-formed");
        let guard = document
            .descendants()
            .find(|n| is_scxml_element(n, "if"))
            .and_then(|n| n.attribute("cond"))
            .expect("the <if>");
        assert_eq!(guard, "due !== null", "{out}");
    }

    /// An optional value crosses a `<donedata>`'s `<param>` as JSON `null` where
    /// it is absent and as its wire value where it is present: a 64-bit integer
    /// as the text of its digits, through `mapPresent`, and a text as it is.
    #[test]
    fn an_optional_value_crosses_the_wire_as_null_or_its_wire_value() {
        let out = lowered(OPTIONALS);
        assert_eq!(
            exprs_of(&out, "param", "name", "due"),
            ["SceStatic.mapPresent(due, function (scePresent) { return String(scePresent); })"],
            "{out}"
        );
        assert_eq!(exprs_of(&out, "param", "name", "title"), ["title"], "{out}");
    }

    /// The Interpreter's row of the one table of what is lowered of a composed
    /// value says what its target spells, no more and no less, as each
    /// backend's row is held to its static target
    /// (`each_backend_lowers_the_composed_positions_its_spelling_spells`): it
    /// reads a composed payload and takes a list whole exactly where the target
    /// spells the walks for a read, sends one exactly where it spells them for a
    /// send (docs/adr/0086, sections 5 and 6), and holds a union exactly where
    /// that spelling names a read of a union's tag (docs/adr/0092, section 7).
    /// A row flipped without the spelling, or a spelling landed without the
    /// row, is a lowering the gate hands what it cannot write, or refuses what
    /// it writes.
    #[test]
    fn the_interpreter_row_lowers_the_composed_positions_its_target_spells() {
        use crate::forge::event_schema_check::ComposedLowering;
        let lowering = ComposedLowering::ECMASCRIPT;
        let target = JsTarget::new(BTreeMap::new());
        let read = target.composed_spelling("M");
        let sent = target.composed_send_spelling("M");
        let holds_unions = read
            .as_ref()
            .is_some_and(|spelling| spelling.union_tag_read("kind").is_some());
        assert_eq!(lowering.reads, read.is_some());
        assert_eq!(lowering.copies, read.is_some());
        assert_eq!(lowering.sends, sent.is_some());
        assert_eq!(lowering.unions, holds_unions);
        assert_eq!(lowering.name, target.name());
    }

    /// The read that opens a transition's content is placed just past its start
    /// tag, a `>` inside a quoted attribute value of either quote passed over,
    /// and a tag that never closes has no such place.
    #[test]
    fn the_leading_read_goes_just_past_the_start_tag() {
        let text = r#"<s><transition cond="a > b" event='x>y'><assign/></transition></s>"#;
        let start = text.find("<transition").expect("the tag");
        let at = start_tag_end(text, start).expect("the tag closes");
        assert_eq!(&text[at..], "<assign/></transition></s>");
        assert_eq!(start_tag_end("<transition cond=\"a > b", 0), None);
    }

    /// The data of an event whose schema holds no composed value is the
    /// engine's own `_event.data`, a field of which the library's `field`
    /// reads; that of one whose schema holds one is read whole, by the read the
    /// target was built with.
    #[test]
    fn only_a_composed_payload_is_read_whole() {
        let mut target = JsTarget::new(BTreeMap::new());
        assert_eq!(target.payload_accessor("tick"), PAYLOAD_ACCESSOR);
        target.payloads.insert(
            "batch.sent".to_string(),
            "SceStatic.payload(_event.data, {record: {}})".to_string(),
        );
        assert_eq!(
            target.payload_accessor("batch.sent"),
            "SceStatic.payload(_event.data, {record: {}})"
        );
        assert_eq!(target.payload_accessor("tick"), PAYLOAD_ACCESSOR);
    }

    /// An optional payload field is read through the library's `field` with its
    /// type written `{optional: T}`, an enum's as the names of its variants.
    #[test]
    fn an_optional_payload_field_is_read_as_an_optional_value() {
        let target = JsTarget::new(BTreeMap::new());
        let field = |id: &str, elem: SceType| EntryField {
            id: id.to_string(),
            sce_type: crate::forge::model::EntryType::Optional { elem },
            direction: crate::forge::model::Direction::In,
            expr: None,
            expr_spelling: None,
            expr_splices: None,
            quantity: None,
            max_size: None,
            limit: None,
            default_covers: Vec::new(),
            retain: None,
            initial: None,
            initial_spelling: None,
            since: None,
            optional_since: None,
            default: None,
            default_spelling: None,
        };
        assert_eq!(
            target
                .payload_optional_read("_event.data", &field("due", SceType::Int64), &[])
                .as_deref(),
            Some("SceStatic.field(_event.data, 'due', {optional: 'int64'})")
        );
        assert_eq!(
            target
                .payload_optional_read("_event.data", &field("title", SceType::String), &[])
                .as_deref(),
            Some("SceStatic.field(_event.data, 'title', {optional: 'string'})")
        );
        let status = SceType::from_attr("enum:Status").expect("an enum type");
        assert_eq!(
            target
                .payload_optional_read("_event.data", &field("status", status), &["open", "done"])
                .as_deref(),
            Some("SceStatic.field(_event.data, 'status', {optional: ['open', 'done']})")
        );
        // A field a later revision added is handed its default, `null` when it
        // declares none, which reads absent (docs/adr/0055, section 4).
        let mut remind = field("remind", SceType::Int64);
        remind.since = Some(2);
        assert_eq!(
            target
                .payload_optional_read("_event.data", &remind, &[])
                .as_deref(),
            Some("SceStatic.field(_event.data, 'remind', {optional: 'int64'}, null)")
        );
        remind.default = Some("5".to_string());
        assert_eq!(
            target
                .payload_optional_read("_event.data", &remind, &[])
                .as_deref(),
            Some("SceStatic.field(_event.data, 'remind', {optional: 'int64'}, \"5\")")
        );
        // A field that is never absent is no optional read.
        let mut plain = field("id", SceType::Uint32);
        plain.sce_type = crate::forge::model::EntryType::Scalar(SceType::Uint32);
        assert_eq!(
            target.payload_optional_read("_event.data", &plain, &[]),
            None
        );
    }
}
