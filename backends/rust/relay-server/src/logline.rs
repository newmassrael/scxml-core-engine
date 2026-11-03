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
//!
//! Two kinds of call write lines. The program's own (a start, a stop, a sweep) call [`Log`]. The
//! relay library logs each of its decisions through the `log` facade (`sce_rust_relay::logs`), and
//! the program installs [`Log::install`] as the facade's sink, which writes each record in the same
//! form, the library's words after the level, and the module that decided it as `target`. How much
//! of the library's is written is `SCE_RELAY_LOG` ([`level_from_environment`]): `info` unless it
//! says otherwise, which shows every refusal, limit and admission and no request answered as asked.

use std::fmt::Display;
use std::io::Write;

/// `value` as one word of a line: the relay library's form ([`sce_rust_relay::logs::word`]), so
/// that a line of the program's and a record of the library's are read back alike.
pub use sce_rust_relay::logs::word;

/// The variable that says how much of the relay library's records is written.
pub const LEVEL_VARIABLE: &str = "SCE_RELAY_LOG";

/// How much a line matters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Error,
    Warning,
    Info,
    Debug,
    Trace,
}

impl Level {
    fn word(self) -> &'static str {
        match self {
            Level::Error => "error",
            Level::Warning => "warning",
            Level::Info => "info",
            Level::Debug => "debug",
            Level::Trace => "trace",
        }
    }

    /// The syslog priority the journal reads from a line's `<N>` prefix.
    fn priority(self) -> u8 {
        match self {
            Level::Error => 3,
            Level::Warning => 4,
            Level::Info => 6,
            Level::Debug | Level::Trace => 7,
        }
    }

    /// The level of a record of the facade.
    fn of(level: log::Level) -> Self {
        match level {
            log::Level::Error => Level::Error,
            log::Level::Warn => Level::Warning,
            log::Level::Info => Level::Info,
            log::Level::Debug => Level::Debug,
            log::Level::Trace => Level::Trace,
        }
    }
}

/// How much of the relay library's records `text` (the value of [`LEVEL_VARIABLE`]) asks for: `off`,
/// `error`, `warning` (or `warn`), `info`, `debug` or `trace`, in any case. `None` for another word.
pub fn level_of(text: &str) -> Option<log::LevelFilter> {
    match text.trim().to_ascii_lowercase().as_str() {
        "off" => Some(log::LevelFilter::Off),
        "error" => Some(log::LevelFilter::Error),
        "warning" | "warn" => Some(log::LevelFilter::Warn),
        "info" => Some(log::LevelFilter::Info),
        "debug" => Some(log::LevelFilter::Debug),
        "trace" => Some(log::LevelFilter::Trace),
        _ => None,
    }
}

/// How much of the relay library's records this process writes: what [`LEVEL_VARIABLE`] says, or
/// `info` when it says nothing. `Err` with the word when it says something [`level_of`] does not
/// read, which the program says and writes `info`.
pub fn level_from_environment() -> Result<log::LevelFilter, String> {
    match std::env::var(LEVEL_VARIABLE) {
        Err(_) => Ok(log::LevelFilter::Info),
        Ok(text) => level_of(&text).ok_or(text),
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

    /// The start of a line at `level`: the journal's prefix when there is one, and the level.
    fn head(&self, level: Level) -> String {
        let mut line = String::new();
        if self.journal {
            line.push_str(&format!("<{}>", level.priority()));
        }
        line.push_str("level=");
        line.push_str(level.word());
        line
    }

    /// The line for `event` at `level` with `fields`, as it is written.
    pub fn line(&self, level: Level, event: &str, fields: Fields<'_>) -> String {
        let mut line = self.head(level);
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

    /// The line for a record of the relay library, as it is written: its level, the library's
    /// words (`sce_rust_relay::logs::line`), and the module that decided it.
    pub fn record_line(&self, record: &log::Record<'_>) -> String {
        let mut line = self.head(Level::of(record.level()));
        line.push(' ');
        line.push_str(&sce_rust_relay::logs::line(record));
        line.push_str(" target=");
        line.push_str(&word(record.target()));
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

    /// Install this log as the sink of the relay library's records, writing those at `level` and
    /// above: once for the process, `false` when a sink was installed already (and this one is not).
    pub fn install(&self, level: log::LevelFilter) -> bool {
        // The sink lives as long as the process, as the facade asks of one.
        let sink: &'static Sink = Box::leak(Box::new(Sink { log: *self, level }));
        if log::set_logger(sink).is_err() {
            return false;
        }
        log::set_max_level(level);
        true
    }
}

/// The facade's sink a process installs ([`Log::install`]).
struct Sink {
    log: Log,
    level: log::LevelFilter,
}

impl log::Log for Sink {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &log::Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let line = self.log.record_line(record);
        let _ = writeln!(std::io::stderr().lock(), "{line}");
    }

    fn flush(&self) {
        let _ = std::io::stderr().lock().flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_is_its_level_its_event_and_its_words() {
        let address = "127.0.0.1:8780";
        let line = Log::plain().line(
            Level::Info,
            "listening",
            &[("transport", &"ws"), ("address", &address), ("open", &3u64)],
        );
        assert_eq!(
            line,
            "level=info event=listening transport=ws address=127.0.0.1:8780 open=3"
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
        assert!(journal
            .line(Level::Debug, "answered", &[])
            .starts_with("<7>level=debug"));
    }

    #[test]
    fn a_value_that_would_split_the_line_is_quoted() {
        // One line, whatever the value.
        let line = Log::plain().line(Level::Error, "x", &[("reason", &"a\nb\rc")]);
        assert_eq!(line.lines().count(), 1, "{line}");
        assert_eq!(word("two words"), "\"two words\"");
    }

    /// A record of the relay library is written as a line of the program's is: its level, its
    /// event and pairs in the library's form, and the module that decided it.
    #[test]
    fn a_record_of_the_library_is_a_line_in_the_same_form() {
        let line = Log::journal().record_line(
            &log::Record::builder()
                .level(log::Level::Info)
                .target("sce_rust_relay::login")
                .args(format_args!("rate_limited"))
                .build(),
        );
        assert_eq!(
            line,
            "<6>level=info event=rate_limited target=sce_rust_relay::login"
        );
    }

    #[test]
    fn the_level_of_the_library_is_read_from_its_word() {
        assert_eq!(level_of("info"), Some(log::LevelFilter::Info));
        assert_eq!(level_of(" DEBUG "), Some(log::LevelFilter::Debug));
        assert_eq!(level_of("warn"), Some(log::LevelFilter::Warn));
        assert_eq!(level_of("warning"), Some(log::LevelFilter::Warn));
        assert_eq!(level_of("off"), Some(log::LevelFilter::Off));
        assert_eq!(level_of("error"), Some(log::LevelFilter::Error));
        assert_eq!(level_of("trace"), Some(log::LevelFilter::Trace));
        assert_eq!(level_of("loud"), None);
    }
}
