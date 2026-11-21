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
//! A variable is stated in the form a saved state writes it in, and is compared
//! here as the type the document declares it in, read through the reader the
//! machine publishes it by, in the C the lowering holds it in:
//!
//!   - a number or a bool as itself, a 64-bit integer stated as its digits too;
//!   - a real by its 64 bits against the double the scenario states (a scenario
//!     states a real exactly or not at all), so the image holds no formatter;
//!   - a value of an enum by the number its enum document declares for the name;
//!   - a text and a byte string by their length and their bytes (a byte string
//!     is stated as its Latin-1 text);
//!   - a value that may be absent by its presence, then its value;
//!   - a list by its length, then each element;
//!   - a record field by field, and a union by its tag, then the fields of its
//!     case (`tag` and `as.<case>`);
//!   - a map or a set by its length, then each entry in the map's order, walked
//!     through the runtime's view (`entries`, `first`, each slot's
//!     `links.next`), a map's entry `[key, value]` and a set's its key.
//!
//! What the target does not judge is a variable the machine does not publish and
//! what a machine asked of its host, sent its peers or handed its parent
//! (`host_calls`, `sent`, `child_hosts`, `donedata`): the table names each one,
//! and the host C11 lane judges it on the same scenario. A step the target cannot
//! make at all is refused, since a table without it would replay another
//! scenario: a `restore` (saved state is a hosted unit, docs/adr/0110 D2, which
//! the image does not link) and an `invoke` (the image routes no event to a
//! child, and no machine of the gate's set starts one).
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
use crate::forge::composed_lowering::{c11_case_ident, C11_UNION_CASES, C11_UNION_TAG};
use crate::forge::model::{Direction, EnumModel, SceType};
use crate::forge::value_type::{ScopedEnum, ScopedRecord, ScopedUnion, TypeOf, TypeScope};
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
    /// How many checks the target makes: one for each state, end, count,
    /// presence, tag and scalar the scenario states.
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
    let scope = crate::forge::value_path::statechart_scope(model);
    let enums = model.held_enums();
    let seeded = crate::forge::static_lowering::takes_map_seed(model, Some(document_dir));

    let mut table = Table {
        names: &names,
        model,
        scope: &scope,
        enums: &enums,
        judged: 0,
        walks: 0,
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
        "{} step(s) replayed, {judged} check(s) made on the target",
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

/// The C condition that holds when `expr`, a number, is not `n`: compared as an
/// unsigned 64-bit number where `unsigned` says its type is one or `n` is past
/// what a signed one holds, and as a signed one otherwise.
fn number_differs(expr: &str, n: i128, unsigned: bool) -> Result<String, String> {
    if unsigned || n > i128::from(i64::MAX) {
        let n = u64::try_from(n).map_err(|_| format!("{n} is no unsigned 64-bit number"))?;
        Ok(format!("(uint64_t)({expr}) != UINT64_C({n})"))
    } else {
        let n = i64::try_from(n).map_err(|_| format!("{n} is no signed 64-bit number"))?;
        Ok(format!("(int64_t)({expr}) != {}", signed_literal(n)))
    }
}

/// `bytes` as a C string literal, each byte an octal escape: a byte string may
/// hold any byte, and an octal escape never runs into the character after it.
fn bytes_literal(bytes: &[u8]) -> String {
    let mut literal = String::from("\"");
    for byte in bytes {
        let _ = write!(literal, "\\{byte:03o}");
    }
    literal.push('"');
    literal
}

/// The bytes of a byte string stated as its Latin-1 text, the form a saved state
/// writes one in: each character one byte, none past U+00FF.
fn latin1_bytes(text: &str) -> Option<Vec<u8>> {
    text.chars()
        .map(|c| u8::try_from(u32::from(c)).ok())
        .collect()
}

/// The first of `scopes` that declares the enum `alias`.
fn enum_in<'s>(scopes: &[&'s TypeScope], alias: &str) -> Option<&'s ScopedEnum> {
    scopes.iter().find_map(|scope| scope.enums.get(alias))
}

/// The first of `scopes` that declares the record `alias`.
fn record_in<'s>(scopes: &[&'s TypeScope], alias: &str) -> Option<&'s ScopedRecord> {
    scopes.iter().find_map(|scope| scope.records.get(alias))
}

/// The first of `scopes` that declares the union `alias`.
fn union_in<'s>(scopes: &[&'s TypeScope], alias: &str) -> Option<&'s ScopedUnion> {
    scopes.iter().find_map(|scope| scope.unions.get(alias))
}

/// `inner` searched before `outer`: a type a record or a union holds is read in
/// its own scope first, then in the one it was named in.
fn within<'s>(inner: &'s TypeScope, outer: &[&'s TypeScope]) -> Vec<&'s TypeScope> {
    std::iter::once(inner)
        .chain(outer.iter().copied())
        .collect()
}

/// One line of a judge: a check, which holds when its condition does not and
/// otherwise answers `what`, or a statement the checks after it read (the
/// position a walk of a map has reached).
enum Line {
    Check { condition: String, what: String },
    Code(String),
}

/// The table being written: what it has judged and left so far.
struct Table<'a> {
    names: &'a Names,
    model: &'a SCXMLModel,
    scope: &'a TypeScope,
    /// Every enum the machine holds, by alias: where a value of an enum no scope
    /// declares is read.
    enums: &'a BTreeMap<String, EnumModel>,
    judged: usize,
    /// How many walks of a map the table has written, each of which names the
    /// position it has reached by its number.
    walks: usize,
    unjudged: Vec<String>,
}

impl Table<'_> {
    /// The body of step `number`'s act: the event it sends, then the time it
    /// moves the machine's clock on by, in the order the host's replay makes them.
    /// `None` for a step that does neither.
    fn act(&self, number: usize, step: &Map<String, Value>) -> Result<Option<String>, String> {
        if step.contains_key("restore") {
            return Err(format!(
                "step {number} states `restore`, which the target cannot make: saved state is a \
                 hosted unit (docs/adr/0110, D2), which the image does not link"
            ));
        }
        if step.contains_key("invoke") {
            return Err(format!(
                "step {number} states `invoke`, which the target cannot make: the image routes no \
                 event to a child, and no machine of the gate's set starts one"
            ));
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
        let mut lines = Vec::new();
        if let Some(expect) = step.get("expect") {
            let expect = expect
                .as_object()
                .ok_or_else(|| format!("step {number}'s `expect` is not an object"))?;
            for (member, value) in expect {
                self.expectation(number, member, value, &mut lines)?;
            }
        }
        let mut body = String::new();
        for line in lines {
            match line {
                Line::Check { condition, what } => {
                    self.judged += 1;
                    let _ = writeln!(
                        body,
                        "    if ({condition}) {{\n        SCE_MCU_FAIL({});\n    }}",
                        c_literal(&format!("step {number}: {what}"))
                    );
                }
                Line::Code(code) => {
                    let _ = writeln!(body, "    {code}");
                }
            }
        }
        Ok(body)
    }

    /// The lines of the member `member` of a step's `expect`, stated as `value`.
    fn expectation(
        &mut self,
        number: usize,
        member: &str,
        value: &Value,
        lines: &mut Vec<Line>,
    ) -> Result<(), String> {
        let sm = self.names.sm.clone();
        match member {
            "state" => {
                let state = value
                    .as_str()
                    .ok_or_else(|| format!("step {number}'s `state` is not a state id"))?;
                lines.push(self.state_active(number, state)?);
            }
            "configuration" => {
                let states = value
                    .as_array()
                    .ok_or_else(|| format!("step {number}'s `configuration` is not an array"))?;
                for state in states {
                    let state = state.as_str().ok_or_else(|| {
                        format!(
                            "step {number}'s `configuration` holds something that is no state id"
                        )
                    })?;
                    lines.push(self.state_active(number, state)?);
                }
                lines.push(Line::Check {
                    condition: format!("sce_mcu_bits({sm}_active_states(m)) != {}u", states.len()),
                    what: format!("other than {} state(s) are active", states.len()),
                });
            }
            "ended" => {
                let ended = value
                    .as_bool()
                    .ok_or_else(|| format!("step {number}'s `ended` is not a bool"))?;
                let is_final = format!("{sm}_is_in_final_state(m)");
                lines.push(if ended {
                    Line::Check {
                        condition: format!("!{is_final}"),
                        what: "the run has not ended".to_string(),
                    }
                } else {
                    Line::Check {
                        condition: is_final,
                        what: "the run has ended".to_string(),
                    }
                });
            }
            "variables" => {
                let variables = value
                    .as_object()
                    .ok_or_else(|| format!("step {number}'s `variables` is not an object"))?;
                for (name, value) in variables {
                    self.variable(number, name, value, lines)?;
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
        Ok(())
    }

    fn state_active(&self, number: usize, state: &str) -> Result<Line, String> {
        if !self.model.states.contains_key(state) {
            return Err(format!(
                "step {number} names the state `{state}`, which `{}` does not declare",
                self.names.sm
            ));
        }
        Ok(Line::Check {
            condition: format!(
                "!{}_in_state(m, {})",
                self.names.sm,
                self.names.state(state)
            ),
            what: format!("state `{state}` is not active"),
        })
    }

    /// The checks of the variable `name` stated as `value`, or its place among
    /// the expectations left to the host when the machine publishes no reader of it.
    fn variable(
        &mut self,
        number: usize,
        name: &str,
        value: &Value,
        lines: &mut Vec<Line>,
    ) -> Result<(), String> {
        let declared = variable(self.model, name).ok_or_else(|| {
            format!(
                "step {number} states the variable `{name}`, which `{}` does not declare",
                self.names.sm
            )
        })?;
        let (Some(Direction::Out), Some(ty)) = (&declared.direction, &declared.value_type) else {
            // No reader: the machine does not publish it.
            self.unjudged
                .push(format!("step {number}: variable `{name}`"));
            return Ok(());
        };
        let read = self.names.read(name);
        let scope = self.scope;
        self.value_checks(
            &[scope],
            &format!("`{name}`"),
            &read,
            false,
            ty,
            value,
            lines,
        )
        .map_err(|e| format!("step {number}: {e}"))
    }

    /// The checks `value` states of the value `read` reads, of the type `ty`
    /// whose names `scopes` declare, pushed onto `lines`. `held` says the value
    /// is held inside a struct (a record's field, a list's element, a map's entry,
    /// a present value): a text is then its buffer, whose `data` is the text,
    /// where a variable's reader answers the text itself. `place` names the value
    /// in a failure.
    #[allow(clippy::too_many_arguments)]
    fn value_checks<B>(
        &mut self,
        scopes: &[&TypeScope],
        place: &str,
        read: &str,
        held: bool,
        ty: &TypeOf<B>,
        value: &Value,
        lines: &mut Vec<Line>,
    ) -> Result<(), String> {
        match ty {
            TypeOf::Scalar(scalar) => {
                self.scalar_checks(scopes, place, read, held, scalar, value, lines)
            }
            TypeOf::Text { .. } => {
                lines.push(text_check(place, read, held, value)?);
                Ok(())
            }
            TypeOf::Bytes { .. } => {
                lines.push(bytes_check(place, read, value)?);
                Ok(())
            }
            TypeOf::Optional { elem } => {
                if value.is_null() {
                    lines.push(Line::Check {
                        condition: format!("{read}.present"),
                        what: format!("{place} is present, and the scenario states it absent"),
                    });
                    return Ok(());
                }
                lines.push(Line::Check {
                    condition: format!("!{read}.present"),
                    what: format!("{place} is absent, and the scenario states a value"),
                });
                self.value_checks(
                    scopes,
                    place,
                    &format!("{read}.value"),
                    true,
                    &**elem,
                    value,
                    lines,
                )
            }
            TypeOf::List { elem, .. } => {
                let elements = value
                    .as_array()
                    .ok_or_else(|| format!("{place} is a list, and {value} is not an array"))?;
                // The count first: an element past it is not read.
                lines.push(Line::Check {
                    condition: format!("{read}.len != {}u", elements.len()),
                    what: format!("{place} does not hold {} element(s)", elements.len()),
                });
                for (at, element) in elements.iter().enumerate() {
                    self.value_checks(
                        scopes,
                        &format!("{place}[{at}]"),
                        &format!("{read}.data[{at}]"),
                        true,
                        &**elem,
                        element,
                        lines,
                    )?;
                }
                Ok(())
            }
            TypeOf::Record { alias } => {
                let record = record_in(scopes, alias)
                    .ok_or_else(|| format!("{place} is a record `{alias}` no scope declares"))?;
                let inner = within(&record.scope, scopes);
                self.fields_checks(&inner, place, read, record, value, &[], lines)
            }
            TypeOf::Union { alias } => {
                let union = union_in(scopes, alias)
                    .ok_or_else(|| format!("{place} is a union `{alias}` no scope declares"))?;
                let inner = within(&union.scope, scopes);
                self.union_checks(&inner, place, read, union, value, lines)
            }
            TypeOf::Map {
                key,
                value: held_value,
                ..
            } => {
                let entries = value.as_array().ok_or_else(|| {
                    format!("{place} is a map, and {value} is not an array of entries")
                })?;
                let key: &TypeOf<B> = key;
                let held_value: &TypeOf<B> = held_value;
                self.walk(
                    place,
                    read,
                    entries,
                    lines,
                    |table, slot, at, entry, lines| {
                        let pair = entry
                            .as_array()
                            .filter(|pair| pair.len() == 2)
                            .ok_or_else(|| format!("{place}'s entry {at} is not `[key, value]`"))?;
                        table.value_checks(
                            scopes,
                            &format!("{place}'s key {at}"),
                            &format!("{slot}.key"),
                            true,
                            key,
                            &pair[0],
                            lines,
                        )?;
                        table.value_checks(
                            scopes,
                            &format!("{place}'s value {at}"),
                            &format!("{slot}.value"),
                            true,
                            held_value,
                            &pair[1],
                            lines,
                        )
                    },
                )
            }
            TypeOf::Set { key, .. } => {
                let entries = value.as_array().ok_or_else(|| {
                    format!("{place} is a set, and {value} is not an array of keys")
                })?;
                let key: &TypeOf<B> = key;
                self.walk(
                    place,
                    read,
                    entries,
                    lines,
                    |table, slot, at, entry, lines| {
                        table.value_checks(
                            scopes,
                            &format!("{place}'s key {at}"),
                            &format!("{slot}.key"),
                            true,
                            key,
                            entry,
                            lines,
                        )
                    },
                )
            }
        }
    }

    /// The walk of the map or set `read` reads, stated as `entries`: its length,
    /// then each entry in the map's order, the slot of each handed to `entry` as
    /// the C place `slot`. The position starts at the view's `first` and moves to
    /// each slot's `links.next`; the slot at position `p` is `entries[p - 1]`
    /// (`sce/forge/map.h`).
    fn walk(
        &mut self,
        place: &str,
        read: &str,
        entries: &[Value],
        lines: &mut Vec<Line>,
        mut entry: impl FnMut(&mut Self, &str, usize, &Value, &mut Vec<Line>) -> Result<(), String>,
    ) -> Result<(), String> {
        lines.push(Line::Check {
            condition: format!("{read}.len != {}u", entries.len()),
            what: format!("{place} does not hold {} entr(ies)", entries.len()),
        });
        if entries.is_empty() {
            return Ok(());
        }
        self.walks += 1;
        let at = format!("sce_mcu_at_{}", self.walks);
        lines.push(Line::Code("{".to_string()));
        lines.push(Line::Code(format!("uint32_t {at} = {read}.first;")));
        for (index, stated) in entries.iter().enumerate() {
            lines.push(Line::Check {
                condition: format!("{at} == 0u"),
                what: format!("{place} ends before its entry {index}"),
            });
            let slot = format!("{read}.entries[{at} - 1u]");
            entry(self, &slot, index, stated, lines)?;
            lines.push(Line::Code(format!("{at} = {slot}.links.next;")));
        }
        lines.push(Line::Code("}".to_string()));
        Ok(())
    }

    /// The checks of the fields `value` states of the record `record`, whose
    /// fields `read` holds, but for `skip` (a union's tag, which is no field of
    /// its case).
    #[allow(clippy::too_many_arguments)]
    fn fields_checks(
        &mut self,
        scopes: &[&TypeScope],
        place: &str,
        read: &str,
        record: &ScopedRecord,
        value: &Value,
        skip: &[&str],
        lines: &mut Vec<Line>,
    ) -> Result<(), String> {
        let fields = value
            .as_object()
            .ok_or_else(|| format!("{place} is a record, and {value} is not an object"))?;
        for (field, stated) in fields {
            if skip.contains(&field.as_str()) {
                continue;
            }
            let (_, ty) = record
                .fields
                .iter()
                .find(|(id, _)| id == field)
                .ok_or_else(|| format!("{place} has no field `{field}`"))?;
            if !is_c_identifier(field) {
                return Err(format!("{place}'s field `{field}` is no C member name"));
            }
            self.value_checks(
                scopes,
                &format!("{place}.{field}"),
                &format!("{read}.{field}"),
                true,
                ty,
                stated,
                lines,
            )?;
        }
        Ok(())
    }

    /// The checks of the union `union` stated as `value`: its tag, by the number
    /// the tag's enum declares for the case the scenario names, then the fields of
    /// that case's record, held in `as.<case>`.
    fn union_checks(
        &mut self,
        scopes: &[&TypeScope],
        place: &str,
        read: &str,
        union: &ScopedUnion,
        value: &Value,
        lines: &mut Vec<Line>,
    ) -> Result<(), String> {
        let object = value
            .as_object()
            .ok_or_else(|| format!("{place} is a union, and {value} is not an object"))?;
        let variant = object
            .get(&union.tag)
            .and_then(Value::as_str)
            .ok_or_else(|| format!("{place} states no case under `{}`", union.tag))?;
        let case = union
            .cases
            .iter()
            .find(|case| case.variant.name == variant)
            .ok_or_else(|| {
                format!("{place} states the case `{variant}`, which the union has not")
            })?;
        let unsigned = enum_in(scopes, &union.tag_enum).is_some_and(|e| e.underlying.is_unsigned());
        lines.push(Line::Check {
            condition: number_differs(
                &format!("{read}.{C11_UNION_TAG}"),
                case.variant.value,
                unsigned,
            )?,
            what: format!("{place} is not the case `{variant}`"),
        });
        let case_read = format!("{read}.{C11_UNION_CASES}.{}", c11_case_ident(variant));
        match case.record.as_deref() {
            Some(held) => {
                let record = record_in(scopes, held).ok_or_else(|| {
                    format!("{place}'s case `{variant}` holds a record `{held}` no scope declares")
                })?;
                let inner = within(&record.scope, scopes);
                self.fields_checks(
                    &inner,
                    place,
                    &case_read,
                    record,
                    value,
                    &[union.tag.as_str()],
                    lines,
                )
            }
            None if object.len() == 1 => Ok(()),
            None => Err(format!(
                "{place}'s case `{variant}` holds no field, and the scenario states some"
            )),
        }
    }

    /// The check of a scalar `ty` read by `read`, stated as `value`.
    #[allow(clippy::too_many_arguments)]
    fn scalar_checks(
        &self,
        scopes: &[&TypeScope],
        place: &str,
        read: &str,
        held: bool,
        ty: &SceType,
        value: &Value,
        lines: &mut Vec<Line>,
    ) -> Result<(), String> {
        let what = format!("{place} is not {value}");
        let condition = match ty {
            SceType::Bool => match value.as_bool() {
                Some(true) => format!("!({read})"),
                Some(false) => format!("({read})"),
                None => return Err(format!("{place}: {value} is not a bool")),
            },
            SceType::Uint8 | SceType::Uint16 | SceType::Uint32 | SceType::Uint64 => {
                let n = unsigned_of(value)
                    .ok_or_else(|| format!("{place}: {value} is not an unsigned integer"))?;
                format!("(uint64_t)({read}) != UINT64_C({n})")
            }
            SceType::Int8 | SceType::Int16 | SceType::Int32 | SceType::Int64 => {
                let n = signed_of(value)
                    .ok_or_else(|| format!("{place}: {value} is not an integer"))?;
                format!("(int64_t)({read}) != {}", signed_literal(n))
            }
            // The 64 bits of the double the scenario states, which a float32 is
            // widened to exactly, as a saved state writes it.
            SceType::Float32 | SceType::Float64 => {
                let real = value
                    .as_f64()
                    .ok_or_else(|| format!("{place}: {value} is not a real"))?;
                format!(
                    "sce_mcu_real_bits((double)({read})) != UINT64_C(0x{:016x})",
                    real.to_bits()
                )
            }
            SceType::String => {
                lines.push(text_check(place, read, held, value)?);
                return Ok(());
            }
            SceType::Bytes => {
                lines.push(bytes_check(place, read, value)?);
                return Ok(());
            }
            SceType::Enum(reference) => {
                let alias = &reference.alias;
                let name = value
                    .as_str()
                    .ok_or_else(|| format!("{place}: {value} is not the name of a value"))?;
                // The enum as the scope the value was named in declares it, else as the
                // machine holds it (its imports and its schemas' enums).
                let (number, unsigned) = match enum_in(scopes, alias) {
                    Some(declared) => (
                        declared
                            .variants
                            .iter()
                            .find(|v| v.name == name)
                            .map(|v| v.value),
                        declared.underlying.is_unsigned(),
                    ),
                    None => {
                        let declared = self.enums.get(alias).ok_or_else(|| {
                            format!(
                                "{place} is a value of the enum `{alias}`, which no scope declares"
                            )
                        })?;
                        (
                            declared
                                .variants
                                .iter()
                                .find(|v| v.name == name)
                                .map(|v| v.value),
                            declared.underlying_type.is_unsigned(),
                        )
                    }
                };
                let number =
                    number.ok_or_else(|| format!("{place}: `{name}` is no value of `{alias}`"))?;
                number_differs(read, number, unsigned)?
            }
        };
        lines.push(Line::Check { condition, what });
        Ok(())
    }
}

/// The check of a text read by `read`, stated as `value`: its bytes and its
/// length. `held` says `read` is the buffer that holds the text, whose `data` is
/// the text; otherwise it is the text.
fn text_check(place: &str, read: &str, held: bool, value: &Value) -> Result<Line, String> {
    let text = value
        .as_str()
        .ok_or_else(|| format!("{place}: {value} is not a text"))?;
    let got = if held {
        format!("{read}.data")
    } else {
        read.to_string()
    };
    Ok(Line::Check {
        condition: format!(
            "!sce_mcu_text_is({got}, {}, {}u)",
            bytes_literal(text.as_bytes()),
            text.len()
        ),
        what: format!("{place} is not {value}"),
    })
}

/// The check of a byte string read by `read` (its `data` and its `len`), stated
/// as its Latin-1 text `value`.
fn bytes_check(place: &str, read: &str, value: &Value) -> Result<Line, String> {
    let bytes = value
        .as_str()
        .and_then(latin1_bytes)
        .ok_or_else(|| format!("{place}: {value} is not the Latin-1 text of a byte string"))?;
    Ok(Line::Check {
        condition: format!(
            "!sce_mcu_bytes_are({read}.data, {read}.len, (const uint8_t *){}, {}u)",
            bytes_literal(&bytes),
            bytes.len()
        ),
        what: format!("{place} is not {value}"),
    })
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
        let path = document.to_str().expect("a UTF-8 path");
        let mut model = crate::parser::SCXMLParser::new()
            .parse_file(path)
            .expect("the fixture parses");
        // As `sce-codegen scenario-table` reads it: analysed, as the generator does.
        crate::analyzer::analyze(&mut model, path);
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
        assert!(table.unjudged.is_empty(), "{:?}", table.unjudged);
        // Six states stated, twelve variables and one end: nineteen checks.
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

    /// A machine that waits on a delayed send is driven by its clock: the scenario
    /// of the gate's `plain_footprint_probe` moves it 4 ms and then 1 ms, and
    /// each move is the image's counter advanced and the machine ticked and
    /// stepped. A model read without the analysis waited on nothing, and this
    /// scenario was refused on the target.
    #[test]
    fn a_scenario_that_moves_the_clock_drives_the_machine_s_timer() {
        let table = table_of(
            "tests/fixtures/no_std/plain_footprint_probe.scxml",
            "../backends/c/mcu/scenarios/plain_footprint_probe.json",
        );
        assert_eq!(table.steps, 4, "every step of the scenario is a row");
        for line in [
            "sce_mcu_clock_advance(UINT64_C(4));",
            "sce_mcu_clock_advance(UINT64_C(1));",
            "plain_footprint_probe_tick(m);",
        ] {
            assert!(table.source.contains(line), "{line}");
        }
    }

    #[test]
    fn a_map_is_walked_entry_by_entry_in_its_order() {
        let table = table_of(
            "tests/fixtures/static_datamodel/static_map.scxml",
            "tests/fixtures/static_datamodel/scenarios/static_map.json",
        );
        assert!(
            table.unjudged.is_empty(),
            "the map and the enum are judged on the target: {:?}",
            table.unjudged
        );
        // The map is held under the host lane's seed, so its machine starts with it.
        assert!(table.source.contains(
            "static_map_init((static_map_t *)machine, sce_map_seed_from_halves(UINT64_C(0x0123456789abcdef), UINT64_C(0xfedcba9876543210)))"
        ));
        // Step 4 states `[[7, 5]]`, the first map with an entry: its length, the walk
        // from the view's first position, the key and the value of the slot there,
        // and the position after it.
        for line in [
            "static_map_get_amounts(m).len != 1u",
            "uint32_t sce_mcu_at_1 = static_map_get_amounts(m).first;",
            "if (sce_mcu_at_1 == 0u)",
            "(uint64_t)(static_map_get_amounts(m).entries[sce_mcu_at_1 - 1u].key) != UINT64_C(7)",
            "(int64_t)(static_map_get_amounts(m).entries[sce_mcu_at_1 - 1u].value) != INT64_C(5)",
            "sce_mcu_at_1 = static_map_get_amounts(m).entries[sce_mcu_at_1 - 1u].links.next;",
            // An empty map is its length alone.
            "static_map_get_amounts(m).len != 0u",
        ] {
            assert!(
                table.source.contains(line),
                "no `{line}` in:\n{}",
                table.source
            );
        }
        // The enum by the number its document declares for the name.
        assert!(table.source.contains("(static_map_get_removed(m)) != "));
        assert!(table
            .source
            .contains("(int64_t)(static_map_get_seven(m)) != INT64_C(5)"));
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
    }

    #[test]
    fn a_union_and_a_map_keyed_by_a_record_are_judged() {
        let table = table_of(
            "tests/fixtures/static_datamodel/static_map_take.scxml",
            "tests/fixtures/static_datamodel/scenarios/static_map_take.json",
        );
        assert!(table.unjudged.is_empty(), "{:?}", table.unjudged);
        for line in [
            "(uint64_t)(static_map_take_get_resets(m).entries[sce_mcu_at_1 - 1u].key.low) != UINT64_C(1)",
            "(uint64_t)(static_map_take_get_resets(m).entries[sce_mcu_at_1 - 1u].value) != UINT64_C(1)",
            "(uint64_t)(static_map_take_get_took(m).as.duplicate.last) != UINT64_C(1)",
            "static_map_take_get_took(m).tag) != ",
            "(uint64_t)(static_map_take_get_generation(m)) != UINT64_C(2)",
        ] {
            assert!(table.source.contains(line), "no `{line}` in:\n{}", table.source);
        }
    }

    #[test]
    fn a_record_and_a_list_of_records_are_judged_field_by_field() {
        let table = table_of(
            "tests/fixtures/static_datamodel/static_state_algorithm.scxml",
            "tests/fixtures/static_datamodel/scenarios/static_state_algorithm.json",
        );
        assert!(table.unjudged.is_empty(), "{:?}", table.unjudged);
        for line in [
            "(int64_t)(static_state_algorithm_get_total(m).sum) != INT64_C(50)",
            "static_state_algorithm_get_entries(m).len != 2u",
            "(int64_t)(static_state_algorithm_get_entries(m).data[1].amount) != INT64_C(150)",
            "(uint64_t)(static_state_algorithm_get_ids(m).data[2]) != UINT64_C(3)",
        ] {
            assert!(
                table.source.contains(line),
                "no `{line}` in:\n{}",
                table.source
            );
        }
    }

    #[test]
    fn a_text_bytes_a_real_and_an_absent_value_are_spelled_exactly() {
        let text =
            text_check("`name`", "m_get_name(m)", false, &Value::from("a\u{e9}")).expect("a text");
        let Line::Check { condition, .. } = text else {
            panic!("a text is a check")
        };
        assert_eq!(
            condition,
            "!sce_mcu_text_is(m_get_name(m), \"\\141\\303\\251\", 3u)"
        );
        let bytes =
            bytes_check("`raw`", "m_get_raw(m)", &Value::from("\u{0}\u{ff}")).expect("bytes");
        let Line::Check { condition, .. } = bytes else {
            panic!("bytes are a check")
        };
        assert_eq!(
            condition,
            "!sce_mcu_bytes_are(m_get_raw(m).data, m_get_raw(m).len, (const uint8_t *)\"\\000\\377\", 2u)"
        );
        assert!(
            bytes_check("`raw`", "r", &Value::from("\u{100}")).is_err(),
            "a character past U+00FF is no byte"
        );
        assert_eq!(
            number_differs("e", 3, true).expect("a number"),
            "(uint64_t)(e) != UINT64_C(3)"
        );
        assert_eq!(
            number_differs("e", -1, false).expect("a number"),
            "(int64_t)(e) != INT64_C(-1)"
        );
        let mut real = Vec::new();
        let table = Table {
            names: &Names {
                sm: "m".to_string(),
                upper: "M".to_string(),
            },
            model: &SCXMLModel::default(),
            scope: &TypeScope::default(),
            enums: &BTreeMap::new(),
            judged: 0,
            walks: 0,
            unjudged: Vec::new(),
        };
        table
            .scalar_checks(
                &[],
                "`r`",
                "m_get_r(m)",
                false,
                &SceType::Float32,
                &Value::from(0.5),
                &mut real,
            )
            .expect("a real");
        let [Line::Check { condition, .. }] = real.as_slice() else {
            panic!("a real is one check")
        };
        assert_eq!(
            condition,
            "sce_mcu_real_bits((double)(m_get_r(m))) != UINT64_C(0x3fe0000000000000)"
        );
    }

    #[test]
    fn a_step_the_target_cannot_make_is_refused() {
        let document = fixture("tests/fixtures/static_datamodel/static_counter.scxml");
        let path = document.to_str().expect("a UTF-8 path");
        let mut model = crate::parser::SCXMLParser::new()
            .parse_file(path)
            .expect("the fixture parses");
        crate::analyzer::analyze(&mut model, path);
        let dir = document.parent().expect("a directory");
        let clock = serde_json::json!({"machine": "static_counter", "steps": [{"advance_ms": 1}]});
        let refused = render_c11_scenario_table(&model, "static_counter", dir, &clock)
            .expect_err("a machine with no delayed send has no clock to move");
        assert!(refused.contains("waits on none"), "{refused}");
        let restore =
            serde_json::json!({"machine": "static_counter", "steps": [{"restore": true}]});
        let refused = render_c11_scenario_table(&model, "static_counter", dir, &restore)
            .expect_err("a restore is refused");
        assert!(
            refused.contains("`restore`") && refused.contains("hosted unit"),
            "{refused}"
        );
        let invoke = serde_json::json!({"machine": "static_counter", "steps": [{"event": "tick", "invoke": "child"}]});
        let refused = render_c11_scenario_table(&model, "static_counter", dir, &invoke)
            .expect_err("an event for a child is refused");
        assert!(refused.contains("`invoke`"), "{refused}");
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
