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
//! **A reason that quotes a value of the run.** Some reasons quote a text the
//! machine only has when it runs: the sentence a reader of the runtime refused
//! a payload with, the text a host refused an invocation with. Such a row
//! holds a quote, `{v}`, beside its holes, and every quote carries a bound the
//! build knows ([`Quote`]): a bound the runtime publishes for the sentences it
//! answers with ([`Published`]), which the machine checks against the header
//! it compiles with (`reason_bounds_checked`), or a bound the document's own
//! text gives, which the C library holds by the precision it is spelled with.
//! The reason is measured as its row's text plus each quote's bound, so the
//! slot holds the longest it can be, and a value with no bound has no quote to
//! carry it: it cannot reach a reason.
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
    /// The site handed a number of quoted values other than the row's
    /// quotes, or asked for a row that quotes as a plain literal.
    Quotes {
        row: &'static str,
        wanted: usize,
        given: usize,
    },
    /// A quote named a bound the runtime does not publish.
    UnknownBound(String),
    /// The reason quotes a bound of the runtime after the machine checked the
    /// bounds it relies on ([`bounds_checked`]), so nothing would check it.
    Unchecked(&'static str),
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
            ReasonError::Quotes { row, wanted, given } => write!(
                f,
                "the reason `{row}` quotes {wanted} value(s) of the run, and the site handed \
                 {given}"
            ),
            ReasonError::UnknownBound(name) => write!(
                f,
                "no bound the C11 runtime publishes is named `{name}`, so a value quoted by it \
                 has no bound the build knows"
            ),
            ReasonError::Unchecked(row) => write!(
                f,
                "the reason `{row}` quotes a bound of the runtime after the machine checked the \
                 bounds it relies on, so nothing would check this one"
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

/// The reason a host-run invocation's start raises when the host refused it:
/// the host's own text, whole, which the host ABI bounds
/// ([`HOST_INVOKE_REFUSAL`]).
pub const HOST_INVOKE_REFUSED: Reason = Reason {
    row: "host_invoke_refused",
    text: "{v}",
};

/// A bound the C11 runtime publishes for the sentences it answers a machine
/// with, or for the data a host may hand one ([`UNTYPED_EVENT_DATA`]): the
/// macro its header defines (`name`), the most bytes a sentence or the data
/// holds, and the name a template quotes it by (`key`). The build sizes a
/// reason that quotes such a sentence, or the slot that holds such data, by
/// `max_len`, and the machine asserts that the header it compiles with
/// publishes no more ([`template_bounds_checked`]), so a header that grew a
/// longer sentence fails the machine's build rather than overflowing its slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Published {
    pub key: &'static str,
    pub name: &'static str,
    pub max_len: u64,
}

/// The longest sentence a reader of `sce/event_payload.h` refuses a payload
/// with.
pub const PAYLOAD_REFUSAL: Published = Published {
    key: "payload_refusal",
    name: "SCE_PAYLOAD_REFUSAL_MAX_LEN",
    max_len: 120,
};

/// The longest sentence `sce/request_field.h` refuses a request field with,
/// a payload reader's sentence included.
pub const REQUEST_REFUSAL: Published = Published {
    key: "request_refusal",
    name: "SCE_REQUEST_REFUSAL_MAX_LEN",
    max_len: 120,
};

/// The longest text a host may refuse a host-run invocation's start with
/// (`sce/host_processor.h`): `sce_host_invoke_answer_refusal` refuses a longer
/// one whole.
pub const HOST_INVOKE_REFUSAL: Published = Published {
    key: "host_invoke_refusal",
    name: "SCE_HOST_INVOKE_REFUSAL_MAX_LEN",
    max_len: 255,
};

/// Every bound the C11 runtime publishes for a sentence a reason quotes.
pub const PUBLISHED: &[Published] = &[PAYLOAD_REFUSAL, REQUEST_REFUSAL, HOST_INVOKE_REFUSAL];

/// The most bytes of the data an event may carry into a machine with no script
/// engine when the machine's document declares no schema for the event
/// (`sce/types.h`; docs/adr/0086, section 8.6 (c)): an event a host hands it by
/// name, and the end of a host-run invocation that declares no result. No
/// reason quotes it, so it is not among [`PUBLISHED`]; the machine's slot is
/// sized by it, and the machine checks it as it checks a bound its reasons
/// quote ([`hold_bound`]).
pub const UNTYPED_EVENT_DATA: Published = Published {
    key: "untyped_event_data",
    name: "SCE_UNTYPED_EVENT_DATA_MAX_LEN",
    max_len: 255,
};

/// The published bound a template quotes by `key`.
pub fn published(key: &str) -> Result<Published, ReasonError> {
    PUBLISHED
        .iter()
        .copied()
        .find(|bound| bound.key == key)
        .ok_or_else(|| ReasonError::UnknownBound(key.to_string()))
}

/// The bound of one value of the run a reason quotes (`{v}` in its row).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quote {
    /// A sentence of the runtime, at most the bound it publishes; spelled
    /// `%s`, and checked by the machine against its header.
    Published(Published),
    /// A text the document bounds at `n` bytes (the longest of the names it
    /// can be); spelled `%.<n>s`, so the C library holds the bound whatever
    /// path reaches the reason, and no text the document drives is cut.
    Within(u64),
}

impl Quote {
    /// The most bytes the quoted value adds to the reason.
    pub fn bound(self) -> u64 {
        match self {
            Quote::Published(bound) => bound.max_len,
            Quote::Within(n) => n,
        }
    }

    /// The `printf` conversion the value is written by.
    fn conversion(self) -> String {
        match self {
            Quote::Published(_) => "%s".to_string(),
            Quote::Within(n) => format!("%.{n}s"),
        }
    }
}

/// Every reason a C11 machine raises with an error, by the name its raise
/// sites emit it by.
pub const REASONS: &[Reason] = &[
    LOWERING,
    ACTION_PAYLOAD_ABSENT,
    HOST_INVOKE_REFUSED,
    // A typed payload's lift at the dequeue; `{0}` is the event, `{1}` the
    // field, and the quote the payload reader's sentence.
    Reason {
        row: "lift_payload_refused",
        text: "`{0}` payload: {v}",
    },
    Reason {
        row: "lift_field_refused",
        text: "`{0}` payload: '{1}' {v}",
    },
    // `{2}` is the enum's alias, and the quote the name the delivery gave.
    Reason {
        row: "lift_not_variant",
        text: "`{0}` payload: '{1}' ({v}) is not a variant of {2}",
    },
    // A composed payload's lift (docs/adr/0086, section 4.2): the first quote
    // is the path to the refused place, at the bound of the longest path its
    // type has, and the second the payload reader's sentence.
    Reason {
        row: "lift_place_refused",
        text: "`{0}` payload: '{v}' {v}",
    },
    // A typed host-run request's field; the quote is the request reader's
    // sentence.
    Reason {
        row: "invoke_param_request_field_refused",
        text: "<invoke> <param name='{0}'> cannot be its request field: '{0}' {v}",
    },
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
    // A host-run invocation's deadline the machine cannot read, raised by the
    // one start path every invocation shares: the quotes are the invocation's
    // type and id, the document's texts, and the deadline's text, at the bound
    // of the expression it was written as (section 8.6 (c)).
    Reason {
        row: "invoke_deadline_unreadable",
        text: "<invoke type='{v}' id='{v}'>: _sce_deadline_ms='{v}' is not a whole number of \
               milliseconds",
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

    /// How many values of the run the text quotes: its `{v}`s.
    pub fn quotes(&self) -> usize {
        self.text.matches(QUOTE).count()
    }

    /// The C string literal of the text filled with `holes`, quotes included,
    /// its length recorded in the book of the machine being generated. A row
    /// that quotes a value of the run is no literal ([`Self::format`]).
    pub fn literal(&self, holes: &[&str]) -> Result<String, ReasonError> {
        self.check_quotes(0)?;
        let text = self.fill(holes)?;
        record(self.row, text.len())?;
        Ok(format!("\"{}\"", crate::filters::escape_c(text)))
    }

    /// The C string literal of the `printf` format of the text: each hole
    /// filled with `holes[N]` (a `%` of it doubled, so it is text and not a
    /// conversion), and each quote, in order, the conversion of `quotes`'
    /// bound. The reason is recorded at its longest, the text plus each
    /// quote's bound, and a bound of the runtime as one the machine relies on.
    pub fn format(&self, holes: &[&str], quotes: &[Quote]) -> Result<String, ReasonError> {
        let (format, longest) = self.compose(holes, quotes)?;
        self.hold_composed(quotes, longest)?;
        Ok(format!("\"{}\"", crate::filters::escape_c(format)))
    }

    /// Record the reason as [`Self::format`] does, for a site that hands the
    /// quoted value on as it is (a row whose text is one quote).
    pub fn hold(&self, holes: &[&str], quotes: &[Quote]) -> Result<(), ReasonError> {
        let (_, longest) = self.compose(holes, quotes)?;
        self.hold_composed(quotes, longest)
    }

    fn check_quotes(&self, given: usize) -> Result<(), ReasonError> {
        let wanted = self.quotes();
        if given != wanted {
            return Err(ReasonError::Quotes {
                row: self.row,
                wanted,
                given,
            });
        }
        Ok(())
    }

    /// The `printf` format of the text and the most bytes it writes, in one
    /// pass over the row, so a hole's text that spells `{v}` stays text.
    fn compose(&self, holes: &[&str], quotes: &[Quote]) -> Result<(String, usize), ReasonError> {
        self.check_quotes(quotes.len())?;
        let wanted = self.holes();
        if holes.len() != wanted {
            return Err(ReasonError::Holes {
                row: self.row,
                wanted,
                given: holes.len(),
            });
        }
        let mut format = String::with_capacity(self.text.len());
        let mut longest = 0usize;
        let mut quoted = quotes.iter();
        let mut rest = self.text;
        loop {
            let Some(at) = rest.find('{') else {
                format.push_str(&rest.replace('%', "%%"));
                longest += rest.len();
                break;
            };
            format.push_str(&rest[..at].replace('%', "%%"));
            longest += at;
            let after = &rest[at..];
            let digit = after.as_bytes().get(1).copied();
            if let Some(d) =
                digit.filter(|d| d.is_ascii_digit() && after.as_bytes().get(2) == Some(&b'}'))
            {
                let hole = holes[usize::from(d - b'0')];
                format.push_str(&hole.replace('%', "%%"));
                longest += hole.len();
                rest = &after[3..];
            } else if let Some(tail) = after.strip_prefix(QUOTE) {
                let quote = quoted
                    .next()
                    .expect("the quotes were counted against the row's");
                format.push_str(&quote.conversion());
                longest =
                    longest.saturating_add(usize::try_from(quote.bound()).unwrap_or(usize::MAX));
                rest = tail;
            } else {
                format.push('{');
                longest += 1;
                rest = &after[1..];
            }
        }
        Ok((format, longest))
    }

    fn hold_composed(&self, quotes: &[Quote], longest: usize) -> Result<(), ReasonError> {
        for quote in quotes {
            if let Quote::Published(bound) = quote {
                rely(self.row, *bound)?;
            }
        }
        record(self.row, longest)
    }
}

/// How a row's text spells a value of the run it quotes.
const QUOTE: &str = "{v}";

/// The longest reason the machine being generated has emitted so far, in
/// bytes, whether its slot was computed already, the bounds of the runtime
/// its reasons quote, and whether the machine has checked those already.
#[derive(Debug, Default, Clone)]
struct Book {
    longest: usize,
    sealed: bool,
    relied: Vec<Published>,
    checked: bool,
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

fn rely(row: &'static str, bound: Published) -> Result<(), ReasonError> {
    BOOK.with(|book| match book.borrow_mut().as_mut() {
        Some(book) if book.checked => Err(ReasonError::Unchecked(row)),
        Some(book) => {
            if !book.relied.contains(&bound) {
                book.relied.push(bound);
            }
            Ok(())
        }
        None => Ok(()),
    })
}

/// Take `bound`, a bound of the runtime the slot of the machine being
/// generated is sized by, as one the machine checks against the header it
/// compiles with ([`template_bounds_checked`]), as a reason that quotes it
/// would. Refused once the machine checked its bounds, as such a quote is
/// ([`ReasonError::Unchecked`]).
pub fn hold_bound(bound: Published) -> Result<(), ReasonError> {
    rely(bound.key, bound)
}

/// The bounds of the runtime the reasons of the machine being generated
/// quote, and those its slot is sized by ([`hold_bound`]), each once, in a
/// fixed order; the book is closed to another one, so a reason that quotes a
/// bound after the check is refused ([`ReasonError::Unchecked`]). None
/// outside a scope.
pub fn bounds_checked() -> Vec<Published> {
    BOOK.with(|book| match book.borrow_mut().as_mut() {
        Some(book) => {
            book.checked = true;
            let mut relied = book.relied.clone();
            relied.sort();
            relied
        }
        None => Vec::new(),
    })
}

/// The template function `reason_bounds_checked()`, called once at the end of
/// a machine's source, after every reason it emits: one static assertion per
/// bound of the runtime the machine's reasons quote or its slot is sized by,
/// that the header the machine compiles with publishes no more than the bound
/// its slot was sized by. Empty when it relies on none.
pub fn template_bounds_checked() -> String {
    bounds_checked()
        .iter()
        .map(|bound| {
            format!(
                "SCE_STATIC_ASSERT((size_t)({name}) <= (size_t){max}u,\n                  \
                 \"{name} is past the {max} bytes this machine's slot of _event.data was sized \
                 by: the runtime and the generator that wrote the machine disagree\");\n",
                name = bound.name,
                max = bound.max_len
            )
        })
        .collect()
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
    let texts = hole_texts(&name, &holes)?;
    let filled: Vec<&str> = texts.iter().map(String::as_str).collect();
    row(&name)
        .and_then(|reason| reason.literal(&filled))
        .map_err(|error| refuse(error.to_string()))
}

/// The template function `reason_format(row, quotes, holes...)`: the C string
/// literal of the `printf` format of a row that quotes values of the run
/// ([`Reason::format`]). Each entry of `quotes` is the name of a bound the
/// runtime publishes ([`PUBLISHED`]) or a number of bytes the document bounds
/// the value at ([`Quote::Within`]).
pub fn template_format_function(
    name: String,
    quotes: Vec<minijinja::Value>,
    holes: minijinja::value::Rest<minijinja::Value>,
) -> Result<String, minijinja::Error> {
    let texts = hole_texts(&name, &holes)?;
    let filled: Vec<&str> = texts.iter().map(String::as_str).collect();
    let mut bounds = Vec::with_capacity(quotes.len());
    for quote in &quotes {
        let bound = match (quote.as_str(), quote.as_usize()) {
            (Some(key), _) => published(key).map(Quote::Published),
            (None, Some(n)) => Ok(Quote::Within(n as u64)),
            (None, None) => Err(ReasonError::UnknownBound(quote.to_string())),
        };
        bounds.push(bound.map_err(|error| refuse(error.to_string()))?);
    }
    row(&name)
        .and_then(|reason| reason.format(&filled, &bounds))
        .map_err(|error| refuse(error.to_string()))
}

fn refuse(message: String) -> minijinja::Error {
    minijinja::Error::new(minijinja::ErrorKind::InvalidOperation, message)
}

/// The text of each hole a template hands the reason `name`.
fn hole_texts(
    name: &str,
    holes: &minijinja::value::Rest<minijinja::Value>,
) -> Result<Vec<String>, minijinja::Error> {
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
    Ok(texts)
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
            let filled = reason
                .fill(&holes)
                .expect("the row fills")
                .replace(QUOTE, "");
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

    /// A reason that quotes a value of the run is measured at its text plus
    /// each quote's bound, and spelled as a `printf` format whose holes are
    /// text: a `%` a document wrote is doubled, and a hole that spells `{v}`
    /// is no quote.
    #[test]
    fn a_quoting_reason_is_measured_at_its_text_plus_each_bound() {
        let _scope = scope();
        let reason = row("lift_field_refused").expect("the row");
        assert_eq!(reason.quotes(), 1);
        let format = reason
            .format(&["a%b", "{v}"], &[Quote::Published(PAYLOAD_REFUSAL)])
            .expect("two holes and one quote");
        assert_eq!(format, "\"`a%%b` payload: '{v}' %s\"");
        // "`a%b` payload: '{v}' " is 21 bytes, and the sentence at most 120.
        assert_eq!(seal(), 21 + PAYLOAD_REFUSAL.max_len);

        let _scope = scope();
        let variant = row("lift_not_variant").expect("the row");
        assert_eq!(
            variant
                .format(&["e", "f", "Mode"], &[Quote::Within(7)])
                .expect("three holes and one quote"),
            "\"`e` payload: 'f' (%.7s) is not a variant of Mode\""
        );
        assert_eq!(
            seal(),
            "`e` payload: 'f' () is not a variant of Mode".len() as u64 + 7
        );
    }

    /// docs/adr/0086, section 8.6 (c): the reason a host-run invocation's
    /// unreadable deadline is raised with quotes three texts, the type, the id
    /// and the deadline's own, each at the bound the build knows of it, and is
    /// measured at its row's text plus their sum.
    #[test]
    fn the_unreadable_deadline_is_measured_at_the_bounds_of_its_three_texts() {
        let _scope = scope();
        let reason = row("invoke_deadline_unreadable").expect("the row");
        assert_eq!(reason.quotes(), 3);
        let format = reason
            .format(
                &[],
                &[Quote::Within(10), Quote::Within(3), Quote::Within(20)],
            )
            .expect("three quotes");
        assert_eq!(
            format,
            "\"<invoke type='%.10s' id='%.3s'>: _sce_deadline_ms='%.20s' is not a whole number \
             of milliseconds\""
        );
        let text =
            "<invoke type='' id=''>: _sce_deadline_ms='' is not a whole number of milliseconds";
        assert_eq!(seal(), text.len() as u64 + 10 + 3 + 20);
    }

    /// docs/adr/0086, section 4.2: the reason a composed payload's lift raises
    /// quotes the path to the refused place at the bound of the longest path
    /// its type has, and the reader's sentence at the bound the runtime
    /// publishes, so the slot holds it at its longest and the path is never
    /// cut.
    #[test]
    fn a_composed_lift_quotes_its_path_and_the_readers_sentence_at_their_bounds() {
        let _scope = scope();
        let format = row("lift_place_refused")
            .expect("the row")
            .format(
                &["batch.sent"],
                &[Quote::Within(14), Quote::Published(PAYLOAD_REFUSAL)],
            )
            .expect("two quotes");
        assert_eq!(format, "\"`batch.sent` payload: '%.14s' %s\"");
        let text = "`batch.sent` payload: '' ";
        assert_eq!(seal(), text.len() as u64 + 14 + PAYLOAD_REFUSAL.max_len);
    }

    /// A quoting row is no plain literal, and a site that hands it a number
    /// of quotes other than its own is refused.
    #[test]
    fn a_quoting_row_takes_exactly_its_quotes() {
        let reason = row("lift_payload_refused").expect("the row");
        assert_eq!(
            reason.literal(&["e"]),
            Err(ReasonError::Quotes {
                row: "lift_payload_refused",
                wanted: 1,
                given: 0
            })
        );
        assert_eq!(
            reason.format(&["e"], &[]),
            Err(ReasonError::Quotes {
                row: "lift_payload_refused",
                wanted: 1,
                given: 0
            })
        );
        assert!(matches!(
            published("no_such_bound"),
            Err(ReasonError::UnknownBound(_))
        ));
    }

    /// Each bound of the runtime a machine's reasons quote is checked once,
    /// and a quote of one after the check is refused.
    #[test]
    fn every_bound_a_reason_quotes_is_checked_once() {
        let _scope = scope();
        let field = row("lift_field_refused").expect("the row");
        let whole = row("lift_payload_refused").expect("the row");
        field
            .format(&["e", "f"], &[Quote::Published(PAYLOAD_REFUSAL)])
            .expect("a quote");
        whole
            .format(&["e"], &[Quote::Published(PAYLOAD_REFUSAL)])
            .expect("a quote");
        HOST_INVOKE_REFUSED
            .hold(&[], &[Quote::Published(HOST_INVOKE_REFUSAL)])
            .expect("a quote");
        let checks = template_bounds_checked();
        assert_eq!(checks.matches("SCE_STATIC_ASSERT").count(), 2, "{checks}");
        assert!(checks.contains("(size_t)(SCE_PAYLOAD_REFUSAL_MAX_LEN) <= (size_t)120u"));
        assert!(checks.contains("(size_t)(SCE_HOST_INVOKE_REFUSAL_MAX_LEN) <= (size_t)255u"));
        assert_eq!(
            whole.format(&["e"], &[Quote::Published(PAYLOAD_REFUSAL)]),
            Err(ReasonError::Unchecked("lift_payload_refused"))
        );
    }

    /// docs/adr/0086, section 8.6 (c): a bound of the runtime the slot is
    /// sized by, which no reason quotes, is checked once as a quoted one is,
    /// and held after the check is refused.
    #[test]
    fn an_untyped_data_bound_is_checked_as_a_quoted_one_is() {
        let _scope = scope();
        hold_bound(UNTYPED_EVENT_DATA).expect("held before the check");
        hold_bound(UNTYPED_EVENT_DATA).expect("held twice, checked once");
        let checks = template_bounds_checked();
        assert_eq!(checks.matches("SCE_STATIC_ASSERT").count(), 1, "{checks}");
        assert!(checks.contains("(size_t)(SCE_UNTYPED_EVENT_DATA_MAX_LEN) <= (size_t)255u"));
        assert_eq!(
            hold_bound(UNTYPED_EVENT_DATA),
            Err(ReasonError::Unchecked("untyped_event_data"))
        );
        assert!(
            published(UNTYPED_EVENT_DATA.key).is_err(),
            "no reason quotes the data a host hands"
        );
    }

    /// The value of `#define <name> <value>` in `header`, a further macro's
    /// name resolved through the same header, the `u` suffix dropped.
    fn defined(header: &str, name: &str) -> u64 {
        let line = header
            .lines()
            .map(str::trim_start)
            .find(|line| {
                line.strip_prefix("#define ")
                    .and_then(|rest| rest.split_whitespace().next())
                    == Some(name)
            })
            .unwrap_or_else(|| panic!("the header defines {name}"));
        let value = line
            .split_whitespace()
            .nth(2)
            .unwrap_or_else(|| panic!("{name} has a value"));
        match value.trim_end_matches('u').parse() {
            Ok(number) => number,
            Err(_) => defined(header, value),
        }
    }

    /// Every C string literal in `text`, its adjacent pieces joined.
    fn literal_texts(text: &str) -> String {
        let mut out = String::new();
        let mut inside = false;
        let mut chars = text.chars();
        while let Some(ch) = chars.next() {
            match (inside, ch) {
                (false, '"') => inside = true,
                (true, '"') => inside = false,
                (true, '\\') => out.extend(chars.next()),
                (true, other) => out.push(other),
                (false, _) => {}
            }
        }
        out
    }

    /// The bound each header publishes is the one the build sizes by, and
    /// every sentence the header can answer with fits it, so a reason that
    /// quotes one is never longer than its measure.
    #[test]
    fn each_published_bound_is_the_headers_and_holds_its_sentences() {
        const PAYLOAD: &str =
            include_str!("../../../backends/c/runtime/include/sce/event_payload.h");
        const REQUEST: &str =
            include_str!("../../../backends/c/runtime/include/sce/request_field.h");
        const HOST: &str = include_str!("../../../backends/c/runtime/include/sce/host_processor.h");
        const TYPES: &str = include_str!("../../../backends/c/runtime/include/sce/types.h");
        assert_eq!(
            defined(TYPES, UNTYPED_EVENT_DATA.name),
            UNTYPED_EVENT_DATA.max_len
        );
        assert_eq!(
            defined(PAYLOAD, PAYLOAD_REFUSAL.name),
            PAYLOAD_REFUSAL.max_len
        );
        let request_header = format!("{PAYLOAD}\n{REQUEST}");
        assert_eq!(
            defined(&request_header, REQUEST_REFUSAL.name),
            REQUEST_REFUSAL.max_len
        );
        assert_eq!(
            defined(HOST, HOST_INVOKE_REFUSAL.name),
            HOST_INVOKE_REFUSAL.max_len
        );

        let mut sentences = 0;
        for piece in PAYLOAD
            .split("static const char sce_payload_refuse_")
            .skip(1)
        {
            let (name, rest) = piece.split_once("[]").expect("a sentence is an array");
            let (value, _) = rest.split_once(';').expect("a sentence ends");
            let sentence = literal_texts(value);
            assert!(
                sentence.len() as u64 <= PAYLOAD_REFUSAL.max_len,
                "sce_payload_refuse_{name} is {} bytes, past {}",
                sentence.len(),
                PAYLOAD_REFUSAL.max_len
            );
            sentences += 1;
        }
        assert!(sentences >= 30, "the payload reader's sentences were read");
        let mut own = 0;
        for line in REQUEST
            .lines()
            .filter(|l| l.starts_with("#define SCE_REQUEST_REFUSE_"))
        {
            let sentence = literal_texts(line);
            assert!(sentence.len() as u64 <= REQUEST_REFUSAL.max_len, "{line}");
            own += usize::from(!sentence.is_empty());
        }
        assert!(own >= 4, "the request reader's own sentences were read");
    }
}
