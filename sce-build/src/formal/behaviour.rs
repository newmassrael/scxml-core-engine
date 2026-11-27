// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A behaviour TLC reports of a machine's properties module, read back as the
//! run every lane replays (docs/adr/0115, D15 and the converse of D8).
//!
//! TLC prints a behaviour that breaks a property as its states, each with the
//! action that reached it and the value of every variable: `State n: <Action
//! line .. of module ..>` and then one `/\ name = value` line per variable,
//! continued on the lines after it where the value is long. A temporal
//! property's behaviour ends in a cycle (`Back to state n: <Action ..>`) or in
//! the machine waiting for ever (`State n: Stuttering`). The properties module
//! names each kind of step by an action of its own (`PStart`, ..., `PArrive`,
//! `PDueAny`), so the action says which step of Appendix D a state was
//! reached by, and its variables say what the step did: `told`,
//! `configuration`, `currentEvent` and `outcome` as a lane's trace port
//! tells them, and the external queue an event from the environment joined.
//!
//! The run is written in the run trace format, with the events the
//! environment put on the queue as its inputs and the cycle it ends in, if any
//! ([`crate::run_trace`]): a scenario. A machine is deterministic in its
//! events, its moments and its host's answers, so each lane that replays the
//! inputs must take the same steps; one that does not is a defect of the
//! model, the generator or the lane, and is not a verdict about the document.

use std::collections::BTreeMap;

use crate::run_trace::{Fired, Input, InputKind, Microstep, RunTrace, Step};

/// The lane a scenario names: the model made the run, and no lane did.
pub const MODEL_LANE: &str = "model";

/// What a step of a behaviour of a properties module did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Taken {
    Start,
    Eventless,
    Internal,
    External,
    Stop,
    /// An event came from outside.
    Arrive,
    /// A delayed send came due.
    Due,
}

/// Each action of a properties module a step of a behaviour is reached by,
/// as [`super::properties`] writes it, and what the step did.
pub(super) const ACTIONS: [(&str, Taken); 7] = [
    ("PStart", Taken::Start),
    ("PEventless", Taken::Eventless),
    ("PInternal", Taken::Internal),
    ("PExternal", Taken::External),
    ("PStop", Taken::Stop),
    ("PArrive", Taken::Arrive),
    ("PDueAny", Taken::Due),
];

/// What `currentEvent` and `outcome` hold before there is one: AppendixD's
/// `None`, which is no XML name.
const NO_OUTCOME: &str = "<none>";

/// A value as TLC prints it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Str(String),
    /// A sequence, or a function of `1..n`, which TLC prints as one.
    Seq(Vec<Value>),
    Set(Vec<Value>),
    Record(Vec<(String, Value)>),
    /// A function of another domain, which TLC prints as its pairs:
    /// `(a :> 1 @@ b :> 2)`.
    Function(Vec<(Value, Value)>),
    /// A name TLC prints bare: a model value.
    Name(String),
}

/// One state of a behaviour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    /// The action that reached it, by its name; none for the first state.
    pub action: Option<String>,
    /// Each variable's value, as TLC printed it.
    pub variables: BTreeMap<String, String>,
}

/// A behaviour TLC reported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Behaviour {
    /// Its states, in order. A behaviour that ends in a cycle ends with the
    /// state the cycle returns to, reached again by the action that closes it.
    pub states: Vec<State>,
    /// The place, from 0, of the state the cycle returns to, for a behaviour
    /// that ends in one.
    pub back_to: Option<usize>,
}

/// The behaviour in a report of TLC, or why there is none to read.
pub fn read(output: &str) -> Result<Behaviour, String> {
    let mut states: Vec<State> = Vec::new();
    let mut back_to: Option<(usize, Option<String>)> = None;
    // The variable the last `/\ ` line began, while the state is open.
    let mut open: Option<String> = None;
    let mut inside = false;
    for line in output.lines() {
        if let Some(rest) = line.strip_prefix("State ") {
            if let Some((number, header)) = rest.split_once(": ") {
                if let Ok(number) = number.trim().parse::<usize>() {
                    if header.trim() == "Stuttering" {
                        inside = false;
                        open = None;
                        continue;
                    }
                    if number != states.len() + 1 {
                        return Err(format!(
                            "TLC's state {number} follows {} states of the behaviour",
                            states.len()
                        ));
                    }
                    states.push(State {
                        action: action_of(header),
                        variables: BTreeMap::new(),
                    });
                    inside = true;
                    open = None;
                    continue;
                }
            }
        }
        if let Some(rest) = line.strip_prefix("Back to state ") {
            let (number, header) = rest.split_once(':').unwrap_or((rest, ""));
            let number: usize = number
                .trim()
                .parse()
                .map_err(|_| format!("TLC's cycle returns to no state: {line:?}"))?;
            back_to = Some((number, action_of(header)));
            inside = false;
            open = None;
            continue;
        }
        if !inside {
            continue;
        }
        if line.trim().is_empty() {
            inside = false;
            open = None;
            continue;
        }
        let number = states.len();
        let state = states.last_mut().expect("a state is open");
        if let Some(rest) = line.strip_prefix("/\\ ") {
            let Some((name, value)) = rest.split_once(" = ") else {
                return Err(format!(
                    "state {number}: a variable TLC printed with no value: {line:?}"
                ));
            };
            let name = name.trim().to_string();
            state
                .variables
                .insert(name.clone(), value.trim().to_string());
            open = Some(name);
        } else if let Some(name) = &open {
            let value = state.variables.get_mut(name).expect("the open variable");
            value.push(' ');
            value.push_str(line.trim());
        } else {
            return Err(format!(
                "state {number}: a line that is no variable: {line:?}"
            ));
        }
    }
    if states.is_empty() {
        return Err("TLC's report holds no behaviour".to_string());
    }
    let back_to = match back_to {
        None => None,
        Some((number, action)) => {
            if number == 0 || number > states.len() {
                return Err(format!(
                    "TLC's cycle returns to state {number}, of {} states",
                    states.len()
                ));
            }
            let returned = states[number - 1].variables.clone();
            states.push(State {
                action,
                variables: returned,
            });
            Some(number - 1)
        }
    };
    Ok(Behaviour { states, back_to })
}

/// The name of the action a state's header names, `<PStart line 3, col 1 to
/// ... of module M>`; none for the initial predicate.
fn action_of(header: &str) -> Option<String> {
    let inner = header.trim().strip_prefix('<')?;
    let name: String = inner
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    (!name.is_empty() && !inner.starts_with("Initial predicate")).then_some(name)
}

/// `behaviour` as a scenario: its steps, the inputs its environment gave and
/// the cycle it ends in. `rows` is each row of the machine's transitions,
/// from 1, as [`super::properties::PropertiesModule::rows`] names it.
pub fn scenario(behaviour: &Behaviour, rows: &[Option<Fired>]) -> Result<RunTrace, String> {
    let mut steps: Vec<Step> = Vec::new();
    let mut inputs: Vec<Input> = Vec::new();
    let mut cycle = (behaviour.back_to == Some(0)).then_some(0);
    for (at, state) in behaviour.states.iter().enumerate().skip(1) {
        let place = at + 1;
        let read = Reading { state, place };
        let name = state
            .action
            .as_deref()
            .ok_or_else(|| format!("state {place} is reached by no action TLC names"))?;
        let taken = ACTIONS
            .iter()
            .find(|(action, _)| *action == name)
            .map(|(_, taken)| *taken)
            .ok_or_else(|| {
                format!("state {place} is reached by `{name}`, no action of a properties module")
            })?;
        match taken {
            Taken::Start => steps.push(Step::Start {
                entered: read.told("entered")?,
                configuration: read.configuration()?,
            }),
            Taken::Eventless => steps.push(Step::Eventless(read.microstep(rows)?)),
            Taken::Internal => steps.push(Step::Internal {
                event: read.event()?,
                microstep: read.microstep(rows)?,
            }),
            Taken::External => steps.push(Step::External {
                event: read.event()?,
                microstep: read.microstep(rows)?,
            }),
            Taken::Stop => steps.push(Step::Stop {
                exited: read.told("exited")?,
                outcome: read.outcome()?,
            }),
            Taken::Arrive | Taken::Due => {
                if steps.is_empty() {
                    return Err(format!(
                        "state {place}: an event came before the interpreter started"
                    ));
                }
                inputs.push(Input {
                    kind: if taken == Taken::Arrive {
                        InputKind::Arrive
                    } else {
                        InputKind::Due
                    },
                    after: steps.len(),
                    event: read.last_queued()?,
                });
            }
        }
        if behaviour.back_to == Some(at) {
            cycle = Some(steps.len());
        }
    }
    if steps.is_empty() {
        return Err("the behaviour takes no step of the interpreter".to_string());
    }
    if cycle.is_some_and(|start| start >= steps.len()) {
        return Err(
            "the behaviour's cycle takes no step of the interpreter, which TLC reports as \
             stuttering and never as a cycle"
                .to_string(),
        );
    }
    Ok(RunTrace {
        lane: MODEL_LANE.to_string(),
        steps,
        inputs,
        cycle,
    })
}

/// The variables of one state, read as a step says them.
struct Reading<'a> {
    state: &'a State,
    /// Its place in the behaviour, from 1, as TLC numbers it.
    place: usize,
}

impl Reading<'_> {
    fn value(&self, name: &str) -> Result<Value, String> {
        let text = self
            .state
            .variables
            .get(name)
            .ok_or_else(|| format!("state {}: TLC printed no `{name}`", self.place))?;
        value(text).map_err(|why| format!("state {}: `{name}` {why}", self.place))
    }

    fn refuse(&self, what: &str, value: &Value) -> String {
        format!("state {}: {what}, and TLC printed {value:?}", self.place)
    }

    /// A field of `told`, a sequence of states.
    fn told(&self, field: &str) -> Result<Vec<String>, String> {
        let told = self.value("told")?;
        let found = field_of(&told, field)
            .ok_or_else(|| self.refuse(&format!("`told` has no field `{field}`"), &told))?;
        strings(found)
            .ok_or_else(|| self.refuse(&format!("`told.{field}` is no sequence of states"), found))
    }

    fn configuration(&self) -> Result<Vec<String>, String> {
        let configuration = self.value("configuration")?;
        match &configuration {
            Value::Set(items) => items
                .iter()
                .map(|item| match item {
                    Value::Str(state) => Ok(state.clone()),
                    _ => Err(self.refuse("`configuration` is no set of states", &configuration)),
                })
                .collect(),
            _ => Err(self.refuse("`configuration` is no set of states", &configuration)),
        }
    }

    /// What the step's microstep did: the rows it took, as a run trace names
    /// them, what it exited and entered, and the configuration after it.
    fn microstep(&self, rows: &[Option<Fired>]) -> Result<Microstep, String> {
        let told = self.value("told")?;
        let fired = field_of(&told, "fired")
            .ok_or_else(|| self.refuse("`told` has no field `fired`", &told))?;
        let Value::Seq(taken) = fired else {
            return Err(self.refuse("`told.fired` is no sequence of rows", fired));
        };
        let fired = taken
            .iter()
            .map(|row| match row {
                Value::Int(number) => usize::try_from(*number)
                    .ok()
                    .and_then(|number| number.checked_sub(1))
                    .and_then(|index| rows.get(index))
                    .and_then(Option::clone)
                    .ok_or_else(|| {
                        format!(
                            "state {}: the step took row {number}, which is no written \
                             transition of the machine",
                            self.place
                        )
                    }),
                other => Err(self.refuse("`told.fired` holds what is no row", other)),
            })
            .collect::<Result<Vec<Fired>, String>>()?;
        Ok(Microstep {
            fired,
            exited: self.told("exited")?,
            entered: self.told("entered")?,
            configuration: self.configuration()?,
        })
    }

    /// The event the step took: `currentEvent` after it.
    fn event(&self) -> Result<String, String> {
        let event = self.value("currentEvent")?;
        event_name(&event).ok_or_else(|| self.refuse("`currentEvent` is no event", &event))
    }

    fn outcome(&self) -> Result<Option<String>, String> {
        match self.value("outcome")? {
            Value::Str(state) if state == NO_OUTCOME => Ok(None),
            Value::Str(state) => Ok(Some(state)),
            other => Err(self.refuse("`outcome` is no state", &other)),
        }
    }

    /// The event the environment put last on the external queue.
    fn last_queued(&self) -> Result<String, String> {
        let queue = self.value("externalQueue")?;
        match &queue {
            Value::Seq(events) => events
                .last()
                .and_then(event_name)
                .ok_or_else(|| self.refuse("the external queue ends in no event", &queue)),
            _ => Err(self.refuse("`externalQueue` is no sequence", &queue)),
        }
    }
}

/// The field `name` of a record.
fn field_of<'v>(record: &'v Value, name: &str) -> Option<&'v Value> {
    match record {
        Value::Record(fields) => fields
            .iter()
            .find(|(field, _)| field == name)
            .map(|(_, value)| value),
        _ => None,
    }
}

/// A sequence of strings.
fn strings(value: &Value) -> Option<Vec<String>> {
    match value {
        Value::Seq(items) => items
            .iter()
            .map(|item| match item {
                Value::Str(text) => Some(text.clone()),
                _ => None,
            })
            .collect(),
        _ => None,
    }
}

/// An event's name, which Appendix D holds as the sequence of its tokens.
fn event_name(value: &Value) -> Option<String> {
    let tokens = strings(value)?;
    (!tokens.is_empty() && tokens.iter().all(|token| !token.is_empty())).then(|| tokens.join("."))
}

/// A value as TLC prints it, or why it is none this reader reads.
pub fn value(text: &str) -> Result<Value, String> {
    let mut cursor = Cursor {
        text: text.as_bytes(),
        at: 0,
    };
    let read = cursor.value()?;
    cursor.space();
    if cursor.at != cursor.text.len() {
        return Err(format!(
            "is no value TLC prints: {:?} follows one",
            &text[cursor.at..]
        ));
    }
    Ok(read)
}

/// A place in the text of a value.
struct Cursor<'t> {
    text: &'t [u8],
    at: usize,
}

impl Cursor<'_> {
    fn space(&mut self) {
        while self.text.get(self.at).is_some_and(u8::is_ascii_whitespace) {
            self.at += 1;
        }
    }

    /// Whether the text goes on with `token`, which is then taken.
    fn take(&mut self, token: &str) -> bool {
        self.space();
        if self.text[self.at..].starts_with(token.as_bytes()) {
            self.at += token.len();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, token: &str) -> Result<(), String> {
        if self.take(token) {
            Ok(())
        } else {
            Err(format!(
                "is no value TLC prints: {token:?} is wanted at byte {}",
                self.at
            ))
        }
    }

    /// The items of a sequence or a set, up to `close`.
    fn items(&mut self, close: &str) -> Result<Vec<Value>, String> {
        let mut items = Vec::new();
        if self.take(close) {
            return Ok(items);
        }
        loop {
            items.push(self.value()?);
            if self.take(close) {
                return Ok(items);
            }
            self.expect(",")?;
        }
    }

    fn value(&mut self) -> Result<Value, String> {
        self.space();
        if self.take("<<") {
            return self.items(">>").map(Value::Seq);
        }
        if self.take("{") {
            return self.items("}").map(Value::Set);
        }
        if self.take("[") {
            let mut fields = Vec::new();
            if self.take("]") {
                return Ok(Value::Record(fields));
            }
            loop {
                let name = self.word();
                if name.is_empty() {
                    return Err(format!(
                        "is no value TLC prints: a field with no name at byte {}",
                        self.at
                    ));
                }
                self.expect("|->")?;
                fields.push((name, self.value()?));
                if self.take("]") {
                    return Ok(Value::Record(fields));
                }
                self.expect(",")?;
            }
        }
        if self.take("(") {
            let mut pairs = Vec::new();
            loop {
                let key = self.value()?;
                self.expect(":>")?;
                pairs.push((key, self.value()?));
                if self.take(")") {
                    return Ok(Value::Function(pairs));
                }
                self.expect("@@")?;
            }
        }
        match self.text.get(self.at) {
            Some(b'"') => self.string().map(Value::Str),
            Some(c) if c.is_ascii_digit() || *c == b'-' => self.integer().map(Value::Int),
            _ => match self.word().as_str() {
                "" => Err(format!(
                    "is no value TLC prints: nothing at byte {}",
                    self.at
                )),
                "TRUE" => Ok(Value::Bool(true)),
                "FALSE" => Ok(Value::Bool(false)),
                name => Ok(Value::Name(name.to_string())),
            },
        }
    }

    /// A name: letters, digits and `_`.
    fn word(&mut self) -> String {
        self.space();
        let from = self.at;
        while self
            .text
            .get(self.at)
            .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
        {
            self.at += 1;
        }
        String::from_utf8_lossy(&self.text[from..self.at]).into_owned()
    }

    fn integer(&mut self) -> Result<i64, String> {
        let from = self.at;
        if self.text.get(self.at) == Some(&b'-') {
            self.at += 1;
        }
        while self.text.get(self.at).is_some_and(u8::is_ascii_digit) {
            self.at += 1;
        }
        let digits = String::from_utf8_lossy(&self.text[from..self.at]).into_owned();
        digits
            .parse()
            .map_err(|_| format!("is no value TLC prints: {digits:?} is no integer"))
    }

    /// A string, its escapes undone.
    fn string(&mut self) -> Result<String, String> {
        self.at += 1;
        let mut bytes = Vec::new();
        loop {
            match self.text.get(self.at) {
                None => {
                    return Err("is no value TLC prints: a string that does not end".to_string())
                }
                Some(b'"') => {
                    self.at += 1;
                    return String::from_utf8(bytes)
                        .map_err(|_| "is no value TLC prints: a string of no text".to_string());
                }
                Some(b'\\') => {
                    let escaped = match self.text.get(self.at + 1) {
                        Some(b'n') => b'\n',
                        Some(b't') => b'\t',
                        Some(b'r') => b'\r',
                        Some(other) => *other,
                        None => {
                            return Err(
                                "is no value TLC prints: a string that does not end".to_string()
                            )
                        }
                    };
                    bytes.push(escaped);
                    self.at += 2;
                }
                Some(byte) => {
                    bytes.push(*byte);
                    self.at += 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One state as TLC prints it, with the variables a scenario reads.
    fn state(header: &str, told: &str, configuration: &str, event: &str, queue: &str) -> String {
        format!(
            "State {header}\n\
             /\\ arrived = 0\n\
             /\\ configuration = {configuration}\n\
             /\\ currentEvent = {event}\n\
             /\\ externalQueue = {queue}\n\
             /\\ outcome = \"<none>\"\n\
             /\\ told = {told}\n\n"
        )
    }

    const NO_TOLD: &str = "[exited |-> <<>>, fired |-> <<>>, entered |-> <<>>]";

    /// The rows of the turnstile of the fixture: `locked`'s coin, then
    /// `unlocked`'s coin, push and relock, then the document's own.
    fn rows() -> Vec<Option<Fired>> {
        let fired = |source: &str, index: u32| {
            Some(Fired {
                source: source.to_string(),
                index,
            })
        };
        vec![
            fired("locked", 0),
            fired("unlocked", 0),
            fired("unlocked", 1),
            fired("unlocked", 2),
            None,
        ]
    }

    /// A report of a violated invariant: the start, a coin from outside, the
    /// coin taken, and a delayed send come due.
    fn invariant_report() -> String {
        let mut text = "Error: Invariant Guarantee1 is violated.\n\
                        Error: The behavior up to this point is:\n"
            .to_string();
        text.push_str(&state(
            "1: <Initial predicate>",
            NO_TOLD,
            "{}",
            "<<>>",
            "<<>>",
        ));
        text.push_str(&state(
            "2: <PStart line 40, col 1 to line 40, col 48 of module turnstileProperties>",
            "[exited |-> <<>>, fired |-> <<5>>, entered |-> <<\"locked\">>]",
            "{\"locked\"}",
            "<<>>",
            "<<>>",
        ));
        text.push_str(&state(
            "3: <PArrive line 50, col 1 to line 50, col 80 of module turnstileProperties>",
            NO_TOLD,
            "{\"locked\"}",
            "<<>>",
            "<<<<\"coin\">>>>",
        ));
        // A value TLC breaks over lines, as it does a long one.
        text.push_str(&state(
            "4: <PExternal line 44, col 1 to line 44, col 54 of module turnstileProperties>",
            "[ exited |-> <<\"locked\">>,\n  fired |-> <<1>>,\n  entered |-> <<\"unlocked\">> ]",
            "{\"unlocked\"}",
            "<<\"coin\">>",
            "<<>>",
        ));
        text.push_str(&state(
            "5: <PDueAny line 52, col 1 to line 52, col 40 of module turnstileProperties>",
            "[exited |-> <<\"locked\">>, fired |-> <<1>>, entered |-> <<\"unlocked\">>]",
            "{\"unlocked\"}",
            "<<\"coin\">>",
            "<<<<\"relock\">>>>",
        ));
        text.push_str("6 states generated, 5 distinct states found, 0 states left on queue.\n");
        text
    }

    /// Turns red if a step of the interpreter is read as another kind, its
    /// rows otherwise than a run trace names them, or an event from the
    /// environment is lost or placed after another step.
    #[test]
    fn a_violation_is_read_as_its_steps_and_its_inputs() {
        let behaviour = read(&invariant_report()).expect("a behaviour");
        assert_eq!(behaviour.states.len(), 5);
        assert_eq!(behaviour.back_to, None);
        assert_eq!(behaviour.states[0].action, None);
        assert_eq!(behaviour.states[3].action.as_deref(), Some("PExternal"));
        let run = scenario(&behaviour, &rows()).expect("a scenario");
        assert_eq!(run.lane, MODEL_LANE);
        assert_eq!(
            run.steps,
            vec![
                Step::Start {
                    entered: vec!["locked".to_string()],
                    configuration: vec!["locked".to_string()],
                },
                Step::External {
                    event: "coin".to_string(),
                    microstep: Microstep {
                        fired: vec![Fired {
                            source: "locked".to_string(),
                            index: 0,
                        }],
                        exited: vec!["locked".to_string()],
                        entered: vec!["unlocked".to_string()],
                        configuration: vec!["unlocked".to_string()],
                    },
                },
            ]
        );
        assert_eq!(
            run.inputs,
            vec![
                Input {
                    kind: InputKind::Arrive,
                    after: 1,
                    event: "coin".to_string(),
                },
                Input {
                    kind: InputKind::Due,
                    after: 2,
                    event: "relock".to_string(),
                },
            ]
        );
        assert_eq!(run.cycle, None);
        assert_eq!(
            RunTrace::from_json(&run.to_json()).expect("the scenario reads"),
            run,
            "a scenario is a run trace"
        );
    }

    /// Turns red if a cycle loses the step that closes it or is said to
    /// start at another step.
    #[test]
    fn a_cycle_is_closed_by_its_step_and_starts_where_it_returns() {
        let mut text = invariant_report().replace(
            "6 states generated, 5 distinct states found, 0 states left on queue.\n",
            "",
        );
        text.push_str(
            "Back to state 4: <PExternal line 44, col 1 to line 44, col 54 of module turnstileProperties>\n",
        );
        let behaviour = read(&text).expect("a behaviour");
        assert_eq!(behaviour.back_to, Some(3));
        assert_eq!(behaviour.states.len(), 6);
        assert_eq!(behaviour.states[5].action.as_deref(), Some("PExternal"));
        assert_eq!(behaviour.states[5].variables, behaviour.states[3].variables);
        let run = scenario(&behaviour, &rows()).expect("a scenario");
        assert_eq!(run.steps.len(), 3);
        assert_eq!(
            run.cycle,
            Some(2),
            "the cycle starts after the coin is taken"
        );
        assert_eq!(run.steps[2].kind(), "external");
    }

    /// Turns red if a machine that waits for ever is read as a cycle, or
    /// what follows its stuttering is read as a state.
    #[test]
    fn a_behaviour_that_stutters_ends_where_it_waits() {
        let text = invariant_report().replace(
            "6 states generated",
            "State 6: Stuttering\n6 states generated",
        );
        let behaviour = read(&text).expect("a behaviour");
        assert_eq!(behaviour.states.len(), 5);
        assert_eq!(behaviour.back_to, None);
    }

    /// Turns red if a behaviour that cannot be the run of a properties module
    /// is written as a scenario.
    #[test]
    fn what_is_no_step_of_a_properties_module_is_refused() {
        let renamed = invariant_report().replace("<PExternal line", "<Next line");
        let refused = scenario(&read(&renamed).expect("a behaviour"), &rows())
            .expect_err("no action of a properties module");
        assert!(
            refused.contains("`Next`, no action of a properties module"),
            "{refused}"
        );
        let unwritten = invariant_report().replace("fired |-> <<1>>,\n", "fired |-> <<5>>,\n");
        let refused = scenario(&read(&unwritten).expect("a behaviour"), &rows())
            .expect_err("a row that is no written transition");
        assert!(
            refused.contains("row 5, which is no written transition"),
            "{refused}"
        );
        assert!(read("Model checking completed. No error has been found.").is_err());
        let skipped = invariant_report().replace("State 3:", "State 4:");
        assert!(read(&skipped).is_err(), "a state skipped is no behaviour");
    }

    /// Turns red if a value TLC prints is read as another.
    #[test]
    fn each_value_tlc_prints_is_read() {
        assert_eq!(
            value("[a |-> <<1, -2>>, b |-> {\"x\", \"y\\\"z\"}, c |-> TRUE]"),
            Ok(Value::Record(vec![
                (
                    "a".to_string(),
                    Value::Seq(vec![Value::Int(1), Value::Int(-2)])
                ),
                (
                    "b".to_string(),
                    Value::Set(vec![
                        Value::Str("x".to_string()),
                        Value::Str("y\"z".to_string())
                    ])
                ),
                ("c".to_string(), Value::Bool(true)),
            ]))
        );
        assert_eq!(
            value("(\"h1\" :> {} @@ \"h2\" :> {\"a\"})"),
            Ok(Value::Function(vec![
                (Value::Str("h1".to_string()), Value::Set(Vec::new())),
                (
                    Value::Str("h2".to_string()),
                    Value::Set(vec![Value::Str("a".to_string())])
                ),
            ]))
        );
        assert_eq!(value("<<>>"), Ok(Value::Seq(Vec::new())));
        assert_eq!(value("m1"), Ok(Value::Name("m1".to_string())));
        for refused in ["<<1, 2", "[a 1]", "\"open", "<<1>> 2", ""] {
            assert!(value(refused).is_err(), "{refused:?}");
        }
    }
}
