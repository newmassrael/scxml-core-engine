// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The test log of a test case run (docs/SCE_TEST_PROFILE.md §12.5;
//! docs/adr/0091, C3 and answer 8): the records of TCI's test logging
//! interface (ETSI ES 201 873-6), each written where the profile's semantics
//! places its event, in the order the round scheduler produces them, and the
//! one canonical text of a run's log, `schemas/sce-test-log.v1.schema.json`.
//!
//! The text is the JSON Canonicalization Scheme of the log
//! ([`crate::json::write_canonical`]), and it holds no number: an instant, a
//! duration and every integer of a value are texts of their digits
//! ([`super::canonical`]). So two runs agree, under C3, exactly when their
//! logs are the same bytes, whichever runtime wrote each.

use super::canonical;
use super::port::{Duration, Instant};
use super::run::TestCaseResult;
use super::verdict::Verdict;
use crate::json::{self, Value};

/// The name of the test log's shape, its `schema`.
pub const TEST_LOG_SCHEMA: &str = "sce-test-log.v1";

/// The stability status of the test log's wire schema, pinned to the schema
/// file's own `x-sce-schema-status` by `schema_file_declares_status`.
/// `SCE_WIRE_CONTRACTS.md` requires one commit to move both.
pub const TEST_LOG_SCHEMA_STATUS: &str = "pre-release";

/// One record of the test log: an event and `NOW` when it happened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestLogRecord {
    /// `NOW` when it was recorded.
    pub at: Instant,
    /// What happened.
    pub event: TestLogEvent,
}

/// An event of TCI's test logging interface (ES 201 873-6), as the profile's
/// semantics places it (docs/SCE_TEST_PROFILE.md §12.5).
///
/// A component is named `mtc` or by the PTC's name in its configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TestLogEvent {
    /// The test case started, at `NOW = 0` (P11).
    TestCaseStarted,
    /// The test case ended with its verdict (P14).
    TestCaseEnded {
        /// The test case verdict.
        verdict: Verdict,
    },
    /// A component was created: the MTC as the test case starts, a PTC by
    /// the invocation that creates it (P9).
    ComponentCreated {
        /// The component.
        component: String,
        /// Its component type.
        component_type: String,
    },
    /// A component terminated, with its local verdict (P10).
    ComponentDone {
        /// The component.
        component: String,
        /// Its local verdict when it terminated.
        verdict: Verdict,
    },
    /// A component sent a message on a port (P6).
    MessageSent {
        /// The sender.
        component: String,
        /// The port it sent on.
        port: String,
        /// The message's value in canonical form.
        value: Value,
    },
    /// A receive branch took the head of a port's queue (P4 step 1): the
    /// message was received and matched.
    MessageReceived {
        /// The receiver.
        component: String,
        /// The port it took the message from.
        port: String,
        /// The message's value in canonical form.
        value: Value,
    },
    /// A component started a timer, for a duration in microseconds (P7).
    TimerStarted {
        /// The component.
        component: String,
        /// The timer.
        timer: String,
        /// The duration, in microseconds.
        duration: Duration,
    },
    /// A component stopped a running timer (P7). A stop of a timer that is
    /// not running changes nothing, and is not recorded.
    TimerStopped {
        /// The component.
        component: String,
        /// The timer.
        timer: String,
    },
    /// A timeout branch took a timer that had timed out (P4 step 1).
    TimerTimedOut {
        /// The component.
        component: String,
        /// The timer.
        timer: String,
    },
    /// A component's local verdict was set: by `<sce:setverdict>` (P12), or to
    /// `error` by the runtime (P13). The reason is the document's, and none is
    /// recorded for an empty one or for the runtime's `error`, whose cause is
    /// the runtime's own prose, which another runtime would not write alike.
    VerdictSet {
        /// The component.
        component: String,
        /// The verdict it was given.
        verdict: Verdict,
        /// The reason the document gave, if any.
        reason: Option<String>,
    },
    /// A `<log>` (P8).
    Log {
        /// The component.
        component: String,
        /// The text.
        text: String,
    },
}

impl TestLogEvent {
    /// The record's `kind` in the test log: the TCI event in kebab case.
    pub const fn kind(&self) -> &'static str {
        match self {
            TestLogEvent::TestCaseStarted => "test-case-started",
            TestLogEvent::TestCaseEnded { .. } => "test-case-ended",
            TestLogEvent::ComponentCreated { .. } => "component-created",
            TestLogEvent::ComponentDone { .. } => "component-done",
            TestLogEvent::MessageSent { .. } => "message-sent",
            TestLogEvent::MessageReceived { .. } => "message-received",
            TestLogEvent::TimerStarted { .. } => "timer-started",
            TestLogEvent::TimerStopped { .. } => "timer-stopped",
            TestLogEvent::TimerTimedOut { .. } => "timer-timed-out",
            TestLogEvent::VerdictSet { .. } => "verdict-set",
            TestLogEvent::Log { .. } => "log",
        }
    }
}

fn member(name: &str, value: Value) -> (String, Value) {
    (name.to_string(), value)
}

fn text_member(name: &str, text: &str) -> (String, Value) {
    member(name, canonical::text(text))
}

impl TestLogRecord {
    /// The record as the test log writes it: `at`, `kind`, and the members of
    /// its kind.
    pub fn to_value(&self) -> Value {
        let mut members = vec![
            member("at", canonical::integer(self.at)),
            text_member("kind", self.event.kind()),
        ];
        match &self.event {
            TestLogEvent::TestCaseStarted => {}
            TestLogEvent::TestCaseEnded { verdict } => {
                members.push(text_member("verdict", verdict.name()));
            }
            TestLogEvent::ComponentCreated {
                component,
                component_type,
            } => {
                members.push(text_member("component", component));
                members.push(text_member("type", component_type));
            }
            TestLogEvent::ComponentDone { component, verdict } => {
                members.push(text_member("component", component));
                members.push(text_member("verdict", verdict.name()));
            }
            TestLogEvent::MessageSent {
                component,
                port,
                value,
            }
            | TestLogEvent::MessageReceived {
                component,
                port,
                value,
            } => {
                members.push(text_member("component", component));
                members.push(text_member("port", port));
                members.push(member("value", value.clone()));
            }
            TestLogEvent::TimerStarted {
                component,
                timer,
                duration,
            } => {
                members.push(text_member("component", component));
                members.push(text_member("timer", timer));
                members.push(member("duration_us", canonical::integer(*duration)));
            }
            TestLogEvent::TimerStopped { component, timer }
            | TestLogEvent::TimerTimedOut { component, timer } => {
                members.push(text_member("component", component));
                members.push(text_member("timer", timer));
            }
            TestLogEvent::VerdictSet {
                component,
                verdict,
                reason,
            } => {
                members.push(text_member("component", component));
                members.push(text_member("verdict", verdict.name()));
                if let Some(reason) = reason {
                    members.push(text_member("reason", reason));
                }
            }
            TestLogEvent::Log { component, text } => {
                members.push(text_member("component", component));
                members.push(text_member("text", text));
            }
        }
        Value::Object(members)
    }
}

impl TestCaseResult {
    /// The test log of this run of the test case `test_case` of the suite
    /// `suite`, in its one canonical text: the JSON Canonicalization Scheme of
    /// `schema`, `suite`, `test_case`, `verdict` and `records`, each record in
    /// the order the run produced it.
    ///
    /// # Panics
    ///
    /// When a message's value is not in the canonical form
    /// ([`super::TestValue::canonical`]) but holds a number past the range of
    /// a double, or an object that names a member twice, which no writer of
    /// [`super::canonical`] builds.
    pub fn canonical_log(&self, suite: &str, test_case: &str) -> String {
        let log = Value::Object(vec![
            text_member("schema", TEST_LOG_SCHEMA),
            text_member("suite", suite),
            text_member("test_case", test_case),
            text_member("verdict", self.verdict.name()),
            member(
                "records",
                Value::Array(self.records.iter().map(TestLogRecord::to_value).collect()),
            ),
        ]);
        json::write_canonical(&log).unwrap_or_else(|refused| {
            panic!("the test log of {suite} {test_case} is not canonical: {refused}")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCHEMA: &str = include_str!("../../../../../schemas/sce-test-log.v1.schema.json");

    #[test]
    fn schema_file_declares_status() {
        let schema = json::parse(SCHEMA).expect("the test log's schema is JSON");
        assert_eq!(
            schema.member("x-sce-schema-status"),
            Some(&Value::Text(TEST_LOG_SCHEMA_STATUS.to_string())),
            "schemas/sce-test-log.v1.schema.json x-sce-schema-status disagrees with \
             TEST_LOG_SCHEMA_STATUS; SCE_WIRE_CONTRACTS.md requires one commit to move both",
        );
        assert_eq!(
            schema
                .member("properties")
                .and_then(|properties| properties.member("schema"))
                .and_then(|shape| shape.member("const")),
            Some(&Value::Text(TEST_LOG_SCHEMA.to_string())),
            "the schema pins the name of the shape the runtime writes",
        );
    }

    /// Every kind the runtime writes is one the schema names, each once.
    #[test]
    fn every_kind_is_one_the_schema_names() {
        let kinds = [
            TestLogEvent::TestCaseStarted.kind(),
            TestLogEvent::TestCaseEnded {
                verdict: Verdict::None,
            }
            .kind(),
            TestLogEvent::ComponentCreated {
                component: String::new(),
                component_type: String::new(),
            }
            .kind(),
            TestLogEvent::ComponentDone {
                component: String::new(),
                verdict: Verdict::None,
            }
            .kind(),
            TestLogEvent::MessageSent {
                component: String::new(),
                port: String::new(),
                value: Value::Null,
            }
            .kind(),
            TestLogEvent::MessageReceived {
                component: String::new(),
                port: String::new(),
                value: Value::Null,
            }
            .kind(),
            TestLogEvent::TimerStarted {
                component: String::new(),
                timer: String::new(),
                duration: 0,
            }
            .kind(),
            TestLogEvent::TimerStopped {
                component: String::new(),
                timer: String::new(),
            }
            .kind(),
            TestLogEvent::TimerTimedOut {
                component: String::new(),
                timer: String::new(),
            }
            .kind(),
            TestLogEvent::VerdictSet {
                component: String::new(),
                verdict: Verdict::None,
                reason: None,
            }
            .kind(),
            TestLogEvent::Log {
                component: String::new(),
                text: String::new(),
            }
            .kind(),
        ];
        for kind in kinds {
            let named = format!("\"const\": \"{kind}\"");
            assert_eq!(
                SCHEMA.matches(named.as_str()).count(),
                1,
                "the schema names the kind {kind} once"
            );
        }
    }

    /// A record of each shape: `at` and every integer as the text of its
    /// digits, an absent reason omitted, the members sorted when written.
    #[test]
    fn a_record_is_written_with_the_members_of_its_kind() {
        let written = |event: TestLogEvent| {
            json::write_canonical(&TestLogRecord { at: 250, event }.to_value())
                .expect("a record is canonical")
        };
        assert_eq!(
            written(TestLogEvent::TimerStarted {
                component: "mtc".to_string(),
                timer: "reply".to_string(),
                duration: 1_000_000,
            }),
            "{\"at\":\"250\",\"component\":\"mtc\",\"duration_us\":\"1000000\",\
             \"kind\":\"timer-started\",\"timer\":\"reply\"}"
        );
        assert_eq!(
            written(TestLogEvent::VerdictSet {
                component: "echo".to_string(),
                verdict: Verdict::Inconclusive,
                reason: None,
            }),
            "{\"at\":\"250\",\"component\":\"echo\",\"kind\":\"verdict-set\",\
             \"verdict\":\"inconc\"}"
        );
        assert_eq!(
            written(TestLogEvent::VerdictSet {
                component: "echo".to_string(),
                verdict: Verdict::Fail,
                reason: Some("late".to_string()),
            }),
            "{\"at\":\"250\",\"component\":\"echo\",\"kind\":\"verdict-set\",\
             \"reason\":\"late\",\"verdict\":\"fail\"}"
        );
        assert_eq!(
            written(TestLogEvent::MessageReceived {
                component: "mtc".to_string(),
                port: "sut".to_string(),
                value: canonical::record([("job", Some(canonical::integer(7_u32)))]),
            }),
            "{\"at\":\"250\",\"component\":\"mtc\",\"kind\":\"message-received\",\
             \"port\":\"sut\",\"value\":{\"job\":\"7\"}}"
        );
    }

    /// The log of a run: its shape, suite, test case and verdict before its
    /// records, all sorted, with no whitespace.
    #[test]
    fn a_log_is_its_shape_suite_case_verdict_and_records() {
        let result = TestCaseResult {
            verdict: Verdict::Error,
            ended_at: 0,
            log: Vec::new(),
            records: vec![
                TestLogRecord {
                    at: 0,
                    event: TestLogEvent::TestCaseStarted,
                },
                TestLogRecord {
                    at: 0,
                    event: TestLogEvent::TestCaseEnded {
                        verdict: Verdict::Error,
                    },
                },
            ],
        };
        assert_eq!(
            result.canonical_log("a_suite", "a_case"),
            "{\"records\":[{\"at\":\"0\",\"kind\":\"test-case-started\"},\
             {\"at\":\"0\",\"kind\":\"test-case-ended\",\"verdict\":\"error\"}],\
             \"schema\":\"sce-test-log.v1\",\"suite\":\"a_suite\",\"test_case\":\"a_case\",\
             \"verdict\":\"error\"}"
        );
    }
}
