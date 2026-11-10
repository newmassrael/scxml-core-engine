// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The recorded input trace of a test case and its replay
//! (docs/SCE_TTCN3_LANE.md §10; docs/SCE_TEST_PROFILE.md §3.2, §3.3, §12.4):
//! the inputs of one run from the system under test, each at its logical
//! instant, `schemas/sce-test-trace.v1.schema.json`, and the system adapter
//! that plays them, [`TraceAdapter`].
//!
//! A trace is one text of the JSON Canonicalization Scheme, so that it has one
//! spelling and two traces are equal exactly when their bytes are:
//! `{"inputs":[{"at","port","type","value"}, …],"schema","suite","test_case"}`.
//! `at` is `NOW` in microseconds as the text of its digits, `port` a system
//! port of the test case's configuration, and `type` and `value` the record
//! type's name and the value in the canonical form a message record of the
//! test log writes (§12.5). The inputs stand in order of `at`; the inputs of
//! one instant in the order they are appended to their ports' queues (§3.2
//! step 2).
//!
//! The value of an input is read by its type: the reader of the test case's
//! configuration ([`InputReader`], generated as `input` in the configuration's
//! unit) knows which records each system port takes and reads each with the
//! reader of its record ([`super::from_canonical`]). A trace that does not fit
//! is refused whole before the run starts, which then ends with `error` (P13).

use std::collections::VecDeque;
use std::fmt;
use std::sync::{Arc, Mutex, PoisonError};

use super::from_canonical::decimal;
use super::port::{Instant, Message};
use super::run::SystemAdapter;
use crate::json::{self, Value};

/// The name of the trace's shape, its `schema`.
pub const TEST_TRACE_SCHEMA: &str = "sce-test-trace.v1";

/// The stability status of the trace's wire schema, pinned to the schema
/// file's own `x-sce-schema-status` by `schema_file_declares_status`.
/// `SCE_WIRE_CONTRACTS.md` requires one commit to move both.
pub const TEST_TRACE_SCHEMA_STATUS: &str = "pre-release";

/// Why a trace, or a value of one, is not read: the caller's input, refused by
/// name before anything of the run happens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceRefusal(String);

impl TraceRefusal {
    /// Refuse, saying why.
    pub fn new(reason: impl Into<String>) -> Self {
        Self(reason.into())
    }

    /// Why.
    pub fn reason(&self) -> &str {
        &self.0
    }

    /// An input of the record type `type_name` on `system_port`, which the
    /// configuration whose system ports are `system_ports` does not take: the
    /// port is none of them, or its type's `in` list does not name the type
    /// (docs/SCE_TTCN3_LANE.md §10.2).
    pub fn not_an_input(system_port: &str, type_name: &str, system_ports: &[&str]) -> Self {
        if system_ports.contains(&system_port) {
            Self(format!(
                "'{type_name}' is not on the `in` list of the type of the system port \
                 '{system_port}'"
            ))
        } else {
            Self(format!(
                "'{system_port}' is not a system port of the test case's configuration"
            ))
        }
    }
}

impl fmt::Display for TraceRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for TraceRefusal {}

/// The reader of the inputs of a test case's configuration: the message an
/// input of the record type `type_name` on the system port `system_port`
/// carries, its value read from the canonical form, or why it is not one.
///
/// Generated as `input` in the unit of a test configuration, which names the
/// records each of its system ports takes.
pub type InputReader = fn(&str, &str, &Value) -> Result<Message, TraceRefusal>;

/// The reader of a configuration with no system port: it refuses every input,
/// naming its port.
pub fn no_inputs(
    system_port: &str,
    type_name: &str,
    _value: &Value,
) -> Result<Message, TraceRefusal> {
    Err(TraceRefusal::not_an_input(system_port, type_name, &[]))
}

/// One input of a trace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceInput {
    /// `NOW` when it is due.
    pub at: Instant,
    /// The system port it arrives on.
    pub port: String,
    /// The name of its record type, as the test log names it.
    pub message_type: String,
    /// Its value, in the canonical form of the test log.
    pub value: Value,
}

/// A recorded input trace of one test case of one suite.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trace {
    /// The suite's name.
    pub suite: String,
    /// The test case's name.
    pub test_case: String,
    /// The inputs, in order of `at`, those of one instant in the order they
    /// are appended to their ports.
    pub inputs: Vec<TraceInput>,
}

/// What a slot of [`members`] holds until its member is read.
static UNREAD: Value = Value::Null;

/// The members of `value`, an object whose members are `names` exactly, each
/// in the order of `names`; refused, naming `what`, otherwise.
fn members<'v, const N: usize>(
    value: &'v Value,
    names: [&str; N],
    what: &str,
) -> Result<[&'v Value; N], TraceRefusal> {
    let Value::Object(members) = value else {
        return Err(TraceRefusal::new(format!("{what} is not an object")));
    };
    if let Some((stray, _)) = members
        .iter()
        .find(|(name, _)| !names.contains(&name.as_str()))
    {
        return Err(TraceRefusal::new(format!("{what} has no member '{stray}'")));
    }
    let mut found = [&UNREAD; N];
    for (slot, name) in found.iter_mut().zip(names) {
        *slot = value
            .member(name)
            .ok_or_else(|| TraceRefusal::new(format!("{what} has no '{name}'")))?;
    }
    Ok(found)
}

/// The text `value`, which is not empty, at `what`.
fn name_of(value: &Value, what: &str) -> Result<String, TraceRefusal> {
    match value {
        Value::Text(text) if !text.is_empty() => Ok(text.clone()),
        _ => Err(TraceRefusal::new(format!("{what} is not a name"))),
    }
}

/// An instant: the text of the decimal digits of a count of microseconds from
/// the start of the test case, with no leading zero, within `NOW`'s range.
fn instant_of(value: &Value, what: &str) -> Result<Instant, TraceRefusal> {
    let refused = || {
        TraceRefusal::new(format!(
            "{what} is not an instant, the text of the digits of a count of microseconds"
        ))
    };
    let Value::Text(digits) = value else {
        return Err(refused());
    };
    let count = decimal(digits)
        .filter(|count| *count >= 0)
        .ok_or_else(refused)?;
    Instant::try_from(count).map_err(|_| {
        TraceRefusal::new(format!(
            "{what} ({digits}) is past the microseconds the clock counts to"
        ))
    })
}

impl Trace {
    /// The trace `text` holds: the JSON Canonicalization Scheme of one trace,
    /// which a file may follow with one line feed, as a text file ends.
    ///
    /// Refused, naming why, where the text is not JSON, the shape is not
    /// `sce-test-trace.v1`, the inputs are out of the order of their instants,
    /// or the text is not the trace's one canonical spelling. Whether each
    /// input fits the suite is the configuration's to read
    /// ([`TraceAdapter::replaying`]).
    pub fn parse(text: &str) -> Result<Self, TraceRefusal> {
        let body = text.strip_suffix('\n').unwrap_or(text);
        let value = json::parse(body)
            .map_err(|refused| TraceRefusal::new(format!("the trace is {refused}")))?;
        let [schema, suite, test_case, inputs] = members(
            &value,
            ["schema", "suite", "test_case", "inputs"],
            "the trace",
        )?;
        if *schema != Value::Text(TEST_TRACE_SCHEMA.to_string()) {
            return Err(TraceRefusal::new(format!(
                "the trace's schema is not {TEST_TRACE_SCHEMA}"
            )));
        }
        let Value::Array(items) = inputs else {
            return Err(TraceRefusal::new("the trace's inputs are not an array"));
        };
        let mut read: Vec<TraceInput> = Vec::with_capacity(items.len());
        for (index, item) in items.iter().enumerate() {
            let what = format!("input {index}");
            let [at, port, message_type, value] =
                members(item, ["at", "port", "type", "value"], &what)?;
            let at = instant_of(at, &format!("the instant of {what}"))?;
            if let Some(before) = read.last().filter(|before| before.at > at) {
                return Err(TraceRefusal::new(format!(
                    "{what}, at {at}, comes after an input at {}: the inputs stand in order of \
                     their instants",
                    before.at
                )));
            }
            read.push(TraceInput {
                at,
                port: name_of(port, &format!("the port of {what}"))?,
                message_type: name_of(message_type, &format!("the type of {what}"))?,
                value: value.clone(),
            });
        }
        let trace = Trace {
            suite: name_of(suite, "the trace's suite")?,
            test_case: name_of(test_case, "the trace's test case")?,
            inputs: read,
        };
        if trace.canonical_text() != body {
            return Err(TraceRefusal::new(
                "the trace is not written in its one canonical text, the JSON Canonicalization \
                 Scheme",
            ));
        }
        Ok(trace)
    }

    /// The trace in its one canonical text, the JSON Canonicalization Scheme,
    /// with no line feed after it.
    ///
    /// # Panics
    ///
    /// When a value holds a number past the range of a double, or an object
    /// that names a member twice, which no canonical value holds and
    /// [`Trace::parse`] refuses.
    pub fn canonical_text(&self) -> String {
        let text = |text: &str| Value::Text(text.to_string());
        let inputs = self
            .inputs
            .iter()
            .map(|input| {
                Value::Object(vec![
                    ("at".to_string(), text(&input.at.to_string())),
                    ("port".to_string(), text(&input.port)),
                    ("type".to_string(), text(&input.message_type)),
                    ("value".to_string(), input.value.clone()),
                ])
            })
            .collect();
        let trace = Value::Object(vec![
            ("schema".to_string(), text(TEST_TRACE_SCHEMA)),
            ("suite".to_string(), text(&self.suite)),
            ("test_case".to_string(), text(&self.test_case)),
            ("inputs".to_string(), Value::Array(inputs)),
        ]);
        json::write_canonical(&trace).unwrap_or_else(|refused| {
            panic!(
                "the trace of {} {} is not canonical: {refused}",
                self.suite, self.test_case
            )
        })
    }
}

/// What a component sent to a system port, as a [`TraceAdapter`] records it.
#[derive(Clone, Debug)]
pub struct SystemOutput {
    /// `NOW` when it was sent.
    pub at: Instant,
    /// The system port it was sent to.
    pub system_port: String,
    /// The message.
    pub message: Message,
}

/// The record of what the components of a run sent to the system ports,
/// shared with the caller while the run owns the adapter.
pub type SystemOutputs = Arc<Mutex<Vec<SystemOutput>>>;

/// A system adapter that plays a trace (docs/SCE_TTCN3_LANE.md §10.5): at each
/// instant it delivers the inputs due then, in the trace's order, to their
/// system ports (docs/SCE_TEST_PROFILE.md §3.2 step 2), and it records what
/// the components send to the system ports.
///
/// It announces the instant of its next input ([`SystemAdapter::next_input`]),
/// so when no branch can fire the run's clock advances to the earlier of that
/// instant and the next timer expiry or the guard (§3.3, §12.4 step 4), and
/// the run is the same whenever it is played.
#[derive(Debug)]
pub struct TraceAdapter {
    inputs: VecDeque<(Instant, String, Message)>,
    sent: SystemOutputs,
}

impl TraceAdapter {
    /// The player of `trace`, each input's value read by `read`, the reader of
    /// the test case's configuration. Refused whole, naming the input, where
    /// one does not fit the configuration: its port is no system port of it,
    /// its type is not on the `in` list of that port's type, or its value does
    /// not belong to its type (docs/SCE_TTCN3_LANE.md §10.2).
    pub fn replaying(trace: &Trace, read: InputReader) -> Result<Self, TraceRefusal> {
        let mut inputs = VecDeque::with_capacity(trace.inputs.len());
        for (index, input) in trace.inputs.iter().enumerate() {
            let message =
                read(&input.port, &input.message_type, &input.value).map_err(|refused| {
                    TraceRefusal::new(format!(
                        "input {index}, at {}: {}",
                        input.at,
                        refused.reason()
                    ))
                })?;
            inputs.push_back((input.at, input.port.clone(), message));
        }
        Ok(TraceAdapter {
            inputs,
            sent: SystemOutputs::default(),
        })
    }

    /// What the components send to the system ports, in the order they send
    /// it: a handle that outlives the run, which owns the adapter.
    pub fn sent(&self) -> SystemOutputs {
        Arc::clone(&self.sent)
    }
}

impl SystemAdapter for TraceAdapter {
    fn send(&mut self, system_port: &str, message: &Message, now: Instant) {
        self.sent
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(SystemOutput {
                at: now,
                system_port: system_port.to_string(),
                message: message.clone(),
            });
    }

    fn next_input(&self) -> Option<Instant> {
        self.inputs.front().map(|(at, _, _)| *at)
    }

    fn take_inputs_due(&mut self, now: Instant) -> Vec<(String, Message)> {
        let mut due = Vec::new();
        while self.inputs.front().is_some_and(|(at, _, _)| *at <= now) {
            if let Some((_, port, message)) = self.inputs.pop_front() {
                due.push((port, message));
            }
        }
        due
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::canonical;

    const SCHEMA: &str = include_str!("../../../../../schemas/sce-test-trace.v1.schema.json");

    #[test]
    fn schema_file_declares_status() {
        let schema = json::parse(SCHEMA).expect("the trace's schema is JSON");
        assert_eq!(
            schema.member("x-sce-schema-status"),
            Some(&Value::Text(TEST_TRACE_SCHEMA_STATUS.to_string())),
            "schemas/sce-test-trace.v1.schema.json x-sce-schema-status disagrees with \
             TEST_TRACE_SCHEMA_STATUS; SCE_WIRE_CONTRACTS.md requires one commit to move both",
        );
        assert_eq!(
            schema
                .member("properties")
                .and_then(|properties| properties.member("schema"))
                .and_then(|shape| shape.member("const")),
            Some(&Value::Text(TEST_TRACE_SCHEMA.to_string())),
            "the schema pins the name of the shape the runtime reads",
        );
    }

    const ONE_INPUT: &str = concat!(
        "{\"inputs\":[{\"at\":\"500\",\"port\":\"sut\",\"type\":\"job_reply\",",
        "\"value\":{\"code\":\"0\",\"job\":\"7\"}}],",
        "\"schema\":\"sce-test-trace.v1\",\"suite\":\"runtime_suite\",",
        "\"test_case\":\"answers_the_job\"}"
    );

    /// A trace is read from its canonical text, with or without the line feed
    /// a file ends in, and written back as the same bytes.
    #[test]
    fn a_trace_is_read_from_its_canonical_text_and_written_back() {
        let trace = Trace::parse(ONE_INPUT).expect("a canonical trace");
        assert_eq!(trace.suite, "runtime_suite");
        assert_eq!(trace.test_case, "answers_the_job");
        assert_eq!(
            trace.inputs,
            [TraceInput {
                at: 500,
                port: "sut".to_string(),
                message_type: "job_reply".to_string(),
                // The members in the order the text writes them, which the
                // value read keeps.
                value: canonical::record([
                    ("code", Some(canonical::integer(0_u32))),
                    ("job", Some(canonical::integer(7_u32))),
                ]),
            }]
        );
        assert_eq!(trace.canonical_text(), ONE_INPUT);
        assert_eq!(Trace::parse(&format!("{ONE_INPUT}\n")), Ok(trace));
    }

    /// Each change of one member of a trace it first reads is refused, by
    /// name.
    #[test]
    fn a_trace_that_is_not_of_its_shape_is_refused_by_name() {
        let refused = |text: String| match Trace::parse(&text) {
            Ok(trace) => panic!("read {trace:?} from {text}"),
            Err(refusal) => refusal.reason().to_string(),
        };
        let changed = |from: &str, to: &str| {
            assert!(ONE_INPUT.contains(from), "{from}");
            ONE_INPUT.replacen(from, to, 1)
        };
        for (text, expected) in [
            (
                changed("sce-test-trace.v1", "sce-test-trace.v2"),
                "schema is not sce-test-trace.v1",
            ),
            (changed("\"500\"", "500"), "the instant of input 0"),
            (changed("\"500\"", "\"0500\""), "the instant of input 0"),
            (changed("\"500\"", "\"-5\""), "the instant of input 0"),
            (
                changed("\"500\"", "\"9223372036854775808\""),
                "past the microseconds",
            ),
            (
                changed("\"port\":\"sut\"", "\"port\":\"\""),
                "the port of input 0",
            ),
            (
                changed("\"type\"", "\"kind\""),
                "input 0 has no member 'kind'",
            ),
            (
                changed(",\"suite\":\"runtime_suite\"", ""),
                "the trace has no 'suite'",
            ),
            (changed("{\"inputs\"", "{ \"inputs\""), "one canonical text"),
            (changed("[{", "{"), "not JSON"),
        ] {
            let reason = refused(text);
            assert!(reason.contains(expected), "{expected}: {reason}");
        }
        let out_of_order = concat!(
            "{\"inputs\":[{\"at\":\"5\",\"port\":\"sut\",\"type\":\"t\",\"value\":{}},",
            "{\"at\":\"4\",\"port\":\"sut\",\"type\":\"t\",\"value\":{}}],",
            "\"schema\":\"sce-test-trace.v1\",\"suite\":\"s\",\"test_case\":\"c\"}"
        );
        assert!(
            refused(out_of_order.to_string()).contains("input 1, at 4, comes after an input at 5")
        );
    }

    /// An input on a port the configuration does not have, or of a type its
    /// port does not take, is refused, naming which.
    #[test]
    fn an_input_the_configuration_does_not_take_is_refused_by_name() {
        assert_eq!(
            TraceRefusal::not_an_input("sut", "job_request", &["sut"]).reason(),
            "'job_request' is not on the `in` list of the type of the system port 'sut'"
        );
        assert_eq!(
            TraceRefusal::not_an_input("oracle", "job_reply", &["sut"]).reason(),
            "'oracle' is not a system port of the test case's configuration"
        );
        let trace = Trace::parse(ONE_INPUT).expect("a canonical trace");
        let refused = TraceAdapter::replaying(&trace, no_inputs).expect_err("no system port");
        assert_eq!(
            refused.reason(),
            "input 0, at 500: 'sut' is not a system port of the test case's configuration"
        );
    }
}
