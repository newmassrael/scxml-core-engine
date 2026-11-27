// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The surface a page is written against (docs/adr/0117, D7): from a
//! `datamodel="sce-static"` statechart, a JavaScript module and its TypeScript
//! declaration that name the events the machine takes, raise one, and subscribe
//! to the snapshot the machine publishes.
//!
//! It is not a backend, and no rule of the document is in it. The machine the
//! page holds is reached through a port (`MachinePort` in the declaration): the
//! module hands it an event by name with the JSON text of its payload, and hands
//! the page each snapshot the port gives. What a type cannot hold at run time,
//! since a page may be written in JavaScript, the module checks at the boundary
//! by the payload schema of each event, the text its schema unit declares
//! (docs/adr/0087, section 1; [`crate::forge::payload_schema::json_schema`]),
//! and refuses there what the machine's reader of a payload refuses, before
//! anything reaches the port. The declaration's types are written from the same
//! schemas, so what compiles and what the boundary admits are one language.
//!
//! - The events are those the machine takes from outside
//!   ([`SCXMLModel::externally_drivable_events`], the platform's own left out):
//!   the event each schema the document imports declares, where a transition
//!   takes it, with the schema's payload; and every other event a transition
//!   takes, by its name, which carries no data.
//! - The snapshot is what a host of every backend reads of a machine
//!   (docs/SCE_ACCEPTED_SUBSET.md section 2.15, "Snapshot"): every state the
//!   machine is in, its published variables (`sce:direction="out"`) and whether
//!   its last macrostep was stopped at its bound. Each value is in the wire form
//!   a payload schema states (docs/adr/0086, section 3.1): a 64-bit integer as
//!   the text of its digits, a byte string as a text of one character for each
//!   byte, an absent value as `null`.
//!
//! A value with no wire form (a map or a set, docs/adr/0108, D7; a value of an
//! open enum) is refused by name, as `sce-codegen payload-schema` refuses it.

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use serde_json::{Map, Value};

use crate::event_descriptor::EventDescriptor;
use crate::forge::error::GenerateError;
use crate::forge::model::Direction;
use crate::forge::value_type::{json_schema_object, Keyword, SchemaDefs, ScopedRecord, ValueType};
use crate::model::{Datamodel, SCXMLModel};
use crate::template_lexing::Syntax;

/// The directory of the template tree the surface's templates are in.
const DIRECTORY: &str = "surface/";

/// The template of the module.
const MODULE: &str = "surface/surface.mjs.jinja2";

/// The template of the declaration.
const DECLARATION: &str = "surface/surface.d.mts.jinja2";

/// The surface of one statechart: the events it takes, and its two texts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageSurface {
    /// The statechart's name, the stem of its file, which the two files are
    /// named by: `<machine>.mjs` and `<machine>.d.mts`.
    pub machine: String,
    /// Each event the machine takes from outside, in the order of their names.
    pub events: Vec<TakenEvent>,
    /// The JavaScript module.
    pub module: String,
    /// Its TypeScript declaration. A declaration of an ECMAScript module ends in
    /// `.d.mts`, so a page that imports `./<machine>.mjs` is typed by it.
    pub declaration: String,
}

/// An event the machine takes from outside, and the schema of what it carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TakenEvent {
    /// The event's name, as a page raises it.
    pub name: String,
    /// The payload schema of its data, the text its schema unit declares with no
    /// source hash; `None` for an event that carries no data.
    pub schema: Option<String>,
}

/// The surface of `model`, a statechart parsed with its imports and analyzed.
///
/// Refused, with `generate/unsupported-feature`, for a document of another data
/// model, whose values have no type to write; for an event two schemas declare;
/// and for a published variable or a payload that has no wire form.
pub fn of(model: &SCXMLModel) -> Result<PageSurface, GenerateError> {
    if model.datamodel != Datamodel::SceStatic {
        return Err(GenerateError::unsupported(format!(
            "the page surface of `{}` is written from a document of datamodel=\"sce-static\", \
             whose every value has a type (docs/adr/0117, D7), and it declares another data model",
            model.name
        )));
    }
    let events = taken_events(model)?;
    let mut definitions = Definitions::default();
    let mut payloads = Vec::new();
    for event in &events {
        if let Some(schema) = &event.schema {
            let schema = parsed(schema, &event.name)?;
            definitions.take(&schema)?;
            let typed = ts_type(&schema, 1).map_err(|why| untyped(&event.name, &why))?;
            payloads.push(format!("  readonly {}: {typed};", quoted(&event.name)));
        }
    }
    let published = published_schema(model)?;
    definitions.take(&published)?;
    let what = format!("what `{}` publishes", model.name);
    let variables = published
        .as_object()
        .map_or_else(|| Ok(Vec::new()), |object| members(object, 1))
        .map_err(|why| untyped(&what, &why))?;
    let definition_lines = definitions.lines()?;
    let module_events: Vec<ModuleEvent> = events
        .iter()
        .map(|event| ModuleEvent {
            name: quoted(&event.name),
            schema: event.schema.clone().unwrap_or_else(|| "null".to_string()),
        })
        .collect();
    let context = minijinja::context! {
        machine => model.name.as_str(),
        machine_json => quoted(&model.name),
        events => minijinja::Value::from_serialize(&module_events),
        definitions => definition_lines.join("\n"),
        payloads => payloads.join("\n"),
        event_type => event_type(&events),
        states => state_type(model),
        variables => variables.join("\n"),
    };
    let (module, declaration) = render(&context)?;
    Ok(PageSurface {
        machine: model.name.clone(),
        events,
        module,
        declaration,
    })
}

/// One event as the module's table holds it: its name and its payload schema,
/// each the JavaScript of a value (`null` for an event that carries no data).
#[derive(Serialize)]
struct ModuleEvent {
    name: String,
    schema: String,
}

/// The events the machine takes from outside, each with its payload schema.
fn taken_events(model: &SCXMLModel) -> Result<Vec<TakenEvent>, GenerateError> {
    let descriptors: Vec<&str> = model
        .externally_drivable_events
        .iter()
        .map(String::as_str)
        .filter(|event| !crate::analyzer::is_reserved_ingress_event(event))
        .collect();
    // A transition on `due` takes `due.set` too, as an event descriptor matches
    // the events it is a prefix of, so a schema's event is taken when any
    // descriptor of the machine matches it.
    let taken_by = |event: &str| {
        descriptors
            .iter()
            .any(|descriptor| EventDescriptor::parse(descriptor).matches(event))
    };
    let mut schemas: BTreeMap<&str, &ScopedRecord> = BTreeMap::new();
    for record in model.imported_type_scope.records.values() {
        let event = record.lineage.event.as_str();
        if event.is_empty() || !taken_by(event) {
            continue;
        }
        if let Some(held) = schemas
            .insert(event, record)
            .filter(|held| held.identity != record.identity)
        {
            return Err(GenerateError::unsupported(format!(
                "the schemas `{}` and `{}` both declare the event `{event}`, which a page \
                 raises with one payload",
                held.name, record.name
            )));
        }
    }
    let mut names: BTreeSet<&str> = schemas.keys().copied().collect();
    names.extend(descriptors.iter().copied());
    names
        .into_iter()
        .map(|name| -> Result<TakenEvent, GenerateError> {
            // A schema of no field says the event carries nothing
            // (`<datamodel sce:payload="none"/>`), as an event of no schema does.
            let schema = match schemas.get(name) {
                Some(record) if !record.fields.is_empty() => Some(
                    crate::forge::payload_schema::json_schema(
                        &record.name,
                        record.lineage.revision,
                        &record.fields,
                        &record.scope,
                        None,
                    )
                    .map_err(|why| {
                        GenerateError::unsupported(format!(
                            "the payload of `{name}` has no wire form, so no page can raise it: {why}"
                        ))
                    })?,
                ),
                _ => None,
            };
            Ok(TakenEvent {
                name: name.to_string(),
                schema,
            })
        })
        .collect()
}

/// The schema of the published variables: an object of every variable the
/// document declares `sce:direction="out"`, in the order it declares them, with
/// the records and unions they hold under `$defs`.
fn published_schema(model: &SCXMLModel) -> Result<Value, GenerateError> {
    let mut states: Vec<_> = model.states.values().collect();
    states.sort_by_key(|state| state.document_order);
    let mut fields: Vec<(String, ValueType)> = Vec::new();
    for variable in model
        .variables
        .iter()
        .chain(states.iter().flat_map(|state| state.datamodel.iter()))
    {
        if variable.direction != Some(Direction::Out) {
            continue;
        }
        let ty = variable
            .value_type
            .clone()
            .ok_or_else(|| {
                GenerateError::unsupported(format!(
                    "the published variable `{}` declares no sce:type",
                    variable.id
                ))
            })?
            .bounded()
            .map_err(|why| {
                GenerateError::unsupported(format!(
                    "the published variable `{}` has no wire form: {}",
                    variable.id,
                    why.rule()
                ))
            })?;
        fields.push((variable.id.clone(), ty));
    }
    let mut defs = SchemaDefs::default();
    let mut root =
        json_schema_object(&fields, &model.imported_type_scope, &mut defs).map_err(|why| {
            GenerateError::unsupported(format!(
                "what `{}` publishes has no wire form, so no page can be told it: {why}",
                model.name
            ))
        })?;
    if let Some(node) = defs.into_node() {
        root.insert(Keyword::Defs, node);
    }
    parsed(&root.text(), &model.name)
}

/// The records and unions the schemas hold, by the name each schema's `$defs`
/// gives it: the name of the document that declares it.
#[derive(Default)]
struct Definitions(BTreeMap<String, Value>);

impl Definitions {
    /// Hold the definitions of `schema`. A name another schema defined
    /// otherwise is refused: a page's types name each by that name alone.
    fn take(&mut self, schema: &Value) -> Result<(), GenerateError> {
        let Some(defs) = schema.get("$defs").and_then(Value::as_object) else {
            return Ok(());
        };
        for (name, definition) in defs {
            match self.0.entry(name.clone()) {
                Entry::Vacant(slot) => {
                    slot.insert(definition.clone());
                }
                Entry::Occupied(held) if held.get() != definition => {
                    return Err(GenerateError::unsupported(format!(
                        "two documents named `{name}` declare different records or unions, and \
                         a page's types name each by the name of its document"
                    )));
                }
                Entry::Occupied(_) => {}
            }
        }
        Ok(())
    }

    /// The members of the declaration's `Definitions`, one a definition.
    fn lines(&self) -> Result<Vec<String>, GenerateError> {
        self.0
            .iter()
            .map(|(name, definition)| -> Result<String, GenerateError> {
                let typed = ts_type(definition, 1).map_err(|why| untyped(name, &why))?;
                Ok(format!("  readonly {}: {typed};", quoted(name)))
            })
            .collect()
    }
}

/// The module and the declaration, each rendered from its template with
/// `context`, read as the C family is: `//` and `/* */` comments and `"`
/// strings, which is what the templates write of JavaScript and TypeScript.
fn render(context: &minijinja::Value) -> Result<(String, String), GenerateError> {
    let mut env = crate::generator::new_env();
    for &(name, content) in crate::template_registry::EMBEDDED_TEMPLATES {
        if !name.starts_with(DIRECTORY) {
            continue;
        }
        crate::generator::register_template_in(
            &mut env,
            name.to_string(),
            content,
            Syntax::CFamily,
        )
        .map_err(|e| GenerateError::TemplateLoad(format!("{name}: {e:#}")))?;
    }
    let rendered = |template: &str| {
        env.get_template(template)
            .and_then(|t| t.render(context))
            .map_err(|e| GenerateError::TemplateRender(format!("{template}: {e:#}")))
    };
    Ok((rendered(MODULE)?, rendered(DECLARATION)?))
}

/// `text`, a schema this module made, read back as JSON.
fn parsed(text: &str, of: &str) -> Result<Value, GenerateError> {
    serde_json::from_str(text)
        .map_err(|e| GenerateError::unsupported(format!("the schema of `{of}` is not JSON: {e}")))
}

/// A schema the declaration cannot type: what the fold wrote and the reader
/// below does not read, which is a fault of the two, not of the document.
fn untyped(of: &str, why: &str) -> GenerateError {
    GenerateError::unsupported(format!(
        "the schema of `{of}` holds what the page surface writes no TypeScript type for: {why}"
    ))
}

/// `text` as a string literal of TypeScript and of JavaScript: its JSON text.
fn quoted(text: &str) -> String {
    Value::String(text.to_string()).to_string()
}

/// Two spaces a level, as the declaration is indented.
fn indent(depth: usize) -> String {
    "  ".repeat(depth)
}

/// The TypeScript type of the values `schema` admits, written at `depth`: an
/// object's keys one level deeper, its closing brace at `depth`.
fn ts_type(schema: &Value, depth: usize) -> Result<String, String> {
    let object = schema
        .as_object()
        .ok_or_else(|| format!("a schema that is not an object: {schema}"))?;
    if let Some(reference) = object.get("$ref") {
        let name = reference
            .as_str()
            .and_then(|reference| reference.strip_prefix("#/$defs/"))
            .ok_or_else(|| format!("a reference to no definition: {reference}"))?;
        // RFC 6901, section 4: `~1` is decoded before `~0`.
        let name = name.replace("~1", "/").replace("~0", "~");
        return Ok(format!("Definitions[{}]", quoted(&name)));
    }
    for keyword in ["anyOf", "oneOf"] {
        if let Some(members) = object.get(keyword) {
            let members = members
                .as_array()
                .ok_or_else(|| format!("`{keyword}` that is not a list"))?;
            let typed = members
                .iter()
                .map(|member| ts_type(member, depth))
                .collect::<Result<Vec<_>, _>>()?;
            return Ok(typed.join(" | "));
        }
    }
    if let Some(values) = object.get("enum") {
        let values = values
            .as_array()
            .ok_or_else(|| "`enum` that is not a list".to_string())?;
        let literals: Vec<String> = values.iter().map(Value::to_string).collect();
        return Ok(literals.join(" | "));
    }
    if let Some(value) = object.get("const") {
        return Ok(value.to_string());
    }
    match object.get("type").and_then(Value::as_str) {
        Some("object") => {
            let lines = members(object, depth + 1)?;
            if lines.is_empty() {
                return Ok("Record<string, never>".to_string());
            }
            Ok(format!("{{\n{}\n{}}}", lines.join("\n"), indent(depth)))
        }
        Some("array") => {
            let items = object
                .get("items")
                .ok_or_else(|| "a list whose values are not stated".to_string())?;
            Ok(format!("readonly ({})[]", ts_type(items, depth)?))
        }
        Some("string") => Ok("string".to_string()),
        Some("integer" | "number") => Ok("number".to_string()),
        Some("boolean") => Ok("boolean".to_string()),
        Some("null") => Ok("null".to_string()),
        _ => Err(format!("a schema of no type: {schema}")),
    }
}

/// One line for each key of the object `object` states, at `depth`:
/// `readonly "<key>": <type>;`, `?` after the key where the object does not
/// require it.
fn members(object: &Map<String, Value>, depth: usize) -> Result<Vec<String>, String> {
    let required: BTreeSet<&str> = object
        .get("required")
        .and_then(Value::as_array)
        .map(|keys| keys.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let Some(properties) = object.get("properties") else {
        return Ok(Vec::new());
    };
    let properties = properties
        .as_object()
        .ok_or_else(|| "`properties` that is not an object".to_string())?;
    properties
        .iter()
        .map(|(key, schema)| -> Result<String, String> {
            let optional = if required.contains(key.as_str()) {
                ""
            } else {
                "?"
            };
            Ok(format!(
                "{}readonly {}{optional}: {};",
                indent(depth),
                quoted(key),
                ts_type(schema, depth)?
            ))
        })
        .collect()
}

/// The declaration's `Event`, after its `=`: one case for each event, its name,
/// and its data where it carries some.
fn event_type(events: &[TakenEvent]) -> String {
    if events.is_empty() {
        return " never".to_string();
    }
    events
        .iter()
        .map(|event| {
            let name = quoted(&event.name);
            match event.schema {
                Some(_) => {
                    format!("\n  | {{ readonly name: {name}; readonly data: Payloads[{name}] }}")
                }
                None => format!("\n  | {{ readonly name: {name} }}"),
            }
        })
        .collect()
}

/// The declaration's `StateId`: the id of every state of the machine, a
/// history pseudo-state left out, since no configuration holds one.
fn state_type(model: &SCXMLModel) -> String {
    let ids: Vec<String> = model
        .states
        .keys()
        .filter(|id| !model.history_states.contains_key(*id))
        .map(|id| quoted(id))
        .collect();
    if ids.is_empty() {
        return "never".to_string();
    }
    ids.join(" | ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn typed(schema: Value) -> String {
        ts_type(&schema, 0).expect("the schema has a type")
    }

    #[test]
    fn each_value_of_a_payload_schema_is_typed_as_typescript_holds_it() {
        assert_eq!(
            typed(json!({"type": "integer", "minimum": 0, "maximum": 255})),
            "number"
        );
        assert_eq!(typed(json!({"type": "number"})), "number");
        assert_eq!(
            typed(json!({"type": "string", "pattern": "^(0|[1-9][0-9]*)$"})),
            "string"
        );
        assert_eq!(typed(json!({"type": "boolean"})), "boolean");
        assert_eq!(
            typed(json!({"enum": ["open", "in_progress"]})),
            "\"open\" | \"in_progress\""
        );
        assert_eq!(typed(json!({"const": "circle"})), "\"circle\"");
        assert_eq!(
            typed(json!({"anyOf": [{"type": "string"}, {"type": "null"}]})),
            "string | null"
        );
        assert_eq!(
            typed(json!({"$ref": "#/$defs/a~1b~0c"})),
            "Definitions[\"a/b~c\"]"
        );
        assert_eq!(
            typed(json!({"type": "array", "items": {"type": "boolean"}, "maxItems": 2})),
            "readonly (boolean)[]"
        );
    }

    #[test]
    fn a_record_is_an_object_of_its_keys_each_read_only() {
        let schema = json!({
            "type": "object",
            "properties": {"id": {"type": "integer"}, "tag": {"const": "circle"}},
            "required": ["id", "tag"],
            "additionalProperties": false
        });
        assert_eq!(
            ts_type(&schema, 1).unwrap(),
            "{\n    readonly \"id\": number;\n    readonly \"tag\": \"circle\";\n  }"
        );
    }

    #[test]
    fn a_key_the_object_does_not_require_may_be_left_out() {
        let schema = json!({
            "type": "object",
            "properties": {"a": {"type": "integer"}, "b": {"type": "boolean"}},
            "required": ["a"]
        });
        assert_eq!(
            ts_type(&schema, 0).unwrap(),
            "{\n  readonly \"a\": number;\n  readonly \"b\"?: boolean;\n}"
        );
    }

    #[test]
    fn a_schema_of_no_type_is_not_given_one() {
        assert!(ts_type(&json!({"type": "date"}), 0).is_err());
        assert!(ts_type(&json!({"maximum": 3}), 0).is_err());
        assert!(ts_type(&json!({"$ref": "#/elsewhere"}), 0).is_err());
        assert!(ts_type(&json!("integer"), 0).is_err());
    }

    #[test]
    fn two_definitions_of_one_name_that_differ_are_refused() {
        let mut held = Definitions::default();
        held.take(&json!({"$defs": {"item": {"type": "object"}}}))
            .unwrap();
        held.take(&json!({"$defs": {"item": {"type": "object"}}}))
            .unwrap();
        assert!(held
            .take(&json!({"$defs": {"item": {"type": "array", "items": {"type": "null"}}}}))
            .is_err());
    }

    #[test]
    fn an_event_that_carries_data_is_raised_with_it_and_another_without() {
        let events = [
            TakenEvent {
                name: "go".to_string(),
                schema: None,
            },
            TakenEvent {
                name: "set".to_string(),
                schema: Some("{}".to_string()),
            },
        ];
        assert_eq!(
            event_type(&events),
            "\n  | { readonly name: \"go\" }\n  | { readonly name: \"set\"; readonly data: Payloads[\"set\"] }"
        );
        assert_eq!(event_type(&[]), " never");
    }
}
