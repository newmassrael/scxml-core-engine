// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Every reason a C11 machine raises with an error, from one table
//! (docs/adr/0086, section 8.6 (b)).
//!
//! An `error.execution` or `error.communication` a machine raises itself
//! carries its reason in `_event.data`, and the machine holds that text where it
//! holds every event's data: a queue entry of its slot of `_event.data` on a
//! machine without a script engine. The slot is computed by the build (section
//! 8.1), so the build has to know the longest reason the machine can raise.
//! The reasons used to be literals written at each raise site of the templates
//! and of the lowerings, and nothing measured them; the floor
//! `SCE_MAX_DATA_LEN` was what held them.
//!
//! [`REASONS`] is the single source. A row has a name and a text; a text may
//! hold holes, `{0}` to `{9}`, which the generator fills with the document's
//! own text (an attribute's value, an element's name), never with a value of
//! the run, so every instantiation is a text the build knows. A template emits
//! a reason by the name of its row, `{{ reason("send_delay_not_time") }}`, and
//! a lowering through [`Reason::literal`]; each returns the C string literal of
//! the instantiation and records its length in the book of the machine being
//! generated ([`scope`]). [`seal`] closes the book once every reason is
//! emitted and answers the longest, which is an input of the slot.
//!
//! **Why a book of the thread.** The lowerings reach the reasons through
//! targets that are `'static` values shared by every document
//! (`static_lowering::target_for`), and the templates through a function of
//! the environment, so neither can be handed the machine's book by argument.
//! The book is the thread's for the length of one machine's generation, as the
//! deployment's limits are (`crate::limits::scope`), and a scope that ends
//! puts back the book it replaced, so a machine generated inside another's
//! generation keeps its own. A reason emitted after [`seal`] is refused rather
//! than left out of the slot.

use std::cell::RefCell;
use std::fmt;

/// One reason a C11 machine can raise: the name a raise site emits it by, and
/// its text, whose holes the site fills with the document's own text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reason {
    pub row: &'static str,
    pub text: &'static str,
}

/// Why a reason could not be emitted: a defect of the template or the
/// lowering that asked for it, never of a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReasonError {
    /// No row of [`REASONS`] has the name.
    UnknownRow(String),
    /// The site filled a number of holes other than the row's.
    Holes {
        row: &'static str,
        wanted: usize,
        given: usize,
    },
    /// The machine's slot was computed already ([`seal`]).
    Sealed(&'static str),
}

impl fmt::Display for ReasonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReasonError::UnknownRow(row) => {
                write!(f, "no reason of a C11 machine is named `{row}`")
            }
            ReasonError::Holes { row, wanted, given } => write!(
                f,
                "the reason `{row}` takes {wanted} hole(s), and the site filled {given}"
            ),
            ReasonError::Sealed(row) => write!(
                f,
                "the reason `{row}` was emitted after the machine's slot was computed, so the \
                 slot would not hold it"
            ),
        }
    }
}

impl std::error::Error for ReasonError {}

/// The sentence a `sce-static` lowering writes for a failure of its own (an
/// integer operation that overflowed, a list already at its capacity). The
/// lowering spells it once for every backend, so it is the source of that
/// sentence, and the row carries it into the C11 machine whole.
pub const LOWERING: Reason = Reason {
    row: "lowering",
    text: "{0}",
};

/// The sentence a native action raises when its delivery lacks the typed
/// payload it reads, one sentence for every backend, which takes it from here.
pub const ACTION_PAYLOAD_ABSENT: Reason = Reason {
    row: "action_payload_absent",
    text: "<sce:action name='{0}'> needs the typed payload of '{1}', which this delivery did not carry",
};

/// Every reason a C11 machine raises with an error, by the name its raise
/// sites emit it by.
pub const REASONS: &[Reason] = &[
    LOWERING,
    ACTION_PAYLOAD_ABSENT,
    // A `<param>` of any element; `{0}` is the element, `{1}` the param's name.
    Reason {
        row: "param_names_no_location",
        text: "{0} <param name='{1}'> names no location",
    },
    Reason {
        row: "param_failed",
        text: "{0} <param name='{1}'> failed to evaluate",
    },
    Reason {
        row: "param_expr_failed",
        text: "{0} <param name='{1}'> expr failed to evaluate",
    },
    Reason {
        row: "param_location_unreadable",
        text: "{0} <param name='{1}'> location could not be read",
    },
    Reason {
        row: "param_unreadable",
        text: "{0} <param name='{1}'> could not be read",
    },
    Reason {
        row: "param_empty_location",
        text: "{0} <param name='{1}'> has an empty location",
    },
    Reason {
        row: "cond_failed",
        text: "{0} cond failed to evaluate",
    },
    // `<send>`.
    Reason {
        row: "send_part_past_slot",
        text: "<send> {0} does not fit the event's data, so the message was not sent",
    },
    Reason {
        row: "send_payload_past_slot",
        text: "<send>'s payload does not fit the event's data, so the message was not sent",
    },
    Reason {
        row: "send_payload_unserialised",
        text: "<send>'s payload could not be serialised, so the message was not sent",
    },
    Reason {
        row: "send_namelist_unreadable",
        text: "<send> namelist names '{0}', which cannot be read",
    },
    Reason {
        row: "send_content_expr_failed",
        text: "<send> <content expr> failed to evaluate",
    },
    Reason {
        row: "send_content_expr_past_slot",
        text: "<send> <content expr> does not fit the event's data, so the message was not sent",
    },
    Reason {
        row: "send_content_expr_past_request",
        text: "<send> <content expr> does not fit a request's content, so the message was not sent",
    },
    Reason {
        row: "send_content_failed",
        text: "<send> <content> failed to evaluate",
    },
    Reason {
        row: "send_contentexpr_failed",
        text: "<send> contentexpr failed to evaluate",
    },
    Reason {
        row: "http_reply_past_slot",
        text:
            "a BasicHTTP reply's data does not fit the event's data, so the reply was not delivered",
    },
    Reason {
        row: "send_processor_unregistered",
        text: "<send> names a processor the host declared but never registered",
    },
    Reason {
        row: "send_replies_past_room",
        text: "<send> to a host-served processor reported more events than this build reserved \
               room for (SCE_MAX_HOST_RESPONSES)",
    },
    Reason {
        row: "send_delayexpr_unevaluated",
        text: "<send> delayexpr could not be evaluated",
    },
    Reason {
        row: "send_delayexpr_not_time",
        text: "<send> delayexpr is not a CSS2 time",
    },
    Reason {
        row: "send_delay_not_time",
        text: "<send> delay is not a CSS2 time",
    },
    Reason {
        row: "send_attributes_unemitted",
        text: "<send> uses a combination of attributes this backend cannot emit",
    },
    Reason {
        row: "send_idlocation_failed",
        text: "<send idlocation='{0}'> could not take the send id",
    },
    Reason {
        row: "send_typeexpr_unevaluated",
        text: "<send> typeexpr could not be evaluated",
    },
    Reason {
        row: "send_typeexpr_unsupported",
        text: "<send> typeexpr names a processor this platform does not support",
    },
    Reason {
        row: "send_type_unsupported",
        text: "<send type='{0}'> names a processor this platform does not support",
    },
    Reason {
        row: "send_eventexpr_unevaluated",
        text: "<send> eventexpr could not be evaluated to an event name",
    },
    Reason {
        row: "send_targetexpr_unevaluated",
        text: "<send> targetexpr could not be evaluated",
    },
    Reason {
        row: "send_targetexpr_failed",
        text: "<send> targetexpr failed to evaluate",
    },
    Reason {
        row: "send_targetexpr_unaddressable",
        text: "<send> targetexpr produced a target this processor cannot address",
    },
    Reason {
        row: "send_targetexpr_empty",
        text: "<send> targetexpr evaluated to nothing, so there is no target to reach",
    },
    Reason {
        row: "send_targetexpr_parent_absent",
        text: "<send> targetexpr names #_parent, and nothing invoked this session",
    },
    Reason {
        row: "send_targetexpr_unreachable",
        text: "<send> targetexpr names a session this processor cannot reach",
    },
    Reason {
        row: "send_targetexpr_not_target",
        text: "<send> targetexpr produced a value that is not a target",
    },
    Reason {
        row: "send_http_target_not_url",
        text: "<send> over BasicHTTPEventProcessor resolved a target that is not an http(s) URL",
    },
    Reason {
        row: "send_http_target_absent",
        text: "<send> over BasicHTTPEventProcessor has no target to post to",
    },
    Reason {
        row: "send_target_unaddressable",
        text: "<send target='{0}'> is not a target this processor can address",
    },
    Reason {
        row: "send_target_no_invocation",
        text: "<send target='{0}'> names no invocation of this session",
    },
    Reason {
        row: "send_target_unreachable",
        text: "<send target='{0}'> names a session this processor cannot reach",
    },
    Reason {
        row: "send_target_invocation_stopped",
        text: "<send target='{0}'> names an invocation that is not running",
    },
    Reason {
        row: "send_parent_absent",
        text: "<send target='#_parent'> has no parent session to reach",
    },
    Reason {
        row: "delayed_send_parent_absent",
        text: "a delayed <send target='#_parent'> has no parent session to reach",
    },
    Reason {
        row: "delayed_send_invocation_stopped",
        text: "a delayed <send> names an invocation that is not running",
    },
    Reason {
        row: "scheduled_queue_full_send",
        text: "the scheduled-send queue is full, so this <send delay> was dropped",
    },
    Reason {
        row: "scheduled_queue_full_deadline",
        text: "the scheduled-send queue is full, so an <invoke> deadline was dropped",
    },
    // Executable content.
    Reason {
        row: "assign_location_illegal",
        text: "<assign> has an invalid or read-only location '{0}'",
    },
    Reason {
        row: "assign_failed",
        text: "<assign> to '{0}' failed",
    },
    Reason {
        row: "foreach_attribute_missing",
        text: "<foreach> is missing its 'array' or 'item' attribute",
    },
    Reason {
        row: "foreach_not_array",
        text: "<foreach array='{0}'> is not an array",
    },
    Reason {
        row: "log_expr_failed",
        text: "<log> expr failed to evaluate",
    },
    Reason {
        row: "cancel_sendidexpr_failed",
        text: "<cancel> sendidexpr failed to evaluate",
    },
    Reason {
        row: "script_failed",
        text: "<script> failed to execute",
    },
    Reason {
        row: "top_script_failed",
        text: "a top-level <script> failed to execute",
    },
    Reason {
        row: "built_expression_failed",
        text: "a runtime-built expression failed to evaluate",
    },
    // The data model.
    Reason {
        row: "data_unstored",
        text: "<data id='{0}'> could not be stored",
    },
    Reason {
        row: "data_expr_failed",
        text: "<data id='{0}'> expr failed to evaluate",
    },
    Reason {
        row: "system_variable_unbound",
        text: "the system variable {0} could not be bound",
    },
    // An event's data that the machine cannot hold.
    Reason {
        row: "event_data_unallocated",
        text: "the script engine could not allocate storage for an event's data, so the event was \
               refused",
    },
    Reason {
        row: "event_data_past_slot",
        text: "an event's data does not fit this machine's slot of _event.data, so the event was \
               refused",
    },
    // `<invoke>`.
    Reason {
        row: "invoke_idlocation_failed",
        text: "<invoke idlocation='{0}'> could not take the invoke id",
    },
    Reason {
        row: "invoke_namelist_undeclared",
        text: "<invoke> namelist names '{0}', which the parent does not declare",
    },
    Reason {
        row: "invoke_namelist_unreadable",
        text: "<invoke> namelist names '{0}', which could not be read",
    },
    Reason {
        row: "invoke_param_not_request_field",
        text: "<invoke> <param name='{0}'> cannot be its request field: it failed to evaluate",
    },
    Reason {
        row: "invoke_srcexpr_failed",
        text: "<invoke> srcexpr failed to evaluate",
    },
    Reason {
        row: "invoke_content_expr_failed",
        text: "<invoke> content expr failed to evaluate",
    },
    Reason {
        row: "invoke_attribute_expr_failed",
        text: "<invoke> {0}expr failed to evaluate",
    },
    Reason {
        row: "invoke_request_past_slot",
        text: "<invoke>'s request does not fit the event's data, so nothing was started",
    },
    Reason {
        row: "invoke_invoker_unregistered",
        text: "<invoke> names an invoker the host declared but never registered",
    },
    Reason {
        row: "invoke_type_unsupported",
        text: "<invoke type='{0}'> names an external service this platform does not support",
    },
    Reason {
        row: "invoke_child_host_incomplete",
        text: "<invoke id='{0}'> was given no complete host for its child",
    },
    Reason {
        row: "invoke_session_id_past_room",
        text: "<invoke id='{0}'>'s session id does not fit the id this build holds",
    },
    Reason {
        row: "invoke_srcexpr_unevaluated",
        text: "<invoke srcexpr> could not be evaluated",
    },
    Reason {
        row: "invoke_srcexpr_unevaluated_named",
        text: "<invoke srcexpr='{0}'> could not be evaluated",
    },
    Reason {
        row: "invoke_contentexpr_unevaluated",
        text: "<invoke contentexpr='{0}'> could not be evaluated",
    },
    Reason {
        row: "invoke_document_undeclared",
        text: "<invoke srcexpr> evaluated to a document it did not declare",
    },
    Reason {
        row: "invoke_document_undeclared_named",
        text: "<invoke srcexpr='{0}'> evaluated to a document it did not declare",
    },
    Reason {
        row: "invoke_child_ungiven",
        text: "<invoke> could not give the child '{0}'",
    },
    // `<donedata>`.
    Reason {
        row: "donedata_failed",
        text: "<donedata> failed to evaluate",
    },
    Reason {
        row: "donedata_param_failed",
        text: "<donedata> <param> failed to evaluate",
    },
    Reason {
        row: "donedata_content_expr_failed",
        text: "<donedata> <content expr> failed to evaluate",
    },
    Reason {
        row: "donedata_past_slot",
        text: "<donedata> does not fit the event's data, so it was left empty",
    },
];

/// The row of [`REASONS`] named `row`.
pub fn row(row: &str) -> Result<&'static Reason, ReasonError> {
    REASONS
        .iter()
        .find(|reason| reason.row == row)
        .ok_or_else(|| ReasonError::UnknownRow(row.to_string()))
}

impl Reason {
    /// How many holes the text holds: one more than the greatest `{N}`.
    pub fn holes(&self) -> usize {
        let bytes = self.text.as_bytes();
        let mut wanted = 0;
        for (at, byte) in bytes.iter().enumerate() {
            if *byte == b'{' {
                if let Some(digit) = bytes.get(at + 1).filter(|d| d.is_ascii_digit()) {
                    wanted = wanted.max(usize::from(digit - b'0') + 1);
                }
            }
        }
        wanted
    }

    /// The text with each `{N}` replaced by `holes[N]`.
    pub fn fill(&self, holes: &[&str]) -> Result<String, ReasonError> {
        let wanted = self.holes();
        if holes.len() != wanted {
            return Err(ReasonError::Holes {
                row: self.row,
                wanted,
                given: holes.len(),
            });
        }
        let mut out = String::with_capacity(self.text.len());
        let mut rest = self.text;
        while let Some(at) = rest.find('{') {
            out.push_str(&rest[..at]);
            let digit = rest.as_bytes().get(at + 1).copied();
            match digit {
                Some(d) if d.is_ascii_digit() && rest.as_bytes().get(at + 2) == Some(&b'}') => {
                    out.push_str(holes[usize::from(d - b'0')]);
                    rest = &rest[at + 3..];
                }
                _ => {
                    out.push('{');
                    rest = &rest[at + 1..];
                }
            }
        }
        out.push_str(rest);
        Ok(out)
    }

    /// The C string literal of the text filled with `holes`, quotes included,
    /// its length recorded in the book of the machine being generated.
    pub fn literal(&self, holes: &[&str]) -> Result<String, ReasonError> {
        let text = self.fill(holes)?;
        record(self.row, text.len())?;
        Ok(format!("\"{}\"", crate::filters::escape_c(text)))
    }
}

/// The longest reason the machine being generated has emitted so far, in
/// bytes, and whether its slot was computed already.
#[derive(Debug, Default, Clone, Copy)]
struct Book {
    longest: usize,
    sealed: bool,
}

thread_local! {
    static BOOK: RefCell<Option<Book>> = const { RefCell::new(None) };
}

fn record(row: &'static str, len: usize) -> Result<(), ReasonError> {
    BOOK.with(|book| match book.borrow_mut().as_mut() {
        Some(book) if book.sealed => Err(ReasonError::Sealed(row)),
        Some(book) => {
            book.longest = book.longest.max(len);
            Ok(())
        }
        // Outside a machine's generation (a unit test of a lowering) there is
        // no slot to size.
        None => Ok(()),
    })
}

/// The book of one machine's generation on this thread, until the guard is
/// dropped, when the book it replaced is put back.
pub struct Scope {
    before: Option<Option<Book>>,
}

impl Drop for Scope {
    fn drop(&mut self) {
        if let Some(before) = self.before.take() {
            BOOK.with(|book| *book.borrow_mut() = before);
        }
    }
}

/// Open an empty book for the machine this thread generates next.
pub fn scope() -> Scope {
    let before = BOOK.with(|book| book.replace(Some(Book::default())));
    Scope {
        before: Some(before),
    }
}

/// Close the book of the machine being generated and answer its longest
/// reason in bytes, the input of its slot (docs/adr/0086, section 8.6 (b)); a
/// reason emitted after this is refused ([`ReasonError::Sealed`]). Zero
/// outside a scope.
pub fn seal() -> u64 {
    BOOK.with(|book| match book.borrow_mut().as_mut() {
        Some(book) => {
            book.sealed = true;
            book.longest as u64
        }
        None => 0,
    })
}

/// The template function `reason(row, holes...)`: the C string literal of the
/// row's text, its holes filled with the document's text the site hands.
pub fn template_function(
    name: String,
    holes: minijinja::value::Rest<minijinja::Value>,
) -> Result<String, minijinja::Error> {
    let refuse =
        |message: String| minijinja::Error::new(minijinja::ErrorKind::InvalidOperation, message);
    let mut texts = Vec::with_capacity(holes.len());
    for hole in holes.iter() {
        if hole.is_undefined() || hole.is_none() {
            return Err(refuse(format!(
                "the reason `{name}` was handed an undefined hole"
            )));
        }
        texts.push(match hole.as_str() {
            Some(text) => text.to_string(),
            None => hole.to_string(),
        });
    }
    let filled: Vec<&str> = texts.iter().map(String::as_str).collect();
    row(&name)
        .and_then(|reason| reason.literal(&filled))
        .map_err(|error| refuse(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_row_is_named_once_and_says_something_no_other_row_says() {
        let mut names = std::collections::BTreeSet::new();
        let mut texts = std::collections::BTreeSet::new();
        for reason in REASONS {
            assert!(names.insert(reason.row), "`{}` is named twice", reason.row);
            assert!(texts.insert(reason.text), "`{}` repeats a text", reason.row);
        }
    }

    #[test]
    fn every_brace_of_a_text_opens_a_hole_and_the_holes_are_numbered_from_zero() {
        for reason in REASONS {
            let holes: Vec<&str> = (0..reason.holes()).map(|_| "x").collect();
            let filled = reason.fill(&holes).expect("the row fills");
            assert!(
                !filled.contains('{') && !filled.contains('}'),
                "`{}` leaves a brace that opens no hole: {filled}",
                reason.row
            );
            for n in 0..reason.holes() {
                assert!(
                    reason.text.contains(&format!("{{{n}}}")),
                    "`{}` skips the hole {n}",
                    reason.row
                );
            }
        }
    }

    #[test]
    fn a_reason_fills_its_holes_with_the_text_it_is_handed() {
        let reason = row("param_names_no_location").expect("the row");
        assert_eq!(
            reason.fill(&["<send>", "a\"b"]).expect("two holes"),
            "<send> <param name='a\"b'> names no location"
        );
        assert_eq!(
            reason.literal(&["<send>", "a\"b"]).expect("two holes"),
            "\"<send> <param name='a\\\"b'> names no location\""
        );
        assert_eq!(
            reason.fill(&["<send>"]),
            Err(ReasonError::Holes {
                row: "param_names_no_location",
                wanted: 2,
                given: 1
            })
        );
        assert!(matches!(
            row("no_such_row"),
            Err(ReasonError::UnknownRow(_))
        ));
    }

    #[test]
    fn the_book_keeps_the_longest_reason_until_it_is_sealed() {
        let _scope = scope();
        LOWERING.literal(&["four"]).expect("one hole");
        LOWERING.literal(&["sixteen bytes..."]).expect("one hole");
        LOWERING.literal(&["two"]).expect("one hole");
        assert_eq!(seal(), 16, "the longest reason is the one of 16 bytes");
        assert_eq!(
            LOWERING.literal(&["late"]),
            Err(ReasonError::Sealed("lowering")),
            "a reason emitted after the slot was computed is refused"
        );
    }

    #[test]
    fn a_scope_inside_another_keeps_its_own_book_and_gives_the_outer_back() {
        let _outer = scope();
        LOWERING.literal(&["outer"]).expect("one hole");
        {
            let _inner = scope();
            LOWERING
                .literal(&["the inner machine's reason"])
                .expect("one hole");
            assert_eq!(seal(), 26);
        }
        assert_eq!(seal(), 5, "the outer book holds the outer reason alone");
    }
}
