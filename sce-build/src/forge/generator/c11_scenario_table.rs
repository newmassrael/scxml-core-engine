// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A replay scenario (`crate::scenario_replay`) as a C11 table, for a program that
//! reads no file (docs/adr/0110, D3 item 3): the target of the `c11-mcu` gate, a
//! Cortex-M4 under QEMU whose image links no C library.
//!
//! The host C11 lane reads the scenario's JSON when it runs
//! (`backends/c/tests/integration/static_scenario.h`). A freestanding image has
//! neither a file nor a JSON reader, so the scenario is read here, when the gate
//! generates the machine, and what it states becomes C the machine's own types
//! check: each step is the act it makes (the event sent in, the time moved on)
//! and a judge that answers the first expectation that does not hold. The
//! runner that walks the table is the gate's (`backends/c/mcu/sce_mcu_scenario.h`),
//! and every name the table calls is one the machine's header declares: the
//! state and event constants, `_init`, `_raise_external`, `_step`, `_tick`,
//! `_in_state`, `_active_states`, `_is_in_final_state` and the readers of its
//! published variables (`_get_<id>`).
//!
//! A value is compared as the type the document declares it in, read through the
//! reader the machine publishes it by: a number or a bool, a list of them, and a
//! record or a list of records whose fields are. What has no reader on the
//! target or no comparison written here (a map, a set, a union, an enum, a text,
//! a real, a value that may be absent, a variable the machine does not publish,
//! what a machine asked of its host or sent its peers) is not judged on the
//! target: the table names each one, and the host C11 lane judges it on the same
//! scenario. A step the target cannot make at all (a restore, an event for a
//! child) is refused, since a table without it would replay another scenario.
//!
//! Each piece of text the runner writes is a literal with its length beside it:
//! the image has no `strlen`.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use serde_json::{Map, Value};

use super::c11_test_template::c_literal;
use crate::filters;
use crate::forge::model::{Direction, EntrySchema, SceType};
use crate::forge::value_type::{TypeOf, WrittenType};
use crate::model::{SCXMLModel, Variable};

/// The seed every map of a tabled machine hashes its keys under: the host C11
/// lane's (`MAP_SCENARIO_SEED` in `test_static_scalars.c`), which nothing a
/// scenario observes depends on.
const MAP_SEED_HALVES: (&str, &str) = (
    "UINT64_C(0x0123456789abcdef)",
    "UINT64_C(0xfedcba9876543210)",
);

/// A scenario as a C11 translation unit, and what it judges.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct C11ScenarioTable {
    /// The translation unit: the machine's storage, a start, one act and one
    /// judge for each step, the table of steps and `sce_mcu_scenario`, the one
    /// definition the runner reads.
    pub source: String,
    /// How many steps the table replays: every step of the scenario.
    pub steps: usize,
    /// How many expectations the target judges.
    pub judged: usize,
    /// Each expectation the target does not judge, as `step <n>: <what>`; the
    /// host C11 lane judges it on the same scenario.
    pub unjudged: Vec<String>,
}

/// The C11 table of `scenario` for the machine of `model`, generated from the
/// document whose file stem is `stem` and which sits in `document_dir` (where the
/// algorithms it imports are read from, to tell whether it takes a map seed).
///
/// A scenario the vocabulary or the machine refuses is refused here too: the
/// caller holds it to [`crate::scenario_replay::problems`] and
/// [`crate::scenario_replay::machine_problems`] first, and what this refuses on
/// top is what the target cannot do (a restore, an event for a child, a machine
/// with a script engine) or a value that is not of its variable's type.
pub fn render_c11_scenario_table(
    model: &SCXMLModel,
    stem: &str,
    document_dir: &Path,
    scenario: &Value,
) -> Result<C11ScenarioTable, String> {
    if model.needs_script_engine {
        return Err(format!(
            "`{}` needs a script engine, which a freestanding image does not hold",
            model.name
        ));
    }
    let steps = scenario
        .get("steps")
        .and_then(Value::as_array)
        .filter(|steps| !steps.is_empty())
        .ok_or_else(|| "the scenario has no steps".to_string())?;
    let names = Names::of(model);
    let records = model.entry_records();
    let seeded = crate::forge::static_lowering::takes_map_seed(model, Some(document_dir));

    let mut table = Table {
        names: &names,
        model,
        records: &records,
        judged: 0,
        unjudged: Vec::new(),
    };
    let mut functions = String::new();
    let mut rows = Vec::new();
    for (index, step) in steps.iter().enumerate() {
        let number = index + 1;
        let step = step
            .as_object()
            .ok_or_else(|| format!("step {number} is not an object"))?;
        let act = table.act(number, step)?;
        let judge = table.judge(number, step)?;
        let act_name = match act {
            Some(body) => {
                let name = format!("{}_mcu_act_{number}", names.sm);
                let _ = write!(
                    functions,
                    "static bool {name}(void *machine) {{\n    {sm}_t *m = ({sm}_t *)machine;\n{body}    return true;\n}}\n\n",
                    sm = names.sm
                );
                name
            }
            None => "NULL".to_string(),
        };
        let judge_name = format!("{}_mcu_judge_{number}", names.sm);
        let _ = write!(
            functions,
            "static const sce_mcu_text_t *{judge_name}(const void *machine) {{\n    const {sm}_t *m = (const {sm}_t *)machine;\n    (void)m;\n{judge}    return NULL;\n}}\n\n",
            sm = names.sm
        );
        rows.push(format!(
            "    {{SCE_MCU_TEXT({}), {act_name}, {judge_name}}},\n",
            c_literal(&format!("step {number}"))
        ));
    }

    let Table {
        judged, unjudged, ..
    } = table;
    let summary = format!(
        "{} step(s) replayed, {judged} expectation(s) judged on the target",
        steps.len()
    );
    let left = if unjudged.is_empty() {
        "{NULL, 0u}".to_string()
    } else {
        format!(
            "SCE_MCU_TEXT({})",
            c_literal(&format!(
                "{} expectation(s) judged by the host C11 lane, not here: {}",
                unjudged.len(),
                unjudged.join("; ")
            ))
        )
    };
    let seed = if seeded {
        format!(
            ", sce_map_seed_from_halves({}, {})",
            MAP_SEED_HALVES.0, MAP_SEED_HALVES.1
        )
    } else {
        String::new()
    };
    let seed_include = if seeded {
        "#include <sce/forge/map.h>\n"
    } else {
        ""
    };
    let sm = &names.sm;
    let upper = &names.upper;
    let machine = scenario
        .get("machine")
        .and_then(Value::as_str)
        .unwrap_or(stem);

    let mut source = String::new();
    let _ = write!(
        source,
        "/* The replay scenario of `{machine}` as a table (docs/adr/0110, D3 item 3), written by\n   \
`sce-codegen scenario-table`. Do not edit: the scenario's JSON is the one answer every\n   \
engine is held to, and this is that answer read once, for an image that reads no file. */\n\n\
#include <stdbool.h>\n#include <stddef.h>\n#include <stdint.h>\n\n\
{seed_include}#include \"sce_mcu_scenario.h\"\n#include \"{stem}_sm.h\"\n\n\
/* The machine, in static storage: an image's stack does not hold one. */\n\
static {sm}_t {sm}_mcu_machine;\n\n\
static bool {sm}_mcu_start(void *machine) {{\n    \
return {sm}_init(({sm}_t *)machine{seed}) == {upper}_START_OK;\n}}\n\n\
{functions}\
static const sce_mcu_step_t {sm}_mcu_steps[] = {{\n{rows}}};\n\n\
const sce_mcu_scenario_t sce_mcu_scenario = {{\n    \
SCE_MCU_TEXT({name}),\n    \
&{sm}_mcu_machine,\n    \
{sm}_mcu_start,\n    \
{sm}_mcu_steps,\n    \
sizeof({sm}_mcu_steps) / sizeof({sm}_mcu_steps[0]),\n    \
SCE_MCU_TEXT({summary}),\n    \
{left},\n}};\n",
        name = c_literal(machine),
        rows = rows.concat(),
        summary = c_literal(&summary),
    );
    Ok(C11ScenarioTable {
        source,
        steps: steps.len(),
        judged,
        unjudged,
    })
}

/// The machine's C names: its prefix, the prefix of its constants, and the
/// constant of each state and event by its document name.
struct Names {
    sm: String,
    upper: String,
}

impl Names {
    fn of(model: &SCXMLModel) -> Self {
        Self {
            sm: model.name.clone(),
            upper: model.name.to_uppercase(),
        }
    }

    /// The constant of the state `id` (`<MACHINE>_STATE_<ID>`, as the header's
    /// state enumeration spells it).
    fn state(&self, id: &str) -> String {
        format!(
            "{}_STATE_{}",
            self.upper,
            filters::to_c11_upper_ident(Cow::Borrowed(id))
        )
    }

    /// The constant of the event `name` (`<MACHINE>_EVENT_<NAME>`).
    fn event(&self, name: &str) -> String {
        format!(
            "{}_EVENT_{}",
            self.upper,
            filters::to_c11_upper_ident(Cow::Borrowed(name))
        )
    }

    /// The reader the machine publishes the variable `id` by
    /// (`CTarget::reader_name`, `<machine>_get_<id>`), called on `m`.
    fn read(&self, id: &str) -> String {
        format!("{}_get_{}(m)", self.sm, id.replace(['.', '-'], "_"))
    }
}

/// The event a machine delivers a name sent to it as: the name
/// itself when its events hold it, else the longest token prefix of it they do,
/// the rule the header's `_resolve_event_by_name` keeps; `None` for a name the
/// machine drops.
fn resolve_event<'a>(model: &'a SCXMLModel, name: &str) -> Option<&'a str> {
    let mut candidate = name;
    loop {
        if let Some(found) = model.events.get(candidate) {
            return Some(found.as_str());
        }
        candidate = &candidate[..candidate.rfind('.')?];
    }
}

/// Every variable the document declares, top-level and per state.
fn variable<'a>(model: &'a SCXMLModel, id: &str) -> Option<&'a Variable> {
    model
        .variables
        .iter()
        .chain(model.states.values().flat_map(|s| s.datamodel.iter()))
        .find(|v| v.id == id)
}

/// Whether `text` is a C identifier, which a record's field must be to be named
/// as a member of the struct that holds it.
fn is_c_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// An unsigned integer a scenario states as a number, or as the text of its
/// digits (the form a saved state writes a 64-bit integer in).
fn unsigned_of(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
}

/// A signed integer, as [`unsigned_of`] reads an unsigned one.
fn signed_of(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
}

/// `n` as a C11 constant of `int64_t`: the least one is spelled as a sum, since
/// its digits alone are a constant too large for the type.
fn signed_literal(n: i64) -> String {
    if n == i64::MIN {
        "(INT64_C(-9223372036854775807) - 1)".to_string()
    } else {
        format!("INT64_C({n})")
    }
}

/// The C condition that holds when `expr`, a value of the scalar `ty`, is not
/// `value`; `None` for a scalar compared nowhere on the target (a real, a text,
/// bytes, a value of an enum).
fn differs(expr: &str, ty: &SceType, value: &Value) -> Result<Option<String>, String> {
    match ty {
        SceType::Bool => match value.as_bool() {
            Some(true) => Ok(Some(format!("!({expr})"))),
            Some(false) => Ok(Some(format!("({expr})"))),
            None => Err(format!("{value} is not a bool")),
        },
        SceType::Uint8 | SceType::Uint16 | SceType::Uint32 | SceType::Uint64 => {
            let n =
                unsigned_of(value).ok_or_else(|| format!("{value} is not an unsigned integer"))?;
            Ok(Some(format!("(uint64_t)({expr}) != UINT64_C({n})")))
        }
        SceType::Int8 | SceType::Int16 | SceType::Int32 | SceType::Int64 => {
            let n = signed_of(value).ok_or_else(|| format!("{value} is not an integer"))?;
            Ok(Some(format!("(int64_t)({expr}) != {}", signed_literal(n))))
        }
        SceType::Float32
        | SceType::Float64
        | SceType::String
        | SceType::Bytes
        | SceType::Enum(_) => Ok(None),
    }
}

/// The table being written: what it has judged and left so far.
struct Table<'a> {
    names: &'a Names,
    model: &'a SCXMLModel,
    records: &'a BTreeMap<String, EntrySchema>,
    judged: usize,
    unjudged: Vec<String>,
}

impl Table<'_> {
    /// The body of step `number`'s act: the event it sends, then the time it
    /// moves the machine's clock on by, in the order the host's replay makes them.
    /// `None` for a step that does neither.
    fn act(&self, number: usize, step: &Map<String, Value>) -> Result<Option<String>, String> {
        for refused in ["restore", "invoke"] {
            if step.contains_key(refused) {
                return Err(format!(
                    "step {number} states `{refused}`, which the target cannot make: it neither saves \
                     a machine nor routes an event to a child"
                ));
            }
        }
        let sm = &self.names.sm;
        let mut body = String::new();
        if let Some(event) = step.get("event") {
            let event = event
                .as_str()
                .ok_or_else(|| format!("step {number}'s `event` is not a name"))?;
            let dropped = step.get("dropped").and_then(Value::as_bool) == Some(true);
            match (resolve_event(self.model, event), dropped) {
                // A name the machine drops is not sent: the machine would refuse it
                // where its door resolves the name, which is what the step states.
                (None, true) => {}
                (None, false) => {
                    return Err(format!(
                        "step {number} sends `{event}`, which no event of `{sm}` resolves"
                    ))
                }
                (Some(_), true) => {
                    return Err(format!(
                    "step {number} expects `{event}` dropped, and an event of `{sm}` resolves it"
                ))
                }
                (Some(resolved), false) => {
                    let data = match step.get("data") {
                        None => String::new(),
                        Some(data) => {
                            format!("        meta.data = {};\n", c_literal(&data.to_string()))
                        }
                    };
                    let _ = write!(
                        body,
                        "    {{\n        {sm}_event_with_meta_t meta = {{0}};\n        meta.event = {};\n{data}        {sm}_raise_external(m, &meta);\n        {sm}_step(m);\n    }}\n",
                        self.names.event(resolved)
                    );
                }
            }
        }
        if let Some(advance) = step.get("advance_ms") {
            let ms = advance
                .as_u64()
                .ok_or_else(|| format!("step {number}'s `advance_ms` is not a time"))?;
            if !self.model.needs_event_scheduler_driving() {
                return Err(format!(
                    "step {number} moves the clock of `{sm}`, which waits on none"
                ));
            }
            // The image's clock is the counter behind `_sce_clock_now_ms`; the
            // machine reads it at its next tick and runs what that made due.
            let _ = write!(
                body,
                "    sce_mcu_clock_advance(UINT64_C({ms}));\n    {sm}_tick(m);\n    {sm}_step(m);\n"
            );
        }
        Ok((!body.is_empty()).then_some(body))
    }

    /// The body of step `number`'s judge: one check for each expectation the
    /// target judges, each returning the sentence of its failure.
    fn judge(&mut self, number: usize, step: &Map<String, Value>) -> Result<String, String> {
        let mut body = String::new();
        let Some(expect) = step.get("expect") else {
            return Ok(body);
        };
        let expect = expect
            .as_object()
            .ok_or_else(|| format!("step {number}'s `expect` is not an object"))?;
        for (member, value) in expect {
            match member.as_str() {
                "state" => {
                    let state = value
                        .as_str()
                        .ok_or_else(|| format!("step {number}'s `state` is not a state id"))?;
                    self.state_active(number, state, &mut body)?;
                }
                "configuration" => {
                    let states = value.as_array().ok_or_else(|| {
                        format!("step {number}'s `configuration` is not an array")
                    })?;
                    for state in states {
                        let state = state.as_str().ok_or_else(|| {
                            format!("step {number}'s `configuration` holds something that is no state id")
                        })?;
                        self.state_active(number, state, &mut body)?;
                    }
                    self.check(
                        number,
                        &format!(
                            "sce_mcu_bits({}_active_states(m)) != {}u",
                            self.names.sm,
                            states.len()
                        ),
                        &format!("other than {} state(s) are active", states.len()),
                        &mut body,
                    );
                }
                "ended" => {
                    let ended = value
                        .as_bool()
                        .ok_or_else(|| format!("step {number}'s `ended` is not a bool"))?;
                    let is_final = format!("{}_is_in_final_state(m)", self.names.sm);
                    let (condition, what) = if ended {
                        (format!("!{is_final}"), "the run has not ended")
                    } else {
                        (is_final, "the run has ended")
                    };
                    self.check(number, &condition, what, &mut body);
                }
                "variables" => {
                    let variables = value
                        .as_object()
                        .ok_or_else(|| format!("step {number}'s `variables` is not an object"))?;
                    for (name, value) in variables {
                        self.variable(number, name, value, &mut body)?;
                    }
                }
                "donedata" | "host_calls" | "child_hosts" | "sent" => {
                    self.unjudged.push(format!("step {number}: `{member}`"));
                }
                other => {
                    return Err(format!(
                        "step {number} expects `{other}`, which no replay reads"
                    ))
                }
            }
        }
        Ok(body)
    }

    /// One check: `condition` holds when the expectation does not, and `what`
    /// says which, in the words the runner writes.
    fn check(&mut self, number: usize, condition: &str, what: &str, body: &mut String) {
        self.judged += 1;
        let _ = writeln!(
            body,
            "    if ({condition}) {{\n        SCE_MCU_FAIL({});\n    }}",
            c_literal(&format!("step {number}: {what}"))
        );
    }

    fn state_active(
        &mut self,
        number: usize,
        state: &str,
        body: &mut String,
    ) -> Result<(), String> {
        if !self.model.states.contains_key(state) {
            return Err(format!(
                "step {number} names the state `{state}`, which `{}` does not declare",
                self.names.sm
            ));
        }
        let condition = format!(
            "!{}_in_state(m, {})",
            self.names.sm,
            self.names.state(state)
        );
        self.check(
            number,
            &condition,
            &format!("state `{state}` is not active"),
            body,
        );
        Ok(())
    }

    /// The checks of the variable `name` stated as `value`, or its place among
    /// the expectations left to the host.
    fn variable(
        &mut self,
        number: usize,
        name: &str,
        value: &Value,
        body: &mut String,
    ) -> Result<(), String> {
        let declared = variable(self.model, name).ok_or_else(|| {
            format!(
                "step {number} states the variable `{name}`, which `{}` does not declare",
                self.names.sm
            )
        })?;
        let left = format!("step {number}: variable `{name}`");
        let (Some(Direction::Out), Some(ty)) = (&declared.direction, &declared.value_type) else {
            // No reader: the machine does not publish it.
            self.unjudged.push(left);
            return Ok(());
        };
        let read = self.names.read(name);
        let mut checks = Vec::new();
        let judged = self.value_checks(name, &read, ty, value, &mut checks)?;
        if !judged {
            self.unjudged.push(left);
            return Ok(());
        }
        for (condition, what) in checks {
            self.check(number, &condition, &what, body);
        }
        Ok(())
    }

    /// The checks `value` stated for the value `read` reads, of the type `ty`,
    /// pushed onto `checks` as `(condition, what)`; false, with nothing pushed,
    /// for a value whose whole is not compared on the target.
    fn value_checks(
        &self,
        place: &str,
        read: &str,
        ty: &WrittenType,
        value: &Value,
        checks: &mut Vec<(String, String)>,
    ) -> Result<bool, String> {
        match ty {
            TypeOf::Scalar(scalar) => {
                let Some(condition) =
                    differs(read, scalar, value).map_err(|e| format!("`{place}`: {e}"))?
                else {
                    return Ok(false);
                };
                checks.push((condition, format!("`{place}` is not {value}")));
                Ok(true)
            }
            TypeOf::List { elem, .. } => {
                let elements = value
                    .as_array()
                    .ok_or_else(|| format!("`{place}` is a list, and {value} is not an array"))?;
                let mut inner = Vec::new();
                for (at, element) in elements.iter().enumerate() {
                    let element_read = format!("{read}.data[{at}]");
                    let element_place = format!("{place}[{at}]");
                    if !self.value_checks(
                        &element_place,
                        &element_read,
                        elem,
                        element,
                        &mut inner,
                    )? {
                        return Ok(false);
                    }
                }
                // The count first: an element past it is not read.
                checks.push((
                    format!("{read}.len != {}u", elements.len()),
                    format!("`{place}` does not hold {} element(s)", elements.len()),
                ));
                checks.extend(inner);
                Ok(true)
            }
            TypeOf::Record { alias } => {
                let schema = self.records.get(alias).ok_or_else(|| {
                    format!("`{place}` is a record of `{alias}`, which the machine does not import")
                })?;
                let fields = value.as_object().ok_or_else(|| {
                    format!("`{place}` is a record, and {value} is not an object")
                })?;
                let mut inner = Vec::new();
                for (field, stated) in fields {
                    let declared = schema
                        .fields
                        .iter()
                        .find(|f| f.id == *field)
                        .ok_or_else(|| format!("`{place}` has no field `{field}`"))?;
                    let Some(scalar) = declared.sce_type.scalar() else {
                        return Ok(false);
                    };
                    if !is_c_identifier(field) {
                        return Ok(false);
                    }
                    let field_read = format!("{read}.{field}");
                    let field_place = format!("{place}.{field}");
                    let Some(condition) = differs(&field_read, scalar, stated)
                        .map_err(|e| format!("`{field_place}`: {e}"))?
                    else {
                        return Ok(false);
                    };
                    inner.push((condition, format!("`{field_place}` is not {stated}")));
                }
                checks.extend(inner);
                Ok(true)
            }
            TypeOf::Text { .. }
            | TypeOf::Bytes { .. }
            | TypeOf::Optional { .. }
            | TypeOf::Union { .. }
            | TypeOf::Map { .. }
            | TypeOf::Set { .. } => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(rel: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
    }

    /// The table of a committed scenario for its committed machine, both read
    /// from the fixtures every lane replays.
    fn table_of(document: &str, scenario: &str) -> C11ScenarioTable {
        let document = fixture(document);
        let model = crate::parser::SCXMLParser::new()
            .parse_file(document.to_str().expect("a UTF-8 path"))
            .expect("the fixture parses");
        let text = std::fs::read_to_string(fixture(scenario)).expect("the scenario reads");
        let scenario: Value = serde_json::from_str(&text).expect("the scenario is JSON");
        let stem = document
            .file_stem()
            .and_then(|s| s.to_str())
            .expect("a stem");
        render_c11_scenario_table(
            &model,
            stem,
            document.parent().expect("a directory"),
            &scenario,
        )
        .expect("the scenario tables")
    }

    #[test]
    fn a_counter_scenario_is_judged_whole_on_the_target() {
        let table = table_of(
            "tests/fixtures/static_datamodel/static_counter.scxml",
            "tests/fixtures/static_datamodel/scenarios/static_counter.json",
        );
        assert_eq!(table.steps, 7, "every step of the scenario is a row");
        assert!(
            table.unjudged.is_empty(),
            "a counter's numbers and bool are all judged here: {:?}",
            table.unjudged
        );
        // Six states stated, twelve variables and one end: nineteen judged.
        assert_eq!(table.judged, 19);
        assert!(table
            .source
            .contains("meta.event = STATIC_COUNTER_EVENT_TICK;"));
        assert!(table
            .source
            .contains("(uint64_t)(static_counter_get_count(m)) != UINT64_C(5)"));
        assert!(table.source.contains("!(static_counter_get_ready(m))"));
        assert!(table
            .source
            .contains("!static_counter_in_state(m, STATIC_COUNTER_STATE_COUNTING)"));
        assert!(table
            .source
            .contains("!static_counter_is_in_final_state(m)"));
        assert!(table.source.contains(
            "return static_counter_init((static_counter_t *)machine) == STATIC_COUNTER_START_OK;"
        ));
    }

    #[test]
    fn a_map_is_left_to_the_host_and_named() {
        let table = table_of(
            "tests/fixtures/static_datamodel/static_map.scxml",
            "tests/fixtures/static_datamodel/scenarios/static_map.json",
        );
        assert!(table
            .unjudged
            .iter()
            .any(|what| what == "step 1: variable `amounts`"));
        assert!(table
            .unjudged
            .iter()
            .any(|what| what == "step 1: variable `removed`"));
        // The map is held under the host lane's seed, so its machine starts with it.
        assert!(table.source.contains(
            "static_map_init((static_map_t *)machine, sce_map_seed_from_halves(UINT64_C(0x0123456789abcdef), UINT64_C(0xfedcba9876543210)))"
        ));
        // The step's data, compact; the order of its members is the JSON map's own.
        assert!(
            table
                .source
                .contains("meta.data = \"{\\\"amount\\\":5,\\\"id\\\":7}\";")
                || table
                    .source
                    .contains("meta.data = \"{\\\"id\\\":7,\\\"amount\\\":5}\";"),
            "{}",
            table.source
        );
        assert!(table
            .source
            .contains("(int64_t)(static_map_get_seven(m)) != INT64_C(5)"));
    }

    #[test]
    fn a_record_and_a_list_of_records_are_judged_field_by_field() {
        let table = table_of(
            "tests/fixtures/static_datamodel/static_state_algorithm.scxml",
            "tests/fixtures/static_datamodel/scenarios/static_state_algorithm.json",
        );
        assert!(table
            .source
            .contains("(int64_t)(static_state_algorithm_get_total(m).sum) != INT64_C(50)"));
        assert!(table
            .source
            .contains("static_state_algorithm_get_entries(m).len != 2u"));
        assert!(table.source.contains(
            "(int64_t)(static_state_algorithm_get_entries(m).data[1].amount) != INT64_C(150)"
        ));
        assert!(table
            .source
            .contains("(uint64_t)(static_state_algorithm_get_ids(m).data[2]) != UINT64_C(3)"));
    }

    #[test]
    fn a_step_the_target_cannot_make_is_refused() {
        let document = fixture("tests/fixtures/static_datamodel/static_counter.scxml");
        let model = crate::parser::SCXMLParser::new()
            .parse_file(document.to_str().expect("a UTF-8 path"))
            .expect("the fixture parses");
        let dir = document.parent().expect("a directory");
        let restore =
            serde_json::json!({"machine": "static_counter", "steps": [{"restore": true}]});
        let refused = render_c11_scenario_table(&model, "static_counter", dir, &restore)
            .expect_err("a restore is refused");
        assert!(refused.contains("`restore`"), "{refused}");
        let unknown =
            serde_json::json!({"machine": "static_counter", "steps": [{"event": "nothing"}]});
        let refused = render_c11_scenario_table(&model, "static_counter", dir, &unknown)
            .expect_err("an event the machine drops is refused unless the step says so");
        assert!(refused.contains("`nothing`"), "{refused}");
        let dropped = serde_json::json!({"machine": "static_counter", "steps": [{"event": "nothing", "dropped": true}]});
        let table = render_c11_scenario_table(&model, "static_counter", dir, &dropped)
            .expect("a dropped event is a step with no act");
        assert!(table
            .source
            .contains("{SCE_MCU_TEXT(\"step 1\"), NULL, static_counter_mcu_judge_1}"));
        let wrong = serde_json::json!({"machine": "static_counter", "steps": [{"expect": {"variables": {"count": -1}}}]});
        let refused = render_c11_scenario_table(&model, "static_counter", dir, &wrong)
            .expect_err("a negative count is no value of uint32");
        assert!(refused.contains("`count`"), "{refused}");
    }
}
