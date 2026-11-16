// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Python printer of a codec's encode plan (docs/adr/0081 §2, step 4).
//!
//! A Python value presents its elements as attributes, the members of a
//! `typing.Protocol` that names what a source of the codec is, and its encode
//! is a function over any source, `encode_source` of the codec's module: it
//! reads each element the source presents and derives every other element of
//! the frame from them, as the plan says, never from the value (docs/adr/0058
//! §1). A byte string or a text is presented as a subclass of the runtime's
//! `BoundedBytes` or `BoundedText` that names its bound, the abstract type of
//! §2 for Python: a class whose constructor checks, of an immutable value, so a
//! value of it holds its bound for as long as it lives and the encode over it
//! refuses nothing the bound decides. An element a gate decides is `None` where
//! it is absent, and the flag that says so is derived from it. What the encode
//! does refuse, it raises before the first byte, as the runtime's exception
//! class of the refusal's name.
//!
//! The printer reads the model only through the plan and lowers its forms one
//! landing at a time (§6): [`unlowered`] names the first form a codec's plan
//! uses that it does not print, and such a codec keeps the template's encode
//! whole, so no codec has two encodes.

use super::*;
use crate::forge::codec_roles::ElementId;
use crate::forge::codec_totality::RefusalKind;
use crate::forge::encode_plan::{
    Absent, Clause, CodecPlan, Derive, Element, EncodePlan, Guard, Measured, Narrow, Number,
    NumberInto, Offset, Presence, PresenceFlag, Value, Write,
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) mod source_case;

/// The first form of `plan` the Python printer does not print, as the clause
/// that names it, or `None` where it prints the codec whole.
pub(super) fn unlowered(plan: &CodecPlan<'_>, imports: &[ImportContext]) -> Option<String> {
    unlowered_forms(plan, imports).into_iter().next()
}

/// Every form of `plan` the Python printer does not print, in the order
/// [`unlowered`] meets them; empty where it prints the codec whole.
///
/// It prints a positional codec whose elements are integers, flag sets, enums,
/// byte strings, texts and bodies of codecs it prints that are handed no flag
/// input, each always there or decided by a gate on a flag of the codec's own;
/// whose every derived element is either a length, a count or an encoded
/// length of the elements it measures, held in a field of its own and gated
/// beside its content where its content is, or a flag set exactly where the one
/// field it gates is there; and whose encode refuses only what it raises
/// itself, before the first byte: a gate it cannot derive, measures that
/// disagree, a number past its field, or what a codec it holds refuses. A CBOR
/// map and a content-line component keep their own templates'
/// encodes. `imports` are the codec's own, which say whether a codec it holds
/// is printed ([`ImportContext::python_printed`]).
pub(super) fn unlowered_forms(plan: &CodecPlan<'_>, imports: &[ImportContext]) -> Vec<String> {
    match plan {
        CodecPlan::Positional(plan) => positional_forms(plan, imports),
        CodecPlan::Cbor(_) => vec!["a CBOR map".to_string()],
        CodecPlan::ContentLine(_) => vec!["a content-line component".to_string()],
    }
}

/// The import of the codec a body of `plan` holds as `alias`, where the
/// Python backend prints it from its plan.
fn printed_import<'i>(imports: &'i [ImportContext], alias: &str) -> Option<&'i ImportContext> {
    imports
        .iter()
        .find(|import| import.alias == alias && import.python_printed)
}

/// [`unlowered_forms`] of a positional codec.
fn positional_forms(plan: &EncodePlan<'_>, imports: &[ImportContext]) -> Vec<String> {
    let m = plan.model;
    let mut forms = Vec::new();
    if m.variant.is_some() {
        forms.push("a variant".to_string());
    }
    if !m.flag_inputs.is_empty() {
        forms.push("a flag input".to_string());
    }
    for element in &plan.elements {
        let id = &element.field.id;
        match &element.value {
            Value::Scalar(t) if t.int_bit_width().is_some() => {}
            Value::Scalar(SceType::Enum(_)) => {}
            Value::Scalar(t) => forms.push(format!("field '{id}': a `{}` field", t.as_attr())),
            Value::Bytes(_) | Value::Text(_) => {}
            // A body is written by its codec's own `encode_source`, which a
            // codec the Python backend prints has, handed nothing.
            Value::Body { codec } => {
                if printed_import(imports, codec).is_none() {
                    forms.push(format!(
                        "field '{id}': a body of `{codec}`, which the Python backend does not print"
                    ));
                }
                if plan.handed.get(*codec).is_some_and(|h| !h.is_empty()) {
                    forms.push(format!("field '{id}': a body handed flag inputs"));
                }
            }
            // A list is walked, each entry written by its codec's own
            // `encode_source`, which a codec the Python backend prints has.
            Value::List { codec, law, .. } => {
                if printed_import(imports, codec).is_none() {
                    forms.push(format!(
                        "field '{id}': a list of `{codec}`, which the Python backend does not print"
                    ));
                }
                if plan.handed.get(*codec).is_some_and(|h| !h.is_empty()) {
                    forms.push(format!(
                        "field '{id}': a list whose entries are handed flag inputs"
                    ));
                }
                if !law.is_none() {
                    forms.push(format!("field '{id}': a list held to a law"));
                }
            }
        }
    }
    if !plan.groups().is_empty() {
        forms.push("a group of fields a gate decides together".to_string());
    }
    for derived in &plan.derivations {
        match (&derived.element, &derived.by) {
            // A number into a field, or into a flag of a carrier.
            (_, Derive::Number(number)) => {
                forms.extend(number_form(&derived.element, number));
            }
            (ElementId::Flag { .. }, Derive::Presence(presence)) => {
                forms.extend(presence_form(&derived.element, presence));
            }
            (element, _) => forms.push(format!(
                "element '{element}': an element the encode derives other than a length \
                 or a presence flag"
            )),
        }
    }
    // A gate the encode cannot derive the presence from is a check the value
    // is held to before the first byte, as the encode writes the carriers it
    // reads.
    for field in &plan.gate_checks {
        forms.extend(gate_form(plan, field));
    }
    if !plan.input_checks.is_empty() {
        forms.push("an input a held codec says, held to what the codec is handed".to_string());
    }
    let numbers = derived_numbers(plan);
    for write in &plan.program {
        match write {
            Write::Whole(field) | Write::Tail { field, .. } => {
                forms.extend(unread_form(plan, &numbers, field));
            }
            Write::Byte { pieces, .. } => {
                for (field, _) in pieces {
                    forms.extend(unread_form(plan, &numbers, field));
                }
            }
            Write::Streamed { field, guard, list } => {
                forms.extend(unread_form(plan, &numbers, field));
                // A walk of each entry in turn; a chain that sets its entries'
                // flag, hands an entry what the one before it was, holds a
                // bare entry's place or is held to a law is not printed yet.
                if list.as_ref().is_some_and(|walk| {
                    !walk.previous.is_empty()
                        || walk.slots.is_some()
                        || walk.ends.is_some()
                        || walk.lawful
                }) {
                    forms.push(format!(
                        "field '{}': a chain whose walk sets or hands its entries a flag",
                        field.id
                    ));
                }
                if *guard == Guard::Gate {
                    forms.extend(gate_form(plan, field));
                }
                match &field.bit_size {
                    BitSize::Fixed { .. } => {
                        let whole =
                            BitSpan::of(field, m.default_endian).is_some_and(|span| span.whole());
                        if !whole {
                            forms.push(format!(
                                "field '{}': a streamed field of part of its bytes",
                                field.id
                            ));
                        }
                    }
                    BitSize::Vle {
                        width_bits: 16 | 32 | 64,
                    }
                    | BitSize::Tail
                    | BitSize::LengthRef => {}
                    BitSize::Vle { width_bits } => forms.push(format!(
                        "field '{}': a variable-length integer of {width_bits} bits",
                        field.id
                    )),
                    // A body or a list, which the elements above name.
                    BitSize::Embed | BitSize::Repeat { .. } | BitSize::TlvChain { .. } => {}
                }
            }
            // The variant above names it.
            Write::Variant => {}
        }
    }
    forms.dedup();
    forms
}

/// What of the derived number of `element` the printer does not print: it
/// prints a length, a count or an encoded length of the elements it measures,
/// each of which that is there says the same number or the value is refused,
/// into a field that holds nothing else, refused where it may not fit; where
/// nothing it measures is there the field holds zero, gated beside its
/// content.
fn number_form(element: &ElementId, number: &Number<'_>) -> Vec<String> {
    let mut forms = Vec::new();
    for measure in &number.measures {
        match &measure.of {
            Measured::Bytes { .. } | Measured::Entries { .. } | Measured::Encoded { .. } => {}
            Measured::Says { .. } | Measured::Start { .. } | Measured::ArmSays { .. } => {
                forms.push(format!("element '{element}': a flag a held codec says"));
            }
        }
    }
    match &number.into {
        NumberInto::Field { absent, .. } => {
            if *absent == Absent::Group {
                forms.push(format!(
                    "element '{element}': a length a group holds on the side without its content"
                ));
            }
        }
        // Folded into its carrier, as a presence flag is.
        NumberInto::Flag => {}
    }
    forms
}

/// What of the presence flag `element` the printer does not print: it prints
/// a flag the presence of one field decides, or of fields one group holds.
fn presence_form(element: &ElementId, presence: &PresenceFlag<'_>) -> Option<String> {
    (presence.of.len() != 1 && !presence.together)
        .then(|| format!("element '{element}': a flag the presence of several fields decides"))
}

/// What of the gate of `field`, a field written where its gate holds, the
/// printer does not print: it tests a flag of the codec's own carriers.
fn gate_form(plan: &EncodePlan<'_>, field: &CodecField) -> Option<String> {
    let gate = field.present_if.as_ref()?;
    plan.clauses(gate)
        .iter()
        .any(|clause| !matches!(clause, Clause::Flag { .. }))
        .then(|| {
            format!(
                "field '{}': a gate on an input or on a chain's entries",
                field.id
            )
        })
}

/// The fields of `plan` whose value the encode derives as a number.
fn derived_numbers(plan: &EncodePlan<'_>) -> BTreeSet<String> {
    plan.derivations
        .iter()
        .filter(|derived| matches!(derived.by, Derive::Number(_)))
        .filter_map(|derived| match &derived.element {
            ElementId::Field(id) => Some(id.clone()),
            _ => None,
        })
        .collect()
}

/// Whether the printer cannot read `field` as a write reads it: the value
/// presents it alone, or the encode derives it as a number.
fn unread_form(
    plan: &EncodePlan<'_>,
    numbers: &BTreeSet<String>,
    field: &CodecField,
) -> Option<String> {
    (element_of(plan, &field.id).is_none() && !numbers.contains(&field.id)).then(|| {
        format!(
            "field '{}': a field the value neither presents nor the encode derives as a number",
            field.id
        )
    })
}

/// The element of `plan` the value presents as `id`, if it presents one.
fn element_of<'p, 'm>(plan: &'p EncodePlan<'m>, id: &str) -> Option<&'p Element<'m>> {
    plan.elements.iter().find(|e| e.field.id == id)
}

/// Whether the Python value of `plan`, a plan [`unlowered`] answers `None` for,
/// holds no field `id`: the encode derives it (docs/adr/0081 §3) and the value
/// does not state it while its content is absent.
pub(super) fn derived_field(plan: &EncodePlan<'_>, id: &str) -> bool {
    derived_numbers(plan).contains(id) && element_of(plan, id).is_none()
}

/// The fields the number the value states as `id` measures, each gated: the
/// value states the number only while none of them is there.
fn stated_contents<'m>(plan: &EncodePlan<'m>, id: &str) -> Vec<&'m CodecField> {
    plan.derivations
        .iter()
        .filter(|derived| matches!(&derived.element, ElementId::Field(field) if field == id))
        .filter_map(|derived| match &derived.by {
            Derive::Number(number) => Some(number),
            _ => None,
        })
        .flat_map(|number| number.measures.iter())
        .map(|measure| match &measure.of {
            Measured::Bytes { field, .. } => *field,
            Measured::Entries { list } => *list,
            Measured::Encoded { body, .. } => *body,
            _ => unreachable!("a number the value states is a length or a count of gated content"),
        })
        .collect()
}

/// The Python text the printer gives a codec's module.
pub(super) struct PythonSource {
    /// Before the value's class: the class of each byte string's and text's
    /// bound, the `Protocol` a source of the codec satisfies, and the encode
    /// over any source, `encode_source`.
    pub(super) before: String,
    /// The value's `__setattr__`, which holds each byte string and text to its
    /// bound wherever it is set, where the value is made and after; empty
    /// where it presents none.
    pub(super) hold: String,
    /// For each byte string, text and body the value presents, by field id,
    /// the type the value's field is declared as and the default it takes.
    pub(super) declared: BTreeMap<String, (String, String)>,
    /// The runtime's classes the module names, joined for its import; empty
    /// where it names none.
    pub(super) runtime: String,
    /// The modules of the printed codecs the value holds a body of, each
    /// imported whole under its alias, one statement a line; empty where it
    /// holds none.
    pub(super) modules: String,
    /// Whether a default of the value's fields is made by a factory, a body's
    /// class, which the module imports `field` of `dataclasses` for.
    pub(super) factory: bool,
    /// For each element the decode makes other than as the local it read, by
    /// field id, the expression it makes it of.
    pub(super) decoded: BTreeMap<String, String>,
}

/// The module alias a printed codec's module is imported under where a
/// printed parent writes a body of it (`python_module_alias`), and the class
/// of its value.
fn body_module(import: &ImportContext) -> (String, &str) {
    (python_module_alias(&import.namespace), &import.type_name)
}

/// The class a byte string, a text or a list of `struct_name` is held in, the
/// runtime class it is a subclass of, what it counts, and a literal of its
/// fewest `least` units, which a list has none of: a list of a least past zero
/// is a chain closed by its entries' flag, which the printer does not print.
fn bounded_class(
    struct_name: &str,
    id: &str,
    value: &Value<'_>,
    least: u64,
) -> (String, &'static str, &'static str, String) {
    let pascal = filters::to_pascal_case(id.to_string());
    // The fewest units the class holds, zeros, spelled as a literal: the
    // default is evaluated in the class body, where a field named `bytes`
    // declared before it would hide the builtin.
    match value {
        Value::Text(_) => (
            format!("{struct_name}{pascal}Text"),
            "BoundedText",
            "UTF-8 bytes",
            format!("\"\\x00\" * {least}"),
        ),
        Value::List { .. } => (
            format!("{struct_name}{pascal}List"),
            "BoundedTuple",
            "entries",
            "()".to_string(),
        ),
        _ => (
            format!("{struct_name}{pascal}Bytes"),
            "BoundedBytes",
            "bytes",
            format!("b\"\\x00\" * {least}"),
        ),
    }
}

/// The Python text of the codec `plan`, whose value's class is `struct_name`;
/// the plan is one [`unlowered`] answers `None` for.
pub(super) fn render(
    plan: &EncodePlan<'_>,
    l: &LangCtx,
    imports: &[ImportContext],
    struct_name: &str,
) -> PythonSource {
    let mut before = String::new();
    let mut hold = String::new();
    let mut declared = BTreeMap::new();
    let mut runtime: BTreeSet<&'static str> = BTreeSet::new();
    let mut modules: BTreeSet<String> = BTreeSet::new();
    let mut factory = false;
    let mut decoded = BTreeMap::new();
    // The type the source presents each element as.
    let mut presented: BTreeMap<String, String> = BTreeMap::new();
    for element in &plan.elements {
        let id = &element.field.id;
        let gated = element.presence == Presence::Gated;
        // A body is any source of its codec, which that codec's module names;
        // the value holds its codec's value, made by default by its class.
        if let Value::Body { codec } = &element.value {
            let import = printed_import(imports, codec)
                .expect("`unlowered` prints a body of a printed codec only");
            let (alias, class) = body_module(import);
            modules.insert(format!("from . import {} as {alias}", import.namespace));
            let source = format!("{alias}.{class}Source");
            presented.insert(
                id.clone(),
                if gated {
                    format!("Optional[{source}]")
                } else {
                    source
                },
            );
            if gated {
                declared.insert(
                    id.clone(),
                    (format!("Optional[{class}]"), "None".to_string()),
                );
            } else {
                factory = true;
                declared.insert(
                    id.clone(),
                    (class.to_string(), format!("field(default_factory={class})")),
                );
            }
            continue;
        }
        let (Value::Bytes(bound) | Value::Text(bound) | Value::List { bound, .. }) = &element.value
        else {
            let ty = l.type_name(&element.field.sce_type).into_owned();
            // A number the value states only while the content it measures is
            // absent (docs/adr/0058 §1): `None` while it is there, which the
            // encode then derives from it, and the decode gives back `None`
            // where it read any of it.
            if element.presence == Presence::Stated {
                let optional = format!("Optional[{ty}]");
                presented.insert(id.clone(), optional.clone());
                declared.insert(id.clone(), (optional, "None".to_string()));
                let contents: Vec<String> = stated_contents(plan, id)
                    .iter()
                    .map(|content| format!("{} is None", l.codec_field_decode_local(&content.id)))
                    .collect();
                decoded.insert(
                    id.clone(),
                    format!(
                        "{} if {} else None",
                        l.codec_field_decode_local(id),
                        contents.join(" and ")
                    ),
                );
                continue;
            }
            presented.insert(
                id.clone(),
                if gated { format!("Optional[{ty}]") } else { ty },
            );
            continue;
        };
        // A list's entries are written by their codec's module, imported whole
        // as a body's is.
        if let Value::List { codec, .. } = &element.value {
            let import = printed_import(imports, codec)
                .expect("`unlowered` prints a list of a printed codec only");
            let (alias, _) = body_module(import);
            modules.insert(format!("from . import {} as {alias}", import.namespace));
        }
        let (class, base, unit, fewest) =
            bounded_class(struct_name, id, &element.value, bound.least);
        runtime.insert(base);
        let held = match bound.most {
            Some(most) => format!("{} to {most} {unit}", bound.least),
            None => format!(
                "{} or more {unit}, the protocol stating no most",
                bound.least
            ),
        };
        let most = bound
            .most
            .map_or_else(|| "None".to_string(), |most| most.to_string());
        before.push_str(&format!(
            "class {class}({base}):\n\
             \x20   \"\"\"The ``{id}`` element of :class:`{struct_name}`, {held},\n\
             \x20   held to that bound where a value of it is made (docs/adr/0077 §3).\"\"\"\n\
             \n\
             \x20   LEAST = {least}\n\
             \x20   MOST = {most}\n\n\n",
            least = bound.least
        ));
        let keyword = if hold.is_empty() { "if" } else { "elif" };
        let made = if gated {
            format!("None if value is None else {class}(value)")
        } else {
            format!("{class}(value)")
        };
        hold.push_str(&format!(
            "        {keyword} name == \"{}\":\n            value = {made}\n",
            l.codec_field_id(id)
        ));
        let (ty, default) = if gated {
            (format!("Optional[{class}]"), "None".to_string())
        } else if bound.least == 0 {
            (class.clone(), format!("{class}()"))
        } else {
            (class.clone(), format!("{class}({fewest})"))
        };
        presented.insert(id.clone(), ty.clone());
        declared.insert(id.clone(), (ty, default));
    }

    // The source: what the encode reads, each element by the attribute a value
    // holds it in, `None` where a gate leaves it out.
    before.push_str(&format!(
        "class {struct_name}Source(Protocol):\n\
         \x20   \"\"\"The elements a value of :class:`{struct_name}` presents, which its encode\n\
         \x20   reads (docs/adr/0081 §2): any object that presents them is a source of it.\n\
         \x20   The encode derives every other element of the frame.\"\"\"\n"
    ));
    for element in &plan.elements {
        let id = &element.field.id;
        before.push_str(&format!(
            "\n    @property\n    def {}(self) -> {}: ...\n",
            l.codec_field_id(id),
            presented[id]
        ));
    }

    // A dataclass's constructor sets each field as an attribute, so one
    // `__setattr__` holds a byte string or a text to its bound where the value
    // is made and wherever it is set after.
    let hold = if hold.is_empty() {
        hold
    } else {
        format!(
            "    def __setattr__(self, name: str, value: object) -> None:\n\
             \x20       # Each byte string and text is held to its bound wherever it is\n\
             \x20       # set, where the value is made and after (docs/adr/0081 §2),\n\
             \x20       # whatever it was handed.\n\
             {hold}\
             \x20       object.__setattr__(self, name, value)\n"
        )
    };

    PythonSource {
        before: format!(
            "{}\n\n\n{}",
            before.trim_end_matches('\n'),
            encode_source(plan, l, imports, struct_name)
        ),
        hold: hold.trim_end_matches('\n').to_string(),
        declared,
        runtime: runtime.into_iter().collect::<Vec<_>>().join(", "),
        modules: modules.into_iter().collect::<Vec<_>>().join("\n"),
        factory,
        decoded,
    }
}

/// `value`, the bytes or text `field` holds, as the bytes it is written as.
fn wire_bytes(field: &CodecField, value: &str) -> String {
    if field.sce_type == SceType::String {
        format!("{value}.encode(\"utf-8\")")
    } else {
        value.to_string()
    }
}

/// Each line of `body` one level deeper.
fn indented(body: &str) -> String {
    body.lines().map(|line| format!("    {line}\n")).collect()
}

/// The encode of `struct_name` over any source of it, a function of the
/// module, which is the codec's namespace as a C++ codec's namespace is: each
/// derived number made from what it measures and each derived flag folded into
/// its carrier, then the writes in wire order, each element read off the
/// source. A function of the module and not a method of the value's class, so
/// no field of the value, a class attribute, can take its name.
///
/// A body whose encoded length the codec carries is encoded once, into a
/// buffer whose length is the number, and the buffer is what is written where
/// the body is; any other body is encoded straight into the sink.
fn encode_source(
    plan: &EncodePlan<'_>,
    l: &LangCtx,
    imports: &[ImportContext],
    struct_name: &str,
) -> String {
    let numbers = derived_numbers(plan);
    let (stated, derived): (Vec<&String>, Vec<&String>) = numbers
        .iter()
        .partition(|id| element_of(plan, id).is_some());
    let named = |ids: &[&String]| {
        ids.iter()
            .map(|id| format!("``{id}``"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut derives = String::new();
    if !derived.is_empty() {
        derives.push_str(&format!(
            " It derives {} from what each measures, and never reads it off\n    the source.",
            named(&derived)
        ));
    }
    if !stated.is_empty() {
        derives.push_str(&format!(
            " It derives {} where what it measures is there, and reads\n    what the source \
             states where none is.",
            named(&stated)
        ));
    }
    // What the encode refuses, each refusal the runtime's exception class:
    // nothing, or what the plan's totality names, before the first byte.
    let refuses = if plan.totality.is_total() {
        " It refuses no value;".to_string()
    } else {
        let mut raised: Vec<String> = plan
            .totality
            .refusals()
            .iter()
            .map(|r| match r.kind.error_variant() {
                Some(variant) => format!(":class:`{variant}`"),
                None => "what a codec it holds raises".to_string(),
            })
            .collect();
        raised.sort();
        raised.dedup();
        format!(
            " It raises\n    {} for a value no frame carries, before\n    it writes a byte to ``w``;",
            raised.join(", ")
        )
    };
    let mut out = format!(
        "def encode_source(src: {struct_name}Source, w: SceSink) -> None:\n\
         \x20   \"\"\"Write any source of :class:`{struct_name}` into ``w``, each element read\n\
         \x20   off ``src`` and written where the codec's encode plan lays it out\n\
         \x20   (docs/adr/0081).{derives}{refuses} a bounded sink raises\n\
         \x20   :class:`BufferOverflow` where it has no room left.\"\"\"\n"
    );
    let source = |id: &str| format!("src.{}", l.codec_field_id(id));
    // The `encode_source` of the module that writes the body `field`, or each
    // entry of the list `field`.
    let body_writer = |field: &CodecField| -> String {
        let (Value::Body { codec } | Value::List { codec, .. }) = element_of(plan, &field.id)
            .map(|e| &e.value)
            .expect("a body or a list is an element the value presents")
        else {
            unreachable!("the field holds a codec")
        };
        let import =
            printed_import(imports, codec).expect("`unlowered` prints a codec it holds printed");
        format!("{}.encode_source", body_module(import).0)
    };
    // The bodies encoded into a buffer first, by field id: those whose length
    // a number measures.
    let mut buffered: BTreeSet<String> = BTreeSet::new();
    // Each flag the encode derives, by its carrier: the statements that fold
    // it into the carrier's local, `_c_<carrier>`, in place of the bits the
    // value presents.
    let mut folds: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for derived in &plan.derivations {
        match (&derived.element, &derived.by) {
            (element, Derive::Number(number)) => {
                let mut measured: Vec<MeasureText> = Vec::new();
                for measure in &number.measures {
                    let (content, size) = match &measure.of {
                        Measured::Bytes { field, offset } => {
                            // The content is never shorter than what the
                            // wire's length takes off.
                            let arith = match offset {
                                Offset::None => String::new(),
                                Offset::More { by } => format!(" + {by}"),
                                Offset::Less { by, held: true } => format!(" - {by}"),
                                Offset::Less { held: false, .. } => {
                                    unreachable!("`{}`: {}", field.id, LESS_IS_HELD)
                                }
                            };
                            let content = source(&field.id);
                            let size = format!("len({}){arith}", wire_bytes(field, &content));
                            (content, size)
                        }
                        // The body's frame, encoded once into a buffer the
                        // write of the body then writes; `None` where it is
                        // absent. A body whose encode refuses raises here,
                        // before the first byte of this frame.
                        Measured::Encoded { body, .. } => {
                            let content = source(&body.id);
                            let buffer = format!("_b_{}", l.codec_field_id(&body.id));
                            if buffered.insert(body.id.clone()) {
                                let encode = format!(
                                    "{buffer} = bytearray()\n{}({content}, BytearraySink({buffer}))\n",
                                    body_writer(body)
                                );
                                if measure.gated {
                                    out.push_str(&format!("    {buffer} = None\n"));
                                    out.push_str(&format!(
                                        "    if {content} is not None:\n{}",
                                        indented(&indented(&encode))
                                    ));
                                } else {
                                    out.push_str(&indented(&encode));
                                }
                            }
                            let size = format!("len({buffer})");
                            (buffer, size)
                        }
                        // How many entries the list holds, within its bound.
                        Measured::Entries { list } => {
                            let content = source(&list.id);
                            let size = format!("len({content})");
                            (content, size)
                        }
                        _ => unreachable!("`unlowered` prints a length or a count only"),
                    };
                    measured.push(MeasureText {
                        size,
                        present: measure.gated.then(|| format!("{content} is not None")),
                    });
                }
                derive_number(
                    plan,
                    l,
                    element,
                    &number.into,
                    &measured,
                    &mut out,
                    &mut folds,
                );
            }
            (ElementId::Flag { field, flag }, Derive::Presence(presence)) => {
                let (gated, when_clear) =
                    presence.of.first().expect("a presence flag gates a field");
                let def = plan
                    .model
                    .fields
                    .iter()
                    .find(|f| f.id == *field)
                    .and_then(|carrier| carrier.flags.iter().find(|d| d.name == *flag))
                    .expect("the parser found the flag a gate reads");
                let mask = ((1u64 << def.width) - 1) << def.bit;
                let test = if *when_clear {
                    "is None"
                } else {
                    "is not None"
                };
                let c = format!("_c_{}", l.codec_field_id(field));
                folds.entry(field.clone()).or_default().push(format!(
                    "{c} = ({c} & ~0x{mask:X}) | (0x{mask:X} if {} {test} else 0)\n",
                    source(&gated.id)
                ));
            }
            _ => unreachable!("`unlowered` prints no other derivation"),
        }
    }
    // Each carrier as the encode writes it: its flags the value presents, and
    // those the encode derives folded in, in place of the value's.
    for (carrier, carried) in &folds {
        let name = l.codec_field_id(carrier);
        out.push_str(&format!("    _c_{name} = {}\n", source(carrier)));
        for statement in carried {
            out.push_str(&indented(statement));
        }
    }
    let read = |f: &CodecField| -> String {
        let name = l.codec_field_id(&f.id);
        if numbers.contains(&f.id) {
            format!("_d_{name}")
        } else if folds.contains_key(&f.id) {
            format!("_c_{name}")
        } else {
            format!("src.{name}")
        }
    };
    // A gate, tested on the carriers as the encode writes them.
    let gate_test = |f: &CodecField| -> String {
        let gate = f
            .present_if
            .as_ref()
            .expect("a field written where its gate holds is gated");
        plan.clauses(gate)
            .iter()
            .map(|clause| match clause {
                Clause::Flag {
                    carrier,
                    bit,
                    negate,
                } => format!(
                    "(({} >> {bit}) & 1) {} 0",
                    read(carrier),
                    if *negate { "==" } else { "!=" }
                ),
                _ => unreachable!("`unlowered` prints a gate on a carrier's flags only"),
            })
            .collect::<Vec<_>>()
            .join(" or ")
    };
    // Each gate the encode cannot derive the presence from holds the field it
    // gates, before the first byte (docs/adr/0065 §2).
    for field in &plan.gate_checks {
        out.push_str(&format!(
            "    if ({}) != ({} is not None):\n        {}\n",
            gate_test(field),
            source(&field.id),
            raise(plan, RefusalKind::PresentIfMismatch, &field.id)
        ));
    }
    // A codec it holds can refuse the value as it is written, so the frame is
    // written whole first and handed to the caller's sink only once it is, as
    // a count over a sink that keeps nothing precedes the write in the other
    // printers (docs/adr/0081 §1): a refusal leaves the caller's sink as it
    // was.
    let mut staged = false;
    for r in plan.totality.refusals() {
        if r.kind == RefusalKind::Child {
            plan.totality.require(RefusalKind::Child, &r.element);
            staged = true;
        }
    }
    if staged {
        out.push_str("    _w = w\n    _frame = bytearray()\n    w = BytearraySink(_frame)\n");
    }
    let endian = plan.model.default_endian;
    for write in &plan.program {
        match write {
            Write::Whole(field) => {
                for byte in encode_whole_field_read_as(field, endian, &read(field), l) {
                    out.push_str(&format!("    w.write_u8({byte})\n"));
                }
            }
            Write::Byte { pieces, .. } => {
                out.push_str(&format!(
                    "    w.write_u8({})\n",
                    encode_byte_read_by(pieces, l, &read)
                ));
            }
            Write::Tail { field, pad } => {
                // A field the layout aligns follows zeros up to its offset.
                if *pad {
                    out.push_str(&format!(
                        "    while w.position() < {}:\n        w.write_u8(0)\n",
                        field.byte_offset
                    ));
                }
                out.push_str(&format!(
                    "    w.write_bytes({})\n",
                    wire_bytes(field, &read(field))
                ));
            }
            Write::Streamed { field, guard, .. } => {
                let body = match &field.bit_size {
                    BitSize::Fixed { .. } => {
                        encode_whole_field_read_as(field, endian, &read(field), l)
                            .iter()
                            .map(|byte| format!("    w.write_u8({byte})\n"))
                            .collect::<String>()
                    }
                    // The runtime's writer, as the template's encode calls it,
                    // at the body's indentation of a function of the module.
                    BitSize::Vle { width_bits } => {
                        let value = l.codec_carrier_expr(&field.sce_type, &read(field));
                        format!(
                            "    {}\n",
                            vle_encode_block(&value, *width_bits, l.lang).trim()
                        )
                    }
                    BitSize::Tail | BitSize::LengthRef => {
                        format!("    w.write_bytes({})\n", wire_bytes(field, &read(field)))
                    }
                    // A body: the buffer its length was measured over, or its
                    // codec's encode over the source it is presented as.
                    BitSize::Embed if buffered.contains(&field.id) => {
                        format!("    w.write_bytes(_b_{})\n", l.codec_field_id(&field.id))
                    }
                    BitSize::Embed => {
                        format!("    {}({}, w)\n", body_writer(field), read(field))
                    }
                    // A list or a chain walked entry by entry, each written by
                    // its codec's encode over the source the list holds it as.
                    BitSize::Repeat { .. } | BitSize::TlvChain { .. } => format!(
                        "    for _entry in {}:\n        {}(_entry, w)\n",
                        read(field),
                        body_writer(field)
                    ),
                };
                out.push_str(&match guard {
                    Guard::Always => body,
                    // Where the value holds it.
                    Guard::Present => {
                        format!("    if {} is not None:\n{}", read(field), indented(&body))
                    }
                    // Derived whole and gated: where its gate holds, as the
                    // carriers the encode writes say.
                    Guard::Gate => format!("    if {}:\n{}", gate_test(field), indented(&body)),
                });
            }
            Write::Variant => unreachable!("`unlowered` prints no variant"),
        }
    }
    if staged {
        out.push_str("    _w.write_bytes(_frame)\n");
    }
    // Every refusal the totality analysis names is one the encode raises
    // (docs/adr/0065 §2).
    plan.totality.assert_all_written(struct_name);
    out.trim_end_matches('\n').to_string()
}

/// One measure of a derived number as the encode reads it.
struct MeasureText {
    /// The number, which is read only where the measure is there.
    size: String,
    /// Where the measure may be absent, the test that it is there.
    present: Option<String>,
}

/// Why a length the wire holds less a constant is never past its content: a
/// length-ref field's bound takes its fewest bytes from that constant, so the
/// class of its bytes holds at least what the wire takes off
/// (`codec_totality::held_to_offset`, docs/adr/0077 §3), and the analysis
/// names no refusal at the content.
const LESS_IS_HELD: &str = "a length-ref's bound holds at least the bytes its length-arith takes \
     off (LengthBound::of), so the length is never negative";

/// The statement that raises the refusal of `kind` at `at`, the runtime's
/// exception class of the refusal's name, which the totality analysis names
/// (`Totality::require`, docs/adr/0065 §2).
fn raise(plan: &EncodePlan<'_>, kind: RefusalKind, at: &str) -> String {
    let error = plan.totality.require(kind, at);
    let class = error.trim_start_matches("CodecError::");
    format!("raise {class}(\"{at}\")")
}

/// The largest value the integer type `ty` holds, or `None` for another type.
fn largest_of(ty: &SceType) -> Option<u64> {
    let bits = ty.int_bit_width()?;
    let bits = if ty.is_signed() { bits - 1 } else { bits };
    Some(if bits >= 64 {
        u64::MAX
    } else {
        (1u64 << bits) - 1
    })
}

/// The name the encode's locals of the derived `element` are named by.
fn derived_local(l: &LangCtx, element: &ElementId) -> String {
    match element {
        ElementId::Field(id) => l.codec_field_id(id),
        ElementId::Flag { field, flag } => format!(
            "{}_{}",
            l.codec_field_id(field),
            filters::to_snake_case(flag.clone())
        ),
        ElementId::Variant => unreachable!("the plan derives nothing into the variant"),
    }
}

/// The statements, written into `out`, that make the derived number of
/// `element` from what it measures, as the Rust encode's `combine` makes it
/// (docs/adr/0081 §1): the first measure always there fixes it, and every
/// other one there must say the same, or the value is refused as
/// `InconsistentSource`; where every measure may be absent, it is the first
/// one there. A field holds it as `_d_<field>`, refused as `LengthOutOfRange`
/// where it may not fit; where nothing it measures is there, the field holds
/// zero.
fn derive_number(
    plan: &EncodePlan<'_>,
    l: &LangCtx,
    element: &ElementId,
    into: &NumberInto<'_>,
    measures: &[MeasureText],
    out: &mut String,
    folds: &mut BTreeMap<String, Vec<String>>,
) {
    let named = element.to_string();
    let disagree = || raise(plan, RefusalKind::InconsistentSource, &named);
    let there = |m: &MeasureText| {
        m.present
            .clone()
            .expect("a measure that may be absent has a test that it is there")
    };
    // The number, and where it may not be there, the test that it is.
    let (size, present) = match measures {
        // Nothing measured: zero, as the Rust encode's `combine` holds it.
        [] => ("0".to_string(), None),
        [one] => (one.size.clone(), one.present.clone()),
        _ => {
            let always: Vec<&MeasureText> =
                measures.iter().filter(|m| m.present.is_none()).collect();
            let gated: Vec<&MeasureText> =
                measures.iter().filter(|m| m.present.is_some()).collect();
            if let Some((anchor, others)) = always.split_first() {
                for other in others {
                    out.push_str(&format!(
                        "    if {} != {}:\n        {}\n",
                        other.size,
                        anchor.size,
                        disagree()
                    ));
                }
                for g in &gated {
                    out.push_str(&format!(
                        "    if {} and {} != {}:\n        {}\n",
                        there(g),
                        g.size,
                        anchor.size,
                        disagree()
                    ));
                }
                (anchor.size.clone(), None)
            } else {
                let first = format!("_f_{}", derived_local(l, element));
                out.push_str(&format!("    {first} = None\n"));
                for (i, g) in gated.iter().enumerate() {
                    let keyword = if i == 0 { "if" } else { "elif" };
                    out.push_str(&format!(
                        "    {keyword} {}:\n        {first} = {}\n",
                        there(g),
                        g.size
                    ));
                }
                for g in gated.iter().skip(1) {
                    out.push_str(&format!(
                        "    if {} and {} != {first}:\n        {}\n",
                        there(g),
                        g.size,
                        disagree()
                    ));
                }
                let found = format!("{first} is not None");
                (first, Some(found))
            }
        }
    };
    let (field, narrow, absent) = match (element, into) {
        (
            ElementId::Field(_),
            NumberInto::Field {
                field,
                narrow,
                absent,
            },
        ) => (field, narrow, absent),
        // Folded into its carrier in place of the bits the value presents,
        // where it is there; where it is free and nothing it measures is
        // there, the carrier keeps the value's bits. A number the flag's bits
        // cannot hold is no frame, where the grammar does not bound it within
        // them.
        (ElementId::Flag { field, flag }, NumberInto::Flag) => {
            let def = plan
                .model
                .fields
                .iter()
                .find(|f| f.id == *field)
                .and_then(|carrier| carrier.flags.iter().find(|d| d.name == *flag))
                .expect("a number held in a flag is held in a flag of its carrier");
            let width_mask = if def.width >= 64 {
                u64::MAX
            } else {
                (1u64 << def.width) - 1
            };
            let mask = width_mask << def.bit;
            let c = format!("_c_{}", l.codec_field_id(field));
            let mut set = String::new();
            if plan.totality.refuses(RefusalKind::LengthOutOfRange, &named) {
                set.push_str(&format!(
                    "if {size} > 0x{width_mask:X}:\n    {}\n",
                    raise(plan, RefusalKind::LengthOutOfRange, &named)
                ));
            }
            set.push_str(&format!(
                "{c} = ({c} & ~0x{mask:X}) | (({size}) << {})\n",
                def.bit
            ));
            folds
                .entry(field.clone())
                .or_default()
                .push(match &present {
                    None => set,
                    Some(there) => format!("if {there}:\n{}", indented(&set)),
                });
            return;
        }
        _ => unreachable!("a number goes into the field or the flag it names"),
    };
    let name = l.codec_field_id(&field.id);
    // A number past what the field holds is no frame (docs/adr/0065 §5).
    if *narrow == Narrow::Checked {
        let most = largest_of(&field.sce_type)
            .expect("a length that may not fit its field is held in an integer field");
        let past = format!("{size} > 0x{most:X}");
        let test = match &present {
            Some(there) => format!("{there} and {past}"),
            None => past,
        };
        out.push_str(&format!(
            "    if {test}:\n        {}\n",
            raise(plan, RefusalKind::LengthOutOfRange, &named)
        ));
    }
    // Where the content is not, zero, the field gated beside it and not
    // written, or what the value states.
    let stated = format!("src.{name}");
    let otherwise = match absent {
        Absent::Never | Absent::Zero => "0".to_string(),
        Absent::Stated => format!("(0 if {stated} is None else {stated})"),
        Absent::Group => unreachable!("`unlowered` prints no number a group states"),
    };
    let value = match &present {
        None => size,
        Some(there) => format!("({size}) if {there} else {otherwise}"),
    };
    out.push_str(&format!("    _d_{name} = {value}\n"));
    // A value that states the number beside content says it twice, and must
    // say what the content does.
    if *absent == Absent::Stated {
        let there = present
            .as_deref()
            .expect("a number the value states is free: every measure may be absent");
        out.push_str(&format!(
            "    if {there} and {stated} is not None and {stated} != _d_{name}:\n        {}\n",
            disagree()
        ));
    }
}
