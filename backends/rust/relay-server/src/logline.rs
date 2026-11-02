// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! One line on stderr for each thing that happened, which the service manager's journal keeps.
//!
//! A line is `level=<level> event=<event>` and then `key=value` words, a value in quotes when it
//! holds a space, a quote, an `=` or a control character, so that a line is read back by splitting
//! it. Under the journal (`JOURNAL_STREAM` is set) a line begins with the priority the journal
//! reads from a `<N>` prefix. What a line names is a count, an address, a path or a reason: never a
//! key, a token or what a list holds; the callers keep to that, and the relay never hands them
//! either.

use std::fmt::Display;
use std::io::Write;

/// How much a line matters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Error,
    Warning,
    Info,
}

impl Level {
    fn word(self) -> &'static str {
        match self {
            Level::Error => "error",
            Level::Warning => "warning",
            Level::Info => "info",
        }
    }

    /// The syslog priority the journal reads from a line's `<N>` prefix.
    fn priority(self) -> u8 {
        match self {
            Level::Error => 3,
            Level::Warning => 4,
            Level::Info => 6,
        }
    }
}

/// Where the lines go: stderr, with the journal's prefix when stderr is the journal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Log {
    journal: bool,
}

/// The words of a line after its level and event: each a key and its value.
pub type Fields<'a> = &'a [(&'a str, &'a dyn Display)];

impl Log {
    /// The log of this process: the journal's when the service manager connected stderr to it.
    pub fn from_environment() -> Self {
        Self {
            journal: std::env::var_os("JOURNAL_STREAM").is_some(),
        }
    }

    /// A log with no journal prefix.
    pub fn plain() -> Self {
        Self { journal: false }
    }

    /// A log whose lines carry the journal's prefix.
    pub fn journal() -> Self {
        Self { journal: true }
    }

    /// The line for `event` at `level` with `fields`, as it is written.
    pub fn line(&self, level: Level, event: &str, fields: Fields<'_>) -> String {
        let mut line = String::new();
        if self.journal {
            line.push_str(&format!("<{}>", level.priority()));
        }
        line.push_str("level=");
        line.push_str(level.word());
        line.push_str(" event=");
        line.push_str(&word(event));
        for (key, value) in fields {
            line.push(' ');
            line.push_str(key);
            line.push('=');
            line.push_str(&word(&value.to_string()));
        }
        line
    }

    /// Write the line for `event` to stderr. A line that cannot be written is lost, and the relay
    /// goes on: a log is no reason to stop answering.
    pub fn say(&self, level: Level, event: &str, fields: Fields<'_>) {
        let line = self.line(level, event, fields);
        let _ = writeln!(std::io::stderr().lock(), "{line}");
    }

    pub fn info(&self, event: &str, fields: Fields<'_>) {
        self.say(Level::Info, event, fields);
    }

    pub fn warning(&self, event: &str, fields: Fields<'_>) {
        self.say(Level::Warning, event, fields);
    }

    pub fn error(&self, event: &str, fields: Fields<'_>) {
        self.say(Level::Error, event, fields);
    }
}

/// `value` as one word of a line: as it is when nothing in it would split the line or be read as
/// its syntax, and in quotes otherwise, with a quote, a backslash and a control character escaped.
pub fn word(value: &str) -> String {
    let plain = !value.is_empty()
        && !value
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || matches!(c, '"' | '=' | '\\'));
    if plain {
        return value.to_string();
    }
    let mut quoted = String::with_capacity(value.len() + 2);
    quoted.push('"');
    for c in value.chars() {
        match c {
            '"' => quoted.push_str("\\\""),
            '\\' => quoted.push_str("\\\\"),
            '\n' => quoted.push_str("\\n"),
            '\r' => quoted.push_str("\\r"),
            '\t' => quoted.push_str("\\t"),
            c if c.is_control() => quoted.push_str(&format!("\\u{{{:x}}}", u32::from(c))),
            c => quoted.push(c),
        }
    }
    quoted.push('"');
    quoted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_is_its_level_its_event_and_its_words() {
        let address = "127.0.0.1:8080";
        let line = Log::plain().line(
            Level::Info,
            "listening",
            &[("transport", &"ws"), ("address", &address), ("open", &3u64)],
        );
        assert_eq!(
            line,
            "level=info event=listening transport=ws address=127.0.0.1:8080 open=3"
        );
    }

    #[test]
    fn a_line_for_the_journal_begins_with_its_priority() {
        let journal = Log::journal();
        assert!(journal
            .line(Level::Error, "refused", &[])
            .starts_with("<3>level=error event=refused"));
        assert!(journal
            .line(Level::Warning, "slow", &[])
            .starts_with("<4>level=warning"));
        assert!(journal
            .line(Level::Info, "ready", &[])
            .starts_with("<6>level=info"));
    }

    #[test]
    fn a_value_that_would_split_the_line_is_quoted() {
        assert_eq!(word("plain"), "plain");
        assert_eq!(word(""), "\"\"");
        assert_eq!(word("two words"), "\"two words\"");
        assert_eq!(word("a=b"), "\"a=b\"");
        assert_eq!(word("say \"x\""), "\"say \\\"x\\\"\"");
        assert_eq!(word("back\\slash"), "\"back\\\\slash\"");
        assert_eq!(word("line\nbreak"), "\"line\\nbreak\"");
        assert_eq!(word("bell\u{7}"), "\"bell\\u{7}\"");
        // One line, whatever the value.
        let line = Log::plain().line(Level::Error, "x", &[("reason", &"a\nb\rc")]);
        assert_eq!(line.lines().count(), 1, "{line}");
    }
}
