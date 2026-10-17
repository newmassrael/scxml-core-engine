// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The journal of a request: what its run did, as typed events kept beside it.
//!
//! A run of an AI takes minutes to hours, and until it ends the request says only that it is
//! running. The executor that holds the request says what its run does, one event at a time (a
//! turn asked, a tool run and what it judged, a message that was not a design, a conversation begun
//! again), and the store keeps them in the order they came, beside the request, for as long as the
//! request is kept. So a request that has ended can be read as one that is running is: by what its
//! run did, and not by a sentence the run left.
//!
//! What is kept is what happened and never what was said. A model's words are not an event: they are
//! long, they are the model's and not the application's, and a journal that carried them would be a
//! second copy of a conversation that nobody asked to keep. An event names a tool, a verdict, the
//! codes of what a check refused (SCE's own, `validation/invalid-reference`), and a cause word of the
//! executor's (`output-limit`).
//!
//! The rules are the request's, not the journal's: only the executor that holds the current attempt
//! of an open request adds an event, which is the same fence every other word of an executor to a
//! request is under (`Request::check_held_by`), and an event is a closed set of shapes, so an
//! executor cannot write what no screen reads.

use serde::{Deserialize, Serialize};

use crate::requests::Failure;

/// The most events one request keeps. A run that reaches it has been going for far longer than any
/// run the application allows, and the journal says that it stopped keeping (`Journal::full`),
/// which is more honest than dropping the oldest and leaving a count that no longer begins at one.
pub const ENTRIES_MAX: usize = 10_000;

/// The most codes one event names: a check that refuses a design says all it found, and the
/// first few are what a person reads.
pub const CODES_MAX: usize = 8;

/// How long a code, or the name of a tool, may be.
const WORD_MAX: usize = 80;

/// How many of the latest events a read gives with the summary.
pub const TAIL: usize = 60;

/// What a tool the run used came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ToolVerdict {
    /// The tool does not judge a design (it reads a spec, it lists tools), and it ran.
    Ran,
    /// A tool that judges a design accepted it.
    Accepted,
    /// A tool that judges a design refused it, and the codes say for what.
    Refused,
    /// The tool failed, and what was sent to it is not what is said to be wrong.
    Failed,
    /// The tool could not judge at all: what it checks with is missing or crashed.
    CannotJudge,
}

/// One thing a run did. A closed set: each is a shape a screen reads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Event {
    /// The runner called the generator, the `n`th time for this attempt (from 1). A later try
    /// begins again from the text and not from the earlier one's work.
    TryBegun { n: u32 },
    /// The model was asked, the `turn`th time of this try (from 0).
    TurnAsked { turn: u32 },
    /// A tool the model called ran.
    ToolRan {
        tool: String,
        verdict: ToolVerdict,
        #[serde(default)]
        codes: Vec<String>,
    },
    /// A message of the model was not the draft. The cause is the executor's word for how it was
    /// not (`output-limit`, `reasoning-only`, `draft-cut`).
    NotTheDraft {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cause: Option<String>,
    },
    /// The conversation was begun again, as it outgrew the model's context. `proactive` is that it
    /// was begun before the server had to refuse.
    HandedOver { proactive: bool },
    /// The core refused the draft the run wrote, and the codes say for what.
    DraftRefused {
        #[serde(default)]
        codes: Vec<String>,
    },
}

impl Event {
    /// Whether the event is one the journal keeps, or why not. The fields are words a screen
    /// shows and counts it adds up, so none is left to be anything.
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Event::TryBegun { n } if *n == 0 => Err("a try is counted from 1".to_string()),
            Event::ToolRan { tool, codes, .. } => {
                if !is_word(tool) {
                    return Err(format!(
                        "a tool is named by 1 to {WORD_MAX} letters, digits and `._:-`, \
                         starting with a letter or a digit"
                    ));
                }
                valid_codes(codes)
            }
            Event::NotTheDraft { cause: Some(cause) } if !Failure::is_cause(cause) => Err(
                "a cause is 1 to 40 lowercase letters, digits and hyphens, none at either end"
                    .to_string(),
            ),
            Event::DraftRefused { codes } => valid_codes(codes),
            _ => Ok(()),
        }
    }
}

fn is_word(word: &str) -> bool {
    (1..=WORD_MAX).contains(&word.len())
        && word.as_bytes()[0].is_ascii_alphanumeric()
        && word
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b':' | b'-'))
}

/// Whether `code` is the shape of SCE's word for what a check refused: 1 to 80 lowercase letters,
/// digits and `/ _ - .`. An executor that lists the codes of a refusal keeps those that are, so
/// that one that is not does not stop the rest being kept.
pub fn is_code(code: &str) -> bool {
    (1..=WORD_MAX).contains(&code.len())
        && code.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'/' | b'_' | b'-' | b'.')
        })
}

fn valid_codes(codes: &[String]) -> Result<(), String> {
    if codes.len() > CODES_MAX {
        return Err(format!("an event names at most {CODES_MAX} codes"));
    }
    match codes.iter().find(|code| !is_code(code)) {
        Some(code) => Err(format!(
            "`{code}` is not a code: 1 to {WORD_MAX} lowercase letters, digits and `/_-.`"
        )),
        None => Ok(()),
    }
}

/// An event and the moment the store wrote it (seconds since the Unix epoch).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub at: u64,
    pub event: Event,
}

/// What a journal adds up to. Computed in one place, so that every screen that reads a run reads
/// the same counts.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Progress {
    /// How many events the journal holds.
    pub entries: u64,
    /// Times the generator was called for this attempt.
    pub tries: u32,
    /// Times the model was asked, over every try.
    pub turns: u32,
    /// Times a tool that judges a design said accepted or refused.
    pub checks: u32,
    /// Of those, the times it refused.
    pub refused: u32,
    /// Times the core refused a draft the run wrote.
    pub drafts_refused: u32,
    /// Messages of the model that were not the draft.
    pub not_drafts: u32,
    /// Times the conversation was begun again.
    pub handovers: u32,
    /// The codes of the latest refusal, by a check or by the core. Empty before any.
    pub last_refusal: Vec<String>,
    /// When the first event was written.
    pub started_at: Option<u64>,
    /// When the latest was.
    pub last_at: Option<u64>,
}

impl Progress {
    /// What `entries` add up to, oldest first.
    pub fn of(entries: &[Entry]) -> Self {
        let mut progress = Progress {
            entries: entries.len() as u64,
            started_at: entries.first().map(|entry| entry.at),
            last_at: entries.last().map(|entry| entry.at),
            ..Progress::default()
        };
        for entry in entries {
            match &entry.event {
                Event::TryBegun { .. } => progress.tries += 1,
                Event::TurnAsked { .. } => progress.turns += 1,
                Event::ToolRan { verdict, codes, .. } => match verdict {
                    ToolVerdict::Accepted => progress.checks += 1,
                    ToolVerdict::Refused => {
                        progress.checks += 1;
                        progress.refused += 1;
                        progress.last_refusal = codes.clone();
                    }
                    ToolVerdict::Ran | ToolVerdict::Failed | ToolVerdict::CannotJudge => {}
                },
                Event::NotTheDraft { .. } => progress.not_drafts += 1,
                Event::HandedOver { .. } => progress.handovers += 1,
                Event::DraftRefused { codes } => {
                    progress.drafts_refused += 1;
                    progress.last_refusal = codes.clone();
                }
            }
        }
        progress
    }
}

/// A request's journal as it is read: what it adds up to, and its latest events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Journal {
    pub progress: Progress,
    /// The latest [`TAIL`] events, oldest first.
    pub tail: Vec<Entry>,
    /// The journal holds [`ENTRIES_MAX`] events and keeps no more: what is counted is what was
    /// kept, and the run went on.
    pub full: bool,
}

impl Journal {
    pub fn of(entries: &[Entry]) -> Self {
        let from = entries.len().saturating_sub(TAIL);
        Journal {
            progress: Progress::of(entries),
            tail: entries[from..].to_vec(),
            full: entries.len() >= ENTRIES_MAX,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(at: u64, event: Event) -> Entry {
        Entry { at, event }
    }

    fn tool(verdict: ToolVerdict, codes: &[&str]) -> Event {
        Event::ToolRan {
            tool: "validate_scxml_set".to_string(),
            verdict,
            codes: codes.iter().map(|c| c.to_string()).collect(),
        }
    }

    #[test]
    fn an_empty_journal_adds_up_to_nothing() {
        assert_eq!(Progress::of(&[]), Progress::default());
        let journal = Journal::of(&[]);
        assert!(journal.tail.is_empty());
        assert!(!journal.full);
    }

    #[test]
    fn what_a_run_did_is_counted_as_it_happened() {
        let entries = vec![
            at(100, Event::TryBegun { n: 1 }),
            at(101, Event::TurnAsked { turn: 0 }),
            at(102, tool(ToolVerdict::Ran, &[])),
            at(103, Event::TurnAsked { turn: 1 }),
            at(
                104,
                tool(ToolVerdict::Refused, &["validation/invalid-reference"]),
            ),
            at(105, Event::TurnAsked { turn: 2 }),
            at(
                106,
                tool(
                    ToolVerdict::Refused,
                    &["scxml/unreachable-state", "scxml/dead-transition"],
                ),
            ),
            at(
                107,
                Event::NotTheDraft {
                    cause: Some("output-limit".to_string()),
                },
            ),
            at(108, Event::HandedOver { proactive: true }),
            at(109, tool(ToolVerdict::Accepted, &[])),
            at(
                110,
                Event::DraftRefused {
                    codes: vec!["manifest/bad".to_string()],
                },
            ),
            at(111, Event::TryBegun { n: 2 }),
        ];

        let progress = Progress::of(&entries);

        assert_eq!(progress.entries, 12);
        assert_eq!(progress.tries, 2);
        assert_eq!(progress.turns, 3);
        // The tool that only ran judged nothing: it is not a check.
        assert_eq!(progress.checks, 3);
        assert_eq!(progress.refused, 2);
        assert_eq!(progress.drafts_refused, 1);
        assert_eq!(progress.not_drafts, 1);
        assert_eq!(progress.handovers, 1);
        assert_eq!(progress.last_refusal, vec!["manifest/bad".to_string()]);
        assert_eq!(progress.started_at, Some(100));
        assert_eq!(progress.last_at, Some(111));
    }

    #[test]
    fn a_tool_that_failed_or_could_not_judge_is_not_a_refusal_of_the_design() {
        let entries = vec![
            at(1, tool(ToolVerdict::Failed, &["x/y"])),
            at(2, tool(ToolVerdict::CannotJudge, &[])),
        ];

        let progress = Progress::of(&entries);

        assert_eq!((progress.checks, progress.refused), (0, 0));
        assert!(progress.last_refusal.is_empty());
    }

    #[test]
    fn the_latest_refusal_is_the_one_a_later_acceptance_does_not_clear() {
        // The codes are what the last refusal said: an acceptance after it is a count, and does
        // not make the refusal not have happened.
        let entries = vec![
            at(1, tool(ToolVerdict::Refused, &["a/b"])),
            at(2, tool(ToolVerdict::Accepted, &[])),
        ];

        assert_eq!(Progress::of(&entries).last_refusal, vec!["a/b".to_string()]);
    }

    #[test]
    fn a_read_gives_the_latest_events_and_counts_all_of_them() {
        let entries: Vec<Entry> = (0..(TAIL as u64 + 25))
            .map(|n| at(n, Event::TurnAsked { turn: n as u32 }))
            .collect();

        let journal = Journal::of(&entries);

        assert_eq!(journal.tail.len(), TAIL);
        assert_eq!(journal.tail.first(), entries.get(25));
        assert_eq!(journal.tail.last(), entries.last());
        assert_eq!(journal.progress.turns as usize, TAIL + 25);
    }

    #[test]
    fn a_journal_that_holds_the_most_says_it_is_full() {
        let entries: Vec<Entry> = (0..ENTRIES_MAX as u64)
            .map(|n| at(n, Event::TurnAsked { turn: 0 }))
            .collect();

        assert!(Journal::of(&entries).full);
        assert!(!Journal::of(&entries[..ENTRIES_MAX - 1]).full);
    }

    #[test]
    fn an_event_is_a_closed_set_of_shapes() {
        let said = serde_json::json!({"kind": "tool-ran", "tool": "validate_scxml", "verdict": "refused",
                                      "codes": ["a/b"]});
        let event: Event = serde_json::from_value(said).unwrap();
        assert_eq!(
            event,
            tool_named("validate_scxml", ToolVerdict::Refused, &["a/b"])
        );
        // A kind that is none of them, and a field that nothing reads, are refused.
        assert!(serde_json::from_value::<Event>(serde_json::json!({"kind": "dreamed"})).is_err());
        assert!(serde_json::from_value::<Event>(
            serde_json::json!({"kind": "turn-asked", "turn": 1, "words": "hello"})
        )
        .is_err());
        // A code said as `code` and not `codes` is not quietly dropped to an empty list.
        assert!(serde_json::from_value::<Event>(
            serde_json::json!({"kind": "draft-refused", "code": ["a/b"]})
        )
        .is_err());
    }

    fn tool_named(name: &str, verdict: ToolVerdict, codes: &[&str]) -> Event {
        Event::ToolRan {
            tool: name.to_string(),
            verdict,
            codes: codes.iter().map(|c| c.to_string()).collect(),
        }
    }

    #[test]
    fn an_event_with_a_word_that_is_not_one_is_not_kept() {
        for bad in [
            Event::TryBegun { n: 0 },
            tool_named("two words", ToolVerdict::Ran, &[]),
            tool_named("", ToolVerdict::Ran, &[]),
            tool_named("ok", ToolVerdict::Refused, &["Has Capitals"]),
            tool_named("ok", ToolVerdict::Refused, &[""]),
            tool_named("ok", ToolVerdict::Refused, &["a/b"; CODES_MAX + 1]),
            Event::NotTheDraft {
                cause: Some("it ran out of room".to_string()),
            },
            Event::DraftRefused {
                codes: vec!["x".repeat(WORD_MAX + 1)],
            },
        ] {
            assert!(bad.validate().is_err(), "{bad:?}");
        }
        for good in [
            Event::TryBegun { n: 1 },
            tool_named("specs_read", ToolVerdict::Ran, &[]),
            tool_named(
                "validate_scxml_set",
                ToolVerdict::Refused,
                &["validation/invalid-reference"],
            ),
            Event::NotTheDraft { cause: None },
            Event::NotTheDraft {
                cause: Some("draft-cut".to_string()),
            },
            Event::HandedOver { proactive: false },
            Event::DraftRefused { codes: vec![] },
        ] {
            assert_eq!(good.validate(), Ok(()), "{good:?}");
        }
    }
}
