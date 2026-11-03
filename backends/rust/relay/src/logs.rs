// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What the relay, a device's side of it and the driver say of each thing they decide, and the one
//! form a record of it is written in.
//!
//! The split is ADR 0060's: the call site, the formatting and the sink are three things. A library
//! of this workspace logs through the `log` facade, a record for each decision with its reason and
//! its counts as key-value pairs, and installs nothing. The program that runs it installs the sink:
//! `sce-relay` writes each record as a line on stderr, which the service manager's journal keeps, and
//! the peer core's C ABI hands each to the function its host gave (`sce_peer_set_log_sink`). Between
//! them is the form, written once here ([`line`]): `event=<the record's message>` and then each pair
//! as `key=value`, a value in quotes when it would split the line ([`word`]), so that a line is read
//! back by splitting it wherever it was written.
//!
//! What a record names is a count, a code, a node's number, a duration, an address, the reason a
//! relay gives a device, and a list by a short digest of its name ([`list_digest`]): never a token, a
//! key, a signature, a nonce, an invitation's text, or anything a list holds.
//!
//! The levels are an operator's: what refuses a request (a limit, a token, a request that does not
//! hold) at `info`, what the relay cannot do at all (a store that did not take a record) at `error`,
//! what a device's side did that its host would look for (a wait for the rate, a link that failed, a
//! login) at `info`, and each request answered as asked at `debug`.

use std::fmt::Write as _;

use log::kv::{Error as KvError, Key, Value, VisitSource};
use sha2::{Digest, Sha256};

use crate::protocol::{Code, Response};

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
            c if c.is_control() => {
                let _ = write!(quoted, "\\u{{{:x}}}", u32::from(c));
            }
            c => quoted.push(c),
        }
    }
    quoted.push('"');
    quoted
}

/// `record` as the words of one line: `event=<its message>`, then each of its pairs as `key=value`,
/// in the order the call site gave them. No level and no target: the sink adds what it keeps.
pub fn line(record: &log::Record<'_>) -> String {
    let mut line = String::from("event=");
    line.push_str(&word(&record.args().to_string()));
    let _ = record.key_values().visit(&mut Words(&mut line));
    line
}

/// What writes a record's pairs onto its line.
struct Words<'a>(&'a mut String);

impl<'kvs> VisitSource<'kvs> for Words<'_> {
    fn visit_pair(&mut self, key: Key<'kvs>, value: Value<'kvs>) -> Result<(), KvError> {
        self.0.push(' ');
        self.0.push_str(&word(key.as_str()));
        self.0.push('=');
        self.0.push_str(&word(&value.to_string()));
        Ok(())
    }
}

/// The domain a list's digest is taken under, so that it is no other hash of the name.
const LIST_DOMAIN: &[u8] = b"sce-relay/log-list/1";

/// What a record names a list by: the first four bytes of SHA-256 over a domain and the list's name,
/// in hexadecimal. Enough for an operator to tell the lists of one relay apart and follow one, and
/// not the name, which a person chose.
pub fn list_digest(list: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(LIST_DOMAIN);
    hasher.update([0u8]);
    hasher.update(list.as_bytes());
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(8);
    for byte in &digest[..4] {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

/// Log the refusal `response` under `target`, the module that decided it: its code and its reason,
/// at `error` for a relay that takes nothing more (`unavailable`) and at `info` otherwise. A refusal
/// for the rate is logged where its limit refused it, with the limit's name and counts, and not here.
pub(crate) fn note_refusal(target: &str, response: &Response) {
    if let Response::Refused { code, reason } = response {
        let level = match code {
            Code::Unavailable => log::Level::Error,
            _ => log::Level::Info,
        };
        log::log!(target: target, level, code = code.tag(), reason = reason.as_str(); "refused");
    }
}

/// A refusal of the relay, logged where it is decided ([`note_refusal`], under the module of the
/// call), and returned.
macro_rules! refused {
    ($code:expr, $reason:expr $(,)?) => {{
        let response = $crate::protocol::Response::refused($code, $reason);
        $crate::logs::note_refusal(module_path!(), &response);
        response
    }};
}
pub(crate) use refused;

/// A sink that keeps each record, as [`line`] writes it, for the thread that logged it: what a test
/// of what this workspace logs installs, so that tests running at once on their own threads read
/// their own records. The relay, the driver and the peer core log on the thread that calls them.
pub mod capture {
    use std::cell::RefCell;
    use std::sync::Once;

    /// One record a thread logged.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Taken {
        pub level: log::Level,
        pub target: String,
        /// The record as [`super::line`] writes it.
        pub line: String,
    }

    thread_local! {
        static TAKEN: RefCell<Vec<Taken>> = const { RefCell::new(Vec::new()) };
    }

    struct Capture;

    impl log::Log for Capture {
        fn enabled(&self, _: &log::Metadata<'_>) -> bool {
            true
        }

        fn log(&self, record: &log::Record<'_>) {
            let taken = Taken {
                level: record.level(),
                target: record.target().to_string(),
                line: super::line(record),
            };
            TAKEN.with(|kept| kept.borrow_mut().push(taken));
        }

        fn flush(&self) {}
    }

    static CAPTURE: Capture = Capture;

    /// Keep this thread's records from now on, every level, and drop what it logged before. The
    /// sink is installed for the process the first time.
    ///
    /// # Panics
    ///
    /// When the process installed another sink first.
    pub fn start() {
        static INSTALLED: Once = Once::new();
        INSTALLED.call_once(|| {
            log::set_logger(&CAPTURE).expect("no other sink of the facade is installed");
            log::set_max_level(log::LevelFilter::Trace);
        });
        TAKEN.with(|kept| kept.borrow_mut().clear());
    }

    /// What this thread logged since [`start`] or since this was last asked, oldest first.
    pub fn taken() -> Vec<Taken> {
        TAKEN.with(|kept| std::mem::take(&mut *kept.borrow_mut()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(word("nul\u{0}"), "\"nul\\u{0}\"");
    }

    #[test]
    fn a_record_is_its_event_and_its_pairs_in_order() {
        capture::start();
        log::info!(target: "here", limit = "burst", retry_after_ms = 20u64, reason = "two words"; "rate_limited");
        let taken = capture::taken();
        assert_eq!(taken.len(), 1, "{taken:?}");
        assert_eq!(taken[0].level, log::Level::Info);
        assert_eq!(taken[0].target, "here");
        assert_eq!(
            taken[0].line,
            "event=rate_limited limit=burst retry_after_ms=20 reason=\"two words\""
        );
    }

    #[test]
    fn a_list_is_named_by_a_digest_that_is_not_its_name() {
        let digest = list_digest("home");
        assert_eq!(digest.len(), 8);
        assert!(digest.chars().all(|c| c.is_ascii_hexdigit()), "{digest}");
        assert_eq!(
            digest,
            list_digest("home"),
            "the same list, the same digest"
        );
        assert_ne!(digest, list_digest("work"));
        assert!(!digest.contains("home"));
    }

    #[test]
    fn a_refusal_is_logged_with_its_code_and_reason_at_its_level() {
        capture::start();
        let answer = refused!(Code::Behind, "the relay holds no log of this list");
        assert_eq!(
            answer,
            Response::refused(Code::Behind, "the relay holds no log of this list")
        );
        let _ = refused!(Code::Unavailable, "the store did not take a record");
        let taken = capture::taken();
        assert_eq!(taken.len(), 2, "{taken:?}");
        assert_eq!(taken[0].level, log::Level::Info);
        assert_eq!(taken[0].target, module_path!());
        assert_eq!(
            taken[0].line,
            "event=refused code=behind reason=\"the relay holds no log of this list\""
        );
        assert_eq!(taken[1].level, log::Level::Error);
        assert!(taken[1].line.starts_with("event=refused code=unavailable"));
    }
}
